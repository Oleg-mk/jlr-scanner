//! Live read of one module (ADR-0015): resolve the plan from the loaded
//! library, prepare the transaction, hand it to the adapter, decode the
//! answer, keep a report. The work happens in the crates; this file holds
//! the last result and the wording.

use crate::diagnostic_service::map_mongoose_error;
use app_contracts::{
    AdapterInfo, DecodedParameterSummary, DiagnosticError, DiagnosticErrorCategory,
    DiagnosticExecutionStage, DtcSummary, ModuleReadKind, ModuleReadReport, ModuleReadRequest,
    ModuleReadSnapshot, ModuleReadState,
};
use diagnostic_environment::{DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver};
use diagnostic_session::decode::decode_parameters;
use diagnostic_session::{validation_label, vehicle_context, KnowledgeLibrary};
use mongoose_jlr::{MongooseDiagnosticError, MongooseUdsReadResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uds_execution::{
    decode_response, prepare_read_only_transaction, DiagnosticTargetIdentity, DtcStatusMask,
    PreparedUdsTransaction, ReadOnlyUdsIntent, UdsReadOutcome, READ_DATA_BY_IDENTIFIER_CAPABILITY,
    READ_DTC_INFORMATION_CAPABILITY,
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A read the library could plan: the typed transaction plus what the UI
/// shows about it before anything is sent.
#[derive(Debug)]
pub struct PreparedModuleRead {
    pub transaction: PreparedUdsTransaction,
    pub operation: String,
    pub route_validation: String,
}

#[derive(Default)]
pub struct ModuleReadService {
    last: Option<(ModuleReadSnapshot, ModuleReadReport)>,
}

impl ModuleReadService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> ModuleReadSnapshot {
        self.last
            .as_ref()
            .map(|(snapshot, _)| snapshot.clone())
            .unwrap_or_else(idle)
    }

    /// Resolve and prepare without touching the adapter. Every refusal is a
    /// `DiagnosticError` in the preparation stage with the resolver's words.
    pub fn prepare(
        library: &KnowledgeLibrary,
        request: &ModuleReadRequest,
    ) -> Result<PreparedModuleRead, DiagnosticError> {
        let context = vehicle_context(&request.context);
        let (capability, operation) = match request.kind {
            ModuleReadKind::FaultCodes => (
                READ_DTC_INFORMATION_CAPABILITY,
                "Read confirmed fault codes".to_string(),
            ),
            ModuleReadKind::Identifier => (
                READ_DATA_BY_IDENTIFIER_CAPABILITY,
                format!(
                    "Read identifier {}",
                    request.identifier.as_deref().unwrap_or("(none)")
                ),
            ),
        };
        let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
            library.store(),
            &context,
            &request.ecu_family,
            capability,
        )
        .map_err(|error| preparation_error(error.to_string()))?;
        let route_validation = match &resolution {
            DiagnosticEnvironmentResolution::Resolved(plan) => {
                validation_label(plan.validation_state).to_string()
            }
            DiagnosticEnvironmentResolution::Indeterminate {
                unresolved_facts, ..
            } => {
                let facts: Vec<String> = unresolved_facts
                    .iter()
                    .map(|fact| format!("{:?}: {}", fact.field, fact.reason))
                    .collect();
                return Err(preparation_error(format!(
                    "the route to {} cannot be resolved from the loaded data: {}",
                    request.ecu_family,
                    facts.join("; ")
                )));
            }
            DiagnosticEnvironmentResolution::Conflict { conflicts, .. } => {
                let fields: Vec<String> = conflicts
                    .iter()
                    .map(|conflict| format!("{:?}", conflict.field))
                    .collect();
                return Err(preparation_error(format!(
                    "the loaded data disagrees about {} for {}",
                    fields.join(", "),
                    request.ecu_family
                )));
            }
        };

        let target = DiagnosticTargetIdentity::family(&request.ecu_family)
            .map_err(|error| preparation_error(error.to_string()))?;
        let intent = match request.kind {
            ModuleReadKind::FaultCodes => {
                ReadOnlyUdsIntent::read_dtc_information(target, DtcStatusMask::Confirmed)
            }
            ModuleReadKind::Identifier => {
                let identifier = parse_identifier(request.identifier.as_deref())?;
                ReadOnlyUdsIntent::read_data_by_identifier(target, identifier)
            }
        };
        let readable = DiagnosticEnvironmentResolver::readable_identifiers(
            library.store(),
            &context,
            &request.ecu_family,
        );
        let transaction = prepare_read_only_transaction(&resolution, intent, &readable)
            .map_err(|error| preparation_error(error.to_string()))?;
        Ok(PreparedModuleRead {
            transaction,
            operation,
            route_validation,
        })
    }

    pub fn finish(
        &mut self,
        request: &ModuleReadRequest,
        prepared: Option<&PreparedModuleRead>,
        adapter: Option<&AdapterInfo>,
        library: Option<&KnowledgeLibrary>,
        result: Result<MongooseUdsReadResult, DiagnosticError>,
    ) -> ModuleReadSnapshot {
        let mut snapshot = idle();
        snapshot.ecu_family = request.ecu_family.clone();
        if let Some(prepared) = prepared {
            snapshot.operation = prepared.operation.clone();
            snapshot.route_id = prepared.transaction.backend_route().to_string();
            snapshot.route_validation = prepared.route_validation.clone();
            snapshot.request_hex = hex(prepared.transaction.encoded_payload());
        }

        let outcome: Result<(MongooseUdsReadResult, UdsReadOutcome), DiagnosticError> =
            match (result, prepared) {
                (Ok(raw), Some(prepared)) => {
                    match decode_response(&prepared.transaction, &raw.raw_diagnostic_response) {
                        Ok(Some(outcome)) => Ok((raw, outcome)),
                        Ok(None) => Err(DiagnosticError {
                            category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
                            message: "Malformed diagnostic response".into(),
                            technical_details: Some(
                                "the final answer was still ResponsePending".into(),
                            ),
                            stage: DiagnosticExecutionStage::DiagnosticDecode,
                        }),
                        Err(error) => Err(DiagnosticError {
                            category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
                            message: "Malformed diagnostic response".into(),
                            technical_details: Some(error.to_string()),
                            stage: DiagnosticExecutionStage::DiagnosticDecode,
                        }),
                    }
                }
                (Ok(_), None) => Err(DiagnosticError {
                    category: DiagnosticErrorCategory::InternalFailure,
                    message: "Diagnostic execution failed".into(),
                    technical_details: Some("a result arrived without a prepared read".into()),
                    stage: DiagnosticExecutionStage::Preparation,
                }),
                (Err(error), _) => Err(error),
            };

        let mut decoded_result = None;
        match &outcome {
            Ok((raw, decoded)) => {
                snapshot.state = ModuleReadState::Succeeded;
                snapshot.responder = Some(format!("0x{:03X}", raw.responder));
                snapshot.raw_response_hex = Some(hex(&raw.raw_diagnostic_response));
                snapshot.pending_responses = raw.pending_responses;
                match decoded {
                    UdsReadOutcome::DataByIdentifier { identifier, data } => {
                        snapshot.data_hex = Some(hex(data));
                        let parameters = prepared
                            .and_then(|value| value.transaction.readable_identifier())
                            .map(|identifier| identifier.parameters.clone())
                            .unwrap_or_default();
                        snapshot.parameters = decode_parameters(&parameters, data)
                            .into_iter()
                            .map(|decoded| DecodedParameterSummary {
                                name: decoded.name,
                                raw: decoded.raw,
                                value: decoded.value,
                                unit: decoded.unit,
                                state: decoded.state,
                                note: decoded.note,
                            })
                            .collect();
                        decoded_result = Some(format!(
                            "identifier 0x{identifier:04X}: {} ({} bytes)",
                            hex(data),
                            data.len()
                        ));
                    }
                    UdsReadOutcome::DtcReport(report) => {
                        snapshot.dtcs = report
                            .records
                            .iter()
                            .map(|record| {
                                let code = record.j2012_code();
                                let described = library
                                    .map(|library| {
                                        library.describe_dtc_with_help(
                                            &code,
                                            record.failure_type_byte(),
                                            &request.ecu_family,
                                            &vehicle_context(&request.context),
                                        )
                                    })
                                    .unwrap_or_default();
                                DtcSummary {
                                    code,
                                    failure_type: format!("{:02X}", record.failure_type_byte()),
                                    status: format!("{:02X}", record.status),
                                    description: described.description,
                                    description_scope: described.description_scope,
                                    failure_type_text: described.failure_type_text,
                                    failure_type_texts: described.failure_type_texts,
                                    description_texts: described.description_texts,
                                    help: described.help,
                                    help_texts: described.help_texts,
                                    help_note: described.help_note,
                                }
                            })
                            .collect();
                        decoded_result = Some(format!(
                            "{} fault code(s), availability mask 0x{:02X}",
                            report.records.len(),
                            report.status_availability_mask
                        ));
                    }
                    UdsReadOutcome::Negative(negative) => {
                        snapshot.negative_response = Some(format!("{:?}", negative.code));
                        decoded_result = Some(format!(
                            "negative response {:?} to service 0x{:02X}",
                            negative.code, negative.request_service_id
                        ));
                    }
                }
            }
            Err(error) => {
                snapshot.state = ModuleReadState::Failed;
                snapshot.error = Some(error.clone());
            }
        }
        snapshot.report_available = true;

        let report = build_report(
            request,
            prepared,
            adapter,
            &snapshot,
            outcome.as_ref().ok().map(|(raw, _)| raw),
            decoded_result,
        );
        self.last = Some((snapshot.clone(), report));
        snapshot
    }

    /// The last read was answered by the bench (ADR-0020): its validation is
    /// `SYNTHETIC`, in the snapshot and in the report, whatever the route's
    /// own state was.
    pub fn mark_synthetic(&mut self) -> ModuleReadSnapshot {
        if let Some((snapshot, report)) = self.last.as_mut() {
            snapshot.route_validation = "SYNTHETIC".into();
            report.route_validation = "SYNTHETIC".into();
        }
        self.snapshot()
    }

    pub fn report_json(&self) -> Result<String, String> {
        let (_, report) = self
            .last
            .as_ref()
            .ok_or_else(|| "No module read report is available yet".to_owned())?;
        serde_json::to_string_pretty(report).map_err(|error| error.to_string())
    }
}

