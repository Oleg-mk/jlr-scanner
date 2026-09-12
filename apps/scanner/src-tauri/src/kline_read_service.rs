//! Reading a module on a K-line (`ADR-0029` slice B).
//!
//! The shell's side of the serial lines, beside `module_read_service` rather
//! than inside it: a K-line read is planned from the same survey, shown in
//! the same panel and recorded in the same report shape, but it resolves
//! against a different protocol, carries no CAN identifier, and comes back as
//! bytes rather than as an ISO-TP payload.
//!
//! What it offers is what `ADR-0029` allows and nothing else: on DS2, the
//! identification and the fault memory; on KWP2000, the fault codes. A
//! KWP2000 identification waits for the local identifiers SDD states per
//! module, which this library does not carry yet, and is refused rather than
//! guessed at with a default option.

use crate::module_read_service::{hex, preparation_error};
use crate::read_record::{serial_read_record, ReadIdentity, SerialOutcome, SerialRead};
use app_contracts::ModuleReadReport;
use app_contracts::{
    AdapterInfo, DecodedParameterSummary, DiagnosticError, DiagnosticErrorCategory,
    DiagnosticExecutionStage, DtcSummary, ModuleReadKind, ModuleReadRequest, ModuleReadSnapshot,
    ModuleReadState,
};
use diagnostic_environment::{DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver};
use diagnostic_session::{validation_label, vehicle_context, KnowledgeLibrary};
use kline_execution::{
    decode_response, prepare_read_only_kline_read, KlineReadOutcome, KlineTarget,
    PreparedKlineTransaction, ReadOnlyKlineIntent,
};
use mongoose_jlr::MongooseKlineReadResult;
use std::time::Duration;

/// How long one module is given to answer on a serial line. A K-line module
/// answers in tens of milliseconds; a second is generous.
pub const KLINE_READ_TIMEOUT: Duration = Duration::from_secs(1);
pub const KLINE_OPERATION_IDENTIFICATION: &str = "KLINE_IDENTIFICATION";
pub const KLINE_OPERATION_FAULT_MEMORY: &str = "KLINE_FAULT_MEMORY";

/// A K-line read the library could plan.
pub struct PreparedKlineRead {
    pub transaction: PreparedKlineTransaction,
    pub operation: String,
    pub operation_id: &'static str,
    pub route_validation: String,
}

/// Planning and reading a K-line, with no state of its own: the snapshot
/// and the record go to the module read service, which is where the panel
/// and the session report already look.
pub struct KlineReadService;

impl KlineReadService {
    /// Plan a K-line read, or say this module is not one.
    ///
    /// `None` means the loaded data places this module on no K-line protocol
    /// this product speaks — the caller then reads it the way it reads every
    /// CAN module. `Some(Err(_))` is a module that is on a K-line and still
    /// cannot be read, with the reason.
    pub fn prepare(
        library: &KnowledgeLibrary,
        request: &ModuleReadRequest,
    ) -> Option<Result<PreparedKlineRead, DiagnosticError>> {
        let context = vehicle_context(&request.context);
        let target = match KlineTarget::family(&request.ecu_family) {
            Ok(target) => target,
            Err(error) => return Some(Err(preparation_error(error.to_string()))),
        };

        // Which protocol this module speaks on its line is decided by which
        // capability the data gives it, not by a guess.
        let candidates: Vec<(&str, ReadOnlyKlineIntent, &'static str, &str)> = match request.kind {
            ModuleReadKind::FaultCodes => [
                (
                    ds2::FAULT_MEMORY_CAPABILITY,
                    ReadOnlyKlineIntent::Ds2FaultMemory {
                        target: target.clone(),
                    },
                    KLINE_OPERATION_FAULT_MEMORY,
                    "Read the DS2 fault memory",
                ),
                (
                    kwp2000::READ_DTC_BY_STATUS_CAPABILITY,
                    ReadOnlyKlineIntent::KwpDtcByStatus {
                        target: target.clone(),
                        // ISO 14230's own "all identified faults" mask.
                        status_mask: 0x00,
                    },
                    KLINE_OPERATION_FAULT_MEMORY,
                    "Read the KWP2000 fault codes",
                ),
            ]
            .to_vec(),
            ModuleReadKind::Identifier => [(
                ds2::ECU_IDENTIFICATION_CAPABILITY,
                ReadOnlyKlineIntent::Ds2Identification {
                    target: target.clone(),
                },
                KLINE_OPERATION_IDENTIFICATION,
                "Read the DS2 identification",
            )]
            .to_vec(),
        };

        for (capability, intent, operation_id, operation) in &candidates {
            let Ok(resolution) = DiagnosticEnvironmentResolver::resolve_ecu_family(
                library.store(),
                &context,
                &request.ecu_family,
                capability,
            ) else {
                continue;
            };
            let DiagnosticEnvironmentResolution::Resolved(plan) = &resolution else {
                continue;
            };
            let route_validation = validation_label(plan.validation_state).to_string();
            return Some(
                prepare_read_only_kline_read(&resolution, intent.clone())
                    .map(|transaction| PreparedKlineRead {
                        transaction,
                        operation: (*operation).to_string(),
                        operation_id,
                        route_validation,
                    })
                    .map_err(|error| preparation_error(error.to_string())),
            );
        }

        // A KWP2000 module asked for an identification: the service exists
        // and the local identifier SDD states per module does not reach this
        // library yet, so the answer is that, not a default option sent to a
        // car (`ADR-0029`'s rule about guessing).
        if matches!(request.kind, ModuleReadKind::Identifier)
            && DiagnosticEnvironmentResolver::resolve_ecu_family(
                library.store(),
                &context,
                &request.ecu_family,
                kwp2000::READ_DTC_BY_STATUS_CAPABILITY,
            )
            .is_ok_and(|resolution| {
                matches!(resolution, DiagnosticEnvironmentResolution::Resolved(_))
            })
        {
            return Some(Err(preparation_error(
                "a KWP2000 identification is read by a local identifier this module's data does \
                 not state; the fault codes can be read instead"
                    .into(),
            )));
        }
        None
    }

