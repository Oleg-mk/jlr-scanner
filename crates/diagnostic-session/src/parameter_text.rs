//! Our own wording for the parameter names SDD's catalogue uses, in the
//! interface's languages.
//!
//! Every number this product reads arrives with a name, and the name is
//! SDD's, in English: `Total distance`, `Short term fuel trim sensor 2 bank
//! 1`. The catalogue holds 15,281 parameter definitions under 4,977 distinct
//! names, and until now all three interfaces showed them raw.
//!
//! SDD publishes its catalogue in twelve languages, none of them Ukrainian,
//! and `ADR-0025` decided not to use its Russian either: the owner's own
//! translation covers both languages in one voice, keeps one rendering per
//! term where SDD keeps two, and is shorter, which matters for a name that
//! shares its table row with a value. The table beside this file holds it.
//!
//! It is a dictionary and not knowledge. It claims nothing about any car —
//! only how an English phrase reads — so it carries no evidence, no
//! applicability and no validation state, and it never enters the knowledge
//! store. The English stays the identity: the library keeps it, every report
//! keeps it whatever the interface language, and a name with no row here is
//! shown in English rather than guessed at.

use crate::dtc_text::{RUSSIAN, UKRAINIAN};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../data/parameter_names.tsv");

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
            let (Some(name), Some(ukrainian), Some(russian)) =
                (columns.next(), columns.next(), columns.next())
            else {
                continue;
            };
            if name.is_empty() || ukrainian.is_empty() || russian.is_empty() {
                continue;
            }
            map.insert(name, (ukrainian, russian));
        }
        map
    })
}

/// Our wording for one parameter name, by language code, or an empty map when
/// the table does not carry it. The caller shows the English in that case.
pub fn parameter_texts(name: &str) -> BTreeMap<String, String> {
    let mut texts = BTreeMap::new();
    if let Some((ukrainian, russian)) = table().get(name) {
        texts.insert(UKRAINIAN.to_string(), (*ukrainian).to_string());
        texts.insert(RUSSIAN.to_string(), (*russian).to_string());
    }
    texts
}

/// The whole table in one language, for an interface that looks names up as
/// it renders them. An unknown language code yields nothing rather than a
/// guess.
pub fn parameter_names(language: &str) -> BTreeMap<String, String> {
    let pick: fn(&(&'static str, &'static str)) -> &'static str = match language {
        UKRAINIAN => |pair| pair.0,
        RUSSIAN => |pair| pair.1,
        _ => return BTreeMap::new(),
    };
    table()
        .iter()
        .map(|(name, pair)| ((*name).to_string(), pick(pair).to_string()))
        .collect()
}

/// How many names the table covers, for the library snapshot and for tests.
pub fn covered_names() -> usize {
    table().len()
}

/// How many of `names` the table does not carry. A parameter renamed by a
/// future SDD release loses its translation silently and falls back to
/// English, which is the intended behaviour; this is how the fall-back is
/// counted rather than merely hoped for (`ADR-0025`).
pub fn untranslated<'a>(names: impl IntoIterator<Item = &'a str>) -> usize {
    let table = table();
    names
        .into_iter()
        .filter(|name| !table.contains_key(name))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_covers_the_catalogue() {
        let table = table();
        assert_eq!(
            table.len(),
            4977,
            "the catalogue's distinct parameter names, all of them"
        );
        // Every row carries both languages; a half-filled row would show a
        // Ukrainian user the Russian, which the decision forbids.
        for (name, (ukrainian, russian)) in table {
            assert!(!ukrainian.trim().is_empty(), "{name} has no Ukrainian");
            assert!(!russian.trim().is_empty(), "{name} has no Russian");
        }
    }

    #[test]
    fn the_english_key_keeps_the_catalogues_own_spacing() {
        // SDD separates a name from its qualifier with two spaces around the
        // hyphen. Reflowing the key would make the whole table unreachable.
        let doubled = table().keys().filter(|name| name.contains("  ")).count();
        assert!(
            doubled > 3000,
            "only {doubled} keys keep the double space; the table has been reflowed"
        );
    }

    #[test]
    fn a_known_name_reads_in_both_languages_and_an_unknown_one_in_neither() {
        let texts = parameter_texts("Total distance");
        assert_eq!(
            texts.get(UKRAINIAN).map(String::as_str),
            Some("Загальний пробіг")
        );
        assert_eq!(texts.get(RUSSIAN).map(String::as_str), Some("Общий пробег"));
        assert!(parameter_texts("No such parameter exists").is_empty());
    }

    #[test]
    fn the_whole_table_comes_in_one_language_and_an_unknown_one_is_empty() {
        let ukrainian = parameter_names(UKRAINIAN);
        let russian = parameter_names(RUSSIAN);
        assert_eq!(ukrainian.len(), covered_names());
        assert_eq!(russian.len(), covered_names());
        assert_ne!(
            ukrainian.get("Total distance"),
            russian.get("Total distance"),
            "the two languages are not copies of one another"
        );
        assert!(parameter_names("eng").is_empty());
        assert!(parameter_names("deu").is_empty());
    }

    #[test]
    fn untranslated_counts_what_the_table_misses() {
        assert_eq!(untranslated(["Total distance", "Absolute load value"]), 0);
        assert_eq!(untranslated(["Total distance", "Invented parameter"]), 1);
    }
}
