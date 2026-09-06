//! The owner's stamp on a library copy (ADR-0019): whose copy it is, until
//! when it is valid, and a signature that proves the owner issued it.
//!
//! The stamp file `issued_to.json` sits beside the bundles. Version 2 of it
//! carries an Ed25519 signature over [`IssueFields::canonical_text`]; the
//! application trusts the public keys in [`TRUSTED_ISSUER_KEYS`] and refuses
//! a copy whose stamp is missing, unsigned, forged, stale against the data,
//! or past its date. Tools that hold the owner's private key sign with
//! [`sign`]; nothing in this module can produce a valid stamp without it.

use app_contracts::{LibraryIssue, LibraryIssueIntegrity};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The owner's stamp on a library copy, beside the bundles.
pub const ISSUE_STAMP_FILE: &str = "issued_to.json";
/// The first stamp version that carries a signature.
pub const ISSUE_SCHEMA_VERSION: u64 = 2;
/// How long a tester's copy lives unless the issuer says otherwise.
pub const DEFAULT_VALIDITY_DAYS: i64 = 30;
/// The marker the stamping tool writes into every manifest's source notes.
pub const ISSUE_MARKER: &str = "[issued ";
/// Who signs the copies, as written into every stamp.
pub const ISSUER_NAME: &str = "JLR Scanner";

/// Hex-encoded Ed25519 public keys whose stamps the application trusts.
/// The matching private keys live on the owner's machine, never here. To
/// rotate a key, append the new public key; old copies stay valid until
/// their date.
pub const TRUSTED_ISSUER_KEYS: &[&str] = &[
    // Owner's issuing key, made 2026-09-06 (ADR-0019); key id 382146ce.
    "382146ce2b5b4a88ecdb4b22f5937a9b86a7a7f6e55213982740c529db7543c5",
];

/// What a stamp states, in the form the signature covers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IssueFields {
    pub issued_to: String,
    pub issued_on: String,
    pub valid_until: String,
    pub issue_code: String,
    pub issuer: String,
    /// Bundle file name to the SHA-256 (lowercase hex) of its bytes.
    pub bundles: BTreeMap<String, String>,
}

impl IssueFields {
    /// The bytes the signature covers: one line per field, then one line
    /// per bundle in name order. Both the signer and the verifier build it
    /// from the fields, never from the JSON's own layout.
    pub fn canonical_text(&self) -> String {
        let mut text = format!(
            "jlr-scanner library issue v{ISSUE_SCHEMA_VERSION}\nissued_to={}\nissued_on={}\nvalid_until={}\nissue_code={}\nissuer={}\n",
            self.issued_to, self.issued_on, self.valid_until, self.issue_code, self.issuer
        );
        for (name, hash) in &self.bundles {
            text.push_str(&format!("bundle={name} {hash}\n"));
        }
        text
    }

