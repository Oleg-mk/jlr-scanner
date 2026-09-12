//! Tauri composition root for ProwlOne.

mod adapter_service;
mod battery_service;
mod capture_service;
mod ccf_service;
mod diagnostic_service;
mod kline_read_service;
mod live_read_service;
mod mileage_service;
mod module_read_service;
mod passport_service;
mod read_record;
mod session_report_service;
mod session_service;
mod standard_obd_service;

use adapter_service::{lock_service, shared_service, SharedAdapterService};
use app_contracts::{
    AdapterSnapshot, AdapterState, BatteryReadRequest, BatteryReadSnapshot, CaptureSnapshot,
    CcfReadRequest, CcfReadSnapshot, DiagnosticError, DiagnosticSnapshot, LibrarySnapshot,
    LiveReadRequest, LiveReadSnapshot, MileageSurveyRequest, MileageSurveySnapshot,
    ModulePassportRequest, ModulePassportSnapshot, ModuleReadRequest, ModuleReadSnapshot,
    SessionReportSnapshot, StandardObdRequest, StandardObdSnapshot, VehicleCatalogueSnapshot,
    VehicleContextInput, VehicleSurveySnapshot, VinDecodeSnapshot,
};
use battery_service::{BatteryService, BATTERY_READ_TIMEOUT};
use bench_vehicle::BenchVehicle;
use capture_service::CaptureService;
use ccf_service::{CcfService, CCF_READ_TIMEOUT};
use diagnostic_service::DiagnosticService;
use diagnostic_session::parameter_text;
use kline_read_service::{KlineReadService, PreparedKlineRead, KLINE_READ_TIMEOUT};
use live_read_service::{LiveReadService, LIVE_READ_TIMEOUT};
use mileage_service::{MileageService, MILEAGE_READ_TIMEOUT};
use module_read_service::{adapter_unavailable, map_live_error, ModuleReadService};
use mongoose_jlr::bench::{share_bus, SharedBenchBus};
use mongoose_jlr::VehicleRouteId;
use passport_service::{PassportService, PASSPORT_READ_TIMEOUT};
use session_report_service::SessionReportService;
use session_report_service::{SESSION_MODE_BENCH, SESSION_MODE_REAL};
use session_service::SessionService;
use standard_obd_service::{StandardObdService, STANDARD_OBD_TIMEOUT};
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;
use tauri::State;
use transport_api::{BenchBus, EmptyBench};

type SharedDiagnosticService = Mutex<DiagnosticService>;
type SharedSessionService = Mutex<SessionService>;
type SharedCaptureService = Mutex<CaptureService>;
type SharedModuleReadService = Mutex<ModuleReadService>;
type SharedStandardObdService = Mutex<StandardObdService>;
type SharedLiveReadService = Mutex<LiveReadService>;
type SharedMileageService = Mutex<MileageService>;
type SharedPassportService = Mutex<PassportService>;
type SharedCcfService = Mutex<CcfService>;
type SharedBatteryService = Mutex<BatteryService>;

fn lock_battery<'a>(state: &'a State<'a, SharedBatteryService>) -> MutexGuard<'a, BatteryService> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_ccf<'a>(state: &'a State<'a, SharedCcfService>) -> MutexGuard<'a, CcfService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_mileage<'a>(state: &'a State<'a, SharedMileageService>) -> MutexGuard<'a, MileageService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_passport<'a>(
    state: &'a State<'a, SharedPassportService>,
) -> MutexGuard<'a, PassportService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_live_read<'a>(
    state: &'a State<'a, SharedLiveReadService>,
) -> MutexGuard<'a, LiveReadService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_standard_obd<'a>(
    state: &'a State<'a, SharedStandardObdService>,
) -> MutexGuard<'a, StandardObdService> {
    state
        .inner()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
type SharedSessionReportService = Mutex<SessionReportService>;
/// The vehicle on the bench (ADR-0020), shared between the bench transport
/// and the session that describes it.
type SharedBench = SharedBenchBus;

/// The scenario the bench answers on (ADR-0020). It outlives any one
/// connection, because the bench is rebuilt whenever the session describes a
/// vehicle and has to keep painting the picture the tester asked for.
struct BenchScenario(Mutex<u32>);

