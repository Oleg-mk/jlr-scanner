use crate::{Frame, ProtocolError};
use std::fmt;
use std::str::FromStr;

const FRAME_HEADER_LENGTH: usize = 4;
pub(crate) const COMMAND_HEADER_LENGTH: usize = 12;
const VERIFIER_XOR: u16 = 0x51E6;
/// The device route every board command travels on (board-info, the jump
/// to firmware): route word A of the request, echoed in route word B of
/// the response.
pub(crate) const DEVICE_ROUTE: u16 = 0x0001;
/// Resource selectors carried in route word A of a channel open; the
/// firmware hands the same word back as the channel token (ADR-0018).
/// Resource 5 is CAN1 on J1962 pins 6/14, resource 21 is CAN2 on pins 3/11;
/// both were read off the firmware's own replies on 2026-09-06.
pub(crate) const CAN1_RESOURCE_ROUTE: u16 = 0x0501;
pub(crate) const CAN2_RESOURCE_ROUTE: u16 = 0x1501;
/// Command word 0x0103 — opcode 0x03 with flag 0x01 — starts the stored
/// firmware from the bootloader the adapter powers up in. It writes
/// nothing; a power cycle returns the adapter to the bootloader. The
/// reflash, unprotect, reset and serial-number commands of the same family
/// are deliberately absent from this crate.
pub(crate) const JUMP_TO_FIRMWARE: u16 = 0x0103;
pub(crate) const JUMP_TO_FIRMWARE_RESPONSE: u16 = 0x8103;
pub(crate) const DT_LISTEN_ONLY: u32 = 0x1000_0000;
pub(crate) const CAN_29BIT_ID: u32 = 0x0000_0100;
pub(crate) const OPEN_CHANNEL: u16 = 0x0006;
pub(crate) const OPEN_CHANNEL_RESPONSE: u16 = 0x8006;
pub(crate) const CLOSE_CHANNEL: u16 = 0x0007;
pub(crate) const CLOSE_CHANNEL_RESPONSE: u16 = 0x8007;
pub(crate) const OUTBOUND_DATA: u16 = 0x0008;
pub(crate) const INBOUND_DATA: u16 = 0x0009;
pub(crate) const OUTBOUND_DATA_RESPONSE: u16 = 0x8008;
pub(crate) const SET_PIN: u16 = 0x0012;
pub(crate) const SET_PIN_RESPONSE: u16 = 0x8012;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleRouteId {
    HsCan,
    MsCan,
    /// K-line on J1962 pin 7 (ADR-0029): DS2 of the L322's body modules,
    /// KWP2000 of the L316's ABS. The bit rate is the bus's, 9600 or 10400.
    KLine7,
    /// K-line on J1962 pin 8 (ADR-0029): ROSCO of the L316's VIM, KWP2000*
    /// of the L322's transfer case. Neither protocol is spoken yet.
    KLine8,
}

impl VehicleRouteId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HsCan => "hs-can",
            Self::MsCan => "ms-can",
            Self::KLine7 => "k-line-7",
            Self::KLine8 => "k-line-8",
        }
    }
}

