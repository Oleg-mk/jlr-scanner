//! Our own wording for the fault-code help, in the interface's languages.
//!
//! The help layer reaches this product from the issued library. `ADR-0026`
//! decided that the words the product shows are its own — the same sentence
//! said by this project rather than quoted — in English, Ukrainian and
//! Russian, so that the three interfaces show one text and not three.
//!
//! A row is found two ways, in this order:
//!
//! - **by the mnemonic's name**, when the library names a screen's items
//!   (libraries exported since 2026-09-12). The name is an identifier, not
//!   text, and it is the unit the owner's translation is keyed by, so every
//!   line the library shows has a row if the owner wrote one;
//! - **by the fingerprint of the line**, for a library issued before that:
//!   FNV-1a over the line's UTF-8 bytes with whitespace collapsed. A mnemonic
//!   that holds a newline inside it reaches such a library as fragments, and
//!   a fragment has no row — which is why the name came to be recorded.
//!
//! The line itself is nowhere in this repository. It is a dictionary and not
//! knowledge, exactly as the parameter names are: no evidence, no
//! applicability, no validation state, never in the knowledge store. A line
//! with no row is shown as the library has it, in English, one line at a time
//! — never machine-translated, never half substituted, never filled from the
//! other language.

use crate::dtc_text::{RUSSIAN, UKRAINIAN};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../data/help_texts.tsv");

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// English, Ukrainian, Russian, in that order.
type Wording = (&'static str, &'static str, &'static str);

struct Table {
    by_name: BTreeMap<&'static str, Wording>,
    by_fingerprint: BTreeMap<&'static str, Wording>,
}

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

fn table() -> &'static Table {
    static PARSED: OnceLock<Table> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut by_name = BTreeMap::new();
        let mut by_fingerprint = BTreeMap::new();
        for line in TABLE.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut columns = line.split('\t');
            let (Some(name), Some(digest), Some(english), Some(ukrainian), Some(russian)) = (
                columns.next(),
                columns.next(),
                columns.next(),
                columns.next(),
                columns.next(),
            ) else {
                continue;
            };
            if english.is_empty() || ukrainian.is_empty() || russian.is_empty() {
                continue;
            }
            let wording: Wording = (english, ukrainian, russian);
            if !name.is_empty() {
                by_name.insert(name, wording);
            }
            if !digest.is_empty() {
                // Two names may share one text; the first row keeps the
                // fingerprint, and the name reaches each of them regardless.
                by_fingerprint.entry(digest).or_insert(wording);
            }
        }
        Table {
            by_name,
            by_fingerprint,
        }
    })
}

/// Our wording for a mnemonic, by its name.
pub fn by_name(name: &str) -> Option<Wording> {
    table().by_name.get(name).copied()
}

/// Our wording for a line, by its fingerprint.
pub fn by_line(line: &str) -> Option<Wording> {
    table()
        .by_fingerprint
        .get(fingerprint(line).as_str())
        .copied()
}

