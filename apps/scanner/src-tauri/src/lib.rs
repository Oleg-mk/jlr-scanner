//! Tauri composition root for JLR Scanner.

mod adapter_service;
mod capture_service;
mod diagnostic_service;
mod module_read_service;
mod session_report_service;
mod session_service;

use adapter_service::{lock_service, shared_service, SharedAdapterService};
use app_contracts::{
    AdapterSnapshot, AdapterState, CaptureSnapshot, DiagnosticSnapshot, LibrarySnapshot,
    ModuleReadRequest, ModuleReadSnapshot, SessionReportSnapshot, VehicleCatalogueSnapshot,
    VehicleContextInput, VehicleSurveySnapshot, VinDecodeSnapshot,
};
use bench_vehicle::BenchVehicle;
use capture_service::CaptureService;
use diagnostic_service::DiagnosticService;
use module_read_service::{adapter_unavailable, map_live_error, ModuleReadService};
use mongoose_jlr::bench::{share_bus, SharedBenchBus};
use mongoose_jlr::VehicleRouteId;
use session_report_service::SessionReportService;
use session_report_service::{SESSION_MODE_BENCH, SESSION_MODE_REAL};
use session_service::SessionService;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;
use tauri::State;
use transport_api::{BenchBus, EmptyBench};

type SharedDiagnosticService = Mutex<DiagnosticService>;
type SharedSessionService = Mutex<SessionService>;
type SharedCaptureService = Mutex<CaptureService>;
type SharedModuleReadService = Mutex<ModuleReadService>;
type SharedSessionReportService = Mutex<SessionReportService>;
/// The vehicle on the bench (ADR-0020), shared between the bench transport
/// and the session that describes it.
type SharedBench = SharedBenchBus;

/// Put the vehicle the session last described on the bench, or nothing.
fn refresh_bench(bench: &SharedBench, session: &SessionService) {
    let vehicle: Box<dyn BenchBus> = match session.last_context() {
        Some(context) => Box::new(BenchVehicle::from_library(
            session.library(),
            &context,
            None,
        )),
        None => Box::new(EmptyBench),
    };
    *bench
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = vehicle;
}

fn lock_module_read<'a>(
    state: &'a State<'a, SharedModuleReadService>,
) -> MutexGuard<'a, ModuleReadService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_capture<'a>(state: &'a State<'a, SharedCaptureService>) -> MutexGuard<'a, CaptureService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_session_report<'a>(
    state: &'a State<'a, SharedSessionReportService>,
) -> MutexGuard<'a, SessionReportService> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_session<'a>(state: &'a State<'a, SharedSessionService>) -> MutexGuard<'a, SessionService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_diagnostic<'a>(
    state: &'a State<'a, SharedDiagnosticService>,
) -> MutexGuard<'a, DiagnosticService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[tauri::command]
fn get_adapter_state(state: State<'_, SharedAdapterService>) -> AdapterSnapshot {
    lock_service(&state).snapshot()
}

#[tauri::command]
fn discover_adapters(state: State<'_, SharedAdapterService>) -> AdapterSnapshot {
    lock_service(&state).discover()
}

#[tauri::command]
fn connect_adapter(
    state: State<'_, SharedAdapterService>,
    report_state: State<'_, SharedSessionReportService>,
    port: Option<String>,
) -> AdapterSnapshot {
    let mut service = lock_service(&state);
    if !lock_session_report(&report_state).accepts(SESSION_MODE_REAL) {
        return service.refuse_session_switch(SESSION_MODE_REAL);
    }
    let snapshot = service.connect(port);
    if snapshot.state == AdapterState::Connected {
        lock_session_report(&report_state).set_mode(SESSION_MODE_REAL);
    }
    snapshot
}

/// Connect the bench (ADR-0020): the vehicle the session describes, behind
/// a stand-in adapter. A session that already holds real records refuses.
#[tauri::command]
fn connect_bench(
    state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    report_state: State<'_, SharedSessionReportService>,
    bench: State<'_, SharedBench>,
) -> AdapterSnapshot {
    let mut service = lock_service(&state);
    if !lock_session_report(&report_state).accepts(SESSION_MODE_BENCH) {
        return service.refuse_session_switch(SESSION_MODE_BENCH);
    }
    refresh_bench(&bench, &lock_session(&session_state));
    let snapshot = service.connect_bench();
    if snapshot.state == AdapterState::Connected {
        lock_session_report(&report_state).set_mode(SESSION_MODE_BENCH);
    }
    snapshot
}

