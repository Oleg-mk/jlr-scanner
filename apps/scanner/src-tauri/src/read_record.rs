//! One record of one read, in the shape the intake understands.
//!
//! A module read has always left a `ModuleReadReport` behind it: the request,
//! the route, the responder, the bytes, and what stood in the way. The
//! intake (F13, `ADR-0016`) turns that record into the evidence a tester's
//! session contributes — the module answered at that address, on that bus,
//! over that adapter route, with that protocol. A live read and a mileage
//! survey make the very same request through the very same path, and until
//! the review of 2026-09-12 they threw that record away, so a tester's live
//! session contributed nothing. This is the builder all three share.

use app_contracts::{
    AdapterInfo, DiagnosticError, DiagnosticExecutionStage, ModuleReadReport, VehicleContextInput,
};
use mongoose_jlr::MongooseUdsReadResult;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uds_execution::PreparedUdsTransaction;

static RECORD_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Who asked, of whom, for what — the part of a record that is not the
/// exchange itself.
pub struct ReadIdentity<'a> {
    pub context: &'a VehicleContextInput,
    pub ecu_family: &'a str,
    /// The human name the service gives the read.
    pub operation: &'a str,
    /// What the resolver said of the route, or `SYNTHETIC` on the bench.
    pub route_validation: &'a str,
    pub adapter: Option<&'a AdapterInfo>,
}

/// What one request came to: the answer, or the reason there is none.
pub enum ReadOutcome<'a> {
    Answered(&'a MongooseUdsReadResult),
    Failed(&'a DiagnosticError),
}

/// The record of one read, from the transaction that was sent and what came
/// back.
pub fn read_record(
    identity: ReadIdentity<'_>,
    transaction: &PreparedUdsTransaction,
    outcome: ReadOutcome<'_>,
    decoded_result: Option<String>,
) -> ModuleReadReport {
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let counter = RECORD_COUNTER.fetch_add(1, Ordering::Relaxed);
    let (raw, error) = match outcome {
        ReadOutcome::Answered(raw) => (Some(raw), None),
        ReadOutcome::Failed(error) => (None, Some(error)),
    };
    let adapter = identity.adapter;
    ModuleReadReport {
        schema_version: 1,
        application_version: env!("CARGO_PKG_VERSION").into(),
        timestamp_unix_ms,
        session_id: format!("read-{timestamp_unix_ms}-{counter}"),
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
        vehicle: identity.context.clone(),
        ecu_family: identity.ecu_family.to_string(),
        operation: identity.operation.to_string(),
        logical_bus: transaction.logical_network().to_string(),
        backend_route: transaction.backend_route().to_string(),
        physical_route: format!(
            "{} pins {}",
            transaction.physical_connector(),
            transaction
                .physical_pins()
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("/")
        ),
        bitrate_bps: Some(transaction.bitrate_bps()),
        route_validation: identity.route_validation.to_string(),
        protocol: transaction.protocol_family().to_string(),
        request_id: format!("0x{:03X}", transaction.physical_request_id().value()),
        expected_response_id: format!("0x{:03X}", transaction.expected_response_id().value()),
        capability: transaction.capability_id().to_string(),
        request_payload: hex(transaction.encoded_payload()),
        actual_responder: raw.map(|value| format!("0x{:03X}", value.responder)),
        raw_diagnostic_response: raw.map(|value| hex(&value.raw_diagnostic_response)),
        decoded_result,
        pending_responses: raw.map(|value| value.pending_responses).unwrap_or(0),
        timeout_or_error_category: error.map(|value| value.category),
        execution_stage: error
            .map(|value| value.stage)
            .unwrap_or(DiagnosticExecutionStage::Completed),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