fn bench_scenario(state: &State<'_, BenchScenario>) -> u32 {
    *state
        .inner()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Put the vehicle the session last described on the bench, or nothing.
fn refresh_bench(bench: &SharedBench, session: &SessionService, scenario: u32) {
    let vehicle: Box<dyn BenchBus> = match session.last_context() {
        Some(context) => Box::new(BenchVehicle::from_library(
            session.library(),
            &context,
            None,
            scenario,
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
async fn get_adapter_state(
    state: State<'_, SharedAdapterService>,
) -> Result<AdapterSnapshot, String> {
    Ok(lock_service(&state).snapshot())
}

fn discover_adapters_now(state: State<'_, SharedAdapterService>) -> AdapterSnapshot {
    lock_service(&state).discover()
}

fn connect_adapter_now(
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
fn connect_bench_now(
    state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    report_state: State<'_, SharedSessionReportService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    scenario: Option<u32>,
) -> AdapterSnapshot {
    let mut service = lock_service(&state);
    if !lock_session_report(&report_state).accepts(SESSION_MODE_BENCH) {
        return service.refuse_session_switch(SESSION_MODE_BENCH);
    }
    let scenario = scenario.unwrap_or(bench_vehicle::SCENARIO_DEFAULT);
    *scenario_state
        .inner()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = scenario;
    refresh_bench(&bench, &lock_session(&session_state), scenario);
    let snapshot = service.connect_bench(scenario);
    if snapshot.state == AdapterState::Connected {
        let mut report = lock_session_report(&report_state);
        report.set_mode(SESSION_MODE_BENCH);
        report.set_bench_scenario(scenario);
    }
    snapshot
}

fn disconnect_adapter_now(state: State<'_, SharedAdapterService>) -> AdapterSnapshot {
    lock_service(&state).disconnect()
}

#[tauri::command]
async fn get_diagnostic_state(
    adapter_state: State<'_, SharedAdapterService>,
    diagnostic_state: State<'_, SharedDiagnosticService>,
) -> Result<DiagnosticSnapshot, String> {
    let adapter_ready = lock_service(&adapter_state).connected_adapter().is_some();
    Ok(lock_diagnostic(&diagnostic_state).snapshot(adapter_ready))
}

fn read_calibration_identification_now(
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
async fn get_diagnostic_report_json(
    diagnostic_state: State<'_, SharedDiagnosticService>,
) -> Result<String, String> {
    lock_diagnostic(&diagnostic_state).report_json()
}

// The parameter names in the interface's language (`ADR-0025`). A dictionary,
// not knowledge: it claims nothing about any car, reaches no transport, and
// the English name it is keyed by stays the identity every report carries.

#[tauri::command]
async fn get_parameter_names(language: String) -> Result<BTreeMap<String, String>, String> {
    Ok(parameter_text::parameter_names(&language))
}

// F10 composition root: the data library and the vehicle survey. These read
// knowledge and compute plans; they open no transport and transmit nothing.

#[tauri::command]
async fn get_data_library(
    state: State<'_, SharedSessionService>,
) -> Result<LibrarySnapshot, String> {
    Ok(lock_session(&state).library_snapshot())
}

fn load_data_library_now(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    directory: String,
) -> LibrarySnapshot {
    let mut session = lock_session(&state);
    let snapshot = session.load_directory(&directory);
    // A new library means no surveyed vehicle: the bench empties with it.
    refresh_bench(&bench, &session, bench_scenario(&scenario_state));
    snapshot
}

#[tauri::command]
async fn get_vehicle_catalogue(
    state: State<'_, SharedSessionService>,
) -> Result<VehicleCatalogueSnapshot, String> {
    Ok(lock_session(&state).catalogue())
}

#[tauri::command]
async fn decode_vin(
    state: State<'_, SharedSessionService>,
    vin: String,
) -> Result<VinDecodeSnapshot, String> {
    Ok(lock_session(&state).decode_vin(&vin))
}

fn survey_vehicle_now(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    context: VehicleContextInput,
) -> VehicleSurveySnapshot {
    let mut session = lock_session(&state);
    let survey = session.survey(&context);
    // The bench follows the session: it answers for the vehicle just described.
    refresh_bench(&bench, &session, bench_scenario(&scenario_state));
    survey
}

/// Longest listen the UI may ask for. The adapter service is held for the
/// whole capture, so other adapter commands wait; keep it short.
const CAPTURE_MAX_SECONDS: u32 = 15;

// Listen-only capture: opens a route with the listen-only flag, records what
// the vehicle broadcasts, and never transmits. The saved artifact is a
// `captured` replay fixture with its provenance.

#[tauri::command]
async fn get_capture_state(
    state: State<'_, SharedCaptureService>,
) -> Result<CaptureSnapshot, String> {
    Ok(lock_capture(&state).snapshot())
}

fn capture_bus_now(
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
async fn get_capture_json(state: State<'_, SharedCaptureService>) -> Result<String, String> {
    lock_capture(&state).capture_json()
}

/// Longest wait for one module answer, before ResponsePending extends it.
const MODULE_READ_TIMEOUT: Duration = Duration::from_secs(2);

// Module read (ADR-0015): resolve from the loaded library, prepare the typed
// read-only transaction, execute it live, decode, report.

#[tauri::command]
async fn get_module_read_state(
    state: State<'_, SharedModuleReadService>,
) -> Result<ModuleReadSnapshot, String> {
    Ok(lock_module_read(&state).snapshot())
}

/// One read on a serial line, from the plan to the record (`ADR-0029`
/// slice B). The shape is the CAN read's: refuse without an adapter, execute
/// once, record whatever came back, hand the record to the session bundle.
fn read_kline_module_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    module_read_state: State<'_, SharedModuleReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: ModuleReadRequest,
    prepared: Result<PreparedKlineRead, DiagnosticError>,
) -> ModuleReadSnapshot {
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
        let adapter = service.connected_adapter();
        let bench = service.is_bench();
        let result = service.execute_kline_read(&prepared.transaction, KLINE_READ_TIMEOUT);
        match (adapter, result) {
            (Some(adapter), Some(result)) => (Some(adapter), result.map_err(map_live_error), bench),
            _ => (None, Err(adapter_unavailable()), bench),
        }
    };

    let (snapshot, record) = {
        let session = lock_session(&session_state);
        KlineReadService::finish(
            &request,
            &prepared,
            adapter.as_ref(),
            Some(session.library()),
            bench,
            result,
        )
    };
    let json = serde_json::to_string(&record).ok();
    let snapshot = lock_module_read(&module_read_state).adopt(snapshot, record);
    if let Some(json) = json {
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

fn read_module_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    module_read_state: State<'_, SharedModuleReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: ModuleReadRequest,
) -> ModuleReadSnapshot {
    // A module on a K-line is read over the K-line (`ADR-0029` slice B);
    // every other module is read the way it always was.
    let kline = {
        let session = lock_session(&session_state);
        KlineReadService::prepare(session.library(), &request)
    };
    if let Some(kline) = kline {
        return read_kline_module_now(
            adapter_state,
            session_state,
            module_read_state,
            report_state,
            request,
            kline,
        );
    }

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
async fn get_module_read_report_json(
    state: State<'_, SharedModuleReadService>,
) -> Result<String, String> {
    lock_module_read(&state).report_json()
}

// F11 M1: one report for the session. Bundles what the other services
// recorded; interprets nothing.

#[tauri::command]
async fn get_session_report_state(
    state: State<'_, SharedSessionReportService>,
) -> Result<SessionReportSnapshot, String> {
    Ok(lock_session_report(&state).snapshot())
}

#[tauri::command]
async fn get_session_report_json(
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
///
/// `format` is `json` unless the caller says `csv` — an exported live
/// series (ADR-0022 §5, amended) — or `html`, the readable report
/// (ADR-0031). It picks the dialog's filter and nothing else about the
/// write changes; the bench still writes nothing.
#[tauri::command]
async fn save_text_file(
    app: tauri::AppHandle,
    suggested_name: String,
    contents: String,
    format: Option<String>,
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
    let (label, extension) = match format.as_deref() {
        Some("csv") => ("CSV", "csv"),
        // The readable report (ADR-0031): one web page any browser prints.
        Some("html") => ("Web page", "html"),
        _ => ("JSON", "json"),
    };
    let Some(chosen) = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter(label, &[extension])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = chosen.into_path().map_err(|error| error.to_string())?;
    std::fs::write(&path, contents)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Show a file the application has just written in the system's own file
/// manager, so it can be attached to a message (ADR-0031). Nothing is sent
/// anywhere and nothing is opened: the folder is shown with the file
/// selected. A path the application did not just write is not revealed —
/// the caller passes back what `save_text_file` returned.
#[tauri::command]
async fn reveal_in_folder(path: String) -> Result<(), String> {
    let file = std::path::Path::new(&path);
    if !file.is_file() {
        return Err(format!("{path} is not a file this application wrote"));
    }
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
    } else {
        let folder = file.parent().unwrap_or(file);
        std::process::Command::new("xdg-open").arg(folder).spawn()
    };
    // Windows Explorer answers with a non-zero exit code even when it
    // opens, so only a failure to start the program is an error here.
    result
        .map(|_| ())
        .map_err(|error| format!("could not show {path}: {error}"))
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
    replace(
        app.state::<SharedStandardObdService>(),
        StandardObdService::new(),
    );
    // A live read never survives a new session (ADR-0022, decision 4).
    replace(app.state::<SharedLiveReadService>(), LiveReadService::new());
    replace(app.state::<SharedMileageService>(), MileageService::new());
    replace(app.state::<SharedPassportService>(), PassportService::new());
    replace(app.state::<SharedCcfService>(), CcfService::new());
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

// ---------------------------------------------------------------------------
// The slow commands, off the window's thread
//
// Tauri runs a synchronous command on the thread that owns the window, so the
// window answers nothing while one works: macOS draws its spinning wheel, and
// Windows says "not responding". A 177 MB library takes seconds even on a
// fast machine and tens of seconds on an old one, and a bus listen holds the
// thread for up to fifteen. Each of these is `async`, which makes Tauri spawn
// it on the runtime instead; the body itself still blocks, on a runtime
// worker rather than on the window. An async command that borrows state has
// to return a `Result`, so these return one and never fail.
// ---------------------------------------------------------------------------

#[tauri::command]
async fn discover_adapters(
    state: State<'_, SharedAdapterService>,
) -> Result<AdapterSnapshot, String> {
    Ok(discover_adapters_now(state))
}

#[tauri::command]
async fn connect_adapter(
    state: State<'_, SharedAdapterService>,
    report_state: State<'_, SharedSessionReportService>,
    port: Option<String>,
) -> Result<AdapterSnapshot, String> {
    Ok(connect_adapter_now(state, report_state, port))
}

#[tauri::command]
async fn connect_bench(
    state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    report_state: State<'_, SharedSessionReportService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    scenario: Option<u32>,
) -> Result<AdapterSnapshot, String> {
    Ok(connect_bench_now(
        state,
        session_state,
        report_state,
        bench,
        scenario_state,
        scenario,
    ))
}

#[tauri::command]
async fn disconnect_adapter(
    state: State<'_, SharedAdapterService>,
) -> Result<AdapterSnapshot, String> {
    Ok(disconnect_adapter_now(state))
}

#[tauri::command]
async fn read_calibration_identification(
    adapter_state: State<'_, SharedAdapterService>,
    diagnostic_state: State<'_, SharedDiagnosticService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<DiagnosticSnapshot, String> {
    Ok(read_calibration_identification_now(
        adapter_state,
        diagnostic_state,
        report_state,
    ))
}

#[tauri::command]
async fn load_data_library(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    directory: String,
) -> Result<LibrarySnapshot, String> {
    Ok(load_data_library_now(
        state,
        bench,
        scenario_state,
        directory,
    ))
}

#[tauri::command]
async fn survey_vehicle(
    state: State<'_, SharedSessionService>,
    bench: State<'_, SharedBench>,
    scenario_state: State<'_, BenchScenario>,
    context: VehicleContextInput,
) -> Result<VehicleSurveySnapshot, String> {
    Ok(survey_vehicle_now(state, bench, scenario_state, context))
}

#[tauri::command]
async fn capture_bus(
    adapter_state: State<'_, SharedAdapterService>,
    capture_state: State<'_, SharedCaptureService>,
    report_state: State<'_, SharedSessionReportService>,
    route_id: String,
    seconds: u32,
    context: VehicleContextInput,
) -> Result<CaptureSnapshot, String> {
    Ok(capture_bus_now(
        adapter_state,
        capture_state,
        report_state,
        route_id,
        seconds,
        context,
    ))
}

#[tauri::command]
async fn read_module(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    module_read_state: State<'_, SharedModuleReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: ModuleReadRequest,
) -> Result<ModuleReadSnapshot, String> {
    Ok(read_module_now(
        adapter_state,
        session_state,
        module_read_state,
        report_state,
        request,
    ))
}

// ---------------------------------------------------------------------------
// The legislated OBD-II services (ADR-0022, decision 7)
// ---------------------------------------------------------------------------

/// One legislated read: prepared from the standard alone, executed over the
/// adapter, decoded by the codec, worded from the library, recorded in the
/// session bundle. The shape of a module read, without the resolver.
fn read_standard_obd_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    standard_obd_state: State<'_, SharedStandardObdService>,
    report_state: State<'_, SharedSessionReportService>,
    request: StandardObdRequest,
) -> StandardObdSnapshot {
    let prepared = match StandardObdService::prepare(&request) {
        Ok(prepared) => prepared,
        Err(error) => {
            return lock_standard_obd(&standard_obd_state).finish(
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
            return lock_standard_obd(&standard_obd_state).finish(
                &request,
                Some(&prepared),
                None,
                None,
                Err(adapter_unavailable()),
            );
        };
        let Some(result) = service.execute_j1979_read(&prepared.transaction, STANDARD_OBD_TIMEOUT)
        else {
            return lock_standard_obd(&standard_obd_state).finish(
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
        let mut reads = lock_standard_obd(&standard_obd_state);
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
        let _ = session_report.add_standard_obd_read(&json);
    }
    snapshot
}

#[tauri::command]
async fn get_standard_obd_state(
    state: State<'_, SharedStandardObdService>,
) -> Result<StandardObdSnapshot, String> {
    Ok(lock_standard_obd(&state).snapshot())
}

#[tauri::command]
async fn read_standard_obd(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    standard_obd_state: State<'_, SharedStandardObdService>,
    report_state: State<'_, SharedSessionReportService>,
    request: StandardObdRequest,
) -> Result<StandardObdSnapshot, String> {
    Ok(read_standard_obd_now(
        adapter_state,
        session_state,
        standard_obd_state,
        report_state,
        request,
    ))
}

// ---------------------------------------------------------------------------
// Live reading (ADR-0022): the same read-only reads, repeated at a stated
// cadence. The service owns the rules, the interface owns the timer, and the
// adapter is taken for one request at a time and released.
// ---------------------------------------------------------------------------

/// The live series as a spreadsheet (ADR-0022 §5, amended): a rendering of
/// what the session bundle already holds, for the person with a
/// spreadsheet. The bundle stays the evidence.
#[tauri::command]
async fn live_read_csv(state: State<'_, SharedLiveReadService>) -> Result<String, String> {
    lock_live_read(&state).samples_csv()
}

#[tauri::command]
async fn get_live_read_state(
    state: State<'_, SharedLiveReadService>,
) -> Result<LiveReadSnapshot, String> {
    Ok(lock_live_read(&state).snapshot())
}

/// A stopped run joins the session bundle once; the service forgets it as it
/// hands it over, so a second stop cannot record the same run twice.
fn record_live_run(
    live_state: &State<'_, SharedLiveReadService>,
    report_state: &State<'_, SharedSessionReportService>,
    bench: bool,
) {
    let json = { lock_live_read(live_state).take_report_json() };
    if let Some(json) = json {
        let mut session_report = lock_session_report(report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_live_read_run(&json);
    }
}

fn start_live_read_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: LiveReadRequest,
) -> LiveReadSnapshot {
    let (adapter, bench) = {
        let service = lock_service(&adapter_state);
        (service.connected_adapter(), service.is_bench())
    };
    let Some(adapter) = adapter else {
        return lock_live_read(&live_state).refuse(adapter_unavailable());
    };
    // A run still going is stopped and recorded before another begins.
    if lock_live_read(&live_state).is_running() {
        lock_live_read(&live_state).stop("a new run replaced this one");
        record_live_run(&live_state, &report_state, bench);
    }
    let session = lock_session(&session_state);
    let mut live = lock_live_read(&live_state);
    live.start(session.library(), Some(&adapter), &request)
}

fn live_read_step_now(
    adapter_state: State<'_, SharedAdapterService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
) -> LiveReadSnapshot {
    let due = { lock_live_read(&live_state).next_due() };
    let Some(due) = due else {
        // Not running, too soon for the floor, or the run has just stopped
        // itself at the cap or with an empty set.
        let bench = lock_service(&adapter_state).is_bench();
        record_live_run(&live_state, &report_state, bench);
        return lock_live_read(&live_state).snapshot();
    };

    let (result, bench) = {
        let mut service = lock_service(&adapter_state);
        let bench = service.is_bench();
        let result = service
            .execute_uds_read(&due.transaction, LIVE_READ_TIMEOUT)
            .map(|result| result.map_err(map_live_error));
        (result, bench)
    };
    let connected = result.is_some();
    let outcome = result.unwrap_or_else(|| Err(adapter_unavailable()));

    {
        let mut live = lock_live_read(&live_state);
        live.record(due.index, outcome);
        if bench {
            live.mark_synthetic();
        }
        if !connected {
            live.stop("the adapter is no longer connected");
        }
    }
    record_live_run(&live_state, &report_state, bench);
    lock_live_read(&live_state).snapshot()
}

fn stop_live_read_now(
    adapter_state: State<'_, SharedAdapterService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
) -> LiveReadSnapshot {
    let bench = lock_service(&adapter_state).is_bench();
    lock_live_read(&live_state).stop("stopped by the tester");
    record_live_run(&live_state, &report_state, bench);
    lock_live_read(&live_state).snapshot()
}

#[tauri::command]
async fn start_live_read(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
    request: LiveReadRequest,
) -> Result<LiveReadSnapshot, String> {
    Ok(start_live_read_now(
        adapter_state,
        session_state,
        live_state,
        report_state,
        request,
    ))
}

#[tauri::command]
async fn live_read_step(
    adapter_state: State<'_, SharedAdapterService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<LiveReadSnapshot, String> {
    Ok(live_read_step_now(adapter_state, live_state, report_state))
}

#[tauri::command]
async fn stop_live_read(
    adapter_state: State<'_, SharedAdapterService>,
    live_state: State<'_, SharedLiveReadService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<LiveReadSnapshot, String> {
    Ok(stop_live_read_now(adapter_state, live_state, report_state))
}

// ---------------------------------------------------------------------------
// The odometer read from every module (ADR-0024). One request per module,
// once; readings side by side with the arithmetic done; no verdict drawn.
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_mileage_state(
    state: State<'_, SharedMileageService>,
) -> Result<MileageSurveySnapshot, String> {
    Ok(lock_mileage(&state).snapshot())
}

/// A finished survey joins the session bundle once.
fn record_mileage_survey(
    mileage_state: &State<'_, SharedMileageService>,
    report_state: &State<'_, SharedSessionReportService>,
    bench: bool,
) {
    let json = { lock_mileage(mileage_state).take_report_json() };
    if let Some(json) = json {
        let mut session_report = lock_session_report(report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_mileage_survey(&json);
    }
}

fn start_mileage_survey_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    mileage_state: State<'_, SharedMileageService>,
    request: MileageSurveyRequest,
) -> MileageSurveySnapshot {
    let adapter = { lock_service(&adapter_state).connected_adapter() };
    // Without an adapter the honest answer is that, not that the data
    // names no mileage.
    let Some(adapter) = adapter else {
        return lock_mileage(&mileage_state).refuse(adapter_unavailable());
    };
    let mut session = lock_session(&session_state);
    // Every module the survey reaches is asked; the point is breadth.
    let families: Vec<String> = session
        .survey(&request.context)
        .modules
        .iter()
        .map(|module| module.ecu_family.clone())
        .collect();
    let mut mileage = lock_mileage(&mileage_state);
    mileage.start(
        session.library(),
        Some(&adapter),
        &request.context,
        &families,
    )
}

fn mileage_survey_step_now(
    adapter_state: State<'_, SharedAdapterService>,
    mileage_state: State<'_, SharedMileageService>,
    report_state: State<'_, SharedSessionReportService>,
) -> MileageSurveySnapshot {
    let due = { lock_mileage(&mileage_state).next_due() };
    let Some(due) = due else {
        let bench = lock_service(&adapter_state).is_bench();
        record_mileage_survey(&mileage_state, &report_state, bench);
        return lock_mileage(&mileage_state).snapshot();
    };

    let (result, bench) = {
        let mut service = lock_service(&adapter_state);
        let bench = service.is_bench();
        let result = service
            .execute_uds_read(&due.transaction, MILEAGE_READ_TIMEOUT)
            .map(|result| result.map_err(map_live_error));
        (result, bench)
    };
    let connected = result.is_some();
    let outcome = result.unwrap_or_else(|| Err(adapter_unavailable()));

    {
        let mut mileage = lock_mileage(&mileage_state);
        mileage.record(due.index, outcome);
        if bench {
            mileage.mark_synthetic();
        }
        // An adapter that is gone will not come back for the next module;
        // what was read stays, as the live read does.
        if !connected {
            mileage.finish();
        }
    }
    record_mileage_survey(&mileage_state, &report_state, bench);
    lock_mileage(&mileage_state).snapshot()
}

#[tauri::command]
async fn start_mileage_survey(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    mileage_state: State<'_, SharedMileageService>,
    request: MileageSurveyRequest,
) -> Result<MileageSurveySnapshot, String> {
    Ok(start_mileage_survey_now(
        adapter_state,
        session_state,
        mileage_state,
        request,
    ))
}

#[tauri::command]
async fn mileage_survey_step(
    adapter_state: State<'_, SharedAdapterService>,
    mileage_state: State<'_, SharedMileageService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<MileageSurveySnapshot, String> {
    Ok(mileage_survey_step_now(
        adapter_state,
        mileage_state,
        report_state,
    ))
}

#[tauri::command]
async fn finish_mileage_survey(
    adapter_state: State<'_, SharedAdapterService>,
    mileage_state: State<'_, SharedMileageService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<MileageSurveySnapshot, String> {
    let bench = lock_service(&adapter_state).is_bench();
    lock_mileage(&mileage_state).finish();
    record_mileage_survey(&mileage_state, &report_state, bench);
    Ok(lock_mileage(&mileage_state).snapshot())
}

// ---------------------------------------------------------------------------
// The module passport (ADR-0027): part numbers, serial, hardware and software
// levels, over the identification identifiers SDD declares for each module.
// One request per module and identifier, once; the text shown as it is.
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_passport_state(
    state: State<'_, SharedPassportService>,
) -> Result<ModulePassportSnapshot, String> {
    Ok(lock_passport(&state).snapshot())
}

/// A finished run joins the session bundle once.
fn record_module_passport(
    passport_state: &State<'_, SharedPassportService>,
    report_state: &State<'_, SharedSessionReportService>,
    bench: bool,
) {
    let json = { lock_passport(passport_state).take_report_json() };
    if let Some(json) = json {
        let mut session_report = lock_session_report(report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_module_passport(&json);
    }
}

fn start_module_passport_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    passport_state: State<'_, SharedPassportService>,
    request: ModulePassportRequest,
) -> ModulePassportSnapshot {
    let adapter = { lock_service(&adapter_state).connected_adapter() };
    // Without an adapter the honest answer is that, not that the data
    // names no identification.
    let Some(adapter) = adapter else {
        return lock_passport(&passport_state).refuse(adapter_unavailable());
    };
    let mut session = lock_session(&session_state);
    // The chosen modules, or every module the survey reaches.
    let families: Vec<String> = session
        .survey(&request.context)
        .modules
        .iter()
        .map(|module| module.ecu_family.clone())
        .filter(|family| match &request.ecu_families {
            Some(chosen) => chosen.iter().any(|wanted| wanted == family),
            None => true,
        })
        .collect();
    let mut passport = lock_passport(&passport_state);
    passport.start(
        session.library(),
        Some(&adapter),
        &request.context,
        &families,
    )
}

fn module_passport_step_now(
    adapter_state: State<'_, SharedAdapterService>,
    passport_state: State<'_, SharedPassportService>,
    report_state: State<'_, SharedSessionReportService>,
) -> ModulePassportSnapshot {
    let due = { lock_passport(&passport_state).next_due() };
    let Some(due) = due else {
        let bench = lock_service(&adapter_state).is_bench();
        record_module_passport(&passport_state, &report_state, bench);
        return lock_passport(&passport_state).snapshot();
    };

    let (result, bench) = {
        let mut service = lock_service(&adapter_state);
        let bench = service.is_bench();
        let result = service
            .execute_uds_read(&due.transaction, PASSPORT_READ_TIMEOUT)
            .map(|result| result.map_err(map_live_error));
        (result, bench)
    };
    let connected = result.is_some();
    let outcome = result.unwrap_or_else(|| Err(adapter_unavailable()));

    {
        let mut passport = lock_passport(&passport_state);
        passport.record(due.index, outcome);
        if bench {
            passport.mark_synthetic();
        }
        // An adapter that is gone will not come back for the next read;
        // what was read stays, as the mileage survey does.
        if !connected {
            passport.finish();
        }
    }
    record_module_passport(&passport_state, &report_state, bench);
    lock_passport(&passport_state).snapshot()
}

#[tauri::command]
async fn start_module_passport(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    passport_state: State<'_, SharedPassportService>,
    request: ModulePassportRequest,
) -> Result<ModulePassportSnapshot, String> {
    Ok(start_module_passport_now(
        adapter_state,
        session_state,
        passport_state,
        request,
    ))
}

#[tauri::command]
async fn module_passport_step(
    adapter_state: State<'_, SharedAdapterService>,
    passport_state: State<'_, SharedPassportService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<ModulePassportSnapshot, String> {
    Ok(module_passport_step_now(
        adapter_state,
        passport_state,
        report_state,
    ))
}

#[tauri::command]
async fn finish_module_passport(
    adapter_state: State<'_, SharedAdapterService>,
    passport_state: State<'_, SharedPassportService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<ModulePassportSnapshot, String> {
    let bench = lock_service(&adapter_state).is_bench();
    lock_passport(&passport_state).finish();
    record_module_passport(&passport_state, &report_state, bench);
    Ok(lock_passport(&passport_state).snapshot())
}

// ---------------------------------------------------------------------------
// The car configuration file (ADR-0028): read block by block from the module
// SDD names as its keeper and from the modules holding copies, decoded with
// SDD's own layout. Every value is what its type says; nothing is judged.
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_ccf_state(state: State<'_, SharedCcfService>) -> Result<CcfReadSnapshot, String> {
    Ok(lock_ccf(&state).snapshot())
}

/// A finished run joins the session bundle once.
fn record_ccf_read(
    ccf_state: &State<'_, SharedCcfService>,
    report_state: &State<'_, SharedSessionReportService>,
    bench: bool,
) {
    let json = { lock_ccf(ccf_state).take_report_json() };
    if let Some(json) = json {
        let mut session_report = lock_session_report(report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_ccf_read(&json);
    }
}

fn start_ccf_read_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    ccf_state: State<'_, SharedCcfService>,
    request: CcfReadRequest,
) -> CcfReadSnapshot {
    let adapter = { lock_service(&adapter_state).connected_adapter() };
    // Without an adapter the honest answer is that, not that the data
    // describes no configuration.
    let Some(adapter) = adapter else {
        return lock_ccf(&ccf_state).refuse(adapter_unavailable());
    };
    let session = lock_session(&session_state);
    let mut ccf = lock_ccf(&ccf_state);
    ccf.start(session.library(), Some(&adapter), &request.context)
}

fn ccf_read_step_now(
    adapter_state: State<'_, SharedAdapterService>,
    ccf_state: State<'_, SharedCcfService>,
    report_state: State<'_, SharedSessionReportService>,
) -> CcfReadSnapshot {
    let due = { lock_ccf(&ccf_state).next_due() };
    let Some(due) = due else {
        let bench = lock_service(&adapter_state).is_bench();
        record_ccf_read(&ccf_state, &report_state, bench);
        return lock_ccf(&ccf_state).snapshot();
    };

    let (result, bench) = {
        let mut service = lock_service(&adapter_state);
        let bench = service.is_bench();
        let result = service
            .execute_uds_read(&due.transaction, CCF_READ_TIMEOUT)
            .map(|result| result.map_err(map_live_error));
        (result, bench)
    };
    let connected = result.is_some();
    let outcome = result.unwrap_or_else(|| Err(adapter_unavailable()));

    {
        let mut ccf = lock_ccf(&ccf_state);
        ccf.record(due.index, outcome);
        if bench {
            ccf.mark_synthetic();
        }
        // An adapter that is gone will not come back for the next read;
        // what was read stays.
        if !connected {
            ccf.finish();
        }
    }
    record_ccf_read(&ccf_state, &report_state, bench);
    lock_ccf(&ccf_state).snapshot()
}

#[tauri::command]
async fn start_ccf_read(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    ccf_state: State<'_, SharedCcfService>,
    request: CcfReadRequest,
) -> Result<CcfReadSnapshot, String> {
    Ok(start_ccf_read_now(
        adapter_state,
        session_state,
        ccf_state,
        request,
    ))
}

#[tauri::command]
async fn ccf_read_step(
    adapter_state: State<'_, SharedAdapterService>,
    ccf_state: State<'_, SharedCcfService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<CcfReadSnapshot, String> {
    Ok(ccf_read_step_now(adapter_state, ccf_state, report_state))
}

#[tauri::command]
async fn finish_ccf_read(
    adapter_state: State<'_, SharedAdapterService>,
    ccf_state: State<'_, SharedCcfService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<CcfReadSnapshot, String> {
    let bench = lock_service(&adapter_state).is_bench();
    lock_ccf(&ccf_state).finish();
    record_ccf_read(&ccf_state, &report_state, bench);
    Ok(lock_ccf(&ccf_state).snapshot())
}

// ---------------------------------------------------------------------------
// The battery (ADR-0030): what the battery monitor holds, read from the
// modules the loaded data names, shown as a card of its own. Nothing is
// judged, and nothing is written.
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_battery_state(
    state: State<'_, SharedBatteryService>,
) -> Result<BatteryReadSnapshot, String> {
    Ok(lock_battery(&state).snapshot())
}

/// A finished run joins the session bundle once.
fn record_battery_read(
    battery_state: &State<'_, SharedBatteryService>,
    report_state: &State<'_, SharedSessionReportService>,
    bench: bool,
) {
    let json = { lock_battery(battery_state).take_report_json() };
    if let Some(json) = json {
        let mut session_report = lock_session_report(report_state);
        session_report.set_mode(if bench {
            SESSION_MODE_BENCH
        } else {
            SESSION_MODE_REAL
        });
        let _ = session_report.add_battery_read(&json);
    }
}

fn start_battery_read_now(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    battery_state: State<'_, SharedBatteryService>,
    request: BatteryReadRequest,
) -> BatteryReadSnapshot {
    let adapter = { lock_service(&adapter_state).connected_adapter() };
    // Without an adapter the honest answer is that, not that the data
    // names no battery parameter.
    let Some(adapter) = adapter else {
        return lock_battery(&battery_state).refuse(adapter_unavailable());
    };
    let mut session = lock_session(&session_state);
    // The chosen modules, or every module the survey reaches.
    let families: Vec<String> = session
        .survey(&request.context)
        .modules
        .iter()
        .map(|module| module.ecu_family.clone())
        .filter(|family| match &request.ecu_families {
            Some(chosen) => chosen.iter().any(|wanted| wanted == family),
            None => true,
        })
        .collect();
    let mut battery = lock_battery(&battery_state);
    battery.start(
        session.library(),
        Some(&adapter),
        &request.context,
        &families,
    )
}

fn battery_read_step_now(
    adapter_state: State<'_, SharedAdapterService>,
    battery_state: State<'_, SharedBatteryService>,
    report_state: State<'_, SharedSessionReportService>,
) -> BatteryReadSnapshot {
    let due = { lock_battery(&battery_state).next_due() };
    let Some(due) = due else {
        let bench = lock_service(&adapter_state).is_bench();
        record_battery_read(&battery_state, &report_state, bench);
        return lock_battery(&battery_state).snapshot();
    };

    let (result, bench) = {
        let mut service = lock_service(&adapter_state);
        let bench = service.is_bench();
        let result = service
            .execute_uds_read(&due.transaction, BATTERY_READ_TIMEOUT)
            .map(|result| result.map_err(map_live_error));
        (result, bench)
    };
    let connected = result.is_some();
    let outcome = result.unwrap_or_else(|| Err(adapter_unavailable()));

    {
        let mut battery = lock_battery(&battery_state);
        battery.record(due.index, outcome);
        if bench {
            battery.mark_synthetic();
        }
        // An adapter that is gone will not come back for the next read;
        // what was read stays.
        if !connected {
            battery.finish();
        }
    }
    record_battery_read(&battery_state, &report_state, bench);
    lock_battery(&battery_state).snapshot()
}

#[tauri::command]
async fn start_battery_read(
    adapter_state: State<'_, SharedAdapterService>,
    session_state: State<'_, SharedSessionService>,
    battery_state: State<'_, SharedBatteryService>,
    request: BatteryReadRequest,
) -> Result<BatteryReadSnapshot, String> {
    Ok(start_battery_read_now(
        adapter_state,
        session_state,
        battery_state,
        request,
    ))
}

#[tauri::command]
async fn battery_read_step(
    adapter_state: State<'_, SharedAdapterService>,
    battery_state: State<'_, SharedBatteryService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<BatteryReadSnapshot, String> {
    Ok(battery_read_step_now(
        adapter_state,
        battery_state,
        report_state,
    ))
}

#[tauri::command]
async fn finish_battery_read(
    adapter_state: State<'_, SharedAdapterService>,
    battery_state: State<'_, SharedBatteryService>,
    report_state: State<'_, SharedSessionReportService>,
) -> Result<BatteryReadSnapshot, String> {
    let bench = lock_service(&adapter_state).is_bench();
    lock_battery(&battery_state).finish();
    record_battery_read(&battery_state, &report_state, bench);
    Ok(lock_battery(&battery_state).snapshot())
}

pub fn run() {
    let bench: SharedBench = share_bus(Box::new(EmptyBench));
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(bench.clone())
        .manage(BenchScenario(Mutex::new(bench_vehicle::SCENARIO_DEFAULT)))
        .manage(shared_service(bench))
        .manage(Mutex::new(DiagnosticService::new()))
        .manage(Mutex::new(SessionService::new()))
        .manage(Mutex::new(CaptureService::new()))
        .manage(Mutex::new(ModuleReadService::new()))
        .manage(Mutex::new(StandardObdService::new()))
        .manage(Mutex::new(LiveReadService::new()))
        .manage(Mutex::new(MileageService::new()))
        .manage(Mutex::new(PassportService::new()))
        .manage(Mutex::new(CcfService::new()))
        .manage(Mutex::new(BatteryService::new()))
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
            get_parameter_names,
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
            get_standard_obd_state,
            read_standard_obd,
            get_live_read_state,
            live_read_csv,
            start_live_read,
            live_read_step,
            stop_live_read,
            get_mileage_state,
            start_mileage_survey,
            mileage_survey_step,
            finish_mileage_survey,
            get_passport_state,
            start_module_passport,
            module_passport_step,
            finish_module_passport,
            get_battery_state,
            start_battery_read,
            battery_read_step,
            finish_battery_read,
            get_ccf_state,
            start_ccf_read,
            ccf_read_step,
            finish_ccf_read,
            get_module_read_report_json,
            get_session_report_state,
            get_session_report_json,
            save_text_file,
            reveal_in_folder,
            pick_directory,
            start_new_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ProwlOne");
}

#[cfg(test)]
mod bench_e2e;
