//! F10 golden tests: from SDD-shaped knowledge to an offline UDS read.
//!
//! The store is built the way the product builds it — the platform and DID
//! fixtures through the F9 adapters, the adapter route bindings through the
//! documented manifest — and then resolved, prepared and executed offline.
//! Fixtures are synthetic and prove behaviour only.

use diagnostic_environment::{
    DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver, EnvironmentField,
};
use diagnostic_simulator::{PayloadSimulatorBehavior, PayloadSimulatorScenario, SimulatorSource};
use knowledge::{
    sha256_bytes, ContentFingerprint, JsonManifestAdapter, KnowledgeStore, RedistributionStatus,
    SourceId, SourceRecord, SourceType, ValidationState, VehicleContext,
};
use sdd_ingest::{
    ConverterCatalogue, DidFormattingAdapter, ModelYearTimeline, PlatformAdapter,
    YEAR_BREAKPOINT_DIMENSION,
};
use std::collections::{BTreeMap, BTreeSet};
use transport_api::CanId;
use transport_replay::{PlaybackMode, ReplaySource};
use uds::NegativeResponseCode;
use uds_execution::{
    execute_replay, execute_simulator, prepare_read_only_transaction, DiagnosticTargetIdentity,
    DtcStatusMask, OfflineExecutionSource, PreparationError, ProvenanceField, ReadOnlyUdsIntent,
    TransactionSafetyClass, UdsReadOutcome, READ_DATA_BY_IDENTIFIER_CAPABILITY,
    READ_DTC_INFORMATION_CAPABILITY,
};

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const BINDINGS: &str =
    include_str!("../../../fixtures/knowledge/documented/mongoose_jlr_route_bindings.json");
const REPLAY: &str =
    include_str!("../../../fixtures/synthetic/f10_uds_did_multiframe_after_pending.json");

fn synthetic_source(id: &str, locator: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F10 synthetic fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "F10 golden test".into(),
        source_locator: locator.into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(text.as_bytes())).unwrap(),
        ),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn timeline() -> ModelYearTimeline {
    let mut timeline = ModelYearTimeline::new();
    for marker in ["MY08", "MY10", "MY12"] {
        timeline.observe("SYNTHA", marker).unwrap();
    }
    timeline
}

fn store(with_bindings: bool) -> KnowledgeStore {
    let platform = PlatformAdapter::new(synthetic_source(
        "f10-plat",
        "fixtures/knowledge/synthetic/f9_platform.xml",
        PLATFORM,
    ))
    .unwrap()
    .with_timeline(timeline());
    let mut converters = ConverterCatalogue::new();
    converters.insert_from_xml(CONVERTER).unwrap();
    let dids = DidFormattingAdapter::new(
        synthetic_source(
            "f10-did",
            "fixtures/knowledge/synthetic/f9_did_formatting.xml",
            DIDS,
        ),
        converters,
    )
    .unwrap()
    .with_timeline(timeline());

    let mut store = KnowledgeStore::new();
    store.ingest(&platform, PLATFORM).unwrap();
    store.ingest(&dids, DIDS).unwrap();
    if with_bindings {
        store.ingest(&JsonManifestAdapter, BINDINGS).unwrap();
    }
    store
}

/// What a decoded vehicle would state for the fixture programme: the
/// programme, the year, the engine and SDD's own breakpoint marker.
fn context() -> VehicleContext {
    let mut other = BTreeMap::new();
    other.insert(YEAR_BREAKPOINT_DIMENSION.to_string(), "MY10".to_string());
    VehicleContext {
        vehicle_program: Some("SYNTHA".into()),
        model_year: Some(2010),
        powertrain: Some("SYNTHENGINE".into()),
        other,
        ..VehicleContext::default()
    }
}

fn resolved(capability: &str) -> DiagnosticEnvironmentResolution {
    DiagnosticEnvironmentResolver::resolve_ecu_family(
        &store(true),
        &context(),
        "SYNTHMOD",
        capability,
    )
    .unwrap()
}

fn unresolved_fields(resolution: &DiagnosticEnvironmentResolution) -> Vec<EnvironmentField> {
    let DiagnosticEnvironmentResolution::Indeterminate {
        unresolved_facts, ..
    } = resolution
    else {
        panic!("expected INDETERMINATE, got {resolution:?}");
    };
    unresolved_facts.iter().map(|fact| fact.field).collect()
}

fn family() -> DiagnosticTargetIdentity {
    DiagnosticTargetIdentity::family("SYNTHMOD").unwrap()
}

