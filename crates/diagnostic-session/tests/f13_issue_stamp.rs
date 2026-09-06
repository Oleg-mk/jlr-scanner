//! The issue stamp on a library copy (ADR-0019): the application loads a
//! folder only under a stamp the owner signed, that matches the data and
//! is not past its date. Every refusal names its reason. Tools keep the
//! reporting loader, which refuses nothing.

use app_contracts::{LibraryIssueIntegrity, LibraryState};
use diagnostic_session::issue::{
    key_id, public_key_hex, sign, signing_key_from_hex, IssueFields, ISSUER_NAME,
};
use diagnostic_session::KnowledgeLibrary;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// A synthetic manifest, not one of the built-ins: the built-in copy would
// make the directory's one a duplicate source and count as a failure.
const MANIFEST: &str = include_str!("../../../fixtures/knowledge/synthetic/f5_architecture.json");
const TODAY: &str = "2026-09-06";
/// A test key; the application trusts only the owner's, never this one.
const TEST_SEED: &str = "0102030405060708091011121314151617181920212223242526272829303132";
const OTHER_SEED: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

fn temp_library(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("jlr-issue-stamp-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn bundle(notes_suffix: Option<&str>) -> String {
    let mut manifest: serde_json::Value = serde_json::from_str(MANIFEST).expect("fixture parses");
    if let Some(suffix) = notes_suffix {
        let source = manifest["source"].as_object_mut().expect("source object");
        let notes = source
            .get("notes")
            .and_then(|value| value.as_str())
            .map(|existing| format!("{existing} {suffix}"))
            .unwrap_or_else(|| suffix.to_string());
        source.insert("notes".into(), serde_json::Value::String(notes));
    }
    serde_json::to_string(&serde_json::Value::Array(vec![manifest])).expect("bundle serialises")
}

fn fields(bundle_text: &str, valid_until: &str) -> IssueFields {
    IssueFields {
        issued_to: "Тест Тестенко".into(),
        issued_on: "2026-09-05".into(),
        valid_until: valid_until.into(),
        issue_code: "AB12-CD34".into(),
        issuer: ISSUER_NAME.into(),
        bundles: BTreeMap::from([(
            "platform.json".to_string(),
            format!("{:x}", Sha256::digest(bundle_text.as_bytes())),
        )]),
    }
}

/// A stamp signed with the test key over `fields`, optionally edited after
/// signing (which is what a forger does).
fn signed_stamp(
    fields: &IssueFields,
    seed: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) -> String {
    let key = signing_key_from_hex(seed).expect("seed is a key");
    let mut stamp = fields.to_stamp(&key_id(&public_key_hex(&key)), &sign(fields, &key));
    edit(&mut stamp);
    stamp.to_string()
}

fn trusted_key() -> String {
    public_key_hex(&signing_key_from_hex(TEST_SEED).expect("seed is a key"))
}

fn load(dir: &Path, enforce: bool) -> KnowledgeLibrary {
    let trusted = trusted_key();
    KnowledgeLibrary::load_directory_with(dir, TODAY, &[trusted.as_str()], enforce)
}

fn write_copy(tag: &str, bundle_text: &str, stamp: Option<&str>) -> PathBuf {
    let dir = temp_library(tag);
    std::fs::write(dir.join("platform.json"), bundle_text).unwrap();
    if let Some(stamp) = stamp {
        std::fs::write(dir.join("issued_to.json"), stamp).unwrap();
    }
    dir
}

#[test]
fn a_signed_unexpired_copy_loads_and_is_named_with_its_validity() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let stamp = signed_stamp(&fields(&text, "2026-10-05"), TEST_SEED, |_| {});
    let dir = write_copy("matches", &text, Some(&stamp));

    let library = load(&dir, true);
    let snapshot = library.snapshot();
    assert_eq!(snapshot.state, LibraryState::Loaded);
    let issue = snapshot.issue.as_ref().expect("the stamp is read");
    assert_eq!(issue.issued_to, "Тест Тестенко");
    assert_eq!(issue.issued_on, "2026-09-05");
    assert_eq!(issue.valid_until, "2026-10-05");
    assert_eq!(issue.days_left, 29);
    assert_eq!(issue.issue_code, "AB12-CD34");
    assert_eq!(issue.issuer, ISSUER_NAME);
    assert_eq!(issue.integrity, LibraryIssueIntegrity::Matches);
    // The stamp file is not a manifest: nothing failed, the bundle loaded.
    assert_eq!(snapshot.manifests_failed, 0);
    assert!(snapshot.manifests_loaded > 0);
}

#[test]
fn the_last_valid_day_still_loads() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let stamp = signed_stamp(&fields(&text, TODAY), TEST_SEED, |_| {});
    let dir = write_copy("last-day", &text, Some(&stamp));
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Loaded);
    assert_eq!(library.snapshot().issue.as_ref().unwrap().days_left, 0);
}

