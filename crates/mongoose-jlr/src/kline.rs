//! The adapter's K-line words (`ADR-0029`, decision 6), and what the adapter
//! itself has said about each of them.
//!
//! This module is the one place those words live. On 2026-09-12 the probe
//! `scripts/mongoose-probe/mongoose_kline_probe.py` asked the adapter on the
//! bench, with no vehicle attached, and it answered
//! (`docs/evidence/mongoose-probe-2026-09-12/`): the two resources open, the
//! pin selection is taken and is **validated** — a wrong pin is refused with
//! *"MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8"* — and
//! the fast init ISO 14230 asks for is a command the firmware has.
//!
//! One word is still a guess and is marked as one: the parity. The firmware
//! takes the config command with a parity of 99 as readily as with the real
//! value, so its acceptance proves nothing. Both protocols this product
//! speaks on a K-line want even parity, so the first module that answers is
//! what confirms it — `ADR-0015`'s rule, on a line instead of a bus.

use crate::codec::Frame;
use crate::error::ProtocolError;
use crate::passive::{
    self, command, outer_frame, read_u16, read_u32, OpenReceiveRoute, VehicleRouteId,
    COMMAND_HEADER_LENGTH, INBOUND_DATA, OUTBOUND_DATA,
};
use kline_execution::{DS2_PROTOCOL_FAMILY, KWP2000_PROTOCOL_FAMILY};

/// **Confirmed 2026-09-12.** The resource word for a K-line, built the way
/// the two CAN words are: CAN1 is resource 5 as `0x0501`, CAN2 is resource 21
/// as `0x1501`, so resource `n` is `(n << 8) | 0x01`. Both open on this
/// adapter — 3 at 9600 baud and 4 at 10400 — and answer status 0.
pub const ISO9141_RESOURCE_ROUTE: u16 = 0x0301;
pub const ISO14230_RESOURCE_ROUTE: u16 = 0x0401;

/// **Handled, but unconfirmed.** The command that sets a line parameter.
/// `0x0010` is a command this firmware handles — it does not answer
/// *"Unknown/Unhandled Command"* as it does for `0x0016` and above — and it
/// takes J2534's own `SCONFIG` shape. It also takes a parity of 99 and a
/// parameter that does not exist, so nothing about what it *sets* is known.
/// A module answering is what will confirm it.
///
/// Its neighbours were named by the same run and are not to be sent:
/// `0x0011` is Fast Init, `0x0013` is `cGetString`, and **`0x0014` is
/// `SetData`, a write to the adapter** — this product does not write to the
/// adapter, and nothing here sends that word.
pub const SET_CONFIG: u16 = 0x0010;
pub const SET_CONFIG_RESPONSE: u16 = 0x8010;

/// **Confirmed as a command 2026-09-12.** The fast init ISO 14230 asks for
/// before the first request: the firmware answered *"Fast Init: Timeout on
/// response"* with no vehicle on the line, which is a fast init that found
/// nobody. It is what the platform documents' `kw2000_fast` wake-up needs,
/// and the adapter does the timing itself.
pub const FAST_INIT: u16 = 0x0011;
pub const FAST_INIT_RESPONSE: u16 = 0x8011;

/// SDD's own names for the wake-ups of the buses this product speaks
/// (`ADR-0029`). `bmw_ds2` needs none: DS2 has no init sequence, a request
/// is simply framed and sent.
pub const WAKEUP_NONE: &str = "none";
pub const WAKEUP_BMW_DS2: &str = "bmw_ds2";
pub const WAKEUP_KW2000_FAST: &str = "kw2000_fast";

/// **Confirmed as the command, not as the parameter.** J2534's own parameter
/// number for parity, and its values:
/// 0 none, 1 odd, 2 even. Both protocols this product speaks on a K-line ask
/// for **even** parity — DS2 at 9600 8E1, KWP2000 at 10400 8E1 — so this is
/// the word that decides whether a K-line read is possible at all. The 8N1
/// buses of the corpus (`ROSCO`, `KW2000STAR`, `NVJCOM`) are protocols this
/// product does not speak.
pub const PARITY_PARAMETER: u32 = 0x16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineParity {
    None,
    Odd,
    Even,
}