fn prepared_did_read(identifier: u16) -> uds_execution::PreparedUdsTransaction {
    let readable =
        DiagnosticEnvironmentResolver::readable_identifiers(&store(true), &context(), "SYNTHMOD");
    prepare_read_only_transaction(
        &resolved(READ_DATA_BY_IDENTIFIER_CAPABILITY),
        ReadOnlyUdsIntent::read_data_by_identifier(family(), identifier),
        &readable,
    )
    .unwrap()
}

fn simulate(
    transaction: &uds_execution::PreparedUdsTransaction,
    response_payload: Vec<u8>,
) -> SimulatorSource {
    SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: transaction.encoded_payload().to_vec(),
            behavior: PayloadSimulatorBehavior::Response { response_payload },
        },
        transaction.encoded_payload(),
        transaction.backend_route(),
        transaction.expected_response_id(),
    )
    .unwrap()
}

#[test]
fn a_module_sdd_describes_resolves_to_a_family_level_plan() {
    let DiagnosticEnvironmentResolution::Resolved(plan) =
        resolved(READ_DATA_BY_IDENTIFIER_CAPABILITY)
    else {
        panic!("expected RESOLVED");
    };

    assert_eq!(plan.ecu_family.value, "SYNTHMOD");
    // SDD names no software build, so none is claimed (ADR-0013).
    assert_eq!(plan.diagnostic_implementation, None);
    assert_eq!(plan.logical_network.value, "CAN_HS");
    assert_eq!(plan.physical_route.value.connector, "J1962");
    assert_eq!(plan.physical_route.value.pins, vec![6, 14]);
    assert_eq!(plan.backend_route.value.backend, "mongoose-jlr");
    assert_eq!(plan.backend_route.value.route_id, "hs-can");
    assert_eq!(plan.bitrate_bps.value, 500_000);
    // The rate is the route's, stated by the binding alone; the platform's
    // per-module claim names the bus only (ADR-0015).
    let rate_sources: BTreeSet<_> = plan
        .bitrate_bps
        .evidence
        .iter()
        .map(|trace| trace.source_id.as_str())
        .collect();
    assert_eq!(
        rate_sources,
        BTreeSet::from(["jlr-scanner-mongoose-jlr-route-bindings"])
    );
    assert_eq!(plan.protocol_family.value, "ISO14229");
    assert_eq!(plan.addressing_mode.value, "normal");
    assert_eq!(plan.physical_request_id.value, 0x7E0);
    assert_eq!(plan.physical_response_id.value, 0x7E8);
    assert_eq!(plan.functional_request_id, None);
    assert_eq!(
        plan.read_only_capability.value.id,
        READ_DATA_BY_IDENTIFIER_CAPABILITY
    );
    // Synthetic sources can never lift the plan above Unverified.
    assert_eq!(plan.validation_state, ValidationState::Unverified);
}

#[test]
fn readable_identifiers_come_only_from_entries_qualified_to_the_module() {
    let readable =
        DiagnosticEnvironmentResolver::readable_identifiers(&store(true), &context(), "SYNTHMOD");
    let identifiers: Vec<_> = readable.iter().map(|entry| entry.identifier).collect();
    // 0x1945 is qualified to the module alone; 0x0347 to module, programme,
    // breakpoint and engine. 0x0301 and 0x0343 are unqualified formatting
    // entries and say nothing about what this module exposes.
    assert_eq!(identifiers, vec![0x0347, 0x1945]);
    let module_scoped = readable.iter().find(|e| e.identifier == 0x1945).unwrap();
    assert_eq!(module_scoped.parameters.len(), 1);
    assert_eq!(
        module_scoped.parameters[0].name,
        "Synthetic module-scoped value"
    );
    assert_eq!(module_scoped.parameters[0].unit.as_deref(), Some("V"));
    assert!(!module_scoped.evidence.is_empty());

    // A different module has no entries at all.
    assert!(DiagnosticEnvironmentResolver::readable_identifiers(
        &store(true),
        &context(),
        "OTHERMOD"
    )
    .is_empty());
}

#[test]
fn without_route_bindings_the_route_is_unresolved_and_named() {
    let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
        &store(false),
        &context(),
        "SYNTHMOD",
        READ_DATA_BY_IDENTIFIER_CAPABILITY,
    )
    .unwrap();
    let fields = unresolved_fields(&resolution);
    assert!(fields.contains(&EnvironmentField::BackendRoute));
    assert!(fields.contains(&EnvironmentField::PhysicalRoute));
    // Everything SDD itself states is still known.
    assert!(!fields.contains(&EnvironmentField::LogicalNetwork));
    // The rate is the route's (ADR-0015), so without a binding it is
    // unresolved together with the route.
    assert!(fields.contains(&EnvironmentField::Bitrate));
    assert!(!fields.contains(&EnvironmentField::ProtocolFamily));
}

