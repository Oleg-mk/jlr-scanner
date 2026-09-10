//! The legislated OBD-II services, live (ADR-0022, decision 7): prepare a
//! request from the standard alone, hand it to the adapter, read the answer
//! with the codec, keep a report. No library and no resolver take part in
//! the asking — every OBD-II car answers these at the same addresses — and
//! the library is consulted only for the wording of a fault code.

use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticErrorCategory, DiagnosticExecutionStage, DtcSummary,
    LabelledValue, ModuleReadState, StandardObdMonitor, StandardObdReadKind, StandardObdRequest,
    StandardObdSnapshot, StandardObdValue,
};
use diagnostic_execution::{
    prepare_standard_obd_transaction, PreparedDiagnosticTransaction, StandardObdResponder,
};
use diagnostic_session::{vehicle_context, KnowledgeLibrary, VehicleContext};
use mongoose_jlr::MongooseJ1979ReadResult;
use obd_j1979::{
    decode_response, negative_response_text, DtcKind, J1979Error, J1979Request, J1979Response,
    MonitorResult, ParameterValue, VehicleInformation,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// How long one legislated read may wait for its answer.
pub const STANDARD_OBD_TIMEOUT: Duration = Duration::from_secs(2);

/// A request the standard could prepare: the typed transaction plus what
/// the interface shows about it before anything is sent.
#[derive(Debug)]
pub struct PreparedStandardObd {
    pub request: J1979Request,
    pub transaction: PreparedDiagnosticTransaction,
    pub operation: String,
}

#[derive(Default)]
pub struct StandardObdService {
    last: Option<(StandardObdSnapshot, Value)>,
}

impl StandardObdService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> StandardObdSnapshot {
        self.last
            .as_ref()
            .map(|(snapshot, _)| snapshot.clone())
            .unwrap_or_else(idle)
    }

    /// Prepare without touching the adapter. Every refusal is a
    /// `DiagnosticError` in the preparation stage with the reason.
    pub fn prepare(request: &StandardObdRequest) -> Result<PreparedStandardObd, DiagnosticError> {
        let items: Vec<u8> = request
            .items
            .iter()
            .map(|item| parse_byte(item))
            .collect::<Result<_, _>>()?;
        let (protocol, operation) = match request.kind {
            StandardObdReadKind::CurrentData => {
                let pids: &[u8] = if items.is_empty() { &[0x00] } else { &items };
                (
                    J1979Request::current_data(pids).map_err(codec_refusal)?,
                    format!("Current data: {}", hex_list(pids)),
                )
            }
            StandardObdReadKind::FreezeFrame => {
                let pid = items.first().copied().unwrap_or(0x02);
                (
                    J1979Request::freeze_frame(pid, 0),
                    format!("Freeze frame 0: PID {pid:#04X}"),
                )
            }
            StandardObdReadKind::StoredDtcs => {
                (J1979Request::stored_dtcs(), "Stored fault codes".into())
            }
            StandardObdReadKind::PendingDtcs => {
                (J1979Request::pending_dtcs(), "Pending fault codes".into())
            }
            StandardObdReadKind::PermanentDtcs => (
                J1979Request::permanent_dtcs(),
                "Permanent fault codes".into(),
            ),
            StandardObdReadKind::MonitorResults => {
                let mids: &[u8] = if items.is_empty() { &[0x00] } else { &items };
                (
                    J1979Request::monitor_results(mids).map_err(codec_refusal)?,
                    format!("Monitor results: {}", hex_list(mids)),
                )
            }
            StandardObdReadKind::VehicleInformation => {
                let info_type = items.first().copied().unwrap_or(0x02);
                (
                    J1979Request::vehicle_information(info_type).map_err(codec_refusal)?,
                    format!("Vehicle information: InfoType {info_type:#04X}"),
                )
            }
        };
        let responder = StandardObdResponder::new(request.responder)
            .map_err(|error| preparation_error(format!("{error}")))?;
        let transaction = prepare_standard_obd_transaction(protocol, responder)
            .map_err(|error| preparation_error(error.to_string()))?;
        Ok(PreparedStandardObd {
            request: protocol,
            transaction,
            operation,
        })
    }

    /// Record the outcome: the answer decoded with the codec, the fault codes
    /// worded from the library, a refusal kept as the module's own reason.
    pub fn finish(
        &mut self,
        request: &StandardObdRequest,
        prepared: Option<&PreparedStandardObd>,
        adapter: Option<&AdapterInfo>,
        library: Option<&KnowledgeLibrary>,
        result: Result<MongooseJ1979ReadResult, DiagnosticError>,
    ) -> StandardObdSnapshot {
        let mut snapshot = idle();
        snapshot.kind = Some(request.kind);
        if let Some(prepared) = prepared {
            snapshot.operation = prepared.operation.clone();
            snapshot.responder = format!(
                "{:#05X} → {:#05X}",
                prepared.transaction.physical_request_id().value(),
                prepared.transaction.expected_response_id().value()
            );
            snapshot.request_hex = hex(prepared.transaction.encoded_payload());
            snapshot.route_validation = "SOURCE_BACKED".into();
        }

        let raw = match (result, prepared) {
            (Ok(raw), Some(prepared)) => {
                snapshot.raw_response_hex = Some(hex(&raw.raw_diagnostic_response));
                snapshot.pending_responses = raw.pending_responses;
                match decode_response(prepared.request, &raw.raw_diagnostic_response) {
                    Ok(response) => {
                        snapshot.state = ModuleReadState::Succeeded;
                        fill(
                            &mut snapshot,
                            response,
                            library,
                            &vehicle_context(&request.context),
                        );
                    }
                    Err(J1979Error::NegativeResponse { code, .. }) => {
                        snapshot.state = ModuleReadState::Succeeded;
                        snapshot.negative_response =
                            Some(format!("{code:#04X} {}", negative_response_text(code)));
                    }
                    Err(error) => {
                        snapshot.state = ModuleReadState::Failed;
                        snapshot.error = Some(DiagnosticError {
                            category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
                            message: "Malformed diagnostic response".into(),
                            technical_details: Some(error.to_string()),
                            stage: DiagnosticExecutionStage::DiagnosticDecode,
                        });
                    }
                }
                Some(raw)
            }
            (Ok(_), None) => {
                snapshot.state = ModuleReadState::Failed;
                snapshot.error = Some(DiagnosticError {
                    category: DiagnosticErrorCategory::InternalFailure,
                    message: "Diagnostic execution failed".into(),
                    technical_details: Some("a result arrived without a prepared read".into()),
                    stage: DiagnosticExecutionStage::Preparation,
                });
                None
            }
            (Err(error), _) => {
                snapshot.state = ModuleReadState::Failed;
                snapshot.error = Some(error);
                None
            }
        };
        snapshot.report_available = true;
        let report = build_report(request, adapter, &snapshot, raw.as_ref());
        self.last = Some((snapshot.clone(), report));
        snapshot
    }

    /// The last read was answered by the bench (ADR-0020): `SYNTHETIC` in
    /// the snapshot and in the report.
    pub fn mark_synthetic(&mut self) -> StandardObdSnapshot {
        if let Some((snapshot, report)) = self.last.as_mut() {
            snapshot.route_validation = "SYNTHETIC".into();
            report["route_validation"] = Value::String("SYNTHETIC".into());
            if let Some(result) = report.get_mut("result") {
                result["routeValidation"] = Value::String("SYNTHETIC".into());
            }
        }
        self.snapshot()
    }

    pub fn report_json(&self) -> Result<String, String> {
        let (_, report) = self
            .last
            .as_ref()
            .ok_or_else(|| "No standard OBD-II read is available yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }
}

fn fill(
    snapshot: &mut StandardObdSnapshot,
    response: J1979Response,
    library: Option<&KnowledgeLibrary>,
    context: &VehicleContext,
) {
    match response {
        J1979Response::CurrentData(values) => {
            for value in values {
                if let ParameterValue::Supported(items) = &value.value {
                    snapshot
                        .supported
                        .extend(items.iter().map(|item| format!("{item:#04X}")));
                }
                snapshot.values.push(standard_value(value));
            }
        }
        J1979Response::FreezeFrame { frame, parameters } => {
            snapshot.freeze_frame = Some(frame);
            for value in parameters {
                // The frame has support maps of its own; the walk reads them first.
                if let ParameterValue::Supported(items) = &value.value {
                    snapshot
                        .supported
                        .extend(items.iter().map(|item| format!("{item:#04X}")));
                }
                snapshot.values.push(standard_value(value));
            }
        }
        J1979Response::Dtcs { kind, codes } => {
            snapshot.dtc_kind = Some(
                match kind {
                    DtcKind::Stored => "stored",
                    DtcKind::Pending => "pending",
                    DtcKind::Permanent => "permanent",
                }
                .into(),
            );
            snapshot.dtcs = codes
                .into_iter()
                .map(|code| {
                    // The legislated codes are the standard's; the library's
                    // programme-independent wording applies, and so does ours.
                    // The legislated codes belong to the engine controller;
                    // the help SDD holds for one is chosen for this car too.
                    let described = library
                        .map(|library| library.describe_dtc_with_help(&code, 0, "PCM", context))
                        .unwrap_or_default();
                    DtcSummary {
                        code,
                        failure_type: String::new(),
                        status: String::new(),
                        description: described.description,
                        description_scope: described.description_scope,
                        failure_type_text: None,
                        failure_type_texts: Default::default(),
                        description_texts: described.description_texts,
                        help: described.help,
                        help_note: described.help_note,
                    }
                })
                .collect();
        }
        J1979Response::MonitorResults(results) => {
            for result in results {
                match result {
                    MonitorResult::Supported { mids, .. } => {
                        snapshot
                            .supported
                            .extend(mids.iter().map(|mid| format!("{mid:#04X}")));
                    }
                    MonitorResult::Result {
                        mid,
                        monitor,
                        tid,
                        uas,
                        unit,
                        value,
                        minimum,
                        maximum,
                        raw_value,
                        raw_minimum,
                        raw_maximum,
                        passed,
                    } => snapshot.monitors.push(StandardObdMonitor {
                        mid: format!("{mid:#04X}"),
                        monitor,
                        tid: format!("{tid:#04X}"),
                        uas: format!("{uas:#04X}"),
                        unit: unit.to_string(),
                        value,
                        minimum,
                        maximum,
                        raw_value,
                        raw_minimum,
                        raw_maximum,
                        passed,
                    }),
                }
            }
        }
        J1979Response::VehicleInformation(information) => {
            snapshot.information = match information {
                VehicleInformation::Supported(items) => {
                    snapshot
                        .supported
                        .extend(items.iter().map(|item| format!("{item:#04X}")));
                    Vec::new()
                }
                VehicleInformation::Vin(vin) => vec![labelled("VIN", vin)],
                VehicleInformation::CalibrationIds(ids) => ids
                    .into_iter()
                    .map(|id| labelled("Calibration ID", id))
                    .collect(),
                VehicleInformation::CalibrationVerificationNumbers(cvns) => cvns
                    .into_iter()
                    .map(|cvn| labelled("Calibration verification number", cvn))
                    .collect(),
                VehicleInformation::InUsePerformance(counters) => counters
                    .into_iter()
                    .map(|(name, count)| labelled(&name, count.to_string()))
                    .collect(),
                VehicleInformation::EcuName(name) => vec![labelled("ECU name", name)],
                VehicleInformation::EcuSerialNumber(serial) => {
                    vec![labelled("ECU serial number", serial)]
                }
            };
        }
        J1979Response::CalibrationIdentification(result) => {
            snapshot.information = result
                .calibration_ids
                .into_iter()
                .map(|id| labelled("Calibration ID", id))
                .collect();
        }
    }
}

fn standard_value(value: obd_j1979::DecodedParameter) -> StandardObdValue {
    let (kind, text, number) = match &value.value {
        ParameterValue::Number(number) => ("number", Some(number_text(*number)), Some(*number)),
        ParameterValue::Text(text) => ("text", Some(text.clone()), None),
        ParameterValue::Flag(flag) => ("flag", Some(if *flag { "yes" } else { "no" }.into()), None),
        ParameterValue::Supported(items) => ("supported", Some(hex_list(items)), None),
        ParameterValue::Raw => ("raw", None, None),
    };
    StandardObdValue {
        pid: format!("{:#04X}", value.pid),
        name: value.name,
        unit: value.unit.to_string(),
        value: text,
        number,
        raw_hex: hex(&value.raw),
        kind: kind.into(),
    }
}

fn labelled(label: &str, value: String) -> LabelledValue {
    LabelledValue {
        label: label.to_string(),
        value,
    }
}

fn number_text(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e12 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn hex_list(items: &[u8]) -> String {
    items
        .iter()
        .map(|item| format!("{item:#04X}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_byte(text: &str) -> Result<u8, DiagnosticError> {
    let trimmed = text.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).ok()
    } else if trimmed.chars().all(|c| c.is_ascii_hexdigit()) && trimmed.len() == 2 {
        u8::from_str_radix(trimmed, 16).ok()
    } else {
        trimmed.parse::<u8>().ok()
    };
    parsed.ok_or_else(|| preparation_error(format!("'{text}' is not a PID, MID or InfoType byte")))
}

fn codec_refusal(error: J1979Error) -> DiagnosticError {
    preparation_error(error.to_string())
}

fn preparation_error(details: String) -> DiagnosticError {
    DiagnosticError {
        category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
        message: "The standard OBD-II request cannot be prepared".into(),
        technical_details: Some(details),
        stage: DiagnosticExecutionStage::Preparation,
    }
}

fn idle() -> StandardObdSnapshot {
    StandardObdSnapshot {
        state: ModuleReadState::Idle,
        kind: None,
        responder: String::new(),
        operation: String::new(),
        route_validation: String::new(),
        request_hex: String::new(),
        raw_response_hex: None,
        supported: Vec::new(),
        values: Vec::new(),
        dtc_kind: None,
        dtcs: Vec::new(),
        freeze_frame: None,
        monitors: Vec::new(),
        information: Vec::new(),
        negative_response: None,
        pending_responses: 0,
        error: None,
        report_available: false,
    }
}

fn build_report(
    request: &StandardObdRequest,
    adapter: Option<&AdapterInfo>,
    snapshot: &StandardObdSnapshot,
    raw: Option<&MongooseJ1979ReadResult>,
) -> Value {
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    json!({
        "schema": "prowlone.standard-obd-read",
        "schema_version": 1,
        "application_version": env!("CARGO_PKG_VERSION"),
        "timestamp_unix_ms": timestamp_unix_ms,
        "session_id": format!("obd-{timestamp_unix_ms}-{counter}"),
        "execution_source": "LIVE_MONGOOSE",
        "adapter_model": adapter.map(|value| value.name.clone()).unwrap_or_else(|| "MongoosePro JLR".into()),
        "usb_vid": adapter.map(|value| format!("{:04X}", value.usb_vid)).unwrap_or_else(|| "18E1".into()),
        "usb_pid": adapter.map(|value| format!("{:04X}", value.usb_pid)).unwrap_or_else(|| "0104".into()),
        "vehicle": request.context,
        "request": request,
        "standard": "SAE J1979 / ISO 15765-4, legislated OBD-II, read-only",
        "responder": snapshot.responder,
        "request_hex": snapshot.request_hex,
        "raw_response_hex": raw.map(|value| hex(&value.raw_diagnostic_response)),
        "pending_responses": raw.map(|value| value.pending_responses).unwrap_or(0),
        "route_validation": snapshot.route_validation,
        "validation": "a legislated answer proves the responder answered at the standard's address; the meaning of a value is the standard's, never ours",
        "result": snapshot,
    })
}
