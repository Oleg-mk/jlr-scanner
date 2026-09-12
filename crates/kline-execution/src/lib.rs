//! Offline-only compilation of read-only K-line intent (`ADR-0029`, slice B).
//!
//! The sibling of `uds-execution` for the serial lines: the same one-way
//! boundary from an F6 resolved plan into a prepared transaction, for the two
//! protocols this product speaks on a K-line — BMW's **DS2** and ISO 14230
//! **KWP2000**. Only the four read-only intents of `ADR-0029` can be built.
//! There is no clear-fault service, no security access, no session control
//! beyond the link handshake KWP2000 requires, no routine, no write, and no
//! way to hand this crate arbitrary bytes.
//!
//! What it decides and what it leaves alone: this crate turns a plan into
//! bytes and reads bytes back. Which firmware resource opens the line, how a
//! pin is selected and how the line is woken are the adapter's business and
//! live in `mongoose-jlr`, where they are hypotheses until the probe of
//! `ADR-0029` decision 6 has answered them.

use diagnostic_environment::{
    DiagnosticEnvironmentResolution, PlanEvidenceTrace, ResolutionConflict, UnresolvedFact,
};
use std::collections::BTreeMap;
use std::fmt;

/// The addressing mode a K-line module carries (`ADR-0029`): the one-byte
/// node address is the whole of the addressing. The word is the knowledge
/// model's; a test asserts this literal agrees with it.
pub const SERIAL_NODE_ADDRESSING_MODE: &str = "iso9141_node";

/// SDD's own names for the two protocols this crate speaks.
pub const DS2_PROTOCOL_FAMILY: &str = ds2::PROTOCOL_FAMILY;
pub const KWP2000_PROTOCOL_FAMILY: &str = kwp2000::PROTOCOL_FAMILY;

/// Which module a transaction is for. A K-line target is a module family:
/// the corpus names no software build on these buses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KlineTarget {
    ecu_family: String,
}

impl KlineTarget {
    pub fn family(ecu_family: impl Into<String>) -> Result<Self, PreparationError> {
        let ecu_family = ecu_family.into();
        if ecu_family.trim().is_empty() {
            return Err(PreparationError::InvalidTarget("ecu_family"));
        }
        Ok(Self { ecu_family })
    }

    pub fn ecu_family(&self) -> &str {
        &self.ecu_family
    }
}

/// The four reads `ADR-0029` allows on a K-line. There is no fifth, and no
/// constructor takes a service number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOnlyKlineIntent {
    /// DS2 `0x00`: what the module says it is.
    Ds2Identification { target: KlineTarget },
    /// DS2 `0x04`: the fault memory, as bytes.
    Ds2FaultMemory { target: KlineTarget },
    /// KWP2000 `0x1A`: one identification record, by SDD's own local
    /// identifier for it.
    KwpEcuIdentification { target: KlineTarget, option: u8 },
    /// KWP2000 `0x18`: the fault codes matching a status mask.
    KwpDtcByStatus {
        target: KlineTarget,
        status_mask: u8,
    },
}

impl ReadOnlyKlineIntent {
    pub fn target(&self) -> &KlineTarget {
        match self {
            Self::Ds2Identification { target }
            | Self::Ds2FaultMemory { target }
            | Self::KwpEcuIdentification { target, .. }
            | Self::KwpDtcByStatus { target, .. } => target,
        }
    }

    /// The protocol family this intent belongs to, as SDD names it.
    pub fn protocol_family(&self) -> &'static str {
        match self {
            Self::Ds2Identification { .. } | Self::Ds2FaultMemory { .. } => DS2_PROTOCOL_FAMILY,
            Self::KwpEcuIdentification { .. } | Self::KwpDtcByStatus { .. } => {
                KWP2000_PROTOCOL_FAMILY
            }
        }
    }

    /// The read-only capability the plan must carry for this intent.
    pub fn required_capability(&self) -> &'static str {
        match self {
            Self::Ds2Identification { .. } => ds2::ECU_IDENTIFICATION_CAPABILITY,
            Self::Ds2FaultMemory { .. } => ds2::FAULT_MEMORY_CAPABILITY,
            Self::KwpEcuIdentification { .. } => kwp2000::READ_ECU_IDENTIFICATION_CAPABILITY,
            Self::KwpDtcByStatus { .. } => kwp2000::READ_DTC_BY_STATUS_CAPABILITY,
        }
    }
}

