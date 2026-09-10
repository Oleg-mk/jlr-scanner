//! Mode 06: on-board monitoring test results. On CAN each result is nine
//! bytes — monitor, test, scaling id, value, minimum, maximum — and the
//! scaling id names the unit and the scale from the standard's own table.
//! A scaling id this crate does not carry is shown raw and says so; the
//! value is never invented.

use crate::support::{decode_support_bitmap, is_support_item};
use crate::J1979Error;

#[derive(Clone, Debug, PartialEq)]
pub enum MonitorResult {
    /// Which monitors the module reports, from a support bitmap.
    Supported { base: u8, mids: Vec<u8> },
    Result {
        mid: u8,
        monitor: String,
        tid: u8,
        /// The unit-and-scaling id the module named.
        uas: u8,
        unit: &'static str,
        /// Scaled per the standard's table, or `None` with the raw words
        /// below when the id is not in it.
        value: Option<f64>,
        minimum: Option<f64>,
        maximum: Option<f64>,
        raw_value: u16,
        raw_minimum: u16,
        raw_maximum: u16,
        /// Whether the value lies within the limits: the test passed.
        passed: bool,
    },
}

/// Unit and scaling, SAE J1979-DA table: scale, offset, unit, signed. Only
/// the ids whose definition is beyond doubt are carried; the rest decode
/// raw. Adding one means adding it with its source.
pub fn unit_and_scaling(uas: u8) -> Option<(f64, f64, &'static str, bool)> {
    Some(match uas {
        0x01 => (1.0, 0.0, "count", false),
        0x02 => (0.1, 0.0, "count", false),
        0x03 => (0.01, 0.0, "count", false),
        0x04 => (0.001, 0.0, "count", false),
        0x05 => (0.0000305, 0.0, "count", false),
        0x06 => (0.000305, 0.0, "count", false),
        0x07 => (0.25, 0.0, "rpm", false),
        0x08 => (0.01, 0.0, "km/h", false),
        0x09 => (1.0, 0.0, "km/h", false),
        0x0A => (0.122, 0.0, "mV", false),
        0x0B => (0.001, 0.0, "V", false),
        0x0C => (0.01, 0.0, "V", false),
        0x0D => (0.00390625, 0.0, "mA", false),
        0x0E => (0.001, 0.0, "A", false),
        0x0F => (0.01, 0.0, "A", false),
        0x10 => (1.0, 0.0, "ms", false),
        0x11 => (100.0, 0.0, "ms", false),
        0x12 => (1.0, 0.0, "s", false),
        0x13 => (1.0, 0.0, "mΩ", false),
        0x14 => (1.0, 0.0, "Ω", false),
        0x15 => (1.0, 0.0, "kΩ", false),
        0x16 => (0.1, -40.0, "°C", false),
        0x17 => (0.01, 0.0, "kPa", false),
        0x18 => (0.0117, 0.0, "kPa", false),
        0x19 => (0.079, 0.0, "kPa", false),
        0x1A => (1.0, 0.0, "kPa", false),
        0x1B => (10.0, 0.0, "kPa", false),
        0x1C => (0.01, 0.0, "°", false),
        0x1D => (0.5, 0.0, "°", false),
        0x1E => (0.0000305, 0.0, "λ", false),
        0x1F => (0.05, 0.0, "λ", false),
        0x20 => (0.00390625, 0.0, "λ", false),
        0x21 => (1.0, 0.0, "mHz", false),
        0x22 => (1.0, 0.0, "Hz", false),
        0x23 => (1.0, 0.0, "kHz", false),
        0x24 => (1.0, 0.0, "count", false),
        0x25 => (1.0, 0.0, "km", false),
        0x26 => (0.1, 0.0, "mV/ms", false),
        0x27 => (0.01, 0.0, "g/s", false),
        0x28 => (1.0, 0.0, "g/s", false),
        0x29 => (0.25, 0.0, "Pa/s", false),
        0x2A => (0.001, 0.0, "kg/h", false),
        0x2B => (1.0, 0.0, "switches", false),
        0x2C => (0.01, 0.0, "g/cyl", false),
        0x2D => (0.01, 0.0, "mg/stroke", false),
        0x2E => (1.0, 0.0, "true/false", false),
        0x2F => (0.01, 0.0, "%", false),
        0x30 => (0.001526, 0.0, "%", false),
        0x31 => (0.001, 0.0, "L", false),
        0x32 => (0.0000305, 0.0, "in", false),
        0x33 => (0.00024414, 0.0, "in", false),
        0x34 => (1.0, 0.0, "min", false),
        0x35 => (10.0, 0.0, "ms", false),
        0x36 => (0.01, 0.0, "g", false),
        0x37 => (0.1, 0.0, "g", false),
        0x38 => (1.0, 0.0, "g", false),
        0x39 => (0.01, -327.68, "%", false),
        0x3A => (0.001, 0.0, "g", false),
        0x3B => (0.0001, 0.0, "g", false),
        0x3C => (0.00001, 0.0, "g", false),
        0x3D => (0.01, 0.0, "Pa", false),
        0x3E => (0.1, 0.0, "Pa", false),
        0x3F => (1.0, 0.0, "Pa", false),
        0x81 => (1.0, 0.0, "count", true),
        0x82 => (0.1, 0.0, "count", true),
        0x83 => (0.01, 0.0, "count", true),
        0x84 => (0.001, 0.0, "count", true),
        0x85 => (0.0000305, 0.0, "count", true),
        0x86 => (0.000305, 0.0, "count", true),
        0x8A => (0.122, 0.0, "mV", true),
        0x8B => (0.001, 0.0, "V", true),
        0x8C => (0.01, 0.0, "V", true),
        0x8D => (0.00390625, 0.0, "mA", true),
        0x8E => (0.001, 0.0, "A", true),
        0x90 => (1.0, 0.0, "ms", true),
        0x96 => (0.1, 0.0, "°C", true),
        0x99 => (0.1, 0.0, "kPa", true),
        0x9C => (0.01, 0.0, "°", true),
        0x9D => (0.5, 0.0, "°", true),
        0xA8 => (1.0, 0.0, "g/s", true),
        0xA9 => (0.25, 0.0, "Pa/s", true),
        0xAD => (0.01, 0.0, "mg/stroke", true),
        0xAE => (0.01, 0.0, "%", true),
        0xAF => (0.003052, 0.0, "%", true),
        0xB0 => (2.0, 0.0, "mV/s", true),
        0xFC => (0.01, 0.0, "kPa", true),
        0xFD => (0.001, 0.0, "kPa", true),
        0xFE => (0.25, 0.0, "Pa", true),
        _ => return None,
    })
}

