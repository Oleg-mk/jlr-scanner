//! Our own wording for the fault-code help, in the interface's languages.
//!
//! The help layer reaches this product as SDD's English, line by line, in the
//! issued library. `ADR-0026` decided that the words the product shows are its
//! own — the same sentence said by this project rather than quoted — in
//! English, Ukrainian and Russian, so that the three interfaces show one text
//! and not three.
//!
//! A row is found by the **fingerprint** of the library's line rather than by
//! the line: FNV-1a over its UTF-8 bytes with whitespace collapsed, sixteen
//! hexadecimal digits. The line itself is nowhere in this repository. The
//! ingest trims a mnemonic's text without flattening it, so the collapse
//! happens on both sides and a line broken across two source rows still finds
//! its row.
//!
//! It is a dictionary and not knowledge, exactly as the parameter names are:
//! no evidence, no applicability, no validation state, and never in the
//! knowledge store. A line with no row is shown as the library has it, in
//! English, one line at a time — never machine-translated, never half
//! substituted, never filled from the other language.

use crate::dtc_text::{RUSSIAN, UKRAINIAN};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../data/help_texts.tsv");

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// English, Ukrainian, Russian, in that order.
type Wording = (&'static str, &'static str, &'static str);

/// The library's line, reduced to what the table was keyed by: runs of
/// whitespace become one space and the ends are trimmed.
fn fingerprint(line: &str) -> String {
    let mut hash = FNV_OFFSET;
    let mut pending_space = false;
    let mut started = false;
    let feed = |byte: u8, hash: &mut u64| {
        *hash = (*hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
    };
    for byte in line.as_bytes() {
        if byte.is_ascii_whitespace() {
            if started {
                pending_space = true;
            }
            continue;
        }
        if pending_space {
            feed(b' ', &mut hash);
            pending_space = false;
        }
        started = true;
        feed(*byte, &mut hash);
    }
    format!("{hash:016x}")
}

fn table() -> &'static BTreeMap<&'static str, Wording> {
    static PARSED: OnceLock<BTreeMap<&'static str, Wording>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut map = BTreeMap::new();
        for line in TABLE.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut columns = line.split('\t');
            let (Some(digest), Some(english), Some(ukrainian), Some(russian)) = (
                columns.next(),
                columns.next(),
                columns.next(),
                columns.next(),
            ) else {
                continue;
            };
            if digest.is_empty() || english.is_empty() || ukrainian.is_empty() {
                continue;
            }
            map.insert(digest, (english, ukrainian, russian));
        }
        map
    })
}

/// Our English for one help line, or nothing when the table has no row.
pub fn english(line: &str) -> Option<&'static str> {
    table().get(fingerprint(line).as_str()).map(|row| row.0)
}

/// A whole screen said in our words: the English we show in place of the
/// library's, and the same lines by language code for the interface to pick
/// from. A line we have no row for keeps the library's English in every
/// language, which is what fail-closed means here.
pub fn screen(lines: &[String]) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    let table = table();
    let mut shown = Vec::with_capacity(lines.len());
    let mut ukrainian = Vec::with_capacity(lines.len());
    let mut russian = Vec::with_capacity(lines.len());
    let mut translated = false;
    for line in lines {
        match table.get(fingerprint(line).as_str()) {
            Some((our_english, uk, ru)) => {
                shown.push((*our_english).to_string());
                ukrainian.push((*uk).to_string());
                russian.push((*ru).to_string());
                translated = true;
            }
            None => {
                shown.push(line.clone());
                ukrainian.push(line.clone());
                russian.push(line.clone());
            }
        }
    }
    let mut texts = BTreeMap::new();
    if translated {
        texts.insert(UKRAINIAN.to_string(), ukrainian);
        texts.insert(RUSSIAN.to_string(), russian);
    }
    (shown, texts)
}

/// How many lines the table covers, for the state document and for tests.
pub fn covered_lines() -> usize {
    table().len()
}

/// How many of `lines` the table does not carry — the fall-back counted
/// rather than hoped for (`ADR-0026`).
pub fn untranslated<'a>(lines: impl IntoIterator<Item = &'a str>) -> usize {
    let table = table();
    lines
        .into_iter()
        .filter(|line| !table.contains_key(fingerprint(line).as_str()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_covers_the_corpus() {
        let table = table();
        assert_eq!(table.len(), 25393, "one row per distinct help line");
        for (digest, (english, ukrainian, russian)) in table {
            assert_eq!(digest.len(), 16, "{digest} is not a fingerprint");
            assert!(!english.trim().is_empty(), "{digest} has no English");
            assert!(!ukrainian.trim().is_empty(), "{digest} has no Ukrainian");
            assert!(!russian.trim().is_empty(), "{digest} has no Russian");
        }
    }

    #[test]
    fn the_fingerprint_ignores_how_the_line_is_broken() {
        // The ingest trims a mnemonic but keeps the newlines inside it, so
        // the same sentence can arrive on one line or on three.
        let one = fingerprint("Possible causes");
        assert_eq!(fingerprint("  Possible   causes  "), one);
        assert_eq!(fingerprint("Possible\n causes"), one);
        assert_ne!(fingerprint("Possible causes:"), one);
    }

    #[test]
    fn a_known_line_is_said_in_three_languages() {
        let (shown, texts) = screen(&["Possible causes".to_string()]);
        assert_eq!(shown, vec!["Possible causes".to_string()]);
        assert_eq!(
            texts.get(UKRAINIAN).map(Vec::as_slice),
            Some(["Можливі причини".to_string()].as_slice())
        );
        assert_eq!(
            texts.get(RUSSIAN).map(Vec::as_slice),
            Some(["Возможные причины".to_string()].as_slice())
        );
    }

    #[test]
    fn an_unknown_line_keeps_its_english_in_every_language() {
        let line = "No such help line was ever written by anyone".to_string();
        let (shown, texts) = screen(std::slice::from_ref(&line));
        assert_eq!(shown, vec![line.clone()]);
        assert!(texts.is_empty(), "nothing to offer, so nothing is offered");
        assert_eq!(untranslated([line.as_str()]), 1);
        assert_eq!(untranslated(["Possible causes"]), 0);
    }

    #[test]
    fn a_screen_of_mixed_lines_keeps_its_order_and_length() {
        let lines = vec![
            "Possible causes".to_string(),
            "An invented line with no row of its own".to_string(),
            "Actions required:".to_string(),
        ];
        let (shown, texts) = screen(&lines);
        assert_eq!(shown.len(), 3);
        let ukrainian = texts.get(UKRAINIAN).expect("two lines are known");
        assert_eq!(ukrainian.len(), 3);
        // The line we do not have stands in English in the middle of the
        // Ukrainian screen rather than being dropped.
        assert_eq!(ukrainian[1], lines[1]);
        assert_ne!(ukrainian[0], lines[0]);
    }
}
