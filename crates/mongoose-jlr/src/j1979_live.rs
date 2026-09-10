//! Live execution of a prepared legislated OBD-II read on the MongoosePro
//! JLR (ADR-0022, decision 7).
//!
//! Mirrors the UDS path: only a `PreparedDiagnosticTransaction` from
//! `diagnostic-execution` can reach this code, the route is validated against
//! the adapter's own descriptor before anything is sent, one single-frame
//! request goes out padded to eight bytes, a First Frame is answered with one
//! Flow Control, ResponsePending is waited through, and the route is closed
//! whatever happens. The raw response is returned undecoded; decoding is the
//! codec's own `obd_j1979::decode_response`, as the offline paths use it.

use crate::device::{MongooseDiagnosticError, MongooseJlrDevice};
use crate::passive::{self, CanIdFormat, VehicleRouteId};
use diagnostic_execution::{
    PreparedDiagnosticTransaction, TransactionSafetyClass, CALIBRATION_IDENTIFICATION_CAPABILITY,
    STANDARD_OBD_CAPABILITY,
};
use obd_j1979::J1979Request;
use std::time::{Duration, Instant};
use transport_api::ByteTransport;

/// How long to keep waiting once a module has answered ResponsePending.
pub const J1979_PENDING_TIMEOUT: Duration = Duration::from_secs(5);
/// ISO 15765-4 frames are eight bytes; modules are strict about it.
const PADDING: u8 = 0x00;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MongooseJ1979ReadResult {
    pub route: VehicleRouteId,
    pub responder: u32,
    pub request_payload: Vec<u8>,
    pub raw_diagnostic_response: Vec<u8>,
    /// ResponsePending answers received before the final one.
    pub pending_responses: u32,
}

impl<T: ByteTransport> MongooseJlrDevice<T> {
    /// Executes one prepared legislated read. The wire-level primitives are
    /// private; callers must provide a transaction `diagnostic-execution`
    /// prepared from the standard.
    pub fn execute_prepared_j1979_read(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        timeout: Duration,
    ) -> Result<MongooseJ1979ReadResult, MongooseDiagnosticError> {
        let route_id = validate_j1979_transaction(transaction)?;
        self.open_route_internal(route_id, false)
            .map_err(MongooseDiagnosticError::CanConnection)?;
        let result = self.execute_j1979_inner(transaction, route_id, timeout);
        let close = self.close_route();
        match (result, close) {
            (Ok(result), Ok(())) => Ok(result),
            (Ok(_), Err(error)) => Err(MongooseDiagnosticError::ChannelClose(error)),
            (Err(error), _) => Err(error),
        }
    }

    fn execute_j1979_inner(
        &mut self,
        transaction: &PreparedDiagnosticTransaction,
        route_id: VehicleRouteId,
        timeout: Duration,
    ) -> Result<MongooseJ1979ReadResult, MongooseDiagnosticError> {
        let request_payload = transaction.encoded_payload().to_vec();
        let request_frames = isotp::segment(&request_payload, Some(PADDING))?;
        let [request_frame] = request_frames.as_slice() else {
            return Err(MongooseDiagnosticError::UnsupportedTransaction(
                "a legislated OBD request must be one ISO-TP single frame",
            ));
        };
        let request_id = transaction.physical_request_id();
        let expected_response = transaction.expected_response_id();
        let mode = transaction.protocol_request().mode();
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
            // Other identifiers are other conversations on a shared bus —
            // including the other seven responders answering someone else.
            if frame.arbitration_id != expected_response.value()
                || frame.id_format != CanIdFormat::Standard
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
            if is_response_pending(&raw_response, mode) {
                // NRC 0x78: the module is working on it. The response timer
                // restarts, at its enhanced value.
                pending_responses += 1;
                started = Instant::now();
                budget = J1979_PENDING_TIMEOUT.max(timeout);
                reassembler = isotp::Reassembler::new();
                continue;
            }
            return Ok(MongooseJ1979ReadResult {
                route: route_id,
                responder: expected_response.value(),
                request_payload,
                raw_diagnostic_response: raw_response,
                pending_responses,
            });
        }
    }
}

fn is_response_pending(payload: &[u8], mode: u8) -> bool {
    matches!(payload, [0x7F, sid, 0x78, ..] if *sid == mode)
}

