//! Live execution of a prepared UDS read on the MongoosePro JLR (ADR-0015).
//!
//! Mirrors the F8 calibration path: only a `PreparedUdsTransaction` from
//! `uds-execution` can reach this code, the route is validated against the
//! adapter's own descriptor before anything is sent, one single-frame request
//! goes out padded to eight bytes, a First Frame is answered with one Flow
//! Control, ResponsePending is waited through, and the route is closed
//! whatever happens. The raw response is returned undecoded; decoding is the
//! `uds_execution::decode_response` the offline paths use.

use crate::device::{MongooseDiagnosticError, MongooseJlrDevice};
use crate::passive::CanIdFormat;
use crate::passive::{self, VehicleRouteId};
use std::time::{Duration, Instant};
use transport_api::ByteTransport;
use uds_execution::{
    PreparedUdsTransaction, TransactionSafetyClass, READ_DATA_BY_IDENTIFIER_CAPABILITY,
    READ_DTC_INFORMATION_CAPABILITY, UDS_PROTOCOL_FAMILY,
};

/// How long to keep waiting once a module has answered ResponsePending.
pub const UDS_PENDING_TIMEOUT: Duration = Duration::from_secs(5);
/// ISO 15765-4 frames are eight bytes; JLR modules are strict about it.
const PADDING: u8 = 0x00;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongooseUdsReadResult {
    pub route: VehicleRouteId,
    pub responder: u32,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    /// ResponsePending answers received before the final one.
    pub pending_responses: u32,
}

impl<T: ByteTransport> MongooseJlrDevice<T> {
    /// Executes one prepared read-only UDS request. The wire-level primitives
    /// are private; callers must provide a transaction `uds-execution`
    /// prepared from a RESOLVED plan.
    pub fn execute_prepared_uds_read(
        &mut self,
        transaction: &PreparedUdsTransaction,
        timeout: Duration,
    ) -> Result<MongooseUdsReadResult, MongooseDiagnosticError> {
        let route_id = validate_uds_transaction(transaction)?;
        self.open_route_internal(route_id, false)
            .map_err(MongooseDiagnosticError::CanConnection)?;
        let result = self.execute_uds_inner(transaction, route_id, timeout);
        let close = self.close_route();
        match (result, close) {
            (Ok(result), Ok(())) => Ok(result),
            (Ok(_), Err(error)) => Err(MongooseDiagnosticError::ChannelClose(error)),
            (Err(error), _) => Err(error),
        }
    }

    fn execute_uds_inner(
        &mut self,
        transaction: &PreparedUdsTransaction,
        route_id: VehicleRouteId,
        timeout: Duration,
    ) -> Result<MongooseUdsReadResult, MongooseDiagnosticError> {
        let request_payload = transaction.encoded_payload().to_vec();
        let request_frames = isotp::segment(&request_payload, Some(PADDING))?;
        let [request_frame] = request_frames.as_slice() else {
            return Err(MongooseDiagnosticError::UnsupportedTransaction(
                "UDS read request must be one ISO-TP single frame",
            ));
        };
        let request_id = transaction.physical_request_id();
        let expected_response = transaction.expected_response_id();
        let expected_format = if expected_response.is_extended() {
            CanIdFormat::Extended
        } else {
            CanIdFormat::Standard
        };
        let service_id = transaction.protocol_request().service_id();
        self.transmit_diagnostic_frame(request_id, request_frame)
            .map_err(MongooseDiagnosticError::RequestTransmission)?;

        let mut started = Instant::now();
        let mut budget = timeout;
        let mut pending_responses = 0u32;
        let mut reassembler = isotp::Reassembler::new();
        loop {
            let Some(remaining) = budget.checked_sub(started.elapsed()) else {
                return Err(MongooseDiagnosticError::Timeout);
            };
            let Some(frame) = self
                .receive_frame(remaining)
                .map_err(MongooseDiagnosticError::ResponseReception)?
            else {
                return Err(MongooseDiagnosticError::Timeout);
            };
            // Other identifiers are other conversations on a shared bus; an
            // 11-bit and a 29-bit identifier with the same value are different
            // identifiers.
            if frame.arbitration_id != expected_response.value()
                || frame.id_format != expected_format
            {
                continue;
            }

            let first_frame = matches!(
                isotp::decode_frame(frame.payload())?,
                isotp::IsoTpFrame::First { .. }
            );
            let completed = reassembler.accept(
                frame.payload(),
                u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
            )?;
            if first_frame {
                self.transmit_diagnostic_frame(
                    request_id,
                    &[
                        0x30, 0x00, 0x00, PADDING, PADDING, PADDING, PADDING, PADDING,
                    ],
                )
                .map_err(MongooseDiagnosticError::RequestTransmission)?;
            }
            let Some(raw_response) = completed else {
                continue;
            };
            if is_response_pending(&raw_response, service_id) {
                // NRC 0x78: the module is working on it. ISO 14229 restarts the
                // response timer, at its enhanced value.
                pending_responses += 1;
                started = Instant::now();
                budget = UDS_PENDING_TIMEOUT.max(timeout);
                reassembler = isotp::Reassembler::new();
                continue;
            }
            return Ok(MongooseUdsReadResult {
                route: route_id,
                responder: expected_response.value(),
                request_payload,
                raw_diagnostic_response: raw_response,
                pending_responses,
            });
        }
    }
}

