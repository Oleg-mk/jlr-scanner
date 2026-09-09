//! Our own wording for the fault codes SAE J2012 defines, in the interface's
//! languages.
//!
//! SDD's text database carries module names and failure-type wording in
//! twelve languages, but a fault code's own description exists in English
//! only, and Ukrainian is not among SDD's languages at all. So the wording a
//! Ukrainian- or Russian-speaking user reads has to be ours.
//!
//! The table beside this file holds it: a standard code, then Ukrainian,
//! then Russian, written from what the standard says the code means. It is
//! not a translation of any manufacturer's phrasing, and it carries none:
//! only codes and our own text. A manufacturer-specific code's wording stays
//! where it belongs, in the issued library.
//!
//! The English wording still comes from the loaded library, and the report
//! keeps it whatever the interface language is: a report is evidence, and
//! evidence does not change language.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The language codes this table uses, SDD's three-letter form so that the
/// interface can pick a description the same way it picks a module name.
pub const UKRAINIAN: &str = "ukr";
pub const RUSSIAN: &str = "rus";

const TABLE: &str = include_str!("../data/dtc_standard_text.tsv");

fn table() -> &'static BTreeMap<&'static str, (&'static str, &'static str)> {
    static PARSED: OnceLock<BTreeMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut map = BTreeMap::new();
        for line in TABLE.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut columns = line.split('\t');
            let (Some(code), Some(ukrainian), Some(russian)) =
                (columns.next(), columns.next(), columns.next())
            else {
                continue;
            };
            if code.is_empty() || ukrainian.is_empty() || russian.is_empty() {
                continue;
            }
            map.insert(code, (ukrainian, russian));
        }
        map
    })
}

/// Our wording for `code`, by language, or nothing when the table does not
/// carry it. The caller decides what to do with an empty map; the interface
/// falls back to the library's English.
pub fn standard_texts(code: &str) -> BTreeMap<String, String> {
    let mut texts = BTreeMap::new();
    if let Some((ukrainian, russian)) = table().get(code) {
        texts.insert(UKRAINIAN.to_string(), (*ukrainian).to_string());
        texts.insert(RUSSIAN.to_string(), (*russian).to_string());
    }
    texts
}

/// How many codes the table covers, for the library snapshot and for tests.
pub fn covered_codes() -> usize {
    table().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only codes the standard defines: the generic ranges. A
    /// manufacturer-specific code's wording is the manufacturer's text and
    /// must never enter this file.
    fn is_standard(code: &str) -> bool {
        let bytes = code.as_bytes();
        if bytes.len() != 5 || !matches!(bytes[0], b'P' | b'B' | b'C' | b'U') {
            return false;
        }
        if !bytes[1..].iter().all(|b| b.is_ascii_hexdigit()) {
            return false;
        }
        match (bytes[0], bytes[1]) {
            (b'P', b'0') | (b'B', b'0') | (b'C', b'0') | (b'U', b'0') => true,
            // P2 is generic throughout; P3 is generic from P34 up.
            (b'P', b'2') => true,
            (b'P', b'3') => matches!(bytes[2], b'4'..=b'9'),
            _ => false,
        }
    }

    #[test]
    fn the_table_parses_and_holds_only_standard_codes() {
        let table = table();
        assert!(table.len() > 200, "table has {} codes", table.len());
        for code in table.keys() {
            assert!(is_standard(code), "{code} is not a standard-range code");
        }
    }

    #[test]
    fn every_row_carries_both_languages_and_no_duplicates() {
        let mut seen = Vec::new();
        for line in TABLE.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let columns: Vec<&str> = line.split('\t').collect();
            assert_eq!(columns.len(), 3, "three columns expected in: {line}");
            assert!(
                columns.iter().all(|column| !column.trim().is_empty()),
                "empty column in: {line}"
            );
            assert!(!seen.contains(&columns[0]), "{} appears twice", columns[0]);
            seen.push(columns[0]);
        }
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        assert_eq!(seen, sorted, "the table is kept sorted by code");
    }

    #[test]
    fn a_known_code_answers_in_both_languages_and_an_unknown_one_answers_with_nothing() {
        let misfire = standard_texts("P0300");
        assert!(misfire[UKRAINIAN].contains("пропуски"));
        assert!(misfire[RUSSIAN].contains("пропуски"));
        assert!(standard_texts("P1234").is_empty(), "not a standard code");
        assert!(standard_texts("").is_empty());
    }
}
