//! What a module puts on the wire, for the bench and the fixtures: the
//! positive responses, built the way the standard lays them out. Nothing
//! here decodes, and nothing here transmits.

use crate::dtc::{dtc_bytes, DtcKind};
use crate::support::encode_support_bitmap;
use crate::vehicle_info::{INFO_TYPE_CVN, INFO_TYPE_ECU_NAME, INFO_TYPE_VIN};

/// `41 PID data …` for the pairs given, in order.
pub fn current_data_response(pids: &[(u8, &[u8])]) -> Vec<u8> {
    let mut payload = vec![0x41];
    for (pid, data) in pids {
        payload.push(*pid);
        payload.extend_from_slice(data);
    }
    payload
}

/// `41 PID <bitmap>` declaring `supported` after the support PID `base`.
pub fn support_response(mode: u8, base: u8, supported: &[u8]) -> Vec<u8> {
    let mut payload = vec![mode + 0x40, base];
    payload.extend_from_slice(&encode_support_bitmap(base, supported));
    payload
}

/// `42 PID frame data`.
pub fn freeze_frame_response(pid: u8, frame: u8, data: &[u8]) -> Vec<u8> {
    let mut payload = vec![0x42, pid, frame];
    payload.extend_from_slice(data);
    payload
}

/// `43|47|4A count pairs…`; a code that is not a code is skipped.
pub fn dtc_list_response(kind: DtcKind, codes: &[&str]) -> Vec<u8> {
    let pairs: Vec<[u8; 2]> = codes.iter().filter_map(|code| dtc_bytes(code)).collect();
    let mut payload = vec![kind.mode() + 0x40, pairs.len() as u8];
    for pair in pairs {
        payload.extend_from_slice(&pair);
    }
    payload
}

/// `46 MID TID UAS value min max` for one result.
pub fn monitor_result_response(results: &[(u8, u8, u8, u16, u16, u16)]) -> Vec<u8> {
    let mut payload = vec![0x46];
    for (mid, tid, uas, value, minimum, maximum) in results {
        payload.extend_from_slice(&[*mid, *tid, *uas]);
        payload.extend_from_slice(&value.to_be_bytes());
        payload.extend_from_slice(&minimum.to_be_bytes());
        payload.extend_from_slice(&maximum.to_be_bytes());
    }
    payload
}

/// `49 02 01 <17 characters>`.
pub fn vin_response(vin: &str) -> Vec<u8> {
    let mut payload = vec![0x49, INFO_TYPE_VIN, 0x01];
    let mut bytes = vin.as_bytes().to_vec();
    bytes.resize(17, 0);
    payload.extend_from_slice(&bytes[..17]);
    payload
}

/// `49 06 count <4 bytes each>`.
pub fn cvn_response(cvns: &[[u8; 4]]) -> Vec<u8> {
    let mut payload = vec![0x49, INFO_TYPE_CVN, cvns.len() as u8];
    for cvn in cvns {
        payload.extend_from_slice(cvn);
    }
    payload
}

/// `49 0A 01 <20 bytes>`: the short name, a NUL, the long one.
pub fn ecu_name_response(short: &str, long: &str) -> Vec<u8> {
    let mut payload = vec![0x49, INFO_TYPE_ECU_NAME, 0x01];
    let mut name = short.as_bytes().to_vec();
    name.push(0);
    name.extend_from_slice(long.as_bytes());
    name.resize(20, 0);
    payload.extend_from_slice(&name[..20]);
    payload
}

/// `49 08|0B count <2 bytes each>`.
pub fn in_use_performance_response(info_type: u8, counters: &[u16]) -> Vec<u8> {
    let mut payload = vec![0x49, info_type, counters.len() as u8];
    for counter in counters {
        payload.extend_from_slice(&counter.to_be_bytes());
    }
    payload
}

/// `7F mode code`.
pub fn negative_response(mode: u8, code: u8) -> Vec<u8> {
    vec![0x7F, mode, code]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle_info::VehicleInformation;
    use crate::{decode_response, J1979Request, J1979Response, ParameterValue};

    #[test]
    fn what_the_bench_writes_the_decoder_reads_back() {
        let request = J1979Request::current_data(&[0x0C, 0x0D]).unwrap();
        let payload = current_data_response(&[(0x0C, &[0x1A, 0xF8]), (0x0D, &[0x3C])]);
        match decode_response(request, &payload).unwrap() {
            J1979Response::CurrentData(values) => {
                assert_eq!(values[0].value, ParameterValue::Number(1726.0));
                assert_eq!(values[1].value, ParameterValue::Number(60.0));
            }
            other => panic!("{other:?}"),
        }

        let support = support_response(0x01, 0x00, &[0x0C, 0x0D, 0x20]);
        match decode_response(J1979Request::current_data(&[0x00]).unwrap(), &support).unwrap() {
            J1979Response::CurrentData(values) => {
                assert_eq!(
                    values[0].value,
                    ParameterValue::Supported(vec![0x0C, 0x0D, 0x20])
                );
            }
            other => panic!("{other:?}"),
        }

        let codes = dtc_list_response(DtcKind::Pending, &["P0300", "nonsense", "U0100"]);
        match decode_response(J1979Request::pending_dtcs(), &codes).unwrap() {
            J1979Response::Dtcs { kind, codes } => {
                assert_eq!(kind, DtcKind::Pending);
                assert_eq!(codes, ["P0300", "U0100"]);
            }
            other => panic!("{other:?}"),
        }

        let vin = vin_response("SAJBENCH000000001");
        match decode_response(J1979Request::vehicle_information(0x02).unwrap(), &vin).unwrap() {
            J1979Response::VehicleInformation(VehicleInformation::Vin(text)) => {
                assert_eq!(text, "SAJBENCH000000001");
            }
            other => panic!("{other:?}"),
        }

        let name = ecu_name_response("ECM", "EngineControl");
        match decode_response(J1979Request::vehicle_information(0x0A).unwrap(), &name).unwrap() {
            J1979Response::VehicleInformation(VehicleInformation::EcuName(text)) => {
                assert_eq!(text, "ECM EngineControl");
            }
            other => panic!("{other:?}"),
        }

        let frozen = freeze_frame_response(0x05, 0, &[0x7B]);
        match decode_response(J1979Request::freeze_frame(0x05, 0), &frozen).unwrap() {
            J1979Response::FreezeFrame { frame, parameters } => {
                assert_eq!(frame, 0);
                assert_eq!(parameters[0].value, ParameterValue::Number(83.0));
            }
            other => panic!("{other:?}"),
        }

        let monitors = monitor_result_response(&[(0x21, 0x80, 0x0B, 500, 0, 1000)]);
        match decode_response(J1979Request::monitor_results(&[0x21]).unwrap(), &monitors).unwrap() {
            J1979Response::MonitorResults(results) => assert_eq!(results.len(), 1),
            other => panic!("{other:?}"),
        }
    }
}
