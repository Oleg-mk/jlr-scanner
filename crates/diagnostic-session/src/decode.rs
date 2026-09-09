//! Decoding identifier payloads with the catalogue's own formatting.
//!
//! The DID catalogue records, per parameter, which bytes of the response it
//! occupies, a mask, and the converter SDD applies — a linear multiplier and
//! offset or a piecewise-linear map — together with the output unit. This
//! module applies exactly that and nothing more: a parameter whose bytes the
//! response does not contain, or whose scaling the catalogue never recorded,
//! is shown with the reason instead of a guessed value.
//!
//! Byte ranges are read as big-endian integers and linear converters are
//! applied as recorded: the engine-speed identifier `0xF40C` (`bytes=0..1`,
//! multiplier 0.25, `rpm`) then agrees with SAE J1979's 0.25 rpm per count.
//! Map converters are recorded verbatim but not applied: their breakpoint
//! values are not consistently in the declared unit (the signed engine-speed
//! map lists y = 8 191 750 for x = 32 767, three orders beyond rpm), and what
//! SDD does with them is not established. Bit-level packing beyond byte
//! ranges is not interpreted.

use diagnostic_environment::ReadableParameter;

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedParameter {
    pub name: String,
    /// The masked integer read from the response, when its bytes were there.
    pub raw: Option<u64>,
    /// The scaled value as text, when the catalogue recorded how to scale it.
    pub value: Option<String>,
    pub unit: Option<String>,
    /// The name SDD shows for the raw counts, when the catalogue names the
    /// range they fall in ("Variant not programmed", "UK", …).
    pub state: Option<String>,
    /// Why there is no value, or a caveat about the one shown.
    pub note: Option<String>,
}

#[derive(Debug, Default)]
struct Encoding {
    bytes: Option<(usize, usize)>,
    mask: Option<u64>,
    scale: Option<f64>,
    offset: Option<f64>,
    offset_first: bool,
    has_map: bool,
    states: Vec<(u64, u64, String)>,
}

fn parse_encoding(text: &str) -> Encoding {
    let mut encoding = Encoding::default();
    for part in text.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key.trim() {
            "bytes" => {
                if let Some((from, to)) = value.split_once("..") {
                    if let (Ok(from), Ok(to)) = (from.trim().parse(), to.trim().parse()) {
                        encoding.bytes = Some((from, to));
                    }
                }
            }
            "mask" => {
                let value = value.trim();
                if value != "all" {
                    encoding.mask = value
                        .strip_prefix("0x")
                        .and_then(|hex| u64::from_str_radix(hex, 16).ok());
                }
            }
            "scale" => encoding.scale = value.trim().parse().ok(),
            "offset" => encoding.offset = value.trim().parse().ok(),
            "offset_first" => encoding.offset_first = value.trim() == "true",
            "map" => encoding.has_map = !value.trim().is_empty(),
            "states" => {
                encoding.states = value
                    .split('|')
                    .filter_map(|state| {
                        let (range, name) = state.split_once('=')?;
                        let (low, high) = range.split_once("..")?;
                        Some((
                            low.trim().parse().ok()?,
                            high.trim().parse().ok()?,
                            unescape_state_name(name.trim()),
                        ))
                    })
                    .collect();
            }
            _ => {}
        }
    }
    encoding
}

/// How many data bytes an identifier's parameters span, by the catalogue's
/// byte ranges; `None` when no parameter records a range. The bench answers
/// an identifier with exactly this many bytes (ADR-0020).
pub fn parameters_span(parameters: &[ReadableParameter]) -> Option<usize> {
    parameters
        .iter()
        .filter_map(|parameter| parameter.encoding.as_deref())
        .filter_map(|text| parse_encoding(text).bytes)
        .map(|(_, to)| to + 1)
        .max()
}

/// Decode every parameter of an identifier from the data bytes a module
/// returned for it.
pub fn decode_parameters(parameters: &[ReadableParameter], data: &[u8]) -> Vec<DecodedParameter> {
    parameters
        .iter()
        .map(|parameter| decode_one(parameter, data))
        .collect()
}

fn decode_one(parameter: &ReadableParameter, data: &[u8]) -> DecodedParameter {
    let mut decoded = DecodedParameter {
        name: parameter.name.clone(),
        raw: None,
        value: None,
        unit: parameter.unit.clone(),
        state: None,
        note: None,
    };
    let Some(text) = parameter.encoding.as_deref() else {
        decoded.note = Some("the catalogue records no byte layout for this parameter".into());
        return decoded;
    };
    let encoding = parse_encoding(text);
    let Some((from, to)) = encoding.bytes else {
        decoded.note = Some("the catalogue records no byte range for this parameter".into());
        return decoded;
    };
    if from > to || to >= data.len() {
        decoded.note = Some(format!(
            "the response holds {} byte(s); the catalogue places this parameter at bytes {from}..{to}",
            data.len()
        ));
        return decoded;
    }
    let slice = &data[from..=to];
    if slice.len() > 8 {
        decoded.value = Some(hex(slice));
        decoded.note = Some("wider than 64 bits; shown as bytes".into());
        return decoded;
    }
    let mut raw = slice
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
    if let Some(mask) = encoding.mask {
        raw &= mask;
    }
    decoded.raw = Some(raw);
    decoded.state = encoding
        .states
        .iter()
        .find(|(low, high, _)| raw >= *low && raw <= *high)
        .map(|(_, _, name)| name.clone());

    if encoding.has_map {
        decoded.value = Some(raw.to_string());
        decoded.unit = None;
        decoded.note = Some(
            "raw counts; the catalogue's map converter is recorded but its scale is not established"
                .into(),
        );
        return decoded;
    }
    let scaled = if let Some(scale) = encoding.scale {
        let offset = encoding.offset.unwrap_or(0.0);
        Some(if encoding.offset_first {
            (raw as f64 + offset) * scale
        } else {
            raw as f64 * scale + offset
        })
    } else {
        None
    };
    match scaled {
        Some(value) => decoded.value = Some(format_number(value)),
        None => {
            decoded.value = Some(raw.to_string());
            decoded.note = Some("raw counts; the catalogue records no scaling".into());
        }
    }
    decoded
}