pub fn map_live_error(error: MongooseDiagnosticError) -> DiagnosticError {
    map_mongoose_error(error)
}

pub fn adapter_unavailable() -> DiagnosticError {
    DiagnosticError {
        category: DiagnosticErrorCategory::AdapterNotFound,
        message: "Adapter not found".into(),
        technical_details: Some("connect and verify the adapter before reading a module".into()),
        stage: DiagnosticExecutionStage::AdapterValidation,
    }
}

fn idle() -> ModuleReadSnapshot {
    ModuleReadSnapshot {
        state: ModuleReadState::Idle,
        ecu_family: String::new(),
        operation: String::new(),
        route_id: String::new(),
        route_validation: String::new(),
        request_hex: String::new(),
        responder: None,
        raw_response_hex: None,
        data_hex: None,
        parameters: Vec::new(),
        dtcs: Vec::new(),
        negative_response: None,
        pending_responses: 0,
        error: None,
        report_available: false,
    }
}

fn preparation_error(details: String) -> DiagnosticError {
    DiagnosticError {
        category: DiagnosticErrorCategory::UnsupportedVehicleProfile,
        message: "The module cannot be read with the loaded data".into(),
        technical_details: Some(details),
        stage: DiagnosticExecutionStage::Preparation,
    }
}