impl fmt::Display for VehicleRouteId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for VehicleRouteId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hs-can" => Ok(Self::HsCan),
            "ms-can" => Ok(Self::MsCan),
            "k-line-7" => Ok(Self::KLine7),
            "k-line-8" => Ok(Self::KLine8),
            _ => Err(format!("unknown route {value}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkType {
    Can,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveCapability {
    Ready,
    Blocked(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VehicleRoute {
    pub id: VehicleRouteId,
    pub network_name: &'static str,
    pub network_type: NetworkType,
    pub obd_pins: &'static [u8],
    pub bitrate: Option<u32>,
    pub passive_capability: PassiveCapability,
}

const HS_CAN_PINS: &[u8] = &[6, 14];
const MS_CAN_PINS: &[u8] = &[3, 11];
const K_LINE_7_PINS: &[u8] = &[7];
const K_LINE_8_PINS: &[u8] = &[8];
/// A K-line is not opened by this build (ADR-0029): the pin selection and
/// the wake-up on this adapter are unverified until the second slice's probe
/// has run, and a K-line has no listen-only mode in any case — a module on
/// it answers only when asked. Every open of one of these routes is refused
/// with this reason.
pub(crate) const K_LINE_NOT_OPENED: &str = "K-line is not opened by this build: the adapter's pin selection and wake-up for it are unverified (ADR-0029), and a K-line module answers only when asked";

const VEHICLE_ROUTES: &[VehicleRoute] = &[
    VehicleRoute {
        id: VehicleRouteId::HsCan,
        network_name: "X250 HS-CAN",
        network_type: NetworkType::Can,
        obd_pins: HS_CAN_PINS,
        bitrate: Some(500_000),
        passive_capability: PassiveCapability::Ready,
    },
    VehicleRoute {
        id: VehicleRouteId::MsCan,
        network_name: "X250 MS-CAN",
        network_type: NetworkType::Can,
        obd_pins: MS_CAN_PINS,
        bitrate: Some(125_000),
        passive_capability: PassiveCapability::Ready,
    },
    VehicleRoute {
        id: VehicleRouteId::KLine7,
        network_name: "K-line pin 7",
        network_type: NetworkType::Other,
        obd_pins: K_LINE_7_PINS,
        // The bus states its baud rate; the route has none of its own.
        bitrate: None,
        passive_capability: PassiveCapability::Blocked(K_LINE_NOT_OPENED),
    },
    VehicleRoute {
        id: VehicleRouteId::KLine8,
        network_name: "K-line pin 8",
        network_type: NetworkType::Other,
        obd_pins: K_LINE_8_PINS,
        bitrate: None,
        passive_capability: PassiveCapability::Blocked(K_LINE_NOT_OPENED),
    },
];

pub fn list_vehicle_routes() -> &'static [VehicleRoute] {
    VEHICLE_ROUTES
}

/// The firmware resource a vehicle route opens (ADR-0018), where one has
/// been seen to answer. The K-line routes have none yet (ADR-0029): the resource is
/// chosen by the protocol's physical layer when the second slice opens a
/// line, and until the adapter has answered that open it stays unknown
/// rather than guessed.
pub(crate) fn resource_route(id: VehicleRouteId) -> Option<u16> {
    match id {
        VehicleRouteId::HsCan => Some(CAN1_RESOURCE_ROUTE),
        VehicleRouteId::MsCan => Some(CAN2_RESOURCE_ROUTE),
        VehicleRouteId::KLine7 | VehicleRouteId::KLine8 => None,
    }
}

pub(crate) fn route_by_id(id: VehicleRouteId) -> &'static VehicleRoute {
    VEHICLE_ROUTES
        .iter()
        .find(|route| route.id == id)
        .expect("every VehicleRouteId must have a descriptor")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenReceiveRoute {
    pub route: VehicleRoute,
    pub device_channel_id: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanIdFormat {
    Standard,
    Extended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawCanFrame {
    pub arbitration_id: u32,
    pub id_format: CanIdFormat,
    pub dlc: u8,
    pub data: [u8; 8],
    pub device_timestamp: u32,
    pub source_route: VehicleRouteId,
    pub source_network: &'static str,
}

impl RawCanFrame {
    pub fn payload(&self) -> &[u8] {
        &self.data[..usize::from(self.dlc)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommandResponse {
    pub channel_id: u16,
}

pub(crate) fn open_channel_request(sequence: u16, route_word: u16, bitrate: u32) -> Vec<u8> {
    open_channel_request_with_flags(sequence, route_word, bitrate, DT_LISTEN_ONLY)
}

pub(crate) fn diagnostic_open_channel_request(
    sequence: u16,
    route_word: u16,
    bitrate: u32,
) -> Vec<u8> {
    open_channel_request_with_flags(sequence, route_word, bitrate, 0)
}

fn open_channel_request_with_flags(
    sequence: u16,
    route_word: u16,
    bitrate: u32,
    flags: u32,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(8);
    body.extend_from_slice(&flags.to_le_bytes());
    body.extend_from_slice(&bitrate.to_le_bytes());
    command(route_word, OPEN_CHANNEL, sequence, &body)
}

/// The jump from the bootloader to the stored firmware (ADR-0018).
pub(crate) fn jump_to_firmware_request(sequence: u16) -> Vec<u8> {
    command(DEVICE_ROUTE, JUMP_TO_FIRMWARE, sequence, &[])
}

pub(crate) fn parse_jump_response(
    frame: &Frame,
    expected_sequence: u16,
) -> Result<CommandResponse, ProtocolError> {
    parse_command_response(
        frame,
        JUMP_TO_FIRMWARE_RESPONSE,
        expected_sequence,
        "cJumpToFirmware",
    )
}

pub(crate) fn outbound_data_request(
    sequence: u16,
    channel_id: u16,
    arbitration_id: u32,
    extended: bool,
    can_payload: &[u8],
) -> Vec<u8> {
    debug_assert!(can_payload.len() <= 8);
    debug_assert!(arbitration_id <= if extended { 0x1FFF_FFFF } else { 0x7FF });
    let data_size = u16::try_from(4 + can_payload.len()).expect("classic CAN frame is bounded");
    // Inbound frames carry the 29-bit flag in their status word (observed
    // in captures). Which of the two status words the device reads on
    // transmit is not documented, so the flag is set in both; an 11-bit
    // frame carries zeros as before (ADR-0017).
    let flags = if extended { CAN_29BIT_ID } else { 0 };
    let mut payload = Vec::with_capacity(24 + usize::from(data_size));
    payload.extend_from_slice(&channel_id.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0x0008_u16.to_le_bytes());
    payload.extend_from_slice(&sequence.to_le_bytes());
    payload.extend_from_slice(&1_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&flags.to_le_bytes());
    payload.extend_from_slice(&flags.to_le_bytes());
    payload.extend_from_slice(&data_size.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&arbitration_id.to_be_bytes());
    payload.extend_from_slice(can_payload);
    outer_frame(&payload)
}

pub(crate) fn set_pin_request(sequence: u16, channel_id: u16, route: VehicleRoute) -> Vec<u8> {
    let [positive_pin, negative_pin] = route.obd_pins else {
        unreachable!("supported passive CAN route must have an exact pin pair")
    };
    let mut body = Vec::with_capacity(12);
    body.extend_from_slice(&1_u32.to_le_bytes());
    body.extend_from_slice(&u32::from(*positive_pin).to_le_bytes());
    body.extend_from_slice(&u32::from(*negative_pin).to_le_bytes());
    command(channel_id, SET_PIN, sequence, &body)
}

pub(crate) fn close_channel_request(sequence: u16, channel_id: u16) -> Vec<u8> {
    command(channel_id, CLOSE_CHANNEL, sequence, &[])
}

pub(crate) fn parse_open_response(
    frame: &Frame,
    expected_sequence: u16,
    route_word: u16,
) -> Result<CommandResponse, ProtocolError> {
    let response = parse_command_response(
        frame,
        OPEN_CHANNEL_RESPONSE,
        expected_sequence,
        "cOpenChannel",
    )?;
    // The firmware hands the resource word back as the channel token.
    if response.channel_id != route_word {
        return Err(ProtocolError::RouteMismatch {
            expected: route_word,
            actual: response.channel_id,
        });
    }
    Ok(response)
}

pub(crate) fn parse_set_pin_response(
    frame: &Frame,
    expected_sequence: u16,
    channel_id: u16,
) -> Result<CommandResponse, ProtocolError> {
    let response = parse_command_response(frame, SET_PIN_RESPONSE, expected_sequence, "cSetPin")?;
    require_channel(response, channel_id)
}

pub(crate) fn parse_outbound_response(
    frame: &Frame,
    expected_sequence: u16,
    channel_id: u16,
) -> Result<CommandResponse, ProtocolError> {
    let response = parse_command_response(
        frame,
        OUTBOUND_DATA_RESPONSE,
        expected_sequence,
        "cOutboundData",
    )?;
    require_channel(response, channel_id)
}

pub(crate) fn parse_close_response(
    frame: &Frame,
    expected_sequence: u16,
    channel_id: u16,
) -> Result<CommandResponse, ProtocolError> {
    let response = parse_command_response(
        frame,
        CLOSE_CHANNEL_RESPONSE,
        expected_sequence,
        "cCloseChannel",
    )?;
    require_channel(response, channel_id)
}

pub(crate) fn is_inbound_data(frame: &Frame) -> bool {
    let payload = frame.payload();
    payload.len() >= COMMAND_HEADER_LENGTH && read_u16(payload, 4) == Some(INBOUND_DATA)
}

pub(crate) fn parse_inbound_can(
    frame: &Frame,
    open_route: OpenReceiveRoute,
) -> Result<RawCanFrame, ProtocolError> {
    let payload = frame.payload();
    if payload.len() < COMMAND_HEADER_LENGTH {
        return Err(ProtocolError::PayloadTooShort(payload.len()));
    }
    let command = read_u16(payload, 4).expect("header length checked");
    if command != INBOUND_DATA {
        return Err(ProtocolError::UnexpectedCommand {
            expected: INBOUND_DATA,
            actual: command,
        });
    }
    let actual_channel = read_u16(payload, 2).expect("header length checked");
    if actual_channel != open_route.device_channel_id {
        return Err(ProtocolError::RouteMismatch {
            expected: open_route.device_channel_id,
            actual: actual_channel,
        });
    }

    let body = &payload[COMMAND_HEADER_LENGTH..];
    if body.len() < 12 {
        return Err(ProtocolError::PayloadTooShort(payload.len()));
    }
    let rx_status = read_u32(body, 0).expect("body length checked");
    let timestamp = read_u32(body, 4).expect("body length checked");
    let data_size = read_u16(body, 8).expect("body length checked");
    if !(4..=12).contains(&data_size) {
        return Err(ProtocolError::InvalidCanDataSize(data_size));
    }
    let required = 12 + usize::from(data_size);
    if body.len() < required {
        return Err(ProtocolError::TruncatedFrame {
            expected: COMMAND_HEADER_LENGTH + required,
            actual: payload.len(),
        });
    }

    let can_data = &body[12..required];
    let arbitration_id = u32::from_be_bytes([can_data[0], can_data[1], can_data[2], can_data[3]]);
    let extended = rx_status & CAN_29BIT_ID != 0;
    let maximum = if extended { 0x1FFF_FFFF } else { 0x7FF };
    if arbitration_id > maximum {
        return Err(ProtocolError::InvalidCanId {
            arbitration_id,
            extended,
        });
    }
    let dlc = u8::try_from(data_size - 4).expect("validated CAN data size");
    let mut data = [0_u8; 8];
    data[..usize::from(dlc)].copy_from_slice(&can_data[4..]);
    Ok(RawCanFrame {
        arbitration_id,
        id_format: if extended {
            CanIdFormat::Extended
        } else {
            CanIdFormat::Standard
        },
        dlc,
        data,
        device_timestamp: timestamp,
        source_route: open_route.route.id,
        source_network: open_route.route.network_name,
    })
}

pub(crate) fn parse_command_response(
    frame: &Frame,
    expected_command: u16,
    expected_sequence: u16,
    operation: &'static str,
) -> Result<CommandResponse, ProtocolError> {
    let payload = frame.payload();
    if payload.len() < COMMAND_HEADER_LENGTH + 4 {
        return Err(ProtocolError::PayloadTooShort(payload.len()));
    }
    let actual_command = read_u16(payload, 4).expect("header length checked");
    if actual_command != expected_command {
        return Err(ProtocolError::UnexpectedCommand {
            expected: expected_command,
            actual: actual_command,
        });
    }
    let actual_sequence = read_u16(payload, 6).expect("header length checked");
    if actual_sequence != expected_sequence {
        return Err(ProtocolError::SequenceMismatch {
            expected: expected_sequence,
            actual: actual_sequence,
        });
    }
    let status = read_u32(payload, COMMAND_HEADER_LENGTH).expect("response length checked");
    if status != 0 {
        return Err(ProtocolError::DeviceStatus {
            operation,
            status,
            message: firmware_message(&payload[COMMAND_HEADER_LENGTH..]),
        });
    }
    Ok(CommandResponse {
        channel_id: read_u16(payload, 2).expect("header length checked"),
    })
}

pub(crate) fn require_channel(
    response: CommandResponse,
    expected: u16,
) -> Result<CommandResponse, ProtocolError> {
    if response.channel_id != expected {
        return Err(ProtocolError::RouteMismatch {
            expected,
            actual: response.channel_id,
        });
    }
    Ok(response)
}

/// The text the firmware puts after the status and tick words of a refusal
/// ("SetPins: … only supports CAN on pins 6 and 14"), when there is one.
fn firmware_message(body: &[u8]) -> String {
    let Some(rest) = body.get(8..) else {
        return String::new();
    };
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(rest.len());
    let text = &rest[..end];
    if text.len() >= 4 && text.iter().all(|byte| (0x20..0x7F).contains(byte)) {
        String::from_utf8_lossy(text).into_owned()
    } else {
        String::new()
    }
}

pub(crate) fn command(route_a: u16, command: u16, sequence: u16, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(COMMAND_HEADER_LENGTH + body.len());
    payload.extend_from_slice(&route_a.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&command.to_le_bytes());
    payload.extend_from_slice(&sequence.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(body);
    outer_frame(&payload)
}

pub(crate) fn outer_frame(payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len()).expect("F2 commands have bounded static lengths");
    let mut bytes = Vec::with_capacity(FRAME_HEADER_LENGTH + payload.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&(length ^ VERIFIER_XOR).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}

pub(crate) fn command_response(
    channel_id: u16,
    command: u16,
    sequence: u16,
    status: u32,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&channel_id.to_le_bytes());
    payload.extend_from_slice(&command.to_le_bytes());
    payload.extend_from_slice(&sequence.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&status.to_le_bytes());
    outer_frame(&payload)
}

/// A command response with text after the status and tick words, the way
/// the firmware refuses: `SetPins: ... only supports CAN on pins 6 and 14`.
pub(crate) fn command_response_with_text(
    channel_id: u16,
    command: u16,
    sequence: u16,
    status: u32,
    tick: u32,
    text: &str,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(24 + text.len());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&channel_id.to_le_bytes());
    payload.extend_from_slice(&command.to_le_bytes());
    payload.extend_from_slice(&sequence.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&status.to_le_bytes());
    payload.extend_from_slice(&tick.to_le_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload.push(0);
    outer_frame(&payload)
}

pub(crate) fn inbound_can_frame(
    channel_id: u16,
    rx_status: u32,
    timestamp: u32,
    arbitration_id: u32,
    data: &[u8],
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(28 + data.len());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&channel_id.to_le_bytes());
    payload.extend_from_slice(&INBOUND_DATA.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&1_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&rx_status.to_le_bytes());
    payload.extend_from_slice(&timestamp.to_le_bytes());
    payload.extend_from_slice(
        &u16::try_from(4 + data.len())
            .expect("test CAN payload length")
            .to_le_bytes(),
    );
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&arbitration_id.to_be_bytes());
    payload.extend_from_slice(data);
    outer_frame(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameDecoder;

    fn decode(bytes: &[u8]) -> Frame {
        let mut decoder = FrameDecoder::default();
        decoder.push(bytes);
        decoder.next_frame().unwrap().unwrap()
    }

    fn open_route(id: VehicleRouteId, channel: u16) -> OpenReceiveRoute {
        OpenReceiveRoute {
            route: *route_by_id(id),
            device_channel_id: channel,
        }
    }

    #[test]
    fn exact_listen_only_open_serialization_is_golden() {
        assert_eq!(
            open_channel_request(1, CAN1_RESOURCE_ROUTE, 500_000),
            [
                0x14, 0x00, 0xF2, 0x51, 0x01, 0x05, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x20, 0xA1, 0x07, 0x00,
            ]
        );
    }

    #[test]
    fn exact_active_open_and_fixed_outbound_serialization_are_golden() {
        let active = diagnostic_open_channel_request(1, CAN1_RESOURCE_ROUTE, 500_000);
        assert_eq!(&active[16..20], &0_u32.to_le_bytes());
        assert_eq!(&active[20..24], &500_000_u32.to_le_bytes());

        assert_eq!(
            outbound_data_request(3, 0x0102, 0x7e0, false, &[0x02, 0x09, 0x04]),
            [
                0x1F, 0x00, 0xF9, 0x51, 0x02, 0x01, 0x00, 0x00, 0x08, 0x00, 0x03, 0x00, 0x01, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x07, 0xE0, 0x02, 0x09, 0x04,
            ]
        );
    }

    #[test]
    fn exact_set_pin_and_close_serialization_is_golden() {
        assert_eq!(
            set_pin_request(2, 0x0102, *route_by_id(VehicleRouteId::HsCan)),
            [
                0x18, 0x00, 0xFE, 0x51, 0x02, 0x01, 0x00, 0x00, 0x12, 0x00, 0x02, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
            ]
        );
        assert_eq!(
            close_channel_request(3, 0x0102),
            [
                0x0C, 0x00, 0xEA, 0x51, 0x02, 0x01, 0x00, 0x00, 0x07, 0x00, 0x03, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ]
        );
    }

    #[test]
    fn parses_variable_dlc_and_route_identity() {
        for dlc in 0..=8 {
            let data = (0..dlc).collect::<Vec<_>>();
            let bytes = inbound_can_frame(0x0102, 0, 42, 0x321, &data);
            let frame =
                parse_inbound_can(&decode(&bytes), open_route(VehicleRouteId::HsCan, 0x0102))
                    .unwrap();
            assert_eq!(frame.dlc, dlc);
            assert_eq!(frame.payload(), data);
            assert_eq!(frame.id_format, CanIdFormat::Standard);
            assert_eq!(frame.source_route, VehicleRouteId::HsCan);
        }
    }

    #[test]
    fn parses_confirmed_29_bit_status_flag() {
        let bytes = inbound_can_frame(0x0102, CAN_29BIT_ID, 77, 0x18DA_F110, &[0xAA]);
        let frame =
            parse_inbound_can(&decode(&bytes), open_route(VehicleRouteId::MsCan, 0x0102)).unwrap();
        assert_eq!(frame.id_format, CanIdFormat::Extended);
        assert_eq!(frame.arbitration_id, 0x18DA_F110);
        assert_eq!(frame.device_timestamp, 77);
    }

    #[test]
    fn malformed_size_id_and_route_fail_closed() {
        let mut invalid_size = inbound_can_frame(0x0102, 0, 0, 0x123, &[]);
        invalid_size[24] = 3;
        assert!(matches!(
            parse_inbound_can(
                &decode(&invalid_size),
                open_route(VehicleRouteId::HsCan, 0x0102)
            ),
            Err(ProtocolError::InvalidCanDataSize(3))
        ));
        let invalid_id = inbound_can_frame(0x0102, 0, 0, 0x800, &[]);
        assert!(matches!(
            parse_inbound_can(
                &decode(&invalid_id),
                open_route(VehicleRouteId::HsCan, 0x0102)
            ),
            Err(ProtocolError::InvalidCanId { .. })
        ));
        let other_route = inbound_can_frame(0x0103, 0, 0, 0x123, &[]);
        assert!(matches!(
            parse_inbound_can(
                &decode(&other_route),
                open_route(VehicleRouteId::HsCan, 0x0102)
            ),
            Err(ProtocolError::RouteMismatch { .. })
        ));
    }

    #[test]
    fn truncated_inbound_record_is_rejected() {
        let mut bytes = inbound_can_frame(0x0102, 0, 0, 0x123, &[1, 2, 3]);
        bytes.truncate(bytes.len() - 2);
        let declared_payload = u16::try_from(bytes.len() - 4).unwrap();
        bytes[0..2].copy_from_slice(&declared_payload.to_le_bytes());
        bytes[2..4].copy_from_slice(&(declared_payload ^ VERIFIER_XOR).to_le_bytes());
        assert!(matches!(
            parse_inbound_can(&decode(&bytes), open_route(VehicleRouteId::HsCan, 0x0102)),
            Err(ProtocolError::TruncatedFrame { .. })
        ));
    }

    #[test]
    fn production_can_routes_exclude_unsupported_pins_12_and_13() {
        assert_eq!(list_vehicle_routes().len(), 4);
        assert_eq!(
            list_vehicle_routes()
                .iter()
                .filter(|route| route.passive_capability == PassiveCapability::Ready)
                .map(|route| route.id)
                .collect::<Vec<_>>(),
            [VehicleRouteId::HsCan, VehicleRouteId::MsCan]
        );
        assert!(list_vehicle_routes()
            .iter()
            .flat_map(|route| route.obd_pins)
            .all(|pin| !matches!(pin, 12 | 13)));
        assert!("ccp-hs-can".parse::<VehicleRouteId>().is_err());
        assert!("tcm-comms".parse::<VehicleRouteId>().is_err());
    }

    /// ADR-0029: the two K-line routes are known by pin and by name, have no
    /// bit rate of their own, no resource word yet, and refuse every open —
    /// passive or diagnostic — with the reason.
    #[test]
    fn k_line_routes_are_named_pinned_and_blocked_until_the_second_slice() {
        for (id, text, pin) in [
            (VehicleRouteId::KLine7, "k-line-7", 7u8),
            (VehicleRouteId::KLine8, "k-line-8", 8u8),
        ] {
            assert_eq!(id.as_str(), text);
            assert_eq!(text.parse::<VehicleRouteId>(), Ok(id));
            let route = route_by_id(id);
            assert_eq!(route.obd_pins, &[pin]);
            assert_eq!(route.bitrate, None);
            assert_eq!(route.network_type, NetworkType::Other);
            assert_eq!(
                route.passive_capability,
                PassiveCapability::Blocked(K_LINE_NOT_OPENED)
            );
            assert_eq!(resource_route(id), None);
        }
        assert_eq!(
            resource_route(VehicleRouteId::HsCan),
            Some(CAN1_RESOURCE_ROUTE)
        );
        assert_eq!(
            resource_route(VehicleRouteId::MsCan),
            Some(CAN2_RESOURCE_ROUTE)
        );
    }
}
