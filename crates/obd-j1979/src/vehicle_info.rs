//! Mode 09: vehicle information. On CAN every answer is `49 InfoType count
//! data`, the count being the number of items of that InfoType's fixed size.

use crate::support::{decode_support_bitmap, is_support_item};
use crate::{decode_calibration_identification, J1979Error, J1979Request};

/// The InfoTypes this crate reads. The others either do not exist on CAN
/// (the K-line message counts, 0x01, 0x03, 0x05, 0x07, 0x09) or are not
/// something a read-only tool has business with.
pub fn is_read_only_info_type(info_type: u8) -> bool {
    is_support_item(info_type)
        || matches!(info_type, 0x02 | 0x04 | 0x06 | 0x08 | 0x0A | 0x0B | 0x0D)
}

pub const INFO_TYPE_VIN: u8 = 0x02;
pub const INFO_TYPE_CALIBRATION_ID: u8 = 0x04;
pub const INFO_TYPE_CVN: u8 = 0x06;
pub const INFO_TYPE_IN_USE_PERFORMANCE_SPARK: u8 = 0x08;
pub const INFO_TYPE_ECU_NAME: u8 = 0x0A;
pub const INFO_TYPE_IN_USE_PERFORMANCE_COMPRESSION: u8 = 0x0B;
pub const INFO_TYPE_ECU_SERIAL: u8 = 0x0D;

/// The counters of InfoType 0x08, in the order the standard lists them, for
/// spark-ignition engines.
pub const IN_USE_PERFORMANCE_SPARK: &[&str] = &[
    "OBD monitoring conditions encountered",
    "ignition cycles",
    "catalyst monitor completions, bank 1",
    "catalyst monitor conditions, bank 1",
    "catalyst monitor completions, bank 2",
    "catalyst monitor conditions, bank 2",
    "O2 sensor monitor completions, bank 1",
    "O2 sensor monitor conditions, bank 1",
    "O2 sensor monitor completions, bank 2",
    "O2 sensor monitor conditions, bank 2",
    "EGR monitor completions",
    "EGR monitor conditions",
    "secondary air monitor completions",
    "secondary air monitor conditions",
    "evaporative system monitor completions",
    "evaporative system monitor conditions",
    "secondary O2 sensor monitor completions, bank 1",
    "secondary O2 sensor monitor conditions, bank 1",
    "secondary O2 sensor monitor completions, bank 2",
    "secondary O2 sensor monitor conditions, bank 2",
];

/// The counters of InfoType 0x0B, for compression-ignition engines.
pub const IN_USE_PERFORMANCE_COMPRESSION: &[&str] = &[
    "OBD monitoring conditions encountered",
    "ignition cycles",
    "NMHC catalyst monitor completions",
    "NMHC catalyst monitor conditions",
    "NOx catalyst monitor completions",
    "NOx catalyst monitor conditions",
    "NOx adsorber monitor completions",
    "NOx adsorber monitor conditions",
    "PM filter monitor completions",
    "PM filter monitor conditions",
    "exhaust gas sensor monitor completions",
    "exhaust gas sensor monitor conditions",
    "EGR and VVT monitor completions",
    "EGR and VVT monitor conditions",
    "boost pressure monitor completions",
    "boost pressure monitor conditions",
    "fuel system monitor completions",
    "fuel system monitor conditions",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VehicleInformation {
    /// Which InfoTypes the module answers, from a support bitmap.
    Supported(Vec<u8>),
    Vin(String),
    CalibrationIds(Vec<String>),
    /// Calibration verification numbers, four bytes each, as hexadecimal.
    CalibrationVerificationNumbers(Vec<String>),
    /// In-use performance tracking: a name and a count per counter.
    InUsePerformance(Vec<(String, u16)>),
    EcuName(String),
    EcuSerialNumber(String),
}

pub fn decode_vehicle_information(
    info_type: u8,
    body: &[u8],
) -> Result<VehicleInformation, J1979Error> {
    let Some((&answered, rest)) = body.split_first() else {
        return Err(J1979Error::TruncatedResponse);
    };
    if answered != info_type {
        return Err(J1979Error::UnexpectedInfoType {
            expected: info_type,
            actual: answered,
        });
    }
    if is_support_item(info_type) {
        return Ok(VehicleInformation::Supported(decode_support_bitmap(
            info_type, rest,
        )));
    }
    let Some((&count, data)) = rest.split_first() else {
        return Err(J1979Error::TruncatedResponse);
    };
    let count = usize::from(count);
    let fixed = |width: usize| fixed_items(data, count, width, info_type, body.len());
    match info_type {
        INFO_TYPE_VIN => {
            // One item of seventeen characters; some modules pad the front
            // of the frame with NUL, which is not part of the number.
            let item = fixed(17)?[0];
            Ok(VehicleInformation::Vin(ascii(item)))
        }
        INFO_TYPE_CALIBRATION_ID => {
            let mut payload = vec![0x49, 0x04];
            payload.extend_from_slice(rest);
            decode_calibration_identification(J1979Request::calibration_identification(), &payload)
                .map(|result| VehicleInformation::CalibrationIds(result.calibration_ids))
        }
        INFO_TYPE_CVN => Ok(VehicleInformation::CalibrationVerificationNumbers(
            fixed(4)?
                .into_iter()
                .map(|cvn| cvn.iter().map(|byte| format!("{byte:02X}")).collect())
                .collect(),
        )),
        INFO_TYPE_IN_USE_PERFORMANCE_SPARK | INFO_TYPE_IN_USE_PERFORMANCE_COMPRESSION => {
            let names = if info_type == INFO_TYPE_IN_USE_PERFORMANCE_SPARK {
                IN_USE_PERFORMANCE_SPARK
            } else {
                IN_USE_PERFORMANCE_COMPRESSION
            };
            let counters = fixed(2)?
                .into_iter()
                .enumerate()
                .map(|(index, pair)| {
                    let name = names
                        .get(index)
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| format!("counter {}", index + 1));
                    (name, u16::from_be_bytes([pair[0], pair[1]]))
                })
                .collect();
            Ok(VehicleInformation::InUsePerformance(counters))
        }
        INFO_TYPE_ECU_NAME => {
            // Twenty bytes: a short name, a NUL, then the long one.
            let item = fixed(20)?[0];
            Ok(VehicleInformation::EcuName(ascii(item)))
        }
        INFO_TYPE_ECU_SERIAL => {
            // The standard fixes the count, not the width; the bytes are
            // shared out evenly and read as text.
            let width = data.len() / count.max(1);
            let item = fixed(width)?[0];
            Ok(VehicleInformation::EcuSerialNumber(ascii(item)))
        }
        other => Err(J1979Error::UnsupportedInfoType(other)),
    }
}

