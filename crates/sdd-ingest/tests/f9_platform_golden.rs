//! F9 golden tests for SDD platform documents.
//!
//! The load-bearing test is that programming-session addressing never enters the
//! knowledge base. A route that exists there is a route something can be built
//! on, so software-download addressing is not ingested at all rather than
//! recorded and trusted to be ignored.

use knowledge::{
    sha256_bytes, CanIdFormat, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind,
    EvidenceClass, KnowledgeQuery, KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId,
    SourceRecord, SourceType, ValidationState, YearConstraint,
};
use sdd_ingest::{ModelYearTimeline, PlatformAdapter, NETWORK_CLAIM};

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");

fn source(source_type: SourceType) -> SourceRecord {
    SourceRecord {
        id: SourceId::new("f9-plat").unwrap(),
        title: "F9 platform fixture".into(),
        source_type,
        origin: "F9 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_platform.xml".into(),
        content_fingerprint: source_type
            .is_real_evidence()
            .then(|| ContentFingerprint::sha256(sha256_bytes(FIXTURE.as_bytes())).unwrap()),
        acquired_on: None,
        declared_vehicle_programs: vec![],
        provenance: "Synthetic fixture reproducing the SDD element shape only".into(),
        redistribution_status: RedistributionStatus::Permitted,
        notes: None,
    }
}

fn ingest(source_type: SourceType) -> KnowledgeStore {
    let adapter = PlatformAdapter::new(source(source_type)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

fn reject(input: &str) -> String {
    let adapter = PlatformAdapter::new(source(SourceType::Documented)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store
        .ingest(&adapter, input)
        .expect_err("input must be rejected");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");
    error.to_string()
}

#[test]
fn programming_session_addressing_never_enters_the_knowledge_base() {
    let store = ingest(SourceType::Documented);
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));

    // The fixture declares 0x111 and 0x222 for the programming session and
    // 0x333/0x33B for a module that has no diagnostic pair at all. None of them
    // may appear anywhere.
    for entry in &all.records {
        if let KnowledgeValue::DiagnosticAddressing {
            request_id,
            response_id,
            ..
        } = &entry.record.value
        {
            for forbidden in [0x111, 0x222, 0x333, 0x33B] {
                assert_ne!(*request_id, Some(forbidden), "{}", entry.record.id);
                assert_ne!(*response_id, Some(forbidden), "{}", entry.record.id);
            }
        }
    }

    // A module with only programming addresses yields no claim at all.
    assert!(store
        .get_record("f9-plat.module.PROGONLY.addressing")
        .is_none());
}

#[test]
fn a_module_carries_its_diagnostic_route_and_its_bus() {
    let store = ingest(SourceType::Documented);

    let record = store
        .get_record("f9-plat.module.SYNTHMOD.addressing")
        .unwrap();
    assert_eq!(record.entity.kind, EntityKind::EcuFamily);
    assert_eq!(record.entity.id, "SYNTHMOD");
    let KnowledgeValue::DiagnosticAddressing {
        request_id,
        response_id,
        can_id_format,
        addressing_mode,
        functional_request_id,
    } = &record.value
    else {
        panic!("expected diagnostic addressing");
    };
    assert_eq!(*request_id, Some(0x7E0));
    assert_eq!(*response_id, Some(0x7E8));
    // Width and mode come from the bus the module sits on, not from the module.
    assert_eq!(*can_id_format, Some(CanIdFormat::Standard11Bit));
    assert_eq!(addressing_mode.as_deref(), Some("normal"));
    // The platform states no functional address, so none is invented.
    assert_eq!(*functional_request_id, None);

    let network = store.get_record("f9-plat.module.SYNTHMOD.network").unwrap();
    assert_eq!(
        network.key,
        ClaimKey::Custom {
            name: NETWORK_CLAIM.into()
        }
    );
    assert_eq!(
        network.value,
        KnowledgeValue::Text {
            value: "CAN_HS".into()
        }
    );

    // A module on the slower bus inherits that bus's width, not the first one's.
    let other = store
        .get_record("f9-plat.module.OTHERMOD.addressing")
        .unwrap();
    let KnowledgeValue::DiagnosticAddressing { can_id_format, .. } = &other.value else {
        panic!("expected diagnostic addressing");
    };
    assert_eq!(*can_id_format, Some(CanIdFormat::Extended29Bit));
}

#[test]
fn buses_are_recorded_with_their_rate_but_never_with_assumed_pins() {
    let store = ingest(SourceType::Documented);

    let fast = store.get_record("f9-plat.network.CAN_HS").unwrap();
    assert_eq!(fast.entity.kind, EntityKind::NetworkRoute);
    assert_eq!(fast.entity.id, "SYNTHA-CAN_HS");
    let KnowledgeValue::NetworkRoute {
        logical_name,
        bitrate_bps,
        connector,
        pins,
    } = &fast.value
    else {
        panic!("expected a network route");
    };
    assert_eq!(logical_name, "CAN_HS");
    // SDD states kbit/s; the record holds bit/s.
    assert_eq!(*bitrate_bps, Some(500_000));
    // The platform never states J1962 pins, so none are fabricated.
    assert!(pins.is_empty());
    assert_eq!(*connector, None);

    let slow = store.get_record("f9-plat.network.CAN_MS").unwrap();
    let KnowledgeValue::NetworkRoute { bitrate_bps, .. } = &slow.value else {
        panic!("expected a network route");
    };
    assert_eq!(*bitrate_bps, Some(125_000));
}

#[test]
fn the_program_and_marker_come_from_the_document_not_the_file_name() {
    let store = ingest(SourceType::Documented);
    let record = store
        .get_record("f9-plat.module.SYNTHMOD.addressing")
        .unwrap();

    assert_eq!(
        record.applicability.vehicle_program,
        DimensionConstraint::one_of(["SYNTHA".to_string()]).unwrap()
    );
    // Without a timeline the marker is kept and no year is claimed.
    assert_eq!(record.applicability.model_year, YearConstraint::Unknown);
    assert_eq!(
        record
            .applicability
            .other
            .get(sdd_ingest::YEAR_BREAKPOINT_DIMENSION),
        Some(&DimensionConstraint::one_of(["MY10".to_string()]).unwrap())
    );

    // A document naming more than one program would make every claim in it
    // ambiguous, so it is refused rather than resolved arbitrarily.
    let ambiguous = FIXTURE.replace(
        "<qualifier model=\"SYNTHA\" year=\"MY10\"/>",
        "<qualifier model=\"SYNTHA\" year=\"MY10\"/><qualifier model=\"SYNTHB\" year=\"MY10\"/>",
    );
    assert!(reject(&ambiguous).contains("exactly one program"));
}

#[test]
fn opting_in_derives_the_year_range_for_every_claim() {
    let mut timeline = ModelYearTimeline::new();
    for marker in ["MY08", "MY10", "MY12"] {
        timeline.observe("SYNTHA", marker).unwrap();
    }
    let adapter = PlatformAdapter::new(source(SourceType::Documented))
        .unwrap()
        .with_timeline(timeline);
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();

    assert_eq!(
        store
            .get_record("f9-plat.module.SYNTHMOD.addressing")
            .unwrap()
            .applicability
            .model_year,
        YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: Some(2011)
        }
    );
}

