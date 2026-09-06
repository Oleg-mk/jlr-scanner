//! Decoding a VIN with SDD's own tables.
//!
//! The rules and attributes arrive as the text claims `sdd-ingest` recorded
//! verbatim from `VINDecode.xml`: a rule `1..3=SAJ;12!=B` places a VIN in a
//! decode model when every test passes; an attribute is `const=Jaguar` or
//! `chars=6..7;01=X200|02=X200`. Positions are one-based and inclusive, as
//! SDD counts them. Nothing is inferred beyond the tables: a VIN no rule
//! claims is reported as such, and an attribute whose characters the table
//! does not list stays absent.
//!
//! The format check — seventeen characters, digits and capital letters
//! without I, O and Q — is ISO 3779's, not JLR's, and is applied before the
//! tables so a mistyped VIN is named as one rather than "not in the tables".

use app_contracts::{DecodedAttribute, VinDecodeSnapshot};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VinTest {
    pub from: usize,
    pub to: usize,
    pub equal: bool,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VinRule {
    pub model: u32,
    pub tests: Vec<VinTest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VinAttribute {
    Constant(String),
    Lookup {
        from: usize,
        to: usize,
        values: Vec<(String, String)>,
    },
}

/// SDD's VIN tables as the library holds them.
#[derive(Clone, Debug, Default)]
pub struct VinTables {
    /// In document order, which is SDD's matching order.
    pub rules: Vec<VinRule>,
    pub attributes: BTreeMap<u32, BTreeMap<String, VinAttribute>>,
    pub versions: BTreeMap<u32, String>,
}

fn unescape(text: &str) -> String {
    text.replace("%3D", "=")
        .replace("%7C", "|")
        .replace("%3B", ";")
        .replace("%25", "%")
}

fn positions(text: &str) -> Option<(usize, usize)> {
    let (from, to) = text.split_once("..")?;
    let (from, to) = (from.trim().parse().ok()?, to.trim().parse().ok()?);
    (from >= 1 && from <= to && to <= 17).then_some((from, to))
}

/// Parse a rule claim; `None` when the text is not one this decoder reads.
pub fn parse_rule(text: &str) -> Option<Vec<VinTest>> {
    let mut tests = Vec::new();
    for part in text.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (range, equal, value) = if let Some((range, value)) = part.split_once("!=") {
            (range, false, value)
        } else if let Some((range, value)) = part.split_once('=') {
            (range, true, value)
        } else {
            return None;
        };
        let (from, to) = positions(range)?;
        let value = value.trim().to_ascii_uppercase();
        if value.len() != to - from + 1 {
            return None;
        }
        tests.push(VinTest {
            from,
            to,
            equal,
            value,
        });
    }
    (!tests.is_empty()).then_some(tests)
}

/// Parse an attribute claim; `None` when the text is not one this decoder reads.
pub fn parse_attribute(text: &str) -> Option<VinAttribute> {
    if let Some(constant) = text.strip_prefix("const=") {
        return Some(VinAttribute::Constant(unescape(constant.trim())));
    }
    let rest = text.strip_prefix("chars=")?;
    let (range, rows) = rest.split_once(';')?;
    let (from, to) = positions(range)?;
    let values: Vec<(String, String)> = rows
        .split('|')
        .filter_map(|row| {
            let (value, decoded) = row.split_once('=')?;
            Some((value.trim().to_ascii_uppercase(), unescape(decoded.trim())))
        })
        .collect();
    (!values.is_empty()).then_some(VinAttribute::Lookup { from, to, values })
}

/// Upper-case, without spaces or hyphens people type into a VIN.
pub fn normalise_vin(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

/// Why a VIN cannot be a VIN, by ISO 3779's format, or `None`.
pub fn format_issue(vin: &str) -> Option<String> {
    let count = vin.chars().count();
    if count != 17 {
        return Some(format!("a VIN has 17 characters; this has {count}"));
    }
    if let Some(bad) = vin.chars().find(|c| {
        !(c.is_ascii_digit() || (c.is_ascii_uppercase() && !matches!(c, 'I' | 'O' | 'Q')))
    }) {
        return Some(format!(
            "'{bad}' cannot occur in a VIN: only digits and capital letters other than I, O and Q"
        ));
    }
    None
}

fn slice(vin: &str, from: usize, to: usize) -> &str {
    &vin[from - 1..to]
}

impl VinTables {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Decode a VIN with the tables; the snapshot says what was and was not
    /// established.
    pub fn decode(&self, input: &str) -> VinDecodeSnapshot {
        let vin = normalise_vin(input);
        let mut snapshot = VinDecodeSnapshot {
            vin: vin.clone(),
            valid: false,
            message: String::new(),
            decode_model: None,
            candidates: Vec::new(),
            program: None,
            model_year: None,
            attributes: Vec::new(),
            table_version: None,
        };
        if let Some(issue) = format_issue(&vin) {
            snapshot.message = format!("Not a VIN: {issue}.");
            return snapshot;
        }
        snapshot.valid = true;
        if self.is_empty() {
            snapshot.message =
                "The loaded data holds no VIN tables; choose the vehicle by hand.".into();
            return snapshot;
        }

        let mut candidates: Vec<u32> = Vec::new();
        for rule in &self.rules {
            let matches = rule
                .tests
                .iter()
                .all(|test| (slice(&vin, test.from, test.to) == test.value) == test.equal);
            if matches && !candidates.contains(&rule.model) {
                candidates.push(rule.model);
            }
        }
        snapshot.candidates = candidates.clone();
        let Some(model) = candidates.first().copied() else {
            snapshot.message =
                "SDD's VIN tables do not describe this VIN; choose the vehicle by hand.".into();
            return snapshot;
        };
        snapshot.decode_model = Some(model);
        snapshot.table_version = self.versions.get(&model).cloned();

        let mut missing = Vec::new();
        for (name, attribute) in self.attributes.get(&model).into_iter().flatten() {
            let value = match attribute {
                VinAttribute::Constant(value) => Some(value.clone()),
                VinAttribute::Lookup { from, to, values } => {
                    let chars = slice(&vin, *from, *to);
                    values
                        .iter()
                        .find(|(value, _)| value == chars)
                        .map(|(_, decoded)| decoded.clone())
                }
            };
            match value {
                Some(value) => {
                    if name == "Model" {
                        snapshot.program = Some(value.clone());
                    }
                    if name == "ModelYear" {
                        snapshot.model_year = value.trim().parse().ok();
                    }
                    snapshot.attributes.push(DecodedAttribute {
                        name: name.clone(),
                        value,
                    });
                }
                None => missing.push(name.clone()),
            }
        }

        let mut message = match (&snapshot.program, snapshot.model_year) {
            (Some(program), Some(year)) => format!("SDD's VIN tables read this as {program}, model year {year}."),
            (Some(program), None) => format!("SDD's VIN tables read this as {program}; the model year letter is not in the table."),
            _ => "SDD's VIN tables place this VIN but do not name the programme; choose it by hand.".into(),
        };
        if candidates.len() > 1 {
            message.push_str(&format!(
                " More than one decode model claims it ({}); the first in SDD's order is used.",
                candidates
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !missing.is_empty() {
            message.push_str(&format!(
                " Not in the table for this VIN: {}.",
                missing.join(", ")
            ));
        }
        snapshot.message = message;
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> VinTables {
        let mut tables = VinTables::default();
        tables.rules.push(VinRule {
            model: 1,
            tests: parse_rule("1..3=SYN;12..12!=B").unwrap(),
        });
        tables.rules.push(VinRule {
            model: 2,
            tests: parse_rule("1..3=SYN;12..12=B").unwrap(),
        });
        let mut one = BTreeMap::new();
        one.insert(
            "Brand".to_string(),
            parse_attribute("const=Synthetic").unwrap(),
        );
        one.insert(
            "Model".to_string(),
            parse_attribute("chars=6..7;01=SYNTHA|02=SYNTHA").unwrap(),
        );
        one.insert(
            "ModelName".to_string(),
            parse_attribute("chars=6..7;02=Synth A estate%3B long").unwrap(),
        );
        one.insert(
            "ModelYear".to_string(),
            parse_attribute("chars=10..10;A=2010|B=2011").unwrap(),
        );
        tables.attributes.insert(1, one);
        tables
            .versions
            .insert(1, "SDD table version: issue 1".into());
        tables
    }

    #[test]
    fn a_vin_is_placed_by_the_rules_and_read_by_the_attributes() {
        let decoded = tables().decode("syn a1 02 v b b ac12345");
        assert_eq!(decoded.vin, "SYNA102VBBAC12345");
        assert!(decoded.valid);
        assert_eq!(decoded.decode_model, Some(1));
        assert_eq!(decoded.program.as_deref(), Some("SYNTHA"));
        assert_eq!(decoded.model_year, Some(2011));
        let names: Vec<&str> = decoded.attributes.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Brand", "Model", "ModelName", "ModelYear"]);
        assert_eq!(decoded.attributes[2].value, "Synth A estate; long");
        assert!(decoded.message.contains("SYNTHA, model year 2011"));
        assert_eq!(
            decoded.table_version.as_deref(),
            Some("SDD table version: issue 1")
        );
    }

    #[test]
    fn a_not_equal_test_routes_to_the_other_model_and_missing_rows_are_named() {
        let decoded = tables().decode("SYNA102VBBAB12345");
        assert_eq!(decoded.decode_model, Some(2));
        // Model 2 has no attributes in this table set.
        assert_eq!(decoded.program, None);
        assert!(decoded.message.contains("do not name the programme"));

        let decoded = tables().decode("SYNA109VBCAC12345");
        assert_eq!(decoded.decode_model, Some(1));
        assert_eq!(decoded.program, None);
        assert!(decoded
            .message
            .contains("Not in the table for this VIN: Model, ModelName, ModelYear"));
        assert_eq!(decoded.model_year, None);
    }

    #[test]
    fn a_malformed_vin_is_named_before_any_table_is_consulted() {
        let decoded = tables().decode("SYNA102VBBAC1234");
        assert!(!decoded.valid);
        assert!(decoded.message.contains("17 characters; this has 16"));
        let decoded = tables().decode("SYNA1O2VBBAC12345");
        assert!(decoded.message.contains("'O' cannot occur"));
        let decoded = tables().decode("SAJA102VBBAC12345");
        assert!(decoded.valid);
        assert_eq!(decoded.decode_model, None);
        assert!(decoded.message.contains("do not describe this VIN"));
    }
}
