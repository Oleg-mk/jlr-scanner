//! What a module declares it will accept, as the session reports it
//! (`ADR-0035`).
//!
//! The rows are knowledge and nothing sends them. What the session adds to
//! the ingest is the class each operation would have to be granted: a value
//! written into a module persists, an output driven by hand does not, and a
//! routine is a procedure of its own. Three classes, all three unused by
//! this build, and the row carries its own so the boundary is readable
//! beside the thing it stops.

use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use sdd_ingest::ModuleAccessAdapter;

const MODULE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_mdx_module.xml");

fn library() -> diagnostic_session::KnowledgeLibrary {
    let source = SourceRecord {
        id: SourceId::new("mdx-test").unwrap(),
        title: "Module index fixture".into(),
        source_type: SourceType::Synthetic,
        origin: "diagnostic-session test".into(),
        source_locator: "fixtures/knowledge/synthetic".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(MODULE.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    };
    let batch = ModuleAccessAdapter::new(source, "SYNTHA", "MY10")
        .unwrap()
        .parse(MODULE)
        .unwrap();
    let manifest = serde_json::to_string(&batch).unwrap();
    diagnostic_session::KnowledgeLibrary::from_manifests([(
        "module_access.json",
        manifest.as_str(),
    )])
}

fn context() -> diagnostic_session::VehicleContext {
    diagnostic_session::VehicleContext {
        vehicle_program: Some("SYNTHA".into()),
        ..Default::default()
    }
}

#[test]
fn every_kind_carries_the_class_it_would_have_to_be_granted() {
    let library = library();
    let rows = library.accepted_operations(&context(), "SYNTHMOD");

    let classes: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.kind.as_str(), row.safety_class.as_str()))
        .collect();
    assert!(
        classes.contains(&("WRITE", "PERSISTENT_CHANGE")),
        "{classes:?}"
    );
    assert!(
        classes.contains(&("CONTROL", "VOLATILE_CONTROL")),
        "{classes:?}"
    );
    assert!(
        classes.contains(&("ROUTINE", "SERVICE_ROUTINE")),
        "{classes:?}"
    );
    // The readable identifier of the same fixture is nobody's operation.
    assert!(
        rows.iter().all(|row| row.identifier != "0x0100"),
        "{rows:?}"
    );
}

#[test]
fn the_cost_of_an_operation_is_carried_beside_it() {
    let library = library();
    let rows = library.accepted_operations(&context(), "SYNTHMOD");

    let control = rows
        .iter()
        .find(|row| row.identifier == "0x0300")
        .expect("the controllable identifier");
    assert_eq!(control.service.as_deref(), Some("0x2F"));
    assert_eq!(control.sessions, vec!["03".to_string()]);
    assert_eq!(control.security.as_deref(), Some("level_1"));

    let routine = rows
        .iter()
        .find(|row| row.identifier == "0x0400")
        .expect("the routine");
    assert_eq!(routine.name.as_deref(), Some("Synthetic clear adaptions"));
    assert_eq!(routine.max_run_time.as_deref(), Some("30"));
    assert_eq!(routine.restart_while_running.as_deref(), Some("no"));
    // SDD names no security level beside a routine anywhere in the corpus,
    // and the row says so by leaving it empty rather than by inventing one.
    assert!(routine.security.is_none(), "{routine:?}");
}

#[test]
fn a_module_of_another_name_is_given_nothing() {
    let library = library();
    assert!(library
        .accepted_operations(&context(), "OTHERMOD")
        .is_empty());
}