impl LineParity {
    pub const fn value(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Odd => 1,
            Self::Even => 2,
        }
    }
}

/// The resource a line is opened on, chosen by the protocol's physical layer
/// (`ADR-0029`, decision 2). A protocol this product does not speak has no
/// resource here, so it cannot be opened by accident.
pub(crate) fn resource_for_protocol(protocol_family: &str) -> Option<u16> {
    match protocol_family {
        DS2_PROTOCOL_FAMILY => Some(ISO9141_RESOURCE_ROUTE),
        KWP2000_PROTOCOL_FAMILY => Some(ISO14230_RESOURCE_ROUTE),
        _ => None,
    }
}

/// The fast init request, for a bus whose wake-up asks for one.
pub(crate) fn fast_init_request(sequence: u16, channel_id: u16, target: u8) -> Vec<u8> {
    // The address the init addresses, in the same little-endian word shape
    // the other channel commands use.
    let mut body = Vec::with_capacity(4);
    body.extend_from_slice(&u32::from(target).to_le_bytes());
    command(channel_id, FAST_INIT, sequence, &body)
}

pub(crate) fn parse_fast_init_response(
    frame: &Frame,
    expected_sequence: u16,
    channel_id: u16,
) -> Result<passive::CommandResponse, ProtocolError> {
    let response =
        passive::parse_command_response(frame, FAST_INIT_RESPONSE, expected_sequence, "cFastInit")?;
    passive::require_channel(response, channel_id)
}

/// Whether this bus's wake-up is one the adapter can do. A wake-up SDD names
/// and this product cannot perform is refused rather than skipped: a line
/// woken the wrong way is a line that answers nothing, and saying so is
/// better than a silent read.
pub(crate) fn wakeup_is_supported(wakeup: &str) -> bool {
    matches!(wakeup, WAKEUP_NONE | WAKEUP_BMW_DS2 | WAKEUP_KW2000_FAST)
}

/// The J1962 pin a K-line route is on. A route that is not a K-line has none.
pub(crate) fn pin_for_route(route: VehicleRouteId) -> Option<u8> {
    match route {
        VehicleRouteId::KLine7 => Some(7),
        VehicleRouteId::KLine8 => Some(8),
        VehicleRouteId::HsCan | VehicleRouteId::MsCan => None,
    }
}

/// **Confirmed 2026-09-12.** Selecting a single line: a count, then the pin
/// in the first of the two pin fields, and zero in the second. The firmware
/// took it on both K-line resources and refused pin 99 and pin 0 with
/// *"MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8"*, which
/// is what makes the acceptance mean something.
pub(crate) fn set_pin_request(sequence: u16, channel_id: u16, pin: u8) -> Vec<u8> {
    let mut body = Vec::with_capacity(12);
    body.extend_from_slice(&1_u32.to_le_bytes());
    body.extend_from_slice(&u32::from(pin).to_le_bytes());
    body.extend_from_slice(&0_u32.to_le_bytes());
    command(channel_id, passive::SET_PIN, sequence, &body)
}

/// One line parameter, in J2534's own `SCONFIG_LIST` shape: a count, then
/// parameter-and-value pairs. The adapter takes it; whether it acts on it is
/// the open question above.
pub(crate) fn set_parity_request(sequence: u16, channel_id: u16, parity: LineParity) -> Vec<u8> {
    let mut body = Vec::with_capacity(12);
    body.extend_from_slice(&1_u32.to_le_bytes());
    body.extend_from_slice(&PARITY_PARAMETER.to_le_bytes());
    body.extend_from_slice(&parity.value().to_le_bytes());
    command(channel_id, SET_CONFIG, sequence, &body)
}

/// The adapter's answer to the parity command, when it has one to give.
pub(crate) fn parse_set_config_response(
    frame: &Frame,
    expected_sequence: u16,
    channel_id: u16,
) -> Result<passive::CommandResponse, ProtocolError> {
    let response = passive::parse_command_response(
        frame,
        SET_CONFIG_RESPONSE,
        expected_sequence,
        "cSetConfig",
    )?;
    passive::require_channel(response, channel_id)
}