#[test]
fn malformed_documents_are_rejected_and_classification_follows_the_source() {
    assert!(reject("<other><platform/></other>").contains("expected a <platform> root"));

    let bad_width = FIXTURE.replace("<identifier>11</identifier>", "<identifier>7</identifier>");
    assert!(reject(&bad_width).contains("unsupported identifier width"));

    let bad_address = FIXTURE.replace(">0x7E0<", ">not-a-number<");
    assert!(reject(&bad_address).contains("not a usable identifier"));

    let synthetic = ingest(SourceType::Synthetic);
    assert_eq!(
        synthetic
            .get_record("f9-plat.module.SYNTHMOD.addressing")
            .unwrap()
            .validation_state,
        ValidationState::Unverified
    );
    assert_eq!(
        synthetic
            .trace_back("f9-plat.module.SYNTHMOD.addressing")
            .unwrap()[0]
            .evidence
            .evidence_class,
        Some(EvidenceClass::SyntheticTest)
    );
}

#[test]
fn a_bus_records_the_application_protocol_it_declares() {
    let store = ingest(SourceType::Documented);

    // F6 needs the application protocol, and the platform is the only place in
    // the corpus that states it.
    let protocol = store.get_record("f9-plat.network.CAN_HS.protocol").unwrap();
    assert_eq!(
        protocol.value,
        KnowledgeValue::ProtocolFamily {
            name: "ISO14229".into()
        }
    );
    assert_eq!(protocol.key, ClaimKey::UsesProtocolFamily);

    // A bus that declares none gets no claim rather than an assumed default.
    assert!(store
        .get_record("f9-plat.network.CAN_MS.protocol")
        .is_none());
}

