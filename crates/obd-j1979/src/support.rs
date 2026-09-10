//! The support bitmaps every mode shares: item 0x00 says which of 0x01–0x20
//! the module answers, 0x20 which of 0x21–0x40, and so on. Four bytes, most
//! significant bit first; the last bit of each map says whether the next map
//! exists.

/// The items whose answer is a support bitmap rather than data.
pub const fn is_support_item(item: u8) -> bool {
    item % 0x20 == 0
}

/// The items a four-byte bitmap declares supported, from `base + 1` to
/// `base + 32`, in ascending order.
pub fn decode_support_bitmap(base: u8, bytes: &[u8]) -> Vec<u8> {
    let mut supported = Vec::new();
    for (index, byte) in bytes.iter().take(4).enumerate() {
        for bit in 0..8u8 {
            if byte & (0x80 >> bit) != 0 {
                let offset = (index as u8) * 8 + bit + 1;
                if let Some(item) = base.checked_add(offset) {
                    supported.push(item);
                }
            }
        }
    }
    supported
}

/// The four bytes that declare `supported` (each within `base + 1 ..= base +
/// 32`; others are ignored). What a module puts on the wire, for the bench.
pub fn encode_support_bitmap(base: u8, supported: &[u8]) -> [u8; 4] {
    let mut bytes = [0u8; 4];
    for &item in supported {
        if item > base && item <= base.saturating_add(32) {
            let offset = item - base - 1;
            bytes[usize::from(offset / 8)] |= 0x80 >> (offset % 8);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bitmap_round_trips_and_keeps_its_order() {
        // The example the standard's readers know: BE 1F A8 13 from a PID 00.
        let supported = decode_support_bitmap(0x00, &[0xBE, 0x1F, 0xA8, 0x13]);
        assert_eq!(
            supported,
            [
                0x01, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x13, 0x15,
                0x1C, 0x1F, 0x20
            ]
        );
        assert_eq!(
            encode_support_bitmap(0x00, &supported),
            [0xBE, 0x1F, 0xA8, 0x13]
        );
        // A higher map counts from its own base.
        assert_eq!(decode_support_bitmap(0x20, &[0x80, 0, 0, 1]), [0x21, 0x40]);
        assert_eq!(
            encode_support_bitmap(0x20, &[0x21, 0x40, 0x05]),
            [0x80, 0, 0, 1]
        );
        assert!(is_support_item(0x00) && is_support_item(0xA0) && !is_support_item(0x0C));
    }
}