fn parse_identifier(text: Option<&str>) -> Result<u16, DiagnosticError> {
    let text = text
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| preparation_error("no identifier was chosen".into()))?;
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u16::from_str_radix(digits, 16)
        .map_err(|_| preparation_error(format!("'{text}' is not a 16-bit identifier")))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_report(
    request: &ModuleReadRequest,
    prepared: Option<&PreparedModuleRead>,
    adapter: Option<&AdapterInfo>,
    snapshot: &ModuleReadSnapshot,
    raw: Option<&MongooseUdsReadResult>,
    decoded_result: Option<String>,
) -> ModuleReadReport {
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let transaction = prepared.map(|value| &value.transaction);
    ModuleReadReport {
        schema_version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        timestamp_unix_ms,
        session_id: format!("f10-{timestamp_unix_ms}-{counter}"),
        execution_source: "LIVE_MONGOOSE".into(),
        adapter_model: adapter
            .map(|value| value.name.clone())
            .unwrap_or_else(|| "MongoosePro JLR".into()),
        usb_vid: adapter
            .map(|value| format!("{:04X}", value.usb_vid))
            .unwrap_or_else(|| "18E1".into()),
        usb_pid: adapter
            .map(|value| format!("{:04X}", value.usb_pid))
            .unwrap_or_else(|| "0104".into()),
        adapter_board_info_result: adapter
            .map(|value| format!("OK / {}", value.board_info.response_command))
            .unwrap_or_else(|| "NOT_AVAILABLE".into()),
        vehicle: request.context.clone(),
        ecu_family: request.ecu_family.clone(),
        operation: snapshot.operation.clone(),
        logical_bus: transaction
            .map(|value| value.logical_network().to_string())
            .unwrap_or_default(),
        backend_route: snapshot.route_id.clone(),
        physical_route: transaction
            .map(|value| {
                format!(
                    "{} pins {}",
                    value.physical_connector(),
                    value
                        .physical_pins()
                        .iter()
                        .map(u8::to_string)
                        .collect::<Vec<_>>()
                        .join("/")
                )
            })
            .unwrap_or_default(),
        bitrate_bps: transaction.map(|value| value.bitrate_bps()),
        route_validation: snapshot.route_validation.clone(),
        protocol: transaction
            .map(|value| value.protocol_family().to_string())
            .unwrap_or_default(),
        request_id: transaction
            .map(|value| format!("0x{:03X}", value.physical_request_id().value()))
            .unwrap_or_default(),
        expected_response_id: transaction
            .map(|value| format!("0x{:03X}", value.expected_response_id().value()))
            .unwrap_or_default(),
        capability: transaction
            .map(|value| value.capability_id().to_string())
            .unwrap_or_default(),
        request_payload: snapshot.request_hex.clone(),
        actual_responder: snapshot.responder.clone(),
        raw_diagnostic_response: raw.map(|value| hex(&value.raw_diagnostic_response)),
        decoded_result,
        pending_responses: raw.map(|value| value.pending_responses).unwrap_or(0),
        timeout_or_error_category: snapshot.error.as_ref().map(|value| value.category),
        execution_stage: snapshot
            .error
            .as_ref()
            .map(|value| value.stage)
            .unwrap_or(DiagnosticExecutionStage::Completed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_contracts::VehicleContextInput;
    use diagnostic_environment::{ReadableIdentifier, ValidationState};
    use jlr_profiles::{x250_2010_supercharged_ecm_environment, ECU_FAMILY};
    use mongoose_jlr::VehicleRouteId;
    use uds_execution::UDS_PROTOCOL_FAMILY;

    fn request(kind: ModuleReadKind) -> ModuleReadRequest {
        ModuleReadRequest {
            ecu_family: ECU_FAMILY.into(),
            kind,
            identifier: Some("0x1945".into()),
            context: VehicleContextInput {
                vehicle_program: "PROG".into(),
                model_year: Some(2010),
                ..VehicleContextInput::default()
            },
        }
    }

    /// A prepared read built on the F8 profile's route evidence with the
    /// fields a UDS read needs, so that decoding and reporting can be tested
    /// without a library.
    fn prepared(kind: ModuleReadKind) -> PreparedModuleRead {
        let DiagnosticEnvironmentResolution::Resolved(mut plan) =
            x250_2010_supercharged_ecm_environment()
        else {
            unreachable!()
        };
        plan.protocol_family.value = UDS_PROTOCOL_FAMILY.into();
        plan.addressing_mode.value = "normal".into();
        let target = DiagnosticTargetIdentity::family(ECU_FAMILY).unwrap();
        let (intent, capability, operation) = match kind {
            ModuleReadKind::FaultCodes => (
                ReadOnlyUdsIntent::read_dtc_information(target, DtcStatusMask::Confirmed),
                READ_DTC_INFORMATION_CAPABILITY,
                "Read confirmed fault codes",
            ),
            ModuleReadKind::Identifier => (
                ReadOnlyUdsIntent::read_data_by_identifier(target, 0x1945),
                READ_DATA_BY_IDENTIFIER_CAPABILITY,
                "Read identifier 0x1945",
            ),
        };
        plan.read_only_capability.value.id = capability.into();
        let readable = ReadableIdentifier {
            identifier: 0x1945,
            parameters: Vec::new(),
            evidence: Vec::new(),
            validation_state: ValidationState::SourceBacked,
        };
        PreparedModuleRead {
            transaction: prepare_read_only_transaction(
                &DiagnosticEnvironmentResolution::Resolved(plan),
                intent,
                &[readable],
            )
            .unwrap(),
            operation: operation.into(),
            route_validation: "SOURCE_BACKED".into(),
        }
    }

    fn raw(bytes: &[u8]) -> MongooseUdsReadResult {
        MongooseUdsReadResult {
            route: VehicleRouteId::HsCan,
            responder: 0x7E8,
            request_payload: vec![0x19, 0x02, 0x08],
            raw_diagnostic_response: bytes.to_vec(),
            pending_responses: 1,
        }
    }

    #[test]
    fn a_library_without_the_module_refuses_at_preparation_with_the_resolver_words() {
        let library = KnowledgeLibrary::built_in();
        let error =
            ModuleReadService::prepare(&library, &request(ModuleReadKind::FaultCodes)).unwrap_err();
        assert_eq!(error.stage, DiagnosticExecutionStage::Preparation);
        assert_eq!(
            error.category,
            DiagnosticErrorCategory::UnsupportedVehicleProfile
        );
        assert!(error
            .technical_details
            .as_deref()
            .unwrap()
            .contains("cannot be resolved"));
    }

    #[test]
    fn a_dtc_report_is_decoded_into_codes_and_reported() {
        let mut service = ModuleReadService::new();
        assert_eq!(service.snapshot().state, ModuleReadState::Idle);
        let prepared = prepared(ModuleReadKind::FaultCodes);
        let snapshot = service.finish(
            &request(ModuleReadKind::FaultCodes),
            Some(&prepared),
            None,
            None,
            Ok(raw(&[
                0x59, 0x02, 0xFF, 0x03, 0x01, 0x00, 0x08, 0xC1, 0x23, 0x45, 0x2F,
            ])),
        );
        assert_eq!(snapshot.state, ModuleReadState::Succeeded);
        assert_eq!(snapshot.route_id, "hs-can");
        assert_eq!(snapshot.request_hex, "19 02 08");
        assert_eq!(snapshot.responder.as_deref(), Some("0x7E8"));
        assert_eq!(snapshot.pending_responses, 1);
        let codes: Vec<_> = snapshot
            .dtcs
            .iter()
            .map(|dtc| format!("{}-{} {}", dtc.code, dtc.failure_type, dtc.status))
            .collect();
        assert_eq!(codes, vec!["P0301-00 08", "U0123-45 2F"]);
        assert!(snapshot.report_available);

        let report: ModuleReadReport =
            serde_json::from_str(&service.report_json().unwrap()).unwrap();
        assert_eq!(report.execution_source, "LIVE_MONGOOSE");
        assert_eq!(report.request_payload, "19 02 08");
        assert_eq!(report.route_validation, "SOURCE_BACKED");
        assert_eq!(
            report.decoded_result.as_deref(),
            Some("2 fault code(s), availability mask 0xFF")
        );
        assert_eq!(report.execution_stage, DiagnosticExecutionStage::Completed);
    }

    #[test]
    fn a_negative_response_is_a_completed_read_that_says_so() {
        let mut service = ModuleReadService::new();
        let prepared = prepared(ModuleReadKind::Identifier);
        let snapshot = service.finish(
            &request(ModuleReadKind::Identifier),
            Some(&prepared),
            None,
            None,
            Ok(raw(&[0x7F, 0x22, 0x31])),
        );
        assert_eq!(snapshot.state, ModuleReadState::Succeeded);
        assert_eq!(
            snapshot.negative_response.as_deref(),
            Some("RequestOutOfRange")
        );
        assert!(snapshot.data_hex.is_none());
    }

    #[test]
    fn silence_is_reported_as_no_response_with_the_route_still_named() {
        let mut service = ModuleReadService::new();
        let prepared = prepared(ModuleReadKind::Identifier);
        let snapshot = service.finish(
            &request(ModuleReadKind::Identifier),
            Some(&prepared),
            None,
            None,
            Err(map_live_error(MongooseDiagnosticError::Timeout)),
        );
        assert_eq!(snapshot.state, ModuleReadState::Failed);
        assert_eq!(
            snapshot.error.as_ref().unwrap().category,
            DiagnosticErrorCategory::NoResponseFromEcu
        );
        assert_eq!(snapshot.route_id, "hs-can");
        let report: ModuleReadReport =
            serde_json::from_str(&service.report_json().unwrap()).unwrap();
        assert_eq!(
            report.timeout_or_error_category,
            Some(DiagnosticErrorCategory::NoResponseFromEcu)
        );
    }
}