/// The monitors the standard names by id; anything else is shown by number.
pub fn monitor_name(mid: u8) -> String {
    let named: Option<String> = match mid {
        0x01..=0x08 => {
            let bank = (mid - 1) / 4 + 1;
            let sensor = (mid - 1) % 4 + 1;
            Some(format!(
                "oxygen sensor monitor, bank {bank} sensor {sensor}"
            ))
        }
        0x21..=0x24 => Some(format!("catalyst monitor, bank {}", mid - 0x20)),
        0x31..=0x34 => Some(format!("EGR monitor, bank {}", mid - 0x30)),
        0x35 | 0x36 => Some(format!("VVT monitor, bank {}", mid - 0x34)),
        0x39 => Some("evaporative system monitor, cap off / 0.150\"".into()),
        0x3A => Some("evaporative system monitor, 0.090\"".into()),
        0x3B => Some("evaporative system monitor, 0.040\"".into()),
        0x3C => Some("evaporative system monitor, 0.020\"".into()),
        0x3D => Some("purge flow monitor".into()),
        0x41..=0x48 => {
            let bank = (mid - 0x41) / 4 + 1;
            let sensor = (mid - 0x41) % 4 + 1;
            Some(format!(
                "oxygen sensor heater monitor, bank {bank} sensor {sensor}"
            ))
        }
        0x61..=0x64 => Some(format!("heated catalyst monitor, bank {}", mid - 0x60)),
        0x71..=0x74 => Some(format!("secondary air monitor {}", mid - 0x70)),
        0x81..=0x84 => Some(format!("fuel system monitor, bank {}", mid - 0x80)),
        0x85 => Some("boost pressure control monitor".into()),
        0x90 => Some("NOx adsorber monitor, bank 1".into()),
        0x91 => Some("NOx adsorber monitor, bank 2".into()),
        0x98 => Some("NOx catalyst monitor, bank 1".into()),
        0x99 => Some("NOx catalyst monitor, bank 2".into()),
        0xA1 => Some("misfire monitor, general".into()),
        0xA2..=0xAD => Some(format!("misfire monitor, cylinder {}", mid - 0xA1)),
        0xB0 => Some("PM filter monitor, bank 1".into()),
        0xB1 => Some("PM filter monitor, bank 2".into()),
        _ => None,
    };
    named.unwrap_or_else(|| format!("monitor {mid:#04X}"))
}