#[tauri::command]
fn disconnect_adapter(state: State<'_, SharedAdapterService>) -> AdapterSnapshot {
    lock_service(&state).disconnect()
}

#[tauri::command]
fn get_diagnostic_state(
    adapter_state: State<'_, SharedAdapterService>,
    diagnostic_state: State<'_, SharedDiagnosticService>,
) -> DiagnosticSnapshot {
    let adapter_ready = lock_service(&adapter_state).connected_adapter().is_some();
    lock_diagnostic(&diagnostic_state).snapshot(adapter_ready)
}

#[tauri::command]
fn read_calibration_identification(
    adapter_state: State<'_, SharedAdapterService>,
    diagnostic_state: State<'_, SharedDiagnosticService>,
    report_state: State<'_, SharedSessionReportService>,
) -> DiagnosticSnapshot {
    let transaction = {
        let mut diagnostic = lock_diagnostic(&diagnostic_state);
        diagnostic.begin();
        diagnostic.transaction()
    };

    let (adapter, result, bench) = {
        let mut service = lock_service(&adapter_state);
        let Some(adapter) = service.connected_adapter() else {
            return lock_diagnostic(&diagnostic_state).finish_adapter_unavailable();
        };
        let Some(result) =
            service.execute_calibration_identification(&transaction, Duration::from_secs(3))
        else {
            return lock_diagnostic(&diagnostic_state).finish_adapter_unavailable();
        };
        (adapter, result, service.is_bench())
    };

    let (snapshot, report) = {
        let mut diagnostic = lock_diagnostic(&diagnostic_state);
        let snapshot = if bench {
            diagnostic.finish_bench(&adapter, result)
        } else {
            diagnostic.finish_live(&adapter, result)
        };
        (snapshot, diagnostic.report_json())
    };
    if let Ok(json) = report {
        let mut session_report = lock_session_report(&report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_calibration_read(&json);
    }
    snapshot
}

#[tauri::command]
fn get_diagnostic_report_json(
    diagnostic_state: State<'_, SharedDiagnosticService>,
) -> Result<String, String> {
    lock_diagnostic(&diagnostic_state).report_json()
}

// F10 composition root: the data library and the vehicle survey. These read
// knowledge and compute plans; they open no transport and transmit nothing.

#[tauri::command]
fn get_data_library(state: State<'_, SharedSessionService>) -> LibrarySnapshot {
    lock_session(&state).library_snapshot()
}

#[tauri::command]
fn load_data_library(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    directory: String,
) -> LibrarySnapshot {
    let mut session = lock_session(&state);
    let snapshot = session.load_directory(&directory);
    // A new library means no surveyed vehicle: the bench empties with it.
    refresh_bench(&bench, &session);
    snapshot
}

#[tauri::command]
fn get_vehicle_catalogue(state: State<'_, SharedSessionService>) -> VehicleCatalogueSnapshot {
    lock_session(&state).catalogue()
}

#[tauri::command]
fn decode_vin(state: State<'_, SharedSessionService>, vin: String) -> VinDecodeSnapshot {
    lock_session(&state).decode_vin(&vin)
}

#[tauri::command]
fn survey_vehicle(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    context: VehicleContextInput,
) -> VehicleSurveySnapshot {
    let mut session = lock_session(&state);
    let survey = session.survey(&context);
    // The bench follows the session: it answers for the vehicle just described.
    refresh_bench(&bench, &session);
    survey
}

/// Longest listen the UI may ask for. The adapter service is held for the
/// whole capture, so other adapter commands wait; keep it short.
const CAPTURE_MAX_SECONDS: u32 = 15;

// Listen-only capture: opens a route with the listen-only flag, records what
// the vehicle broadcasts, and never transmits. The saved artifact is a
// `captured` replay fixture with its provenance.

#[tauri::command]
fn get_capture_state(state: State<'_, SharedCaptureService>) -> CaptureSnapshot {
    lock_capture(&state).snapshot()
}

#[tauri::command]
fn capture_bus(
    adapter_state: State<'_, SharedAdapterService>,
    capture_state: State<'_, SharedCaptureService>,
    report_state: State<'_, SharedSessionReportService>,
    route_id: String,
    seconds: u32,
    context: VehicleContextInput,
) -> CaptureSnapshot {
    let route = match route_id.parse::<VehicleRouteId>() {
        Ok(route) => route,
        Err(error) => return lock_capture(&capture_state).record_failure(&route_id, error),
    };
    let duration = Duration::from_secs(u64::from(seconds.clamp(1, CAPTURE_MAX_SECONDS)));

    let (adapter, result, bench) = {
        let mut service = lock_service(&adapter_state);
        let Some(adapter) = service.connected_adapter() else {
            return lock_capture(&capture_state).record_failure(
                route.as_str(),
                "Connect and verify the adapter before listening.".into(),
            );
        };
        let Some(result) = service.capture_route(route, duration) else {
            return lock_capture(&capture_state).record_failure(
                route.as_str(),
                "Connect and verify the adapter before listening.".into(),
            );
        };
        (adapter, result, service.is_bench())
    };

    let (snapshot, json) = {
        let mut capture = lock_capture(&capture_state);
        let mut snapshot = match result {
            Ok(captured) => capture.record(&captured, duration, &context, &adapter),
            Err(error) => capture.record_failure(route.as_str(), error.to_string()),
        };
        if bench {
            snapshot = capture.mark_synthetic();
        }
        (snapshot, capture.capture_json())
    };
    if let Ok(json) = json {
        let mut session_report = lock_session_report(&report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_capture(&json);
    }
    snapshot
}

#[tauri::command]
fn get_capture_json(state: State<'_, SharedCaptureService>) -> Result<String, String> {
    lock_capture(&state).capture_json()
}

/// Longest wait for one module answer, before ResponsePending extends it.
const MODULE_READ_TIMEOUT: Duration = Duration::from_secs(2);

// Module read (ADR-0015): resolve from the loaded library, prepare the typed
// read-only transaction, execute it live, decode, report.

#[tauri::command]
fn get_module_read_state(state: State<'_, SharedModuleReadService>) -> ModuleReadSnapshot {
    lock_module_read(&state).snapshot()
}

#[tauri::command]
fn read_module(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    module_read_state: State<'_, SharedModuleReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: ModuleReadRequest,
) -> ModuleReadSnapshot {
    let prepared = {
        let session = lock_session(&session_state);
        ModuleReadService::prepare(session.library(), &request)
    };
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            return lock_module_read(&module_read_state).finish(
                &request,
                None,
                None,
                None,
                Err(error),
            )
        }
    };

    let (adapter, result, bench) = {
        let mut service = lock_service(&adapter_state);
        let Some(adapter) = service.connected_adapter() else {
            return lock_module_read(&module_read_state).finish(
                &request,
                Some(&prepared),
                None,
                None,
                Err(adapter_unavailable()),
            );
        };
        let Some(result) = service.execute_uds_read(&prepared.transaction, MODULE_READ_TIMEOUT)
        else {
            return lock_module_read(&module_read_state).finish(
                &request,
                Some(&prepared),
                None,
                None,
                Err(adapter_unavailable()),
            );
        };
        (adapter, result.map_err(map_live_error), service.is_bench())
    };

    let (snapshot, report) = {
        let session = lock_session(&session_state);
        let mut reads = lock_module_read(&module_read_state);
        let mut snapshot = reads.finish(
            &request,
            Some(&prepared),
            Some(&adapter),
            Some(session.library()),
            result,
        );
        if bench {
            snapshot = reads.mark_synthetic();
        }
        (snapshot, reads.report_json())
    };
    if let Ok(json) = report {
        let mut session_report = lock_session_report(&report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_module_read(&json);
    }
    snapshot
}

#[tauri::command]
fn get_module_read_report_json(
    state: State<'_, SharedModuleReadService>,
) -> Result<String, String> {
    lock_module_read(&state).report_json()
}

// F11 M1: one report for the session. Bundles what the other services
// recorded; interprets nothing.

#[tauri::command]
fn get_session_report_state(state: State<'_, SharedSessionReportService>) -> SessionReportSnapshot {
    lock_session_report(&state).snapshot()
}

#[tauri::command]
fn get_session_report_json(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<String, String> {
    let adapter = lock_service(&adapter_state).snapshot();
    let (library, survey) = {
        let session = lock_session(&session_state);
        (session.library_snapshot(), session.last_survey())
    };
    lock_session_report(&report_state).report_json(&adapter, &library, survey.as_ref())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Save text where the user chooses. The shell opens the native dialog and
/// writes the file itself; a webview's own download link is not something a
/// tester can rely on. Resolves to the path written, or `None` when the
/// dialog was cancelled.
#[tauri::command]
async fn save_text_file(
    app: tauri::AppHandle,
    suggested_name: String,
    contents: String,
) -> Result<Option<String>, String> {
    use tauri::Manager;
    use tauri_plugin_dialog::DialogExt;
    // On the bench nothing is written to disk (ADR-0020): the refusal lives
    // here, in the command that writes, not in a button.
    if app
        .state::<SharedSessionReportService>()
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_bench()
    {
        return Err(
            "bench session: nothing is written to disk; the report is shown on screen only".into(),
        );
    }
    let Some(chosen) = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|error| error.to_string())?;
    std::fs::write(&path, contents)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Start a new session once the user confirms in a native dialog: the
/// report, the survey and every capture and read of the current session are
/// dropped; the adapter connection and the loaded library stay. Resolves to
/// whether the reset happened. A repeated survey alone does not do this —
/// the report keeps accumulating — which is why the action exists.
#[tauri::command]
async fn start_new_session(
    app: tauri::AppHandle,
    title: String,
    message: String,
    confirm_label: String,
    cancel_label: String,
) -> bool {
    use tauri::Manager;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    let confirmed = app
        .dialog()
        .message(&message)
        .title(&title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            confirm_label,
            cancel_label,
        ))
        .blocking_show();
    if !confirmed {
        return false;
    }
    fn replace<T: Send + 'static>(state: State<'_, Mutex<T>>, value: T) {
        *state
            .inner()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = value;
    }
    replace(
        app.state::<SharedSessionReportService>(),
        SessionReportService::new(),
    );
    replace(app.state::<SharedCaptureService>(), CaptureService::new());
    replace(
        app.state::<SharedModuleReadService>(),
        ModuleReadService::new(),
    );
    replace(
        app.state::<SharedDiagnosticService>(),
        DiagnosticService::new(),
    );
    app.state::<SharedSessionService>()
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear_session();
    // A refused bench/adapter switch was about the old session.
    app.state::<SharedAdapterService>()
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear_session_error();
    true
}

/// Let the user choose a folder — the data library's — through the native
/// dialog. Resolves to its path, or `None` when cancelled.
#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(chosen) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|error| error.to_string())?;
    Ok(Some(path.display().to_string()))
}

pub fn run() {
    let bench: SharedBench = share_bus(Box::new(EmptyBench));
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(bench.clone())
        .manage(shared_service(bench))
        .manage(Mutex::new(DiagnosticService::new()))
        .manage(Mutex::new(SessionService::new()))
        .manage(Mutex::new(CaptureService::new()))
        .manage(Mutex::new(ModuleReadService::new()))
        .manage(Mutex::new(SessionReportService::new()))
        .invoke_handler(tauri::generate_handler![
            get_adapter_state,
            discover_adapters,
            connect_adapter,
            connect_bench,
            disconnect_adapter,
            get_diagnostic_state,
            read_calibration_identification,
            get_diagnostic_report_json,
            get_data_library,
            load_data_library,
            survey_vehicle,
            get_vehicle_catalogue,
            decode_vin,
            get_capture_state,
            capture_bus,
            get_capture_json,
            get_module_read_state,
            read_module,
            get_module_read_report_json,
            get_session_report_state,
            get_session_report_json,
            save_text_file,
            pick_directory,
            start_new_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run JLR Scanner");
}

#[cfg(test)]
mod bench_e2e;