    /// The fields as a stamp file holds them; absent ones are empty.
    pub fn from_stamp(stamp: &Value) -> Self {
        let field = |name: &str| {
            stamp
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let bundles = stamp
            .get("bundles")
            .and_then(Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .map(|(name, hash)| {
                        (
                            name.clone(),
                            hash.as_str().unwrap_or("").to_ascii_lowercase(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            issued_to: field("issued_to"),
            issued_on: field("issued_on"),
            valid_until: field("valid_until"),
            issue_code: field("issue_code"),
            issuer: field("issuer"),
            bundles,
        }
    }

    /// The stamp file's content for these fields and their signature.
    pub fn to_stamp(&self, key_id: &str, signature_hex: &str) -> Value {
        json!({
            "schema_version": ISSUE_SCHEMA_VERSION,
            "issued_to": self.issued_to,
            "issued_on": self.issued_on,
            "valid_until": self.valid_until,
            "issue_code": self.issue_code,
            "issuer": self.issuer,
            "bundles": self.bundles,
            "key_id": key_id,
            "signature": signature_hex,
        })
    }
}

/// Sign the fields with the issuer's private key; hex of the 64 signature bytes.
pub fn sign(fields: &IssueFields, key: &SigningKey) -> String {
    hex_encode(&key.sign(fields.canonical_text().as_bytes()).to_bytes())
}

/// The private key as the key file holds it: 64 hex digits of the seed.
pub fn signing_key_from_hex(seed_hex: &str) -> Option<SigningKey> {
    let bytes = hex_decode(seed_hex.trim())?;
    let seed: [u8; 32] = bytes.try_into().ok()?;
    Some(SigningKey::from_bytes(&seed))
}

/// The public half of a key, as the trusted list holds it.
pub fn public_key_hex(key: &SigningKey) -> String {
    hex_encode(&key.verifying_key().to_bytes())
}

/// The short name of a public key: its first eight hex digits.
pub fn key_id(public_key_hex: &str) -> String {
    public_key_hex
        .trim()
        .chars()
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Read the stamp against the bundles that were read: whose copy, whether
/// the owner signed it, whether every bundle still hashes to what it names,
/// and whether its date has passed. `today` is `YYYY-MM-DD`. Without a
/// stamp file, the marker inside the data still names the code.
pub fn verify(
    stamp: Option<&Value>,
    texts: &[(String, String)],
    trusted_keys: &[&str],
    today: &str,
) -> Option<LibraryIssue> {
    use sha2::{Digest, Sha256};
    let Some(stamp) = stamp else {
        return texts
            .iter()
            .find_map(|(_, text)| embedded_issue_code(text))
            .map(|code| LibraryIssue {
                issued_to: String::new(),
                issued_on: String::new(),
                issue_code: code,
                valid_until: String::new(),
                days_left: -1,
                issuer: String::new(),
                integrity: LibraryIssueIntegrity::StampRemoved,
            });
    };
    let fields = IssueFields::from_stamp(stamp);
    let schema = stamp
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let signature = stamp.get("signature").and_then(Value::as_str);
    let key_id = stamp.get("key_id").and_then(Value::as_str).unwrap_or("");
    let days_left = days_between(today, &fields.valid_until);

    let mut matches = fields.bundles.len() == texts.len();
    for (name, text) in texts {
        let actual = format!("{:x}", Sha256::digest(text.as_bytes()));
        if fields.bundles.get(name) != Some(&actual) {
            matches = false;
        }
    }

    let integrity = match signature {
        None => LibraryIssueIntegrity::Unsigned,
        Some(_) if schema < ISSUE_SCHEMA_VERSION => LibraryIssueIntegrity::Unsigned,
        Some(signature) if !signature_holds(&fields, key_id, signature, trusted_keys) => {
            LibraryIssueIntegrity::BadSignature
        }
        Some(_) if !matches => LibraryIssueIntegrity::Mismatch,
        Some(_) if !days_left.is_some_and(|days| days >= 0) => LibraryIssueIntegrity::Expired,
        Some(_) => LibraryIssueIntegrity::Matches,
    };
    Some(LibraryIssue {
        issued_to: fields.issued_to,
        issued_on: fields.issued_on,
        issue_code: fields.issue_code,
        valid_until: fields.valid_until,
        days_left: days_left.unwrap_or(-1),
        issuer: fields.issuer,
        integrity,
    })
}

/// Whether one of the trusted keys signed these fields. The stamp's
/// `key_id` narrows the search; an empty one tries every key.
fn signature_holds(
    fields: &IssueFields,
    key_id_hint: &str,
    signature_hex: &str,
    trusted_keys: &[&str],
) -> bool {
    let Some(signature_bytes) = hex_decode(signature_hex) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(&signature_bytes) else {
        return false;
    };
    let text = fields.canonical_text();
    let hint = key_id_hint.to_ascii_lowercase();
    trusted_keys
        .iter()
        .filter(|public| hint.is_empty() || key_id(public) == hint)
        .filter_map(|public| {
            let bytes: [u8; 32] = hex_decode(public)?.try_into().ok()?;
            VerifyingKey::from_bytes(&bytes).ok()
        })
        .any(|key| key.verify_strict(text.as_bytes(), &signature).is_ok())
}

/// The issue code inside a bundle's first manifest, if the data was stamped.
pub(crate) fn embedded_issue_code(text: &str) -> Option<String> {
    let head = text.get(..16_384).unwrap_or(text);
    let start = head.find(ISSUE_MARKER)? + ISSUE_MARKER.len();
    let end = head[start..].find(']')? + start;
    let code = head[start..end].trim();
    (!code.is_empty()).then(|| code.to_string())
}

// ---- dates, without a calendar crate (Howard Hinnant's civil algorithms)

/// Today's date, UTC, as `YYYY-MM-DD`.
pub fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format_date(secs.div_euclid(86_400))
}

/// `YYYY-MM-DD` to days since 1970-01-01; `None` for anything else.
pub fn parse_date(text: &str) -> Option<i64> {
    let text = text.trim();
    let mut parts = text.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    // Reject 2026-02-30 and its kind: the round trip must give the date back.
    (format_date(days) == format!("{year:04}-{month:02}-{day:02}")).then_some(days)
}

/// The date `days` after `date`, as `YYYY-MM-DD`.
pub fn add_days(date: &str, days: i64) -> Option<String> {
    Some(format_date(parse_date(date)? + days))
}

/// Days from `from` to `to`; negative when `to` is earlier.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    Some(parse_date(to)? - parse_date(from)?)
}

fn format_date(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip_and_add() {
        assert_eq!(
            parse_date("2026-09-06").map(format_date).as_deref(),
            Some("2026-09-06")
        );
        assert_eq!(add_days("2026-09-06", 30).as_deref(), Some("2026-10-06"));
        assert_eq!(add_days("2028-02-28", 1).as_deref(), Some("2028-02-29"));
        assert_eq!(add_days("2026-12-31", 1).as_deref(), Some("2027-01-01"));
        assert_eq!(days_between("2026-09-06", "2026-10-06"), Some(30));
        assert_eq!(days_between("2026-10-06", "2026-09-06"), Some(-30));
        assert_eq!(parse_date("2026-02-30"), None);
        assert_eq!(parse_date("yesterday"), None);
        assert_eq!(parse_date("1970-01-01"), Some(0));
    }

    #[test]
    fn a_signature_verifies_only_for_the_signed_text_and_key() {
        let key = signing_key_from_hex(&"11".repeat(32)).unwrap();
        let other = signing_key_from_hex(&"22".repeat(32)).unwrap();
        let mut fields = IssueFields {
            issued_to: "Тест".into(),
            issued_on: "2026-09-06".into(),
            valid_until: "2026-10-06".into(),
            issue_code: "AB12-CD34".into(),
            issuer: ISSUER_NAME.into(),
            bundles: BTreeMap::from([("platform.json".to_string(), "00".repeat(32))]),
        };
        let signature = sign(&fields, &key);
        let public = public_key_hex(&key);
        assert!(signature_holds(
            &fields,
            &key_id(&public),
            &signature,
            &[&public]
        ));
        assert!(signature_holds(&fields, "", &signature, &[&public]));
        assert!(!signature_holds(
            &fields,
            &key_id(&public),
            &signature,
            &[&public_key_hex(&other)]
        ));
        fields.issued_to = "Хтось інший".into();
        assert!(!signature_holds(
            &fields,
            &key_id(&public),
            &signature,
            &[&public]
        ));
    }

    #[test]
    fn hex_round_trips() {
        assert_eq!(
            hex_decode(&hex_encode(&[0, 1, 254, 255])),
            Some(vec![0, 1, 254, 255])
        );
        assert_eq!(hex_decode("abc"), None);
        assert_eq!(hex_decode("zz"), None);
    }
}