    /// Record what the line carried back, or why nothing did.
    pub fn finish(
        request: &ModuleReadRequest,
        prepared: &PreparedKlineRead,
        adapter: Option<&AdapterInfo>,
        library: Option<&KnowledgeLibrary>,
        synthetic: bool,
        result: Result<MongooseKlineReadResult, DiagnosticError>,
    ) -> (ModuleReadSnapshot, ModuleReadReport) {
        let transaction = &prepared.transaction;
        let mut snapshot = idle();
        snapshot.ecu_family = request.ecu_family.clone();
        snapshot.operation = prepared.operation.clone();
        snapshot.route_id = transaction.backend_route().to_string();
        snapshot.route_validation = if synthetic {
            "SYNTHETIC".into()
        } else {
            prepared.route_validation.clone()
        };
        snapshot.request_hex = hex(transaction.encoded_payload());
        snapshot.responder = Some(format!("node 0x{:02X}", transaction.node_address()));

        let mut decoded_result = None;
        let outcome = match result {
            Ok(raw) => {
                snapshot.raw_response_hex = Some(hex(&raw.raw_line_bytes));
                match decode_response(transaction, &raw.raw_line_bytes) {
                    Ok(decoded) => {
                        snapshot.state = ModuleReadState::Succeeded;
                        decoded_result = Some(describe(&decoded));
                        fill(&mut snapshot, &decoded, request, library);
                        Ok(raw)
                    }
                    Err(error) => {
                        snapshot.state = ModuleReadState::Failed;
                        let error = DiagnosticError {
                            category: DiagnosticErrorCategory::MalformedDiagnosticResponse,
                            message: "Malformed diagnostic response".into(),
                            technical_details: Some(error.to_string()),
                            stage: DiagnosticExecutionStage::DiagnosticDecode,
                        };
                        snapshot.error = Some(error.clone());
                        Err(error)
                    }
                }
            }
            Err(error) => {
                snapshot.state = ModuleReadState::Failed;
                snapshot.error = Some(error.clone());
                Err(error)
            }
        };
        snapshot.report_available = true;

        // The same record a single read leaves everywhere else, so the
        // intake needs to know nothing about K-lines (ADR-0016).
        let identity = ReadIdentity {
            context: &request.context,
            ecu_family: &request.ecu_family,
            operation: prepared.operation_id,
            route_validation: if synthetic {
                "SYNTHETIC"
            } else {
                prepared.route_validation.as_str()
            },
            adapter,
        };
        let line = SerialRead {
            logical_network: transaction.logical_network(),
            connector: transaction.physical_connector(),
            pins: transaction.physical_pins(),
            backend_route: transaction.backend_route(),
            bitrate_bps: transaction.bitrate_bps(),
            protocol_family: transaction.protocol_family(),
            node_address: transaction.node_address(),
            capability: transaction.capability_id(),
            request: transaction.encoded_payload(),
        };
        let record = match &outcome {
            Ok(raw) => serial_read_record(
                identity,
                &line,
                SerialOutcome::Answered(&raw.raw_line_bytes),
                decoded_result,
            ),
            Err(error) => serial_read_record(identity, &line, SerialOutcome::Failed(error), None),
        };
        (snapshot, record)
    }
}