/// How a serial line carries a byte, as the platform document states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialFraming {
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl SerialFraming {
    /// Reads back the ingest's own text, `data_bits=8;parity=even;stop_bits=1`.
    /// Anything it does not state is refused rather than assumed.
    pub fn parse(text: &str) -> Result<Self, PreparationError> {
        let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
        for part in text.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                fields.insert(key.trim(), value.trim());
            }
        }
        let number = |key: &'static str| -> Result<u8, PreparationError> {
            fields
                .get(key)
                .and_then(|value| value.parse().ok())
                .ok_or(PreparationError::UnreadableFraming(key))
        };
        let parity = match fields.get("parity").copied() {
            Some("none") => Parity::None,
            Some("even") => Parity::Even,
            Some("odd") => Parity::Odd,
            _ => return Err(PreparationError::UnreadableFraming("parity")),
        };
        Ok(Self {
            data_bits: number("data_bits")?,
            parity,
            stop_bits: number("stop_bits")?,
        })
    }
}

/// The request bytes, and which protocol built them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KlineRequest {
    Ds2(ds2::Ds2Request),
    Kwp(kwp2000::KwpRequest),
}

impl KlineRequest {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Ds2(request) => request.as_bytes(),
            Self::Kwp(request) => request.as_bytes(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionSafetyClass {
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProvenanceField {
    VehicleApplicability,
    EcuFamily,
    LogicalNetwork,
    PhysicalRoute,
    BackendRoute,
    Bitrate,
    ProtocolFamily,
    AddressingMode,
    SerialFraming,
    SerialWakeup,
    NodeAddress,
    Capability,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionProvenance {
    traces: BTreeMap<ProvenanceField, Vec<PlanEvidenceTrace>>,
}

impl ExecutionProvenance {
    pub fn traces_for(&self, field: ProvenanceField) -> &[PlanEvidenceTrace] {
        self.traces.get(&field).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// One read-only K-line request, compiled from a resolved plan. Every field
/// comes from the plan; nothing here can be built by hand from outside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedKlineTransaction {
    safety_class: TransactionSafetyClass,
    target: KlineTarget,
    logical_network: String,
    physical_connector: String,
    physical_pins: Vec<u8>,
    backend_route: String,
    bitrate_bps: u32,
    protocol_family: String,
    framing: SerialFraming,
    wakeup: String,
    node_address: u8,
    capability_id: String,
    request: KlineRequest,
    provenance: ExecutionProvenance,
}

impl PreparedKlineTransaction {
    pub fn safety_class(&self) -> TransactionSafetyClass {
        self.safety_class
    }

    pub fn target(&self) -> &KlineTarget {
        &self.target
    }

    pub fn logical_network(&self) -> &str {
        &self.logical_network
    }

    pub fn physical_connector(&self) -> &str {
        &self.physical_connector
    }

    pub fn physical_pins(&self) -> &[u8] {
        &self.physical_pins
    }

    pub fn backend_route(&self) -> &str {
        &self.backend_route
    }

    pub fn bitrate_bps(&self) -> u32 {
        self.bitrate_bps
    }

    pub fn protocol_family(&self) -> &str {
        &self.protocol_family
    }

    /// How the line carries a byte, as the document states it.
    pub fn framing(&self) -> SerialFraming {
        self.framing
    }

    /// SDD's own name for the wake-up this bus needs: `none`, `bmw_ds2`,
    /// `kw2000_fast`. What to do about it is the adapter's business.
    pub fn wakeup(&self) -> &str {
        &self.wakeup
    }

    /// The module's one-byte node address, which is the whole of the
    /// addressing on a serial line.
    pub fn node_address(&self) -> u8 {
        self.node_address
    }

    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn request(&self) -> &KlineRequest {
        &self.request
    }

    /// The bytes to put on the line, framing and checksum included.
    pub fn encoded_payload(&self) -> &[u8] {
        self.request.as_bytes()
    }

    pub fn provenance(&self) -> &ExecutionProvenance {
        &self.provenance
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparationError {
    EnvironmentIndeterminate(Vec<UnresolvedFact>),
    EnvironmentConflict(Vec<ResolutionConflict>),
    InvalidTarget(&'static str),
    TargetEcuMismatch {
        expected: String,
        actual: String,
    },
    UnsupportedProtocol(String),
    UnsupportedCapability(String),
    UnsupportedAddressingMode(String),
    InvalidRoute(&'static str),
    InvalidBitrate(u32),
    /// The plan carries a CAN identifier format: it is not a serial plan,
    /// and this crate will not guess which half of it to believe.
    UnexpectedCanIdFormat,
    /// A node address is one byte, and the request and the response side of
    /// a serial plan carry the same one.
    InvalidNodeAddress {
        value: u32,
    },
    NodeAddressMismatch {
        request: u32,
        response: u32,
    },
    /// The bus states no framing, or none this crate can read; a line is not
    /// opened on a guess.
    MissingFraming,
    UnreadableFraming(&'static str),
    MissingWakeup,
}

impl fmt::Display for PreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvironmentIndeterminate(facts) => {
                write!(formatter, "environment indeterminate: {} fact(s) unresolved", facts.len())
            }
            Self::EnvironmentConflict(conflicts) => {
                write!(formatter, "environment conflict in {} field(s)", conflicts.len())
            }
            Self::InvalidTarget(field) => write!(formatter, "invalid target: {field}"),
            Self::TargetEcuMismatch { expected, actual } => write!(
                formatter,
                "the plan is for module family {actual}, the intent for {expected}"
            ),
            Self::UnsupportedProtocol(protocol) => write!(
                formatter,
                "protocol {protocol} is not a K-line protocol this product speaks"
            ),
            Self::UnsupportedCapability(capability) => {
                write!(formatter, "the plan carries capability {capability}, which is not the one this read needs")
            }
            Self::UnsupportedAddressingMode(mode) => write!(
                formatter,
                "addressing mode {mode} is not the serial node addressing a K-line read needs"
            ),
            Self::InvalidRoute(field) => write!(formatter, "the plan's route is incomplete: {field}"),
            Self::InvalidBitrate(value) => write!(formatter, "bit rate {value} is not a baud rate"),
            Self::UnexpectedCanIdFormat => write!(
                formatter,
                "the plan carries a CAN identifier format, so it is not a serial line plan"
            ),
            Self::InvalidNodeAddress { value } => {
                write!(formatter, "node address 0x{value:X} is not one byte")
            }
            Self::NodeAddressMismatch { request, response } => write!(
                formatter,
                "the plan's node address differs between request (0x{request:X}) and response (0x{response:X})"
            ),
            Self::MissingFraming => write!(
                formatter,
                "the bus states no byte framing, and a line is not opened on a guess"
            ),
            Self::UnreadableFraming(field) => {
                write!(formatter, "the bus's framing does not state {field}")
            }
            Self::MissingWakeup => write!(
                formatter,
                "the bus states no wake-up, and a line is not woken on a guess"
            ),
        }
    }
}

impl std::error::Error for PreparationError {}

/// Compiles one read-only K-line intent against a resolved plan.
///
/// Every refusal here is a refusal to guess: a protocol this product does
/// not speak, an addressing mode that is not a node address, a bus with no
/// stated framing or wake-up. A plan that passes carries everything the
/// adapter needs and nothing it has to invent.
pub fn prepare_read_only_kline_read(
    resolution: &DiagnosticEnvironmentResolution,
    intent: ReadOnlyKlineIntent,
) -> Result<PreparedKlineTransaction, PreparationError> {
    let plan = match resolution {
        DiagnosticEnvironmentResolution::Resolved(plan) => plan,
        DiagnosticEnvironmentResolution::Indeterminate {
            unresolved_facts, ..
        } => {
            return Err(PreparationError::EnvironmentIndeterminate(
                unresolved_facts.clone(),
            ))
        }
        DiagnosticEnvironmentResolution::Conflict { conflicts, .. } => {
            return Err(PreparationError::EnvironmentConflict(conflicts.clone()))
        }
    };

    let target = intent.target().clone();
    if target.ecu_family != plan.ecu_family.value {
        return Err(PreparationError::TargetEcuMismatch {
            expected: target.ecu_family,
            actual: plan.ecu_family.value.clone(),
        });
    }
    if plan.protocol_family.value != intent.protocol_family() {
        return Err(PreparationError::UnsupportedProtocol(
            plan.protocol_family.value.clone(),
        ));
    }
    if plan.read_only_capability.value.id != intent.required_capability() {
        return Err(PreparationError::UnsupportedCapability(
            plan.read_only_capability.value.id.clone(),
        ));
    }
    if plan.addressing_mode.value != SERIAL_NODE_ADDRESSING_MODE {
        return Err(PreparationError::UnsupportedAddressingMode(
            plan.addressing_mode.value.clone(),
        ));
    }
    if plan.can_id_format.is_some() {
        return Err(PreparationError::UnexpectedCanIdFormat);
    }
    if plan.logical_network.value.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("logical_network"));
    }
    if plan.physical_route.value.connector.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("physical_connector"));
    }
    if plan.physical_route.value.pins.is_empty() {
        return Err(PreparationError::InvalidRoute("physical_pins"));
    }
    if plan.backend_route.value.route_id.trim().is_empty() {
        return Err(PreparationError::InvalidRoute("backend_route"));
    }
    if plan.bitrate_bps.value == 0 {
        return Err(PreparationError::InvalidBitrate(0));
    }

    // On a serial line SDD writes the node address into both identifier
    // fields; if they disagree the plan is not describing one module.
    let request_id = plan.physical_request_id.value;
    let response_id = plan.physical_response_id.value;
    if request_id != response_id {
        return Err(PreparationError::NodeAddressMismatch {
            request: request_id,
            response: response_id,
        });
    }
    let node_address = u8::try_from(request_id)
        .map_err(|_| PreparationError::InvalidNodeAddress { value: request_id })?;

    let framing_field = plan
        .serial_framing
        .as_ref()
        .ok_or(PreparationError::MissingFraming)?;
    let framing = SerialFraming::parse(&framing_field.value)?;
    let wakeup_field = plan
        .serial_wakeup
        .as_ref()
        .ok_or(PreparationError::MissingWakeup)?;

    let request = match &intent {
        ReadOnlyKlineIntent::Ds2Identification { .. } => {
            KlineRequest::Ds2(ds2::Ds2Request::ecu_identification(node_address))
        }
        ReadOnlyKlineIntent::Ds2FaultMemory { .. } => {
            KlineRequest::Ds2(ds2::Ds2Request::fault_memory(node_address))
        }
        ReadOnlyKlineIntent::KwpEcuIdentification { option, .. } => KlineRequest::Kwp(
            kwp2000::KwpRequest::read_ecu_identification(node_address, *option),
        ),
        ReadOnlyKlineIntent::KwpDtcByStatus { status_mask, .. } => {
            KlineRequest::Kwp(kwp2000::KwpRequest::read_dtc_by_status(
                node_address,
                *status_mask,
                kwp2000::ALL_DTC_GROUPS,
            ))
        }
    };

    let mut traces = BTreeMap::new();
    traces.insert(
        ProvenanceField::VehicleApplicability,
        plan.vehicle_applicability.evidence.clone(),
    );
    traces.insert(ProvenanceField::EcuFamily, plan.ecu_family.evidence.clone());
    traces.insert(
        ProvenanceField::LogicalNetwork,
        plan.logical_network.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::PhysicalRoute,
        plan.physical_route.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::BackendRoute,
        plan.backend_route.evidence.clone(),
    );
    traces.insert(ProvenanceField::Bitrate, plan.bitrate_bps.evidence.clone());
    traces.insert(
        ProvenanceField::ProtocolFamily,
        plan.protocol_family.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::AddressingMode,
        plan.addressing_mode.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::SerialFraming,
        framing_field.evidence.clone(),
    );
    traces.insert(ProvenanceField::SerialWakeup, wakeup_field.evidence.clone());
    traces.insert(
        ProvenanceField::NodeAddress,
        plan.physical_request_id.evidence.clone(),
    );
    traces.insert(
        ProvenanceField::Capability,
        plan.read_only_capability.evidence.clone(),
    );

    Ok(PreparedKlineTransaction {
        safety_class: TransactionSafetyClass::ReadOnly,
        target,
        logical_network: plan.logical_network.value.clone(),
        physical_connector: plan.physical_route.value.connector.clone(),
        physical_pins: plan.physical_route.value.pins.clone(),
        backend_route: plan.backend_route.value.route_id.clone(),
        bitrate_bps: plan.bitrate_bps.value,
        protocol_family: plan.protocol_family.value.clone(),
        framing,
        wakeup: wakeup_field.value.clone(),
        node_address,
        capability_id: plan.read_only_capability.value.id.clone(),
        request,
        provenance: ExecutionProvenance { traces },
    })
}

/// What came back from a module, read as the protocol reads it and no
/// further. Nothing here is interpreted beyond what the frame says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KlineReadOutcome {
    /// DS2 `0x00`: the bytes and their printable text.
    Ds2Identification {
        node_address: u8,
        bytes: Vec<u8>,
        text: String,
    },
    /// DS2 `0x04`: the fault memory as bytes, with the 16-bit words a fault
    /// index can be asked about. No layout is decided here.
    Ds2FaultMemory {
        node_address: u8,
        bytes: Vec<u8>,
        words: Vec<u16>,
    },
    /// DS2: the module answered, and its first data byte is not acceptance.
    /// Its own status byte, shown as its own.
    Ds2NotAccepted { node_address: u8, status: u8 },
    /// KWP2000 `0x1A`: the record the module holds.
    KwpIdentification {
        option: u8,
        bytes: Vec<u8>,
        text: String,
    },
    /// KWP2000 `0x18`: code, status byte, one row per fault.
    KwpDtcs(Vec<KwpDtc>),
    /// KWP2000: a refusal, with the standard's own word for the code.
    KwpNegative { code: u8, text: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KwpDtc {
    pub code: u16,
    pub status: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Ds2(ds2::Ds2Error),
    Kwp(kwp2000::KwpError),
    /// Nothing came back that was a frame for this module.
    NoAnswer,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ds2(error) => write!(formatter, "{error}"),
            Self::Kwp(error) => write!(formatter, "{error}"),
            Self::NoAnswer => write!(formatter, "no frame for this module came back"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Reads back what the line carried: the echo of our own request first, as
/// a K-line always carries it, then the module's answer.
pub fn decode_response(
    transaction: &PreparedKlineTransaction,
    bytes: &[u8],
) -> Result<KlineReadOutcome, DecodeError> {
    match (&transaction.request, bytes) {
        (KlineRequest::Ds2(request), stream) => {
            let rest = ds2::strip_echo(stream, request);
            if rest.is_empty() {
                return Err(DecodeError::NoAnswer);
            }
            let (frame, _) = ds2::parse_leading_frame(rest).map_err(DecodeError::Ds2)?;
            if !frame.accepted() {
                return Ok(KlineReadOutcome::Ds2NotAccepted {
                    node_address: frame.node_address,
                    status: frame.status().unwrap_or_default(),
                });
            }
            match request.command() {
                ds2::COMMAND_FAULT_MEMORY => {
                    let memory = ds2::fault_memory(&frame).map_err(DecodeError::Ds2)?;
                    Ok(KlineReadOutcome::Ds2FaultMemory {
                        node_address: memory.node_address,
                        words: ds2::words_from(&memory.bytes, 0),
                        bytes: memory.bytes,
                    })
                }
                _ => {
                    let identification = ds2::identification(&frame).map_err(DecodeError::Ds2)?;
                    Ok(KlineReadOutcome::Ds2Identification {
                        node_address: identification.node_address,
                        bytes: identification.bytes,
                        text: identification.text,
                    })
                }
            }
        }
        (KlineRequest::Kwp(request), stream) => {
            let rest = kwp2000::strip_echo(stream, request);
            if rest.is_empty() {
                return Err(DecodeError::NoAnswer);
            }
            let (message, _) = kwp2000::parse_leading_message(rest).map_err(DecodeError::Kwp)?;
            let reply =
                kwp2000::interpret(&message, request.service_id()).map_err(DecodeError::Kwp)?;
            match &reply {
                kwp2000::KwpReply::Negative { code, .. } => Ok(KlineReadOutcome::KwpNegative {
                    code: *code,
                    text: kwp2000::response_code_text(*code).to_string(),
                }),
                kwp2000::KwpReply::Positive { .. }
                    if request.service_id() == kwp2000::SID_READ_DTC_BY_STATUS =>
                {
                    let rows = kwp2000::dtc_by_status(&reply).map_err(DecodeError::Kwp)?;
                    Ok(KlineReadOutcome::KwpDtcs(
                        rows.into_iter()
                            .map(|row| KwpDtc {
                                code: row.code,
                                status: row.status,
                            })
                            .collect(),
                    ))
                }
                kwp2000::KwpReply::Positive { .. } => {
                    let record = kwp2000::ecu_identification(&reply).map_err(DecodeError::Kwp)?;
                    Ok(KlineReadOutcome::KwpIdentification {
                        option: record.option,
                        bytes: record.bytes,
                        text: record.text,
                    })
                }
            }
        }
    }
}
