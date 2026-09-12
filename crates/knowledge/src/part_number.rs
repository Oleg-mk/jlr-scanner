//! JLR part numbers, compared the way a person would (ADR-0033).
//!
//! SDD writes a part number as `8X23-18C808-CE`; a module may answer with
//! the hyphens, without them, or with padding around them. Nothing has met a
//! car yet, so the exact shape a module returns is not known and this module
//! holds the one assumption the comparison rests on: two part numbers are the
//! same number when their letters and digits are the same, ignoring case and
//! everything else. It is a rule about JLR's data, which is why it lives
//! beside the battery rule and not in a protocol crate.

/// A part number reduced to its letters and digits, in upper case.
pub fn normalised(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Whether two part numbers are the same number.
///
/// An empty side never matches: a module that answered nothing has not
/// agreed with the catalogue.
pub fn same(left: &str, right: &str) -> bool {
    let left = normalised(left);
    !left.is_empty() && left == normalised(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphens_spacing_and_case_do_not_make_a_different_part() {
        assert!(same("8X23-18C808-CE", "8X2318C808CE"));
        assert!(same(" 8x23-18c808-ce ", "8X23-18C808-CE"));
        assert_eq!(normalised("6H52-14C526-CD"), "6H5214C526CD");
    }

    #[test]
    fn a_different_number_is_different_and_nothing_matches_nothing() {
        assert!(!same("8X23-18C808-CE", "8X23-18C808-CF"));
        assert!(!same("", ""));
        assert!(!same("", "8X2318C808CE"));
        assert!(!same("---", "8X2318C808CE"));
    }
}
