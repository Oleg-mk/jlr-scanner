//! Modes 03, 07 and 0A: lists of fault codes, two bytes each, in the ISO
//! 15031-6 form — the letter in the top two bits, then the digits.

/// Which list a mode returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DtcKind {
    /// Mode 03: confirmed, MIL-relevant.
    Stored,
    /// Mode 07: seen in this or the last driving cycle, not yet confirmed.
    Pending,
    /// Mode 0A: cleared only by the vehicle itself, never by a tool.
    Permanent,
}

impl DtcKind {
    pub const fn mode(self) -> u8 {
        match self {
            Self::Stored => 0x03,
            Self::Pending => 0x07,
            Self::Permanent => 0x0A,
        }
    }
}

/// The text of a two-byte code: `P0300`, `C1234`, `B0001`, `U0100`.
pub fn dtc_text(high: u8, low: u8) -> String {
    let letter = match high >> 6 {
        0 => 'P',
        1 => 'C',
        2 => 'B',
        _ => 'U',
    };
    format!("{letter}{}{:X}{:02X}", (high >> 4) & 0x3, high & 0x0F, low)
}

/// The two bytes of a code's text, when it is one: the inverse of
/// [`dtc_text`], for the bench.
pub fn dtc_bytes(code: &str) -> Option<[u8; 2]> {
    let mut chars = code.chars();
    let letter = match chars.next()? {
        'P' => 0u8,
        'C' => 1,
        'B' => 2,
        'U' => 3,
        _ => return None,
    };
    let digits = chars.as_str();
    if digits.len() != 4 {
        return None;
    }
    let value = u16::from_str_radix(digits, 16).ok()?;
    if value > 0x3FFF {
        return None;
    }
    let high = (letter << 6) | ((value >> 8) as u8);
    Some([high, (value & 0xFF) as u8])
}

/// The codes after the positive SID: a count byte, then pairs. A pair of
/// zeros is padding, not a code; a count that does not match the bytes is
/// read as far as the bytes go, because modules disagree on it and the pairs
/// themselves are unambiguous.
pub fn decode_dtc_list(body: &[u8]) -> Result<Vec<String>, crate::J1979Error> {
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let count = usize::from(body[0]);
    let pairs = &body[1..];
    let mut codes = Vec::new();
    for pair in pairs.chunks_exact(2).take(count.max(pairs.len() / 2)) {
        if pair == [0, 0] {
            continue;
        }
        codes.push(dtc_text(pair[0], pair[1]));
    }
    Ok(codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_reads_and_writes_the_same() {
        assert_eq!(dtc_text(0x03, 0x00), "P0300");
        assert_eq!(dtc_text(0x41, 0x21), "C0121");
        assert_eq!(dtc_text(0x9A, 0x08), "B1A08");
        assert_eq!(dtc_text(0xC1, 0x00), "U0100");
        for code in ["P0300", "C0121", "B1A08", "U0100", "P3FFF"] {
            let [high, low] = dtc_bytes(code).unwrap();
            assert_eq!(dtc_text(high, low), code);
        }
        assert_eq!(dtc_bytes("X0000"), None);
        assert_eq!(dtc_bytes("P4000"), None);
    }

    #[test]
    fn a_list_skips_the_zero_padding_and_survives_a_wrong_count() {
        assert_eq!(
            decode_dtc_list(&[0x02, 0x03, 0x00, 0x01, 0x71]).unwrap(),
            ["P0300", "P0171"]
        );
        assert_eq!(
            decode_dtc_list(&[0x01, 0x03, 0x00, 0x00, 0x00]).unwrap(),
            ["P0300"]
        );
        assert!(decode_dtc_list(&[0x00]).unwrap().is_empty());
        assert!(decode_dtc_list(&[]).unwrap().is_empty());
        // Three pairs but a count of one: the pairs are what the module sent.
        assert_eq!(
            decode_dtc_list(&[0x01, 0x03, 0x00, 0x01, 0x71, 0xC1, 0x00]).unwrap(),
            ["P0300", "P0171", "U0100"]
        );
    }
}