/// The transaction must describe exactly what this backend can do: a
/// legislated read-only J1979 service — or F8's calibration identification —
/// over normal 11-bit addressing, on one of the adapter's own routes with
/// that route's pins and rate.
fn validate_j1979_transaction(
    transaction: &PreparedDiagnosticTransaction,
) -> Result<VehicleRouteId, MongooseDiagnosticError> {
    if transaction.safety_class() != TransactionSafetyClass::ReadOnly {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "safety class is not READ_ONLY",
        ));
    }
    let allowed = match transaction.capability_id() {
        STANDARD_OBD_CAPABILITY => true,
        CALIBRATION_IDENTIFICATION_CAPABILITY => {
            transaction.protocol_request() == J1979Request::CalibrationIdentification
        }
        _ => false,
    };
    if !allowed {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "capability is not a legislated OBD read",
        ));
    }
    if transaction.protocol_family() != "ISO15765-4 / SAE J1979"
        || transaction.addressing_mode() != "normal_physical"
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "protocol or addressing mode is not legislated OBD over 11-bit CAN",
        ));
    }
    if transaction.physical_request_id().is_extended()
        || transaction.expected_response_id().is_extended()
    {
        return Err(MongooseDiagnosticError::UnsupportedTransaction(
            "legislated OBD is executed on 11-bit identifiers only",
        ));
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
    use crate::bench::{share_bus, BenchTransport};
    use diagnostic_execution::{prepare_standard_obd_transaction, StandardObdResponder};
    use obd_j1979::bench::{current_data_response, negative_response, vin_response};
    use obd_j1979::{decode_response, J1979Response, ParameterValue, VehicleInformation};
    use transport_api::{BenchBus, BenchRoute, CanFrame, CanId};

    /// One emissions controller at the standard's first address, speaking
    /// only what the standard says, through the whole adapter framing.
    #[derive(Default)]
    struct EngineController {
        pending: Vec<Vec<u8>>,
    }

    impl EngineController {
        fn answer(request: &[u8]) -> Option<Vec<u8>> {
            Some(match request {
                [0x01, 0x0C, 0x0D] => {
                    current_data_response(&[(0x0C, &[0x0B, 0xB8]), (0x0D, &[0x00])])
                }
                [0x09, 0x02] => vin_response("SAJWA0HP1AMR12345"),
                [0x03] => negative_response(0x03, 0x12),
                _ => return None,
            })
        }
    }

    impl BenchBus for EngineController {
        fn on_frame(&mut self, route: BenchRoute, frame: &CanFrame) -> Vec<CanFrame> {
            if route != BenchRoute::HsCan || frame.id.value() != 0x7E0 {
                return Vec::new();
            }
            let reply = |data: Vec<u8>| {
                CanFrame::new(0, route.as_str(), CanId::standard(0x7E8).unwrap(), data).unwrap()
            };
            let pci = frame.data[0];
            match pci >> 4 {
                0x0 => {
                    let length = usize::from(pci & 0x0F);
                    let Some(payload) = Self::answer(&frame.data[1..1 + length]) else {
                        return Vec::new();
                    };
                    let mut frames = isotp::segment(&payload, Some(0)).unwrap();
                    let first = frames.remove(0);
                    self.pending = frames;
                    vec![reply(first)]
                }
                0x3 => std::mem::take(&mut self.pending)
                    .into_iter()
                    .map(reply)
                    .collect(),
                _ => Vec::new(),
            }
        }

        fn describe(&self) -> String {
            "one engine controller at 0x7E0".into()
        }
    }

    fn device() -> MongooseJlrDevice<BenchTransport> {
        MongooseJlrDevice::open(BenchTransport::new(share_bus(Box::new(
            EngineController::default(),
        ))))
    }

    #[test]
    fn a_legislated_read_goes_through_the_adapter_framing_and_comes_back_decoded() {
        let request = J1979Request::current_data(&[0x0C, 0x0D]).unwrap();
        let transaction =
            prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap())
                .unwrap();
        let result = device()
            .execute_prepared_j1979_read(&transaction, Duration::from_secs(2))
            .unwrap();
        assert_eq!(result.route, VehicleRouteId::HsCan);
        assert_eq!(result.responder, 0x7E8);
        assert_eq!(result.request_payload, [0x01, 0x0C, 0x0D]);
        assert_eq!(result.pending_responses, 0);
        match decode_response(request, &result.raw_diagnostic_response).unwrap() {
            J1979Response::CurrentData(values) => {
                assert_eq!(values[0].value, ParameterValue::Number(750.0));
                assert_eq!(values[1].value, ParameterValue::Number(0.0));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_multi_frame_vin_is_reassembled_with_one_flow_control() {
        let request = J1979Request::vehicle_information(0x02).unwrap();
        let transaction =
            prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap())
                .unwrap();
        let result = device()
            .execute_prepared_j1979_read(&transaction, Duration::from_secs(2))
            .unwrap();
        match decode_response(request, &result.raw_diagnostic_response).unwrap() {
            J1979Response::VehicleInformation(VehicleInformation::Vin(vin)) => {
                assert_eq!(vin, "SAJWA0HP1AMR12345");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_refusal_is_returned_raw_for_the_codec_to_name() {
        let request = J1979Request::stored_dtcs();
        let transaction =
            prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap())
                .unwrap();
        let result = device()
            .execute_prepared_j1979_read(&transaction, Duration::from_secs(2))
            .unwrap();
        assert_eq!(result.raw_diagnostic_response, [0x7F, 0x03, 0x12]);
        assert!(decode_response(request, &result.raw_diagnostic_response).is_err());
    }

    #[test]
    fn silence_is_a_timeout_and_the_route_is_closed_either_way() {
        let request = J1979Request::pending_dtcs();
        let transaction =
            prepare_standard_obd_transaction(request, StandardObdResponder::new(0).unwrap())
                .unwrap();
        let mut device = device();
        let error = device
            .execute_prepared_j1979_read(&transaction, Duration::from_millis(300))
            .unwrap_err();
        assert!(matches!(error, MongooseDiagnosticError::Timeout));
        // The route was closed on the way out: another read opens it again.
        let again = J1979Request::current_data(&[0x0C, 0x0D]).unwrap();
        let transaction =
            prepare_standard_obd_transaction(again, StandardObdResponder::new(0).unwrap()).unwrap();
        assert!(device
            .execute_prepared_j1979_read(&transaction, Duration::from_secs(2))
            .is_ok());
    }

    #[test]
    fn the_second_responder_is_asked_at_its_own_identifier_and_nobody_answers() {
        let request = J1979Request::current_data(&[0x0C, 0x0D]).unwrap();
        let transaction =
            prepare_standard_obd_transaction(request, StandardObdResponder::new(1).unwrap())
                .unwrap();
        let error = device()
            .execute_prepared_j1979_read(&transaction, Duration::from_millis(300))
            .unwrap_err();
        assert!(matches!(error, MongooseDiagnosticError::Timeout));
    }
}
