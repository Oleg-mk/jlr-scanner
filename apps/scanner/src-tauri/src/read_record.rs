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
    AdapterInfo, AdapterTrace, DiagnosticError, DiagnosticExecutionStage, ModuleReadReport,
    VehicleContextInput,
};
use mongoose_jlr::{status_name, status_words, ExchangeTrace, MongooseUdsReadResult};
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

/// The adapter's trace in the record's shape (`ADR-0042`): the words of the
/// status and of the frames, the counts as they are; the pass entries and
/// the status words' names since `ADR-0043`.
pub fn adapter_trace(trace: &ExchangeTrace) -> AdapterTrace {
    let acknowledgement = trace.last_acknowledgement.as_ref();
    AdapterTrace {
        transmits: trace.transmits,
        acknowledgement_status: acknowledgement.map(|value| format!("{:#010X}", value.status)),
        acknowledgement_status_name: acknowledgement
            .and_then(|value| status_name(value.status))
            .map(str::to_string),
        acknowledgement_message: acknowledgement
            .map(|value| value.message.clone())
            .filter(|message| !message.is_empty()),
        acknowledgement_wait_ms: acknowledgement.map(|value| value.waited_ms),
        inbound_frames: trace.inbound_frames,
        indications: trace.indications,
        last_indication: trace.last_indication.map(|frame| frame.to_string()),
        stale_frames: trace.stale_frames,
        last_stale_frame: trace.last_stale_frame.map(|frame| frame.to_string()),
        dropped_frames: trace.dropped_frames,
        dropped_bytes: trace.dropped_bytes,
        close_error: trace.close_error.clone(),
        receive_filters: trace
            .receive_filters
            .iter()
            .map(ToString::to_string)
            .collect(),
        indications_by_status: trace
            .indications_by_status
            .iter()
            .map(|(status, count)| match status {
                Some(status) => format!("{count} with {}", status_words(*status)),
                None => format!("{count} with no status word"),
            })
            .collect(),
    }
}

/// The record of one read, from the transaction that was sent and what came
/// back. `trace` is what the adapter did around the read, fetched after it
/// whatever the outcome; an answer carries its own and needs none.
pub fn read_record(
    identity: ReadIdentity<'_>,
    transaction: &PreparedUdsTransaction,
    outcome: ReadOutcome<'_>,
    decoded_result: Option<String>,
    trace: Option<&ExchangeTrace>,
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
    let trace = trace.or(raw.map(|value| &value.trace));
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
        error_message: error.map(app_contracts::DiagnosticError::words),
        adapter_trace: trace.map(adapter_trace),
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

/// A read on a serial line (`ADR-0029` slice B). The fields a CAN read fills
/// with identifiers carry the module's node address, which is the whole of
/// the addressing on such a line, and say so in their own format.
pub struct SerialRead<'a> {
    pub logical_network: &'a str,
    pub connector: &'a str,
    pub pins: &'a [u8],
    pub backend_route: &'a str,
    pub bitrate_bps: u32,
    pub protocol_family: &'a str,
    pub node_address: u8,
    pub capability: &'a str,
    pub request: &'a [u8],
}

/// What one K-line request came to: the bytes the line carried, or why none.
pub enum SerialOutcome<'a> {
    Answered(&'a [u8]),
    Failed(&'a DiagnosticError),
}

/// The record of one serial read, in the same shape the intake already reads.
pub fn serial_read_record(
    identity: ReadIdentity<'_>,
    line: &SerialRead<'_>,
    outcome: SerialOutcome<'_>,
    decoded_result: Option<String>,
    trace: Option<&ExchangeTrace>,
) -> ModuleReadReport {
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let counter = RECORD_COUNTER.fetch_add(1, Ordering::Relaxed);
    let (answer, error) = match outcome {
        SerialOutcome::Answered(bytes) => (Some(bytes), None),
        SerialOutcome::Failed(error) => (None, Some(error)),
    };
    let adapter = identity.adapter;
    let node = format!("node 0x{:02X}", line.node_address);
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
        logical_bus: line.logical_network.to_string(),
        backend_route: line.backend_route.to_string(),
        physical_route: format!(
            "{} pins {}",
            line.connector,
            line.pins
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("/")
        ),
        bitrate_bps: Some(line.bitrate_bps),
        route_validation: identity.route_validation.to_string(),
        protocol: line.protocol_family.to_string(),
        request_id: node.clone(),
        expected_response_id: node.clone(),
        capability: line.capability.to_string(),
        request_payload: hex(line.request),
        actual_responder: answer.map(|_| node),
        raw_diagnostic_response: answer.map(hex),
        decoded_result,
        pending_responses: 0,
        timeout_or_error_category: error.map(|value| value.category),
        error_message: error.map(app_contracts::DiagnosticError::words),
        adapter_trace: trace.map(adapter_trace),
        execution_stage: error
            .map(|value| value.stage)
            .unwrap_or(DiagnosticExecutionStage::Completed),
    }
}
