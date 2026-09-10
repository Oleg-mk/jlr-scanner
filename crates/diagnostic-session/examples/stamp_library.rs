//! Issue a stamped, signed copy of the exported library to one named tester
//! (ADR-0019), check a folder the way the application does, or make the
//! issuing key.
//!
//! ```text
//! stamp_library <library dir> <output dir> <issued to> [days] [YYYY-MM-DD]
//! stamp_library --check <dir>
//! stamp_library --new-key <path>
//! ```
//!
//! Every manifest in every bundle gets `[issued CODE]` appended to its
//! source notes, the stamped bundles are written to the output directory,
//! and `issued_to.json` records who the copy is for, when, until when, the
//! code, the SHA-256 of each bundle, and the owner's signature over all of
//! that. Signing needs the private key: the file named by `JLR_ISSUER_KEY`
//! (default `/issuer/library-issuer.key`), 64 hex digits, never in the
//! repository. The application loads a copy only under a stamp signed by a
//! key in its trusted list, matching the data, and not past its date.

use diagnostic_session::issue::{
    add_days, key_id, public_key_hex, sign, signing_key_from_hex, today_utc, IssueFields,
    DEFAULT_VALIDITY_DAYS, ISSUER_NAME, ISSUE_STAMP_FILE, TRUSTED_ISSUER_KEYS,
};
use diagnostic_session::KnowledgeLibrary;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

const DEFAULT_KEY_PATH: &str = "/issuer/library-issuer.key";
const USAGE: &str = "usage:\n  stamp_library <library dir> <output dir> <issued to> [days] [YYYY-MM-DD]\n  stamp_library --check <dir>\n  stamp_library --new-key <path>";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--new-key") => new_key(args.get(2).map(String::as_str)),
        Some("--check") => check(args.get(2).map(String::as_str)),
        Some(_) if args.len() >= 4 => issue(&args),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

/// Make the issuing key pair: the seed goes to `path` as hex, the public
/// half is printed for `TRUSTED_ISSUER_KEYS`. Refuses to overwrite.
fn new_key(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let path = Path::new(path);
    if path.exists() {
        eprintln!("{} exists; not overwriting a key", path.display());
        std::process::exit(1);
    }
    let mut seed = [0_u8; 32];
    getrandom::getrandom(&mut seed).map_err(|error| format!("no random source: {error}"))?;
    let hex: String = seed.iter().map(|byte| format!("{byte:02x}")).collect();
    let key = signing_key_from_hex(&hex).expect("a fresh seed is a key");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{hex}\n"))?;
    let public = public_key_hex(&key);
    println!(
        "private key written to {} — keep it out of the repository",
        path.display()
    );
    println!("public key: {public}");
    println!("key id:     {}", key_id(&public));
    println!("add the public key to TRUSTED_ISSUER_KEYS in crates/diagnostic-session/src/issue.rs");
    Ok(())
}

/// Load a folder exactly as the application does and say what happened.
fn check(directory: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(directory) = directory else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let library = KnowledgeLibrary::load_issued_directory(Path::new(directory));
    let snapshot = library.snapshot();
    println!("state:   {:?}", snapshot.state);
    println!("message: {}", snapshot.message);
    if let Some(issue) = &snapshot.issue {
        println!(
            "issue:   {:?}; issued to {:?} on {} until {} ({} days left), copy {}, issuer {:?}",
            issue.integrity,
            issue.issued_to,
            issue.issued_on,
            issue.valid_until,
            issue.days_left,
            issue.issue_code,
            issue.issuer
        );
    }
    println!(
        "loaded:  {} manifests, {} failed, {} sources, {} records",
        snapshot.manifests_loaded, snapshot.manifests_failed, snapshot.sources, snapshot.records
    );
    if snapshot.state == app_contracts::LibraryState::Failed {
        std::process::exit(1);
    }
    Ok(())
}

