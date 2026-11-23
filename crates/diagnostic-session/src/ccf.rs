//! The car configuration file: its layout, and how a block decodes
//! (ADR-0028).
//!
//! SDD describes the configuration byte by byte — a parameter is a span of
//! bytes and bits with a mask and a type — and the ingest records that
//! description as it stands. This module turns the recorded text back into a
//! layout and reads a value out of a block with it: an option's text for an
//! enumeration or a boolean, a number for a binary field, a string for
//! ASCII, digits for BCD, bytes for anything else. A value is what the type
//! says and nothing more; no word judges it.

/// One option of an enumeration or a boolean, as SDD lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CcfOption {
    pub value: u64,
    pub name: String,
    /// SDD's sales code for the option, when it carries one.
    pub code: String,
    pub text_en: String,
    pub text_ru: String,
}

/// One parameter of the configuration: where it sits and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CcfParameter {
    pub block: String,
    pub name: String,
    pub start_byte: usize,
    pub stop_byte: usize,
    pub start_bit: u32,
    pub stop_bit: u32,
    pub mask: u64,
    /// `ENUM`, `BOOL`, `BIN`, `ASCII`, `BCD`, `UNDEF`, or whatever SDD wrote.
    pub kind: String,
    /// Whether SDD's editor shows the parameter.
    pub display: bool,
    /// Whether SDD's editor edits it — a fact about SDD, driving nothing here.
    pub edit: bool,
    pub scope: String,
    pub group: String,
    pub group_title_en: String,
    pub group_title_ru: String,
    pub title_en: String,
    pub title_ru: String,
    pub options: Vec<CcfOption>,
}

/// One block a module answers inside one identifier's payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CcfBlockRef {
    pub block: String,
    pub offset: usize,
    pub length: usize,
}

/// What a block holds for one parameter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CcfValue {
    /// The number read over the span, when the type is a number.
    pub raw: Option<u64>,
    /// The option's text, the string, the digits: what a person reads.
    pub text_en: Option<String>,
    pub text_ru: Option<String>,
    pub option_name: Option<String>,
    pub option_code: Option<String>,
    /// The bytes, when that is all the type says.
    pub hex: Option<String>,
    /// What stood in the way of a value, or a fact about it.
    pub note: Option<String>,
}

/// The block reference an identifier's parameter encodes:
/// `ccf=<BLOCK>;offset=<n>;length=<n>`.
pub fn parse_block_encoding(encoding: &str) -> Option<CcfBlockRef> {
    let mut block = None;
    let mut offset = None;
    let mut length = None;
    for part in encoding.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key.trim() {
            "ccf" => block = Some(value.trim().to_string()),
            "offset" => offset = value.trim().parse().ok(),
            "length" => length = value.trim().parse().ok(),
            _ => {}
        }
    }
    Some(CcfBlockRef {
        block: block.filter(|block| !block.is_empty())?,
        offset: offset?,
        length: length?,
    })
}