/// The bytes to put on the line, in the outbound record the CAN path uses —
/// minus the four identifier bytes, because a K-line frame has no identifier
/// and its first byte is the protocol's own. The firmware took this record
/// on 2026-09-12 and failed it at the line, with nothing connected, which is
/// what an empty line should do; that it reaches a module correctly is still
/// for a module to say.
pub(crate) fn outbound_line_request(sequence: u16, channel_id: u16, bytes: &[u8]) -> Vec<u8> {
    let data_size = u16::try_from(bytes.len()).expect("a K-line request is bounded");
    let mut payload = Vec::with_capacity(24 + bytes.len());
    payload.extend_from_slice(&channel_id.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&OUTBOUND_DATA.to_le_bytes());
    payload.extend_from_slice(&sequence.to_le_bytes());
    payload.extend_from_slice(&1_u16.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&data_size.to_le_bytes());
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(bytes);
    outer_frame(&payload)
}

/// **Unconfirmed.** An inbound record read as line bytes: the same header the
/// CAN path reads, with every byte of the record's data being the line's,
/// where the CAN path takes the first four as an identifier. Nothing has come
/// back on a K-line yet.
pub(crate) fn parse_inbound_line(
    frame: &Frame,
    open_route: OpenReceiveRoute,
) -> Result<Vec<u8>, ProtocolError> {
    let payload = frame.payload();
    if payload.len() < COMMAND_HEADER_LENGTH {
        return Err(ProtocolError::PayloadTooShort(payload.len()));
    }
    let command_word = read_u16(payload, 4).expect("header length checked");
    if command_word != INBOUND_DATA {
        return Err(ProtocolError::UnexpectedCommand {
            expected: INBOUND_DATA,
            actual: command_word,
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
    let _rx_status = read_u32(body, 0).expect("body length checked");
    let data_size = usize::from(read_u16(body, 8).expect("body length checked"));
    let required = 12 + data_size;
    if body.len() < required {
        return Err(ProtocolError::TruncatedFrame {
            expected: COMMAND_HEADER_LENGTH + required,
            actual: payload.len(),
        });
    }
    Ok(body[12..required].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_resource_word_follows_the_two_that_are_confirmed() {
        // CAN1 is resource 5 as 0x0501 and CAN2 resource 21 as 0x1501; the
        // K-line words are built the same way from resources 3 and 4.
        assert_eq!(ISO9141_RESOURCE_ROUTE, (3 << 8) | 0x01);
        assert_eq!(ISO14230_RESOURCE_ROUTE, (4 << 8) | 0x01);
        assert_eq!(
            resource_for_protocol(DS2_PROTOCOL_FAMILY),
            Some(ISO9141_RESOURCE_ROUTE)
        );
        assert_eq!(
            resource_for_protocol(KWP2000_PROTOCOL_FAMILY),
            Some(ISO14230_RESOURCE_ROUTE)
        );
        // A protocol this product does not speak has no resource at all.
        assert_eq!(resource_for_protocol("KW2000STAR"), None);
        assert_eq!(resource_for_protocol("ROSCO"), None);
    }

    #[test]
    fn a_k_line_request_carries_its_bytes_and_no_identifier() {
        let request = outbound_line_request(0x1234, 0x0301, &[0x72, 0x04, 0x00, 0x76]);
        // The outer frame is the length and its verifier, then the record.
        let body = &request[4..];
        assert_eq!(read_u16(body, 0), Some(0x0301));
        assert_eq!(read_u16(body, 4), Some(OUTBOUND_DATA));
        assert_eq!(read_u16(body, 6), Some(0x1234));
        assert_eq!(read_u16(body, 20), Some(4), "the data size is the bytes");
        assert_eq!(&body[24..], &[0x72, 0x04, 0x00, 0x76]);
    }

    #[test]
    fn the_pin_and_the_parity_are_one_place_each() {
        let pins = set_pin_request(1, 0x0301, 7);
        assert_eq!(&pins[4 + 12..], &[1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]);
        let parity = set_parity_request(2, 0x0301, LineParity::Even);
        assert_eq!(&parity[4 + 12..], &[1, 0, 0, 0, 0x16, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(LineParity::None.value(), 0);
        assert_eq!(LineParity::Odd.value(), 1);
    }
}