pub fn decode_monitor_results(body: &[u8]) -> Result<Vec<MonitorResult>, J1979Error> {
    let mut results = Vec::new();
    let mut rest = body;
    while let Some((&mid, after)) = rest.split_first() {
        if is_support_item(mid) {
            if after.len() < 4 {
                return Err(J1979Error::TruncatedMonitorResult);
            }
            results.push(MonitorResult::Supported {
                base: mid,
                mids: decode_support_bitmap(mid, &after[..4]),
            });
            rest = &after[4..];
            continue;
        }
        if after.len() < 8 {
            return Err(J1979Error::TruncatedMonitorResult);
        }
        let tid = after[0];
        let uas = after[1];
        let raw_value = u16::from_be_bytes([after[2], after[3]]);
        let raw_minimum = u16::from_be_bytes([after[4], after[5]]);
        let raw_maximum = u16::from_be_bytes([after[6], after[7]]);
        let scaled = unit_and_scaling(uas);
        let scale = |raw: u16| {
            scaled.map(|(scale, offset, _, signed)| {
                let base = if signed {
                    f64::from(raw as i16)
                } else {
                    f64::from(raw)
                };
                base * scale + offset
            })
        };
        results.push(MonitorResult::Result {
            mid,
            monitor: monitor_name(mid),
            tid,
            uas,
            unit: scaled.map(|(_, _, unit, _)| unit).unwrap_or("raw"),
            value: scale(raw_value),
            minimum: scale(raw_minimum),
            maximum: scale(raw_maximum),
            raw_value,
            raw_minimum,
            raw_maximum,
            passed: match scaled {
                Some((_, _, _, true)) => {
                    let (v, lo, hi) = (raw_value as i16, raw_minimum as i16, raw_maximum as i16);
                    v >= lo && v <= hi
                }
                _ => raw_value >= raw_minimum && raw_value <= raw_maximum,
            },
        });
        rest = &after[8..];
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_is_scaled_by_its_id_and_judged_against_its_limits() {
        // Catalyst bank 1, TID 0x80, UAS 0x0B (0.001 V): value 0.5 V within 0.0..1.0.
        let body = [0x21, 0x80, 0x0B, 0x01, 0xF4, 0x00, 0x00, 0x03, 0xE8];
        let results = decode_monitor_results(&body).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            MonitorResult::Result {
                mid,
                monitor,
                unit,
                value,
                minimum,
                maximum,
                passed,
                ..
            } => {
                assert_eq!(*mid, 0x21);
                assert_eq!(monitor, "catalyst monitor, bank 1");
                assert_eq!(*unit, "V");
                assert_eq!(*value, Some(0.5));
                assert_eq!(*minimum, Some(0.0));
                assert_eq!(*maximum, Some(1.0));
                assert!(passed);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_unknown_scaling_stays_raw_and_a_signed_one_is_signed() {
        let body = [
            0xA2, 0x0B, 0x77, 0x00, 0x05, 0x00, 0x00, 0x00, 0x10, // unknown UAS 0x77
            0x01, 0x80, 0x96, 0xFF, 0x9C, 0xFF, 0x38, 0x00, 0x64, // -10.0 °C in -20.0..10.0
        ];
        let results = decode_monitor_results(&body).unwrap();
        match &results[0] {
            MonitorResult::Result {
                unit,
                value,
                passed,
                ..
            } => {
                assert_eq!(*unit, "raw");
                assert_eq!(*value, None);
                assert!(passed);
            }
            other => panic!("{other:?}"),
        }
        match &results[1] {
            MonitorResult::Result {
                monitor,
                value,
                minimum,
                maximum,
                passed,
                ..
            } => {
                assert_eq!(monitor, "oxygen sensor monitor, bank 1 sensor 1");
                assert_eq!(*value, Some(-10.0));
                assert_eq!(*minimum, Some(-20.0));
                assert_eq!(*maximum, Some(10.0));
                assert!(passed);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn support_bitmaps_sit_among_results_and_truncation_is_refused() {
        let body = [
            0x00, 0x80, 0x00, 0x00, 0x01, 0x21, 0x80, 0x0B, 0, 1, 0, 0, 0, 2,
        ];
        let results = decode_monitor_results(&body).unwrap();
        assert_eq!(
            results[0],
            MonitorResult::Supported {
                base: 0x00,
                mids: vec![0x01, 0x20]
            }
        );
        assert!(matches!(
            results[1],
            MonitorResult::Result { mid: 0x21, .. }
        ));
        assert_eq!(
            decode_monitor_results(&[0x21, 0x80, 0x0B, 0, 1]),
            Err(J1979Error::TruncatedMonitorResult)
        );
    }
}