#[test]
fn connector_pins_are_recorded_as_listed_and_not_assigned_to_buses() {
    let store = ingest(SourceType::Documented);

    let connector = store.get_record("f9-plat.connector").unwrap();
    let KnowledgeValue::NetworkRoute {
        logical_name,
        connector: name,
        pins,
        bitrate_bps,
    } = &connector.value
    else {
        panic!("expected a network route");
    };
    assert_eq!(logical_name, "J1962");
    assert_eq!(name.as_deref(), Some("J1962"));
    assert_eq!(pins, &vec![5, 6, 14]);
    // A connector has no bit rate of its own.
    assert_eq!(*bitrate_bps, None);

    // The pin names imply which bus each belongs to, but implying is not
    // stating, so the bus records still carry no pins.
    let bus = store.get_record("f9-plat.network.CAN_HS").unwrap();
    let KnowledgeValue::NetworkRoute { pins, .. } = &bus.value else {
        panic!("expected a network route");
    };
    assert!(pins.is_empty());
}

#[test]
fn a_module_carries_its_bus_and_protocol_as_its_own_claims() {
    // ADR-0013: the resolver relates a module only to claims that mention it,
    // so the bus facts SDD states once per network are recorded per module.
    let store = ingest(SourceType::Documented);

    let bus = store.get_record("f9-plat.module.SYNTHMOD.bus").unwrap();
    assert_eq!(bus.entity.kind, EntityKind::EcuFamily);
    assert_eq!(bus.entity.id, "SYNTHMOD");
    assert_eq!(bus.key, ClaimKey::NetworkRoute);
    assert_eq!(
        bus.value,
        KnowledgeValue::NetworkRoute {
            logical_name: "CAN_HS".into(),
            connector: None,
            pins: vec![],
            bitrate_bps: None,
        }
    );
    // The evidence cites both the module's network and the network itself.
    let trace = &store.trace_back("f9-plat.module.SYNTHMOD.bus").unwrap()[0];
    assert!(trace
        .evidence
        .locator
        .description
        .contains("@acronym='SYNTHMOD'"));
    assert!(trace
        .evidence
        .locator
        .description
        .contains("@net_id='CAN_HS'"));

    let protocol = store
        .get_record("f9-plat.module.SYNTHMOD.protocol")
        .unwrap();
    assert_eq!(protocol.entity.id, "SYNTHMOD");
    assert_eq!(
        protocol.value,
        KnowledgeValue::ProtocolFamily {
            name: "ISO14229".into()
        }
    );

    // A module on a bus that declares no protocol gets its bus but no
    // protocol claim, rather than an assumed one. The rate is the route's.
    let other = store.get_record("f9-plat.module.OTHERMOD.bus").unwrap();
    let KnowledgeValue::NetworkRoute { bitrate_bps, .. } = &other.value else {
        panic!("expected a network route");
    };
    assert_eq!(*bitrate_bps, None);
    assert!(store
        .get_record("f9-plat.module.OTHERMOD.protocol")
        .is_none());
}

#[test]
fn uds_read_capabilities_follow_the_declared_diagnostic_protocol() {
    let store = ingest(SourceType::Documented);

    for (suffix, name) in [
        (
            "read_data_by_identifier",
            sdd_ingest::UDS_READ_DATA_BY_IDENTIFIER_CAPABILITY,
        ),
        (
            "read_dtc_information",
            sdd_ingest::UDS_READ_DTC_INFORMATION_CAPABILITY,
        ),
    ] {
        let record = store
            .get_record(&format!("f9-plat.module.SYNTHMOD.capability.{suffix}"))
            .unwrap();
        assert_eq!(record.entity.id, "SYNTHMOD");
        assert_eq!(
            record.key,
            ClaimKey::SupportsCapability {
                capability: name.into()
            }
        );
        assert_eq!(
            record.value,
            KnowledgeValue::Capability {
                name: name.into(),
                supported: true,
                safety_class: Some(knowledge::DiagnosticSafetyClass::ReadOnly),
            }
        );
        // No protocol declared for OTHERMOD's bus, so no capability is claimed.
        assert!(store
            .get_record(&format!("f9-plat.module.OTHERMOD.capability.{suffix}"))
            .is_none());
    }
}