#[test]
fn an_expired_copy_is_refused_by_the_application_and_reported_by_tools() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let stamp = signed_stamp(&fields(&text, "2026-09-05"), TEST_SEED, |_| {});
    let dir = write_copy("expired", &text, Some(&stamp));

    let refused = load(&dir, true);
    let snapshot = refused.snapshot();
    assert_eq!(snapshot.state, LibraryState::Failed);
    // Only the built-in manifests: nothing from the folder was ingested.
    assert_eq!(
        snapshot.manifests_loaded,
        KnowledgeLibrary::built_in().snapshot().manifests_loaded
    );
    assert!(
        snapshot.message.contains("has expired"),
        "{}",
        snapshot.message
    );
    let issue = snapshot.issue.as_ref().unwrap();
    assert_eq!(issue.integrity, LibraryIssueIntegrity::Expired);
    assert_eq!(issue.days_left, -1);
    assert_eq!(issue.issued_to, "Тест Тестенко");

    let reported = load(&dir, false);
    assert_eq!(reported.snapshot().state, LibraryState::Loaded);
    assert_eq!(
        reported.snapshot().issue.as_ref().unwrap().integrity,
        LibraryIssueIntegrity::Expired
    );
}

#[test]
fn a_stamp_edited_after_signing_or_signed_by_another_key_is_refused() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let edited = signed_stamp(&fields(&text, "2026-10-05"), TEST_SEED, |stamp| {
        stamp["valid_until"] = serde_json::Value::String("2030-01-01".into());
    });
    let dir = write_copy("edited", &text, Some(&edited));
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Failed);
    assert_eq!(
        library.snapshot().issue.as_ref().unwrap().integrity,
        LibraryIssueIntegrity::BadSignature
    );
    assert!(library.snapshot().message.contains("signature"));

    let foreign = signed_stamp(&fields(&text, "2026-10-05"), OTHER_SEED, |_| {});
    let dir = write_copy("foreign", &text, Some(&foreign));
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Failed);
    assert_eq!(
        library.snapshot().issue.as_ref().unwrap().integrity,
        LibraryIssueIntegrity::BadSignature
    );
}

#[test]
fn an_unsigned_stamp_of_the_old_form_is_refused() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let old = serde_json::json!({
        "schema_version": 1,
        "issued_to": "Тест Тестенко",
        "issued_on": "2026-09-05",
        "issue_code": "AB12-CD34",
        "bundles": { "platform.json": format!("{:x}", Sha256::digest(text.as_bytes())) }
    })
    .to_string();
    let dir = write_copy("unsigned", &text, Some(&old));
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Failed);
    assert_eq!(
        library.snapshot().issue.as_ref().unwrap().integrity,
        LibraryIssueIntegrity::Unsigned
    );
    assert!(library.snapshot().message.contains("not signed"));
}

#[test]
fn data_that_no_longer_matches_a_signed_stamp_is_refused() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let stamp = signed_stamp(&fields(&text, "2026-10-05"), TEST_SEED, |_| {});
    let dir = write_copy("mismatch", &text, Some(&stamp));
    // The bundle is changed after the stamp was signed over its hash.
    std::fs::write(
        dir.join("platform.json"),
        bundle(Some("[issued ZZ99-ZZ99]")),
    )
    .unwrap();
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Failed);
    let issue = library.snapshot().issue.as_ref().unwrap();
    assert_eq!(issue.integrity, LibraryIssueIntegrity::Mismatch);
    assert_eq!(issue.issue_code, "AB12-CD34");
    assert!(library.snapshot().message.contains("does not match"));
}

#[test]
fn a_removed_stamp_file_is_refused_and_the_code_inside_is_named() {
    let text = bundle(Some("[issued AB12-CD34]"));
    let dir = write_copy("removed", &text, None);
    let library = load(&dir, true);
    assert_eq!(library.snapshot().state, LibraryState::Failed);
    let issue = library.snapshot().issue.as_ref().unwrap();
    assert_eq!(issue.integrity, LibraryIssueIntegrity::StampRemoved);
    assert_eq!(issue.issue_code, "AB12-CD34");
    assert!(library.snapshot().message.contains("AB12-CD34"));
}

#[test]
fn a_folder_without_any_stamp_is_refused_by_the_application_and_loaded_by_tools() {
    let text = bundle(None);
    let dir = write_copy("plain", &text, None);

    let refused = load(&dir, true);
    assert_eq!(refused.snapshot().state, LibraryState::Failed);
    assert_eq!(
        refused.snapshot().issue.as_ref().unwrap().integrity,
        LibraryIssueIntegrity::NoStamp
    );
    assert!(refused.snapshot().message.contains("no issue stamp"));

    let tooling = load(&dir, false);
    assert_eq!(tooling.snapshot().state, LibraryState::Loaded);
    assert!(tooling.snapshot().issue.is_none());
}