#[test]
fn a_bus_without_a_declared_protocol_yields_no_capability_and_no_plan() {
    let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
        &store(true),
        &context(),
        "OTHERMOD",
        READ_DATA_BY_IDENTIFIER_CAPABILITY,
    )
    .unwrap();
    let fields = unresolved_fields(&resolution);
    assert!(fields.contains(&EnvironmentField::ProtocolFamily));
    assert!(fields.contains(&EnvironmentField::ReadOnlyCapability));
    // Its bus is bound, so the route itself is not what is missing.
    assert!(!fields.contains(&EnvironmentField::BackendRoute));
}

#[test]
fn an_unlisted_identifier_is_refused_before_anything_is_encoded() {
    let readable =
        DiagnosticEnvironmentResolver::readable_identifiers(&store(true), &context(), "SYNTHMOD");
    let error = prepare_read_only_transaction(
        &resolved(READ_DATA_BY_IDENTIFIER_CAPABILITY),
        ReadOnlyUdsIntent::read_data_by_identifier(family(), 0x0301),
        &readable,
    )
    .unwrap_err();
    assert_eq!(
        error,
        PreparationError::IdentifierNotReadable { identifier: 0x0301 }
    );

    // Passing no catalogue at all refuses everything.
    assert!(matches!(
        prepare_read_only_transaction(
            &resolved(READ_DATA_BY_IDENTIFIER_CAPABILITY),
            ReadOnlyUdsIntent::read_data_by_identifier(family(), 0x1945),
            &[],
        ),
        Err(PreparationError::IdentifierNotReadable { .. })
    ));
}

#[test]
fn a_prepared_read_names_the_wrong_capability_protocol_or_addressing() {
    let readable =
        DiagnosticEnvironmentResolver::readable_identifiers(&store(true), &context(), "SYNTHMOD");
    let DiagnosticEnvironmentResolution::Resolved(plan) =
        resolved(READ_DATA_BY_IDENTIFIER_CAPABILITY)
    else {
        panic!("expected RESOLVED");
    };

    // A DTC read against a plan resolved for identifier reads.
    assert_eq!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(plan.clone()),
            ReadOnlyUdsIntent::read_dtc_information(family(), DtcStatusMask::Confirmed),
            &readable,
        )
        .unwrap_err(),
        PreparationError::UnsupportedCapability(READ_DATA_BY_IDENTIFIER_CAPABILITY.into())
    );

    let mut enhanced = plan.clone();
    enhanced.addressing_mode.value = "enhanced".into();
    assert_eq!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(enhanced),
            ReadOnlyUdsIntent::read_data_by_identifier(family(), 0x1945),
            &readable,
        )
        .unwrap_err(),
        PreparationError::UnsupportedAddressingMode("enhanced".into())
    );

    let mut legacy = plan.clone();
    legacy.protocol_family.value = "KW2000".into();
    assert_eq!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(legacy),
            ReadOnlyUdsIntent::read_data_by_identifier(family(), 0x1945),
            &readable,
        )
        .unwrap_err(),
        PreparationError::UnsupportedProtocol("KW2000".into())
    );

    // A caller naming a software build cannot be satisfied by a family plan.
    let build = DiagnosticTargetIdentity::implementation("SYNTHMOD", "build-x").unwrap();
    assert_eq!(
        prepare_read_only_transaction(
            &DiagnosticEnvironmentResolution::Resolved(plan),
            ReadOnlyUdsIntent::read_data_by_identifier(build, 0x1945),
            &readable,
        )
        .unwrap_err(),
        PreparationError::MissingDiagnosticImplementation
    );
}

#[test]
fn simulator_read_data_by_identifier_round_trip() {
    let transaction = prepared_did_read(0x1945);
    assert_eq!(transaction.safety_class(), TransactionSafetyClass::ReadOnly);
    assert_eq!(transaction.encoded_payload(), &[0x22, 0x19, 0x45]);
    assert_eq!(transaction.backend_route(), "hs-can");
    assert_eq!(transaction.physical_pins(), &[6, 14]);
    assert_eq!(
        transaction.physical_request_id(),
        CanId::standard(0x7E0).unwrap()
    );
    assert_eq!(
        transaction
            .readable_identifier()
            .map(|entry| entry.identifier),
        Some(0x1945)
    );
    assert!(!transaction
        .provenance()
        .traces_for(ProvenanceField::Identifier)
        .is_empty());
    assert!(transaction
        .provenance()
        .traces_for(ProvenanceField::DiagnosticImplementation)
        .is_empty());

    let mut source = simulate(&transaction, vec![0x62, 0x19, 0x45, 0x12, 0x34]);
    let result = execute_simulator(&transaction, &mut source, "f10-sim-did", 10_000).unwrap();
    assert_eq!(result.source, OfflineExecutionSource::Simulator);
    assert_eq!(
        result.outcome,
        UdsReadOutcome::DataByIdentifier {
            identifier: 0x1945,
            data: vec![0x12, 0x34],
        }
    );
    assert_eq!(
        result.raw_diagnostic_response,
        vec![0x62, 0x19, 0x45, 0x12, 0x34]
    );
    assert_eq!(result.responder, CanId::standard(0x7E8).unwrap());
}