#[test]
fn a_physically_addressed_module_is_recorded_without_can_identifiers() {
    // Older platforms and gatewayed sub-networks give a node address and an
    // addressing scheme instead of CAN identifiers. The module is recorded so
    // it is seen; no identifier is derived, so it cannot be routed yet.
    let store = ingest(SourceType::Documented);

    assert!(store
        .get_record("f9-plat.module.LEGACYMOD.addressing")
        .is_none());
    let physical = store
        .get_record("f9-plat.module.LEGACYMOD.physical_address")
        .unwrap();
    assert_eq!(physical.entity.kind, EntityKind::EcuFamily);
    assert_eq!(physical.entity.id, "LEGACYMOD");
    assert_eq!(
        physical.key,
        ClaimKey::Custom {
            name: sdd_ingest::PHYSICAL_ADDRESS_CLAIM.into()
        }
    );
    assert_eq!(
        physical.value,
        KnowledgeValue::Text {
            value: "0x10".into()
        }
    );
    // Its bus is still stated, so a survey can place it and say why it is out
    // of reach.
    assert!(store
        .get_record("f9-plat.module.LEGACYMOD.network")
        .is_some());
    assert!(store.get_record("f9-plat.module.LEGACYMOD.bus").is_some());
}

const ISO15765_MANIFEST: &str =
    include_str!("../../../fixtures/knowledge/documented/iso15765_normal_fixed_addressing.json");
const TESTER_ADDRESS_MANIFEST: &str = include_str!(
    "../../../fixtures/knowledge/research/normal_fixed_tester_address_hypothesis.json"
);

#[test]
fn normal_fixed_identifiers_are_derived_only_on_request_and_stay_unverified() {
    use knowledge::JsonManifestAdapter;
    use sdd_ingest::{ISO15765_NORMAL_FIXED_LAYOUT_EVIDENCE, NORMAL_FIXED_TESTER_ADDRESS_EVIDENCE};

    // Without the request: the physical address text and no identifiers.
    let plain = ingest(SourceType::Documented);
    assert!(plain
        .get_record("f9-plat.module.FIXEDMOD.physical_address")
        .is_some());
    assert!(plain
        .get_record("f9-plat.module.FIXEDMOD.addressing")
        .is_none());

    // With the request, the built-in manifests must be there to cite.
    let adapter = PlatformAdapter::new(source(SourceType::Documented))
        .unwrap()
        .with_derived_normal_fixed_identifiers();
    let mut without = KnowledgeStore::new();
    assert!(without.ingest(&adapter, FIXTURE).is_err());

    let mut store = KnowledgeStore::new();
    for text in [ISO15765_MANIFEST, TESTER_ADDRESS_MANIFEST] {
        store.ingest(&JsonManifestAdapter, text).unwrap();
    }
    store.ingest(&adapter, FIXTURE).unwrap();
    let derived = store
        .get_record("f9-plat.module.FIXEDMOD.addressing")
        .expect("derived addressing");
    assert_eq!(
        derived.value,
        KnowledgeValue::DiagnosticAddressing {
            request_id: Some(0x18DA_60F1),
            response_id: Some(0x18DA_F160),
            functional_request_id: None,
            can_id_format: Some(CanIdFormat::Extended29Bit),
            addressing_mode: Some("normal_fixed".into()),
        }
    );
    assert_eq!(derived.validation_state, ValidationState::Unverified);
    let cited: Vec<&str> = derived
        .evidence_ids
        .iter()
        .map(|id| id.0.as_str())
        .collect();
    assert!(cited.contains(&"f9-plat.ev.f9-plat.module.FIXEDMOD.physical_address"));
    assert!(cited.contains(&ISO15765_NORMAL_FIXED_LAYOUT_EVIDENCE));
    assert!(cited.contains(&NORMAL_FIXED_TESTER_ADDRESS_EVIDENCE));

    // A physically addressed module on a normal 11-bit bus is not derived.
    assert!(store
        .get_record("f9-plat.module.LEGACYMOD.addressing")
        .is_none());
    // Records that cite only the platform's own evidence keep the source state.
    assert_eq!(
        store
            .get_record("f9-plat.module.FIXEDMOD.physical_address")
            .unwrap()
            .validation_state,
        ValidationState::SourceBacked
    );
}