/// What the answer was, in one line for the record.
fn describe(outcome: &KlineReadOutcome) -> String {
    match outcome {
        KlineReadOutcome::Ds2Identification { bytes, text, .. } => {
            format!("identification: {text:?} ({} bytes)", bytes.len())
        }
        KlineReadOutcome::Ds2FaultMemory { bytes, words, .. } => format!(
            "fault memory: {} byte(s), {} 16-bit word(s)",
            bytes.len(),
            words.len()
        ),
        KlineReadOutcome::Ds2NotAccepted { status, .. } => {
            format!("the module did not accept the request; its status byte is 0x{status:02X}")
        }
        KlineReadOutcome::KwpIdentification { option, bytes, .. } => {
            format!("identification 0x{option:02X}: {} byte(s)", bytes.len())
        }
        KlineReadOutcome::KwpDtcs(rows) => format!("{} fault code(s)", rows.len()),
        KlineReadOutcome::KwpNegative { code, text } => {
            format!("negative response 0x{code:02X}: {text}")
        }
    }
}

/// The answer in the snapshot the panel already draws.
fn fill(
    snapshot: &mut ModuleReadSnapshot,
    outcome: &KlineReadOutcome,
    request: &ModuleReadRequest,
    library: Option<&KnowledgeLibrary>,
) {
    match outcome {
        KlineReadOutcome::Ds2Identification { bytes, text, .. } => {
            snapshot.data_hex = Some(hex(bytes));
            snapshot.parameters = vec![DecodedParameterSummary {
                name: "Identification".into(),
                raw: None,
                value: Some(text.clone()),
                unit: None,
                state: None,
                note: (!text.chars().any(|c| c.is_alphanumeric()))
                    .then(|| "the module's answer holds no printable text".to_string()),
            }];
        }
        KlineReadOutcome::KwpIdentification {
            option,
            bytes,
            text,
        } => {
            snapshot.data_hex = Some(hex(bytes));
            snapshot.parameters = vec![DecodedParameterSummary {
                name: format!("Identification 0x{option:02X}"),
                raw: None,
                value: Some(text.clone()),
                unit: None,
                state: None,
                note: None,
            }];
        }
        KlineReadOutcome::Ds2FaultMemory {
            bytes,
            words,
            node_address,
        } => {
            snapshot.data_hex = Some(hex(bytes));
            let _ = node_address;
            snapshot.dtcs = words
                .iter()
                .map(|word| dtc_row(format!("0x{word:04X}"), 0, request, library))
                .collect();
        }
        KlineReadOutcome::KwpDtcs(rows) => {
            snapshot.dtcs = rows
                .iter()
                .map(|row| dtc_row(format!("0x{:04X}", row.code), row.status, request, library))
                .collect();
        }
        KlineReadOutcome::Ds2NotAccepted { status, .. } => {
            snapshot.negative_response = Some(format!("0x{status:02X}"));
        }
        KlineReadOutcome::KwpNegative { code, text } => {
            snapshot.negative_response = Some(format!("0x{code:02X} {text}"));
        }
    }
}

/// One fault the module reported. The wording is SDD's where the index holds
/// that code for this module, and absent where it does not: a 16-bit code a
/// K-line module reports is its own, and nothing is invented around it.
fn dtc_row(
    code: String,
    status: u8,
    request: &ModuleReadRequest,
    library: Option<&KnowledgeLibrary>,
) -> DtcSummary {
    let described = library
        .map(|library| {
            library.describe_dtc_with_help(
                &code,
                status,
                &request.ecu_family,
                &vehicle_context(&request.context),
            )
        })
        .unwrap_or_default();
    DtcSummary {
        code,
        failure_type: format!("{status:02X}"),
        status: format!("{status:02X}"),
        description: described.description,
        description_scope: described.description_scope,
        failure_type_text: described.failure_type_text,
        failure_type_texts: described.failure_type_texts,
        description_texts: described.description_texts,
        help: described.help,
        help_texts: described.help_texts,
        help_note: described.help_note,
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