/// A screen assembled from what was found for each line: our English where
/// there is a row, the library's line where there is none, and the same by
/// language code for the interface to pick from. `texts` is empty when no
/// line had any wording of ours.
fn assemble<'a>(
    found: impl Iterator<Item = (&'a str, Option<Wording>)>,
) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    let mut shown = Vec::new();
    let mut ukrainian = Vec::new();
    let mut russian = Vec::new();
    let mut translated = false;
    for (line, wording) in found {
        match wording {
            Some((our_english, uk, ru)) => {
                shown.push(our_english.to_string());
                ukrainian.push(uk.to_string());
                russian.push(ru.to_string());
                translated = true;
            }
            None => {
                shown.push(line.to_string());
                ukrainian.push(line.to_string());
                russian.push(line.to_string());
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

/// A screen the library names item by item: each looked up by name, and
/// where the name has no row, by the text's fingerprint, and where neither,
/// kept as the library's English.
pub fn screen_by_names(items: &[(String, String)]) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    assemble(
        items
            .iter()
            .map(|(name, text)| (text.as_str(), by_name(name).or_else(|| by_line(text)))),
    )
}

/// A screen the library gives as lines of text: each looked up by its
/// fingerprint. The path a library from before 2026-09-12 takes.
pub fn screen(lines: &[String]) -> (Vec<String>, BTreeMap<String, Vec<String>>) {
    assemble(lines.iter().map(|line| (line.as_str(), by_line(line))))
}

/// How many named texts the table covers, for the state document and tests.
pub fn covered_names() -> usize {
    table().by_name.len()
}

/// How many of `lines` have no row by fingerprint — the fall-back on an
/// older library, counted rather than hoped for (`ADR-0026`).
pub fn untranslated<'a>(lines: impl IntoIterator<Item = &'a str>) -> usize {
    lines
        .into_iter()
        .filter(|line| by_line(line).is_none())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_covers_the_corpus() {
        let table = table();
        assert_eq!(
            table.by_name.len(),
            26_893,
            "one row per named text in SDD's help layer, all of them"
        );
        for (name, (english, ukrainian, russian)) in &table.by_name {
            assert!(!english.trim().is_empty(), "{name} has no English");
            assert!(!ukrainian.trim().is_empty(), "{name} has no Ukrainian");
            assert!(!russian.trim().is_empty(), "{name} has no Russian");
        }
        assert!(
            table.by_fingerprint.len() > 25_000,
            "the fingerprint path still serves an older library"
        );
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
    fn a_name_reaches_its_row_whatever_the_text_says() {
        // A text the corpus never wrote under this name; by text that is a
        // miss, by name it is a hit.
        assert!(by_line("Possible causes (as a fixture words it)").is_none());
        let (shown, texts) = screen_by_names(&[(
            "J_I_POSSIBLE_CAUSES".to_string(),
            "Possible causes (as a fixture words it)".to_string(),
        )]);
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
    fn a_line_with_no_name_still_finds_its_row_by_text() {
        let (shown, texts) = screen(&["Possible causes".to_string()]);
        assert_eq!(shown, vec!["Possible causes".to_string()]);
        assert!(texts.contains_key(UKRAINIAN));
    }

    #[test]
    fn an_unknown_line_keeps_its_english_in_every_language() {
        let line = "No such help line was ever written by anyone".to_string();
        let (shown, texts) = screen(std::slice::from_ref(&line));
        assert_eq!(shown, vec![line.clone()]);
        assert!(texts.is_empty(), "nothing to offer, so nothing is offered");
        assert_eq!(untranslated([line.as_str()]), 1);
        assert_eq!(untranslated(["Possible causes"]), 0);
        let (by_name_shown, by_name_texts) =
            screen_by_names(&[("NO_SUCH_NAME".to_string(), line.clone())]);
        assert_eq!(by_name_shown, vec![line]);
        assert!(by_name_texts.is_empty());
    }

    #[test]
    fn a_screen_of_mixed_lines_keeps_its_order_and_length() {
        let items = vec![
            (
                "J_I_POSSIBLE_CAUSES".to_string(),
                "Possible causes".to_string(),
            ),
            (
                "H_CAUSE_NOBODY_WROTE".to_string(),
                "An invented line with no row of its own".to_string(),
            ),
            (
                "J_I_ACTIONS_REQUIRED".to_string(),
                "Actions required:".to_string(),
            ),
        ];
        let (shown, texts) = screen_by_names(&items);
        assert_eq!(shown.len(), 3);
        let ukrainian = texts.get(UKRAINIAN).expect("two lines are known");
        assert_eq!(ukrainian.len(), 3);
        // The line we do not have stands in English in the middle of the
        // Ukrainian screen rather than being dropped.
        assert_eq!(ukrainian[1], items[1].1);
        assert_ne!(ukrainian[0], items[0].1);
    }
}