/// The layout the ingest recorded for one parameter, from its entity id
/// (`CCF-<BLOCK>-<NAME>`) and its encoded claim.
pub fn parse_parameter(entity_id: &str, encoded: &str) -> Option<CcfParameter> {
    let mut fields: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for part in encoded.split(';') {
        if let Some((key, value)) = part.split_once('=') {
            fields.insert(key.trim(), value);
        }
    }
    let block = unescape(fields.get("block")?);
    let name = entity_id
        .strip_prefix("CCF-")
        .and_then(|rest| rest.strip_prefix(block.as_str()))
        .and_then(|rest| rest.strip_prefix('-'))
        .unwrap_or(entity_id)
        .to_string();
    let (start_byte, stop_byte) = range(fields.get("bytes")?)?;
    let (start_bit, stop_bit) = range(fields.get("bits")?)?;
    let mask = parse_hex(fields.get("mask")?)?;
    let options = fields
        .get("options")
        .map(|text| {
            text.split('|')
                .filter(|option| !option.is_empty())
                .filter_map(|option| {
                    let mut parts = option.splitn(5, '=');
                    let value = parse_hex(&unescape(parts.next()?))?;
                    Some(CcfOption {
                        value,
                        name: unescape(parts.next().unwrap_or_default()),
                        code: unescape(parts.next().unwrap_or_default()),
                        text_en: unescape(parts.next().unwrap_or_default()),
                        text_ru: unescape(parts.next().unwrap_or_default()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let text = |key: &str| unescape(fields.get(key).copied().unwrap_or_default());
    Some(CcfParameter {
        block,
        name,
        start_byte: start_byte as usize,
        stop_byte: stop_byte as usize,
        start_bit: start_bit as u32,
        stop_bit: stop_bit as u32,
        mask,
        kind: text("type"),
        display: fields
            .get("display")
            .is_some_and(|flag| flag.trim() == "true"),
        edit: fields.get("edit").is_some_and(|flag| flag.trim() == "true"),
        scope: text("scope"),
        group: text("group"),
        group_title_en: text("group_title"),
        group_title_ru: text("group_title_ru"),
        title_en: text("title"),
        title_ru: text("title_ru"),
        options,
    })
}

/// Read one parameter out of its block.
pub fn decode(parameter: &CcfParameter, block: &[u8]) -> CcfValue {
    let mut value = CcfValue::default();
    if parameter.stop_byte < parameter.start_byte || parameter.stop_byte >= block.len() {
        value.note = Some(format!(
            "the block holds {} byte(s); the parameter sits at bytes {}..{}",
            block.len(),
            parameter.start_byte,
            parameter.stop_byte
        ));
        return value;
    }
    let span = &block[parameter.start_byte..=parameter.stop_byte];
    match parameter.kind.as_str() {
        "ENUM" | "BOOL" => {
            let raw = option_value(span, parameter);
            value.raw = Some(raw);
            match parameter.options.iter().find(|option| option.value == raw) {
                Some(option) => {
                    value.option_name = Some(option.name.clone());
                    value.option_code = (!option.code.is_empty()).then(|| option.code.clone());
                    value.text_en = Some(if option.text_en.is_empty() {
                        option.name.clone()
                    } else {
                        option.text_en.clone()
                    });
                    value.text_ru = Some(if option.text_ru.is_empty() {
                        value.text_en.clone().unwrap_or_default()
                    } else {
                        option.text_ru.clone()
                    });
                }
                None => {
                    value.text_en = Some(raw.to_string());
                    value.text_ru = Some(raw.to_string());
                    value.note = Some("not one of the options SDD lists".into());
                }
            }
        }
        "BIN" => {
            let raw = number(span, parameter.mask);
            value.raw = Some(raw);
            value.text_en = Some(raw.to_string());
            value.text_ru = Some(raw.to_string());
        }
        "ASCII" => {
            let start = span.iter().position(|byte| !is_padding(*byte));
            let end = span.iter().rposition(|byte| !is_padding(*byte));
            let content = match (start, end) {
                (Some(start), Some(end)) if start <= end => &span[start..=end],
                _ => &span[..0],
            };
            if content.is_empty() {
                value.note = Some("padding only".into());
            } else if content.iter().all(|byte| (0x20..=0x7E).contains(byte)) {
                let text = String::from_utf8_lossy(content).into_owned();
                value.text_en = Some(text.clone());
                value.text_ru = Some(text);
            } else {
                value.hex = Some(hex(span));
                value.note = Some("not printable text; shown as bytes".into());
            }
        }
        "BCD" => {
            let mut digits = String::new();
            let mut sound = true;
            for byte in span {
                for nibble in [byte >> 4, byte & 0x0F] {
                    if nibble > 9 {
                        sound = false;
                    }
                    digits.push(char::from(b'0' + nibble.min(9)));
                }
            }
            if sound {
                value.text_en = Some(digits.clone());
                value.text_ru = Some(digits);
            } else {
                value.hex = Some(hex(span));
                value.note = Some("not binary-coded decimal; shown as bytes".into());
            }
        }
        _ => {
            value.hex = Some(hex(span));
        }
    }
    value
}

/// The number a span holds under a mask: one byte masked and shifted down
/// to its first set bit, several bytes read big-endian whole.
/// The value an enumeration's or a boolean's options are matched against.
///
/// SDD writes most options IN PLACE — the field's bits where they sit in the
/// byte: speed control at mask `0xE0` lists `0x20`, `0x40`, `0x60`. A few
/// parameters list them SHIFTED down to bit 0 instead: the coolant warning
/// of the L319 and L320 of MY10 at mask `0xF0` lists `0x01`, `0x02`, …
/// Across SDD's configuration documents 15,240 one-byte fields read in
/// place, 23 shifted, and the rest the same either way. Which form a
/// parameter uses is read from its own options: the form more of its
/// non-zero options fit, and in place when as many fit both. An option
/// neither form can hold — SDD's `0xFF` error sentinel under a nibble mask —
/// simply never matches.
fn option_value(span: &[u8], parameter: &CcfParameter) -> u64 {
    if span.len() != 1 {
        return number(span, parameter.mask);
    }
    let mask = (parameter.mask & 0xFF) as u8;
    if mask == 0 {
        return u64::from(span[0]);
    }
    let in_place = u64::from(span[0] & mask);
    let shift = mask.trailing_zeros();
    if shift == 0 {
        return in_place;
    }
    let mask = u64::from(mask);
    let options = parameter.options.iter().filter(|option| option.value != 0);
    let (fit_in_place, fit_shifted) = options.fold((0usize, 0usize), |(here, down), option| {
        (
            here + usize::from(option.value & !mask == 0),
            down + usize::from(option.value <= mask >> shift),
        )
    });
    if fit_shifted > fit_in_place {
        in_place >> shift
    } else {
        in_place
    }
}

fn number(span: &[u8], mask: u64) -> u64 {
    if span.len() == 1 {
        let mask = (mask & 0xFF) as u8;
        if mask == 0 {
            return u64::from(span[0]);
        }
        return u64::from((span[0] & mask) >> mask.trailing_zeros());
    }
    span.iter()
        .take(8)
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte))
}

fn is_padding(byte: u8) -> bool {
    byte == 0x00 || byte == 0xFF || byte == b' '
}

fn range(text: &str) -> Option<(u64, u64)> {
    let (from, to) = text.split_once("..")?;
    Some((from.trim().parse().ok()?, to.trim().parse().ok()?))
}

fn parse_hex(text: &str) -> Option<u64> {
    let text = text.trim();
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u64::from_str_radix(digits, 16).ok()
}

fn unescape(text: &str) -> String {
    text.replace("%3D", "=")
        .replace("%7C", "|")
        .replace("%3B", ";")
        .replace("%25", "%")
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

    const BRAND: &str = "block=CCF;bytes=0..0;bits=0..7;mask=0xFF;type=ENUM;display=true;edit=false;scope=base;group=GROUP_SYNTH_BRAND;group_title=Brand;group_title_ru=Марка;title=;title_ru=;options=0x00=UNDEF==Undefined=Не определено|0x01=ALPHA=VS_A=Alpha%3B edition%3D1%7Cfirst=Альфа|0x02=BETA===";

    #[test]
    fn a_block_reference_and_a_layout_read_back_from_their_text() {
        let block = parse_block_encoding("ccf=RES;offset=4;length=2").unwrap();
        assert_eq!(
            block,
            CcfBlockRef {
                block: "RES".into(),
                offset: 4,
                length: 2
            }
        );
        assert_eq!(parse_block_encoding("bytes=0..1;scale=0.25"), None);

        let brand = parse_parameter("CCF-CCF-PARAM_SYNTH_BRAND", BRAND).unwrap();
        assert_eq!(brand.name, "PARAM_SYNTH_BRAND");
        assert_eq!(brand.block, "CCF");
        assert_eq!((brand.start_byte, brand.stop_byte), (0, 0));
        assert_eq!(brand.mask, 0xFF);
        assert!(brand.display && !brand.edit);
        assert_eq!(brand.group_title_ru, "Марка");
        assert_eq!(brand.options.len(), 3);
        assert_eq!(brand.options[1].text_en, "Alpha; edition=1|first");
        assert_eq!(brand.options[1].code, "VS_A");
        assert_eq!(brand.options[2].text_en, "");
    }

    #[test]
    fn a_value_is_what_the_type_says_and_nothing_more() {
        let brand = parse_parameter("CCF-CCF-PARAM_SYNTH_BRAND", BRAND).unwrap();
        let alpha = decode(&brand, &[0x01, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(alpha.text_en.as_deref(), Some("Alpha; edition=1|first"));
        assert_eq!(alpha.option_code.as_deref(), Some("VS_A"));
        // A text SDD has none for: the name speaks.
        let beta = decode(&brand, &[0x02, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(beta.text_en.as_deref(), Some("BETA"));
        // A value no option lists is a number and a fact, not a verdict.
        let odd = decode(&brand, &[0x07, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(odd.text_en.as_deref(), Some("7"));
        assert!(odd
            .note
            .as_deref()
            .unwrap()
            .contains("not one of the options"));

        let trim = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_TRIM_LEVEL",
            "block=CCF;bytes=1..1;bits=1..2;mask=0x06;type=ENUM;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=0x00=BASE===|0x01=MID===|0x03=TOP===",
        )
        .unwrap();
        // 0b0000_0110 masked and shifted is 3: TOP, whatever bit 0 holds.
        assert_eq!(
            decode(&trim, &[0, 0b0000_0111]).option_name.as_deref(),
            Some("TOP")
        );
        assert_eq!(
            decode(&trim, &[0, 0b0000_0010]).option_name.as_deref(),
            Some("MID")
        );

        // Options written in place, as SDD writes most of them: the shape of
        // speed control at mask 0xE0. The field's bits, unshifted, pick the
        // option; bits outside the mask do not.
        let speed = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_SPEED",
            "block=CCF;bytes=0..0;bits=5..7;mask=0x0E0;type=ENUM;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=0x00=UNDEF===|0x20=NONE===|0x40=FIXED===|0x60=ADAPTIVE===|0xE0=ERR===",
        )
        .unwrap();
        let fixed = decode(&speed, &[0b0101_1011]);
        assert_eq!(fixed.option_name.as_deref(), Some("FIXED"));
        assert_eq!(fixed.raw, Some(0x40));
        assert_eq!(fixed.note, None);
        assert_eq!(decode(&speed, &[0xFF]).option_name.as_deref(), Some("ERR"));

        // Two fields in one byte, one in place and one shifted, as the coolant
        // warning of an L319 of MY10 sits: the low nibble's options start at
        // 0x01 under mask 0x0F, the high nibble's at 0x01 under mask 0xF0 -
        // only a shift down fits them, so they are read shifted.
        let high = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_HIGH",
            "block=CCF;bytes=0..0;bits=4..7;mask=0xF0;type=ENUM;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=0x00=OFF===|0x01=ONE===|0x02=TWO===",
        )
        .unwrap();
        assert_eq!(decode(&high, &[0x2F]).option_name.as_deref(), Some("TWO"));
        // An error sentinel outside a nibble mask never decides the form,
        // and never matches.
        let display = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_DISPLAY",
            "block=CCF;bytes=0..0;bits=0..3;mask=0x0F;type=ENUM;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=0x00=UNDEF===|0x02=TEXT===|0x03=GRAPHIC===|0xFF=ERR===",
        )
        .unwrap();
        assert_eq!(
            decode(&display, &[0xF3]).option_name.as_deref(),
            Some("GRAPHIC")
        );
        assert_eq!(decode(&display, &[0xFF]).option_name, None);

        let code = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_CODE",
            "block=CCF;bytes=2..4;bits=0..23;mask=0xFF;type=ASCII;display=true;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=",
        )
        .unwrap();
        assert_eq!(
            decode(&code, &[0, 0, b'A', b'B', 0x00]).text_en.as_deref(),
            Some("AB")
        );
        let binary = decode(&code, &[0, 0, 0x01, 0x80, 0xFE]);
        assert_eq!(binary.hex.as_deref(), Some("01 80 FE"));
        assert!(binary.note.as_deref().unwrap().contains("not printable"));

        let radius = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_RADIUS",
            "block=CCF;bytes=5..6;bits=0..15;mask=0xFF;type=BIN;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=",
        )
        .unwrap();
        assert_eq!(decode(&radius, &[0, 0, 0, 0, 0, 0x01, 0x2C]).raw, Some(300));

        let day = parse_parameter(
            "CCF-CCF-PARAM_SYNTH_DAY",
            "block=CCF;bytes=7..7;bits=0..7;mask=0xFF;type=BCD;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=",
        )
        .unwrap();
        assert_eq!(
            decode(&day, &[0, 0, 0, 0, 0, 0, 0, 0x27])
                .text_en
                .as_deref(),
            Some("27")
        );
        assert!(decode(&day, &[0, 0, 0, 0, 0, 0, 0, 0xAB]).note.is_some());

        let reserved = parse_parameter(
            "CCF-VB-SYNTH_RESERVED",
            "block=VB;bytes=0..3;bits=0..31;mask=0xFF;type=UNDEF;display=false;edit=false;scope=base;group=G;group_title=;group_title_ru=;title=;title_ru=;options=",
        )
        .unwrap();
        assert_eq!(
            decode(&reserved, &[1, 2, 3, 4]).hex.as_deref(),
            Some("01 02 03 04")
        );
        // A block shorter than the parameter's span is said, not guessed.
        assert!(decode(&reserved, &[1, 2])
            .note
            .as_deref()
            .unwrap()
            .contains("holds 2 byte"));
    }
}