fn is_response_pending(payload: &[u8], service_id: u8) -> bool {
    matches!(payload, [0x7F, sid, 0x78, ..] if *sid == service_id)
}

/// The transaction must describe exactly what this backend can do: a
/// read-only allowlisted UDS service, ISO 14229 over normal 11-bit addressing,
/// on one of the adapter's own routes with that route's pins and rate.
fn validate_uds_transaction(
    transaction: &PreparedUdsTransaction,
) -> Result<VehicleRouteId, MongooseDiagnosticError> {
    if transaction.safety_class() != TransactionSafetyClass::ReadOnly {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "safety class is not READ_ONLY",
        ));
    }
    let capability = transaction.capability_id();
    if capability != READ_DATA_BY_IDENTIFIER_CAPABILITY
        && capability != READ_DTC_INFORMATION_CAPABILITY
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "capability is not an allowlisted UDS read",
        ));
    }
    if transaction.protocol_family() != UDS_PROTOCOL_FAMILY {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "protocol is not ISO 14229",
        ));
    }
    // Normal addressing on 11-bit identifiers, and normal fixed addressing on
    // 29-bit ones (ADR-0017). SDD's `enhanced` scheme is not executed.
    match (
        transaction.addressing_mode(),
        transaction.physical_request_id().is_extended(),
        transaction.expected_response_id().is_extended(),
    ) {
        ("normal", false, false) | ("normal_fixed", true, true) => {}
        _ => {
            return Err(MongooseDiagnosticError::UnsupportedTransaction(
                "only normal 11-bit and normal_fixed 29-bit addressing are executed live",
            ))
        }
    }
    let route_id: VehicleRouteId = transaction.backend_route().parse().map_err(|_| {
        MongooseDiagnosticError::UnsupportedTransaction("backend route is not a Mongoose route")
    })?;
    let route = passive::route_by_id(route_id);
    if transaction.physical_pins() != route.obd_pins
        || Some(transaction.bitrate_bps()) != route.bitrate
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "prepared route does not match the Mongoose route descriptor",
        ));
    }

    Ok(route_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::passive::{
        command_response, inbound_can_frame, CAN_29BIT_ID, CLOSE_CHANNEL_RESPONSE,
        OPEN_CHANNEL_RESPONSE, OUTBOUND_DATA_RESPONSE, SET_PIN_RESPONSE,
    };
    use diagnostic_environment::{
        CanIdFormat as IdFormat, DiagnosticEnvironmentResolution, ReadableIdentifier,
        ValidationState,
    };
    use jlr_profiles::{x250_2010_supercharged_ecm_environment, ECU_FAMILY};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use transport_api::TransportError;
    use uds_execution::{
        prepare_read_only_transaction, DiagnosticTargetIdentity, DtcStatusMask, ReadOnlyUdsIntent,
    };

    enum Step {
        Bytes(Vec<u8>),
        Silent,
    }

    struct ScriptedTransport {
        steps: VecDeque<Step>,
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl ScriptedTransport {
        fn new(steps: Vec<Step>) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
            let writes = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    steps: steps.into(),
                    writes: Arc::clone(&writes),
                },
                writes,
            )
        }
    }

    impl ByteTransport for ScriptedTransport {
        fn write_all(&mut self, bytes: &[u8]) -> Result<(), TransportError> {
            self.writes.lock().unwrap().push(bytes.to_vec());
            if matches!(self.steps.front(), Some(Step::Silent)) {
                self.steps.pop_front();
            }
            Ok(())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            match self.steps.front() {
                Some(Step::Silent) | None => Err(TransportError::Timeout),
                Some(Step::Bytes(_)) => {
                    let Some(Step::Bytes(bytes)) = self.steps.pop_front() else {
                        unreachable!()
                    };
                    let count = bytes.len().min(buffer.len());
                    buffer[..count].copy_from_slice(&bytes[..count]);
                    Ok(count)
                }
            }
        }

        fn set_read_timeout(&mut self, _timeout: Duration) -> Result<(), TransportError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    const CHANNEL: u16 = crate::passive::CAN1_RESOURCE_ROUTE;

    /// Board-info as the running firmware answers it: the first exchange
    /// of every route open (ADR-0018).
    fn firmware_board_info(sequence: u16) -> Vec<u8> {
        crate::codec::encode_board_info_response(
            sequence,
            &[0, 0, 0, 0, 0x10, 0x20, 0x30, 0x40, 5, 1, 1, 1],
        )
    }

    /// A UDS plan for the F8 profile's module and route: the profile's evidence
    /// for HS-CAN, pins 6/14 and 0x7E0/0x7E8, with the protocol, addressing
    /// mode and capability a UDS read needs.
    fn plan(capability: &str, addressing: &str, backend: &str) -> DiagnosticEnvironmentResolution {
        let DiagnosticEnvironmentResolution::Resolved(mut plan) =
            x250_2010_supercharged_ecm_environment()
        else {
            unreachable!("profile environment is RESOLVED");
        };
        plan.protocol_family.value = UDS_PROTOCOL_FAMILY.into();
        plan.addressing_mode.value = addressing.into();
        plan.read_only_capability.value.id = capability.into();
        plan.backend_route.value.route_id = backend.into();
        DiagnosticEnvironmentResolution::Resolved(plan)
    }

    fn did_transaction(addressing: &str, backend: &str) -> PreparedUdsTransaction {
        let readable = ReadableIdentifier {
            identifier: 0x1945,
            parameters: Vec::new(),
            evidence: Vec::new(),
            validation_state: ValidationState::SourceBacked,
        };
        prepare_read_only_transaction(
            &plan(READ_DATA_BY_IDENTIFIER_CAPABILITY, addressing, backend),
            ReadOnlyUdsIntent::read_data_by_identifier(
                DiagnosticTargetIdentity::family(ECU_FAMILY).unwrap(),
                0x1945,
            ),
            &[readable],
        )
        .unwrap()
    }

    fn dtc_transaction() -> PreparedUdsTransaction {
        prepare_read_only_transaction(
            &plan(READ_DTC_INFORMATION_CAPABILITY, "normal", "hs-can"),
            ReadOnlyUdsIntent::read_dtc_information(
                DiagnosticTargetIdentity::family(ECU_FAMILY).unwrap(),
                DtcStatusMask::Confirmed,
            ),
            &[],
        )
        .unwrap()
    }

    fn opened() -> Vec<Step> {
        vec![
            Step::Bytes(firmware_board_info(1)),
            Step::Bytes(command_response(CHANNEL, OPEN_CHANNEL_RESPONSE, 2, 0)),
            Step::Bytes(command_response(CHANNEL, SET_PIN_RESPONSE, 3, 0)),
        ]
    }

    #[test]
    fn a_single_frame_read_is_sent_padded_and_the_answer_returned_raw() {
        let mut steps = opened();
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            4,
            0,
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL,
            0,
            10,
            0x7E8,
            &[0x05, 0x62, 0x19, 0x45, 0x12, 0x34, 0x00, 0x00],
        )));
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            CLOSE_CHANNEL_RESPONSE,
            5,
            0,
        )));
        let (transport, writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device
            .execute_prepared_uds_read(
                &did_transaction("normal", "hs-can"),
                Duration::from_millis(250),
            )
            .unwrap();

        assert_eq!(result.route, VehicleRouteId::HsCan);
        assert_eq!(result.responder, 0x7E8);
        assert_eq!(result.request_payload, [0x22, 0x19, 0x45]);
        assert_eq!(
            result.raw_diagnostic_response,
            [0x62, 0x19, 0x45, 0x12, 0x34]
        );
        assert_eq!(result.pending_responses, 0);

        let writes = writes.lock().unwrap();
        // Board-info, open for transmission (no listen-only flag), pins,
        // request, close.
        assert_eq!(writes.len(), 5);
        assert_eq!(&writes[1][16..20], &0_u32.to_le_bytes());
        assert_eq!(&writes[2][20..24], &6_u32.to_le_bytes());
        assert_eq!(&writes[2][24..28], &14_u32.to_le_bytes());
        assert!(writes[3]
            .ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x03, 0x22, 0x19, 0x45, 0x00, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn a_multi_frame_answer_is_flow_controlled_and_reassembled() {
        let response: Vec<u8> = [0x62, 0x19, 0x45]
            .into_iter()
            .chain(b"SYNTH-DATA".iter().copied())
            .collect();
        let frames = isotp::segment(&response, None).unwrap();
        assert_eq!(frames.len(), 2);

        let mut steps = opened();
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            4,
            0,
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL, 0, 10, 0x7E8, &frames[0],
        )));
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            5,
            0,
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL, 0, 20, 0x7E8, &frames[1],
        )));
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            CLOSE_CHANNEL_RESPONSE,
            6,
            0,
        )));
        let (transport, writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device
            .execute_prepared_uds_read(
                &did_transaction("normal", "hs-can"),
                Duration::from_millis(250),
            )
            .unwrap();
        assert_eq!(result.raw_diagnostic_response, response);

        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 6);
        // One padded Flow Control to the request identifier after the First Frame.
        assert!(writes[4]
            .ends_with(&[0x00, 0x00, 0x07, 0xE0, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn response_pending_is_waited_through_and_counted() {
        let mut steps = opened();
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            4,
            0,
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL,
            0,
            10,
            0x7E8,
            &[0x03, 0x7F, 0x19, 0x78],
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL,
            0,
            20,
            0x7E8,
            &[0x03, 0x59, 0x02, 0xFF],
        )));
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            CLOSE_CHANNEL_RESPONSE,
            5,
            0,
        )));
        let (transport, writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device
            .execute_prepared_uds_read(&dtc_transaction(), Duration::from_millis(250))
            .unwrap();
        assert_eq!(result.request_payload, [0x19, 0x02, 0x08]);
        assert_eq!(result.raw_diagnostic_response, [0x59, 0x02, 0xFF]);
        assert_eq!(result.pending_responses, 1);
        assert_eq!(writes.lock().unwrap().len(), 5);
    }

    #[test]
    fn silence_is_a_timeout_and_the_route_is_still_closed() {
        let mut steps = opened();
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            4,
            0,
        )));
        steps.push(Step::Silent);
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            CLOSE_CHANNEL_RESPONSE,
            5,
            0,
        )));
        let (transport, writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device.execute_prepared_uds_read(
            &did_transaction("normal", "hs-can"),
            Duration::from_millis(40),
        );
        assert!(matches!(result, Err(MongooseDiagnosticError::Timeout)));
        assert_eq!(writes.lock().unwrap().len(), 5);
    }

    #[test]
    fn transactions_outside_the_backend_are_refused_before_anything_is_written() {
        for (addressing, backend, reason) in [
            (
                "normal_fixed",
                "hs-can",
                "only normal 11-bit and normal_fixed 29-bit addressing are executed live",
            ),
            (
                "normal",
                "ms-can",
                "prepared route does not match the Mongoose route descriptor",
            ),
        ] {
            let (transport, writes) = ScriptedTransport::new(Vec::new());
            let mut device = MongooseJlrDevice::open(transport);
            let result = device.execute_prepared_uds_read(
                &did_transaction(addressing, backend),
                Duration::from_millis(10),
            );
            match result {
                Err(MongooseDiagnosticError::UnsupportedTransaction(actual)) => {
                    assert_eq!(actual, reason)
                }
                other => panic!("expected refusal, got {other:?}"),
            }
            assert!(writes.lock().unwrap().is_empty());
        }
    }

    /// ADR-0017: a normal_fixed read goes out as a 29-bit frame with the
    /// device's 29-bit flag, and only a 29-bit answer on the expected
    /// identifier is taken as the module's.
    #[test]
    fn a_normal_fixed_read_uses_29_bit_identifiers_end_to_end() {
        let DiagnosticEnvironmentResolution::Resolved(mut fixed) =
            plan(READ_DATA_BY_IDENTIFIER_CAPABILITY, "normal_fixed", "hs-can")
        else {
            unreachable!("profile environment is RESOLVED");
        };
        fixed.can_id_format.value = IdFormat::Extended29Bit;
        fixed.physical_request_id.value = 0x18DA_60F1;
        fixed.physical_response_id.value = 0x18DA_F160;
        let readable = ReadableIdentifier {
            identifier: 0x1945,
            parameters: Vec::new(),
            evidence: Vec::new(),
            validation_state: ValidationState::Unverified,
        };
        let transaction = prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(fixed),
            ReadOnlyUdsIntent::read_data_by_identifier(
                DiagnosticTargetIdentity::family(ECU_FAMILY).unwrap(),
                0x1945,
            ),
            &[readable],
        )
        .unwrap();

        let mut steps = opened();
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            OUTBOUND_DATA_RESPONSE,
            4,
            0,
        )));
        // An 11-bit frame whose value happens to match is another conversation.
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL,
            0,
            9,
            0x160,
            &[0x03, 0x7F, 0x22, 0x11, 0x00, 0x00, 0x00, 0x00],
        )));
        steps.push(Step::Bytes(inbound_can_frame(
            CHANNEL,
            CAN_29BIT_ID,
            10,
            0x18DA_F160,
            &[0x05, 0x62, 0x19, 0x45, 0x1A, 0xF8, 0x00, 0x00],
        )));
        steps.push(Step::Bytes(command_response(
            CHANNEL,
            CLOSE_CHANNEL_RESPONSE,
            5,
            0,
        )));
        let (transport, writes) = ScriptedTransport::new(steps);
        let mut device = MongooseJlrDevice::open(transport);

        let result = device
            .execute_prepared_uds_read(&transaction, Duration::from_millis(250))
            .unwrap();
        assert_eq!(result.responder, 0x18DA_F160);
        assert_eq!(
            result.raw_diagnostic_response,
            [0x62, 0x19, 0x45, 0x1A, 0xF8]
        );

        let writes = writes.lock().unwrap();
        let request = &writes[3];
        // Outer header, then channel, 0, command, sequence, 1, 0, two status
        // words, size, 0, identifier, eight data bytes.
        let base = request.len() - (28 + 8);
        assert_eq!(&request[base + 12..base + 16], &CAN_29BIT_ID.to_le_bytes());
        assert_eq!(&request[base + 16..base + 20], &CAN_29BIT_ID.to_le_bytes());
        assert_eq!(
            &request[base + 24..base + 28],
            &0x18DA_60F1_u32.to_be_bytes()
        );
        assert_eq!(&request[base + 28..base + 31], &[0x03, 0x22, 0x19]);
    }
}