/// `count` items of `width` bytes each, or the refusal that names the
/// InfoType and the length that did not fit.
fn fixed_items(
    data: &[u8],
    count: usize,
    width: usize,
    info_type: u8,
    actual: usize,
) -> Result<Vec<&[u8]>, J1979Error> {
    if count == 0 || width == 0 || data.len() < count * width {
        return Err(J1979Error::InvalidInformationLength { info_type, actual });
    }
    Ok(data[..count * width].chunks_exact(width).collect())
}

/// Printable ASCII with NUL turned into a space and the ends trimmed; what
/// is not printable is shown as its byte, never dropped in silence.
fn ascii(bytes: &[u8]) -> String {
    let text: String = bytes
        .iter()
        .map(|&byte| match byte {
            0 => ' ',
            0x20..=0x7E => byte as char,
            _ => '\u{FFFD}',
        })
        .collect();
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vin_is_one_item_of_seventeen() {
        let mut body = vec![0x02, 0x01];
        body.extend_from_slice(b"SAJWA0HP1AMR12345");
        assert_eq!(
            decode_vehicle_information(0x02, &body).unwrap(),
            VehicleInformation::Vin("SAJWA0HP1AMR12345".into())
        );
        assert!(matches!(
            decode_vehicle_information(0x02, &[0x02, 0x01, b'S']),
            Err(J1979Error::InvalidInformationLength {
                info_type: 0x02,
                ..
            })
        ));
    }

    #[test]
    fn support_cvn_and_ecu_name_decode() {
        assert_eq!(
            decode_vehicle_information(0x00, &[0x00, 0x55, 0x40, 0x00, 0x00]).unwrap(),
            VehicleInformation::Supported(vec![0x02, 0x04, 0x06, 0x08, 0x0A])
        );
        assert_eq!(
            decode_vehicle_information(0x06, &[0x06, 0x02, 0x17, 0x91, 0xBC, 0x82, 0, 0, 0, 1])
                .unwrap(),
            VehicleInformation::CalibrationVerificationNumbers(vec![
                "1791BC82".into(),
                "00000001".into()
            ])
        );
        let mut body = vec![0x0A, 0x01];
        body.extend_from_slice(b"ECM\0-EngineControl\0\0");
        assert_eq!(
            decode_vehicle_information(0x0A, &body).unwrap(),
            VehicleInformation::EcuName("ECM -EngineControl".into())
        );
    }

    #[test]
    fn in_use_performance_names_its_counters() {
        let body = [0x08, 0x03, 0x00, 0x2A, 0x00, 0x30, 0x00, 0x05];
        assert_eq!(
            decode_vehicle_information(0x08, &body).unwrap(),
            VehicleInformation::InUsePerformance(vec![
                ("OBD monitoring conditions encountered".into(), 42),
                ("ignition cycles".into(), 48),
                ("catalyst monitor completions, bank 1".into(), 5),
            ])
        );
    }

    #[test]
    fn the_wrong_info_type_is_refused() {
        assert_eq!(
            decode_vehicle_information(0x02, &[0x04, 0x01]),
            Err(J1979Error::UnexpectedInfoType {
                expected: 0x02,
                actual: 0x04
            })
        );
        assert!(!is_read_only_info_type(0x01));
        assert!(is_read_only_info_type(0x20));
    }
}