#[test]
fn a_negative_response_is_a_result_with_its_code() {
    let transaction = prepared_did_read(0x0347);
    let mut source = simulate(&transaction, vec![0x7F, 0x22, 0x31]);
    let result = execute_simulator(&transaction, &mut source, "f10-sim-nrc", 10_000).unwrap();
    let UdsReadOutcome::Negative(negative) = result.outcome else {
        panic!("expected a negative outcome");
    };
    assert_eq!(negative.code, NegativeResponseCode::RequestOutOfRange);
    assert_eq!(negative.request_service_id, 0x22);
}

#[test]
fn dtc_report_decodes_sae_codes_with_failure_type() {
    let transaction = prepare_read_only_transaction(
        &resolved(READ_DTC_INFORMATION_CAPABILITY),
        ReadOnlyUdsIntent::read_dtc_information(family(), DtcStatusMask::Confirmed),
        &[],
    )
    .unwrap();
    assert_eq!(transaction.encoded_payload(), &[0x19, 0x02, 0x08]);
    assert!(transaction.readable_identifier().is_none());

    let mut source = simulate(
        &transaction,
        vec![
            0x59, 0x02, 0xFF, 0x03, 0x01, 0x00, 0x08, 0xC1, 0x23, 0x45, 0x2F,
        ],
    );
    let result = execute_simulator(&transaction, &mut source, "f10-sim-dtc", 10_000).unwrap();
    let UdsReadOutcome::DtcReport(report) = result.outcome else {
        panic!("expected a DTC report");
    };
    assert_eq!(report.status_availability_mask, 0xFF);
    let codes: Vec<_> = report
        .records
        .iter()
        .map(|record| (record.code_with_failure_type(), record.status))
        .collect();
    assert_eq!(
        codes,
        vec![
            ("P0301-00".to_string(), 0x08),
            ("U0123-45".to_string(), 0x2F)
        ]
    );
}

#[test]
fn replay_waits_through_response_pending_and_reassembles_multiframe() {
    let transaction = prepared_did_read(0x1945);
    let mut source = ReplaySource::from_json(REPLAY, PlaybackMode::Deterministic).unwrap();
    let result = execute_replay(&transaction, &mut source, 10_000).unwrap();
    assert_eq!(result.source, OfflineExecutionSource::Replay);
    assert_eq!(
        result.execution_fixture.name,
        "f10-synthetic-uds-did-multiframe-after-pending"
    );
    assert_eq!(
        result.outcome,
        UdsReadOutcome::DataByIdentifier {
            identifier: 0x1945,
            data: b"SYNTH-DATA".to_vec(),
        }
    );
    assert_eq!(result.completed_timestamp_us, 3000);
}

#[test]
fn a_silent_module_is_a_timeout_not_a_result() {
    let transaction = prepared_did_read(0x1945);
    let mut source = SimulatorSource::script_payload(
        PayloadSimulatorScenario {
            expected_request_payload: transaction.encoded_payload().to_vec(),
            behavior: PayloadSimulatorBehavior::Timeout,
        },
        transaction.encoded_payload(),
        transaction.backend_route(),
        transaction.expected_response_id(),
    )
    .unwrap();
    assert!(matches!(
        execute_simulator(&transaction, &mut source, "f10-sim-timeout", 10_000),
        Err(uds_execution::ExecutionError::Timeout)
    ));
}

#[test]
fn capability_identifiers_match_the_ingester() {
    assert_eq!(
        READ_DATA_BY_IDENTIFIER_CAPABILITY,
        sdd_ingest::UDS_READ_DATA_BY_IDENTIFIER_CAPABILITY
    );
    assert_eq!(
        READ_DTC_INFORMATION_CAPABILITY,
        sdd_ingest::UDS_READ_DTC_INFORMATION_CAPABILITY
    );
    assert_eq!(
        uds_execution::UDS_PROTOCOL_FAMILY,
        sdd_ingest::UDS_DIAGNOSTIC_PROTOCOL
    );
}