/// Reverse the four replacements the catalogue export applies to a state
/// name so it cannot break the descriptor's separators.
fn unescape_state_name(name: &str) -> String {
    name.replace("%3D", "=")
        .replace("%7C", "|")
        .replace("%3B", ";")
        .replace("%25", "%")
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(name: &str, encoding: &str, unit: Option<&str>) -> ReadableParameter {
        ReadableParameter {
            name: name.into(),
            encoding: Some(encoding.into()),
            unit: unit.map(str::to_string),
        }
    }

    #[test]
    fn engine_speed_scales_a_big_endian_pair_by_a_quarter() {
        let decoded = decode_parameters(
            &[parameter(
                "Engine speed",
                "bytes=0..1;size=2;mask=0xffff;converter=CVT_N_RPM_OFF_0_RES_0PT25;scale=0.25;offset=0;offset_first=true",
                Some("rpm"),
            )],
            &[0x1A, 0xF8],
        );
        assert_eq!(decoded[0].raw, Some(0x1AF8));
        assert_eq!(decoded[0].value.as_deref(), Some("1726"));
        assert_eq!(decoded[0].unit.as_deref(), Some("rpm"));
        assert_eq!(decoded[0].note, None);
    }

    #[test]
    fn offset_first_and_fractions_are_applied_as_recorded() {
        let decoded = decode_parameters(
            &[parameter(
                "Coolant",
                "bytes=0..0;size=1;mask=all;converter=X;scale=0.75;offset=-48;offset_first=true",
                Some("degC"),
            )],
            &[0x80],
        );
        // (128 - 48) * 0.75 = 60
        assert_eq!(decoded[0].value.as_deref(), Some("60"));
        let decoded = decode_parameters(
            &[parameter(
                "Voltage",
                "bytes=0..0;mask=all;scale=0.1;offset=0;offset_first=true",
                Some("V"),
            )],
            &[123],
        );
        assert_eq!(decoded[0].value.as_deref(), Some("12.3"));
    }

    #[test]
    fn a_map_converter_is_recorded_but_not_applied() {
        let encoding =
            "bytes=0..1;mask=all;converter=M;map=0:0|32767:8191750|32768:-8192000|65535:-250";
        let decoded = decode_parameters(&[parameter("Slip", encoding, Some("rpm"))], &[0x40, 0x00]);
        assert_eq!(decoded[0].raw, Some(0x4000));
        assert_eq!(decoded[0].value.as_deref(), Some("16384"));
        // The unit is withheld: raw counts are not rpm.
        assert_eq!(decoded[0].unit, None);
        assert!(decoded[0]
            .note
            .as_deref()
            .unwrap()
            .contains("map converter"));
    }

    #[test]
    fn a_mask_selects_the_packed_field_and_missing_scaling_is_named() {
        let decoded = decode_parameters(
            &[parameter("Flag", "bytes=0..0;size=1;mask=0x0f", None)],
            &[0xA5],
        );
        assert_eq!(decoded[0].raw, Some(0x05));
        assert_eq!(decoded[0].value.as_deref(), Some("5"));
        assert!(decoded[0].note.as_deref().unwrap().contains("no scaling"));
    }

    #[test]
    fn a_short_response_is_named_not_padded() {
        let decoded = decode_parameters(
            &[parameter(
                "Wide",
                "bytes=0..3;size=4;mask=all;scale=1;offset=0;offset_first=true",
                None,
            )],
            &[0x01, 0x02],
        );
        assert_eq!(decoded[0].raw, None);
        assert!(decoded[0]
            .note
            .as_deref()
            .unwrap()
            .contains("holds 2 byte(s)"));
    }

    #[test]
    fn a_named_range_is_shown_beside_the_value() {
        let encoding = "bytes=0..0;size=1;mask=all;converter=C;scale=1;offset=0;offset_first=true;states=0..0=Variant not programmed.|1..1=UK|2..2=Ireland%3B Eire|3..255=Other%3Dunknown";
        let decoded = decode_parameters(&[parameter("Market", encoding, Some("int"))], &[2]);
        assert_eq!(decoded[0].value.as_deref(), Some("2"));
        assert_eq!(decoded[0].state.as_deref(), Some("Ireland; Eire"));
        let decoded = decode_parameters(&[parameter("Market", encoding, Some("int"))], &[7]);
        assert_eq!(decoded[0].state.as_deref(), Some("Other=unknown"));
        // Outside every named range there is no state, and no guess.
        let decoded = decode_parameters(
            &[parameter(
                "Market",
                "bytes=0..0;mask=all;scale=1;offset=0;offset_first=true;states=0..0=Off",
                Some("int"),
            )],
            &[1],
        );
        assert_eq!(decoded[0].state, None);
    }

    #[test]
    fn a_parameter_without_a_layout_says_so() {
        let decoded = decode_parameters(
            &[ReadableParameter {
                name: "Opaque".into(),
                encoding: None,
                unit: None,
            }],
            &[1, 2, 3],
        );
        assert_eq!(decoded[0].value, None);
        assert!(decoded[0]
            .note
            .as_deref()
            .unwrap()
            .contains("no byte layout"));
    }
}