fn issue(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let source = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    let issued_to = args[3].trim().to_string();
    if issued_to.is_empty() {
        eprintln!("the name the copy is issued to must not be empty");
        std::process::exit(2);
    }
    // Optional: a day count, a date, or both in either order.
    let mut days = DEFAULT_VALIDITY_DAYS;
    let mut issued_on = today_utc();
    for extra in args.iter().skip(4) {
        if extra.contains('-') {
            issued_on = extra.trim().to_string();
        } else {
            days = extra.trim().parse()?;
        }
    }
    let Some(valid_until) = add_days(&issued_on, days) else {
        eprintln!("{issued_on} is not a YYYY-MM-DD date");
        std::process::exit(2);
    };

    let key_path = std::env::var("JLR_ISSUER_KEY").unwrap_or_else(|_| DEFAULT_KEY_PATH.to_string());
    let seed = std::fs::read_to_string(&key_path).map_err(|error| {
        format!("cannot read the issuing key {key_path}: {error} (set JLR_ISSUER_KEY, or make one with --new-key)")
    })?;
    let Some(key) = signing_key_from_hex(&seed) else {
        eprintln!("{key_path} does not hold 64 hex digits");
        std::process::exit(2);
    };
    let public = public_key_hex(&key);
    if !TRUSTED_ISSUER_KEYS
        .iter()
        .any(|trusted| trusted.eq_ignore_ascii_case(&public))
    {
        eprintln!(
            "warning: this key is not in the application's trusted list; the copy will be refused"
        );
    }

    std::fs::create_dir_all(out)?;
    let code = issue_code(&issued_to, &issued_on);
    let marker = format!("[issued {code}]");
    let mut paths: Vec<_> = std::fs::read_dir(source)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| {
                    knowledge::manifest_file::is_manifest(name) && name != ISSUE_STAMP_FILE
                })
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        eprintln!("no bundles in {}", source.display());
        std::process::exit(1);
    }

    let mut bundles = BTreeMap::new();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("bundle.json")
            .to_string();
        let text = knowledge::manifest_file::read(&path)?;
        let mut value: Value = serde_json::from_str(&text)?;
        let mut stamped = 0usize;
        if let Value::Array(manifests) = &mut value {
            for manifest in manifests.iter_mut() {
                if let Some(source) = manifest.get_mut("source").and_then(Value::as_object_mut) {
                    let notes = match source.get("notes") {
                        Some(Value::String(existing)) if !existing.is_empty() => {
                            format!("{existing} {marker}")
                        }
                        _ => marker.clone(),
                    };
                    source.insert("notes".to_string(), Value::String(notes));
                    stamped += 1;
                }
            }
        }
        // The stamp covers what a manifest says, not how it is packed
        // (ADR-0023), so the hash is of the JSON and the copy keeps the
        // form it arrived in.
        let text = serde_json::to_string(&value)?;
        knowledge::manifest_file::write(&out.join(&name), &text)?;
        bundles.insert(
            name.clone(),
            format!("{:x}", Sha256::digest(text.as_bytes())),
        );
        println!("{name}: {stamped} manifests stamped");
    }

    let fields = IssueFields {
        issued_to: issued_to.clone(),
        issued_on: issued_on.clone(),
        valid_until: valid_until.clone(),
        issue_code: code.clone(),
        issuer: ISSUER_NAME.to_string(),
        bundles,
    };
    let signature = sign(&fields, &key);
    let stamp = fields.to_stamp(&key_id(&public), &signature);
    std::fs::write(
        out.join(ISSUE_STAMP_FILE),
        serde_json::to_string_pretty(&stamp)?,
    )?;

    // Read the copy back the way the application will, under this key.
    let verdict = KnowledgeLibrary::load_directory_with(out, &today_utc(), &[&public], true);
    let accepted = verdict.snapshot().state != app_contracts::LibraryState::Failed;
    println!(
        "issued to {issued_to} on {issued_on}, valid until {valid_until} ({days} days), code {code}; written to {}",
        out.display()
    );
    println!(
        "{}",
        if accepted {
            "verified: the application accepts this copy under the issuing key"
        } else {
            "NOT ACCEPTED on read-back — do not hand this copy out"
        }
    );
    if !accepted {
        std::process::exit(1);
    }
    Ok(())
}

/// Eight hex characters from the name, the date and the moment of issue,
/// as `XXXX-XXXX`: unique enough to tell copies apart, short enough to
/// read out.
fn issue_code(name: &str, date: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let digest = Sha256::digest(format!("{name}|{date}|{nanos}").as_bytes());
    let hex = format!("{digest:X}");
    format!("{}-{}", &hex[..4], &hex[4..8])
}
