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
/// A document that declares one acronym several times, each under its own
/// `<qualifier>`, which is how SDD says the same module sits at a different
/// address depending on the engine, the build year or the market.
const QUALIFIED: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_platform_qualified.xml");

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

/// A qualified module keeps every address the document gives it, and the
/// described car chooses between them.
///
/// Before 2026-09-09 the record id held no qualifier and the applicability
/// carried none, so two rows for one acronym collided and only the last
/// address in the document survived. On an X250 of 2010 that cost the
/// instrument cluster and the parking-brake module their address on the V6
/// and the 4.2 V8; on L405 and L494 it cost the ABS module its address on
/// cars built in 2014. The cross-check recorded in `docs/evidence/` found it.
#[test]
fn a_module_qualified_by_engine_build_or_market_keeps_every_address() {
    let adapter = PlatformAdapter::new(SourceRecord {
        id: SourceId::new("f9-qual").unwrap(),
        source_locator: "fixtures/knowledge/synthetic/f9_platform_qualified.xml".into(),
        content_fingerprint: Some(
            ContentFingerprint::sha256(sha256_bytes(QUALIFIED.as_bytes())).unwrap(),
        ),
        ..source(SourceType::Documented)
    })
    .unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, QUALIFIED).unwrap();

    fn address_for(
        store: &KnowledgeStore,
        family: &str,
        powertrain: Option<&str>,
        variant: Option<&str>,
        market: Option<&str>,
        other: &[(&str, &str)],
    ) -> Vec<(Option<u32>, Option<u32>)> {
        let context = knowledge::VehicleContext {
            vehicle_program: Some("SYNTHQ".into()),
            model_year: None,
            powertrain: powertrain.map(str::to_string),
            variant: variant.map(str::to_string),
            market: market.map(str::to_string),
            // The document qualifies every claim by its own marker, so a car
            // that does not state one is not this car.
            other: std::iter::once(("sdd_year_breakpoint".to_string(), "MY10".to_string()))
                .chain(
                    other
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.to_string())),
                )
                .collect(),
            ..knowledge::VehicleContext::default()
        };
        let result = store.query(
            // Indeterminate is included because this fixture has no
            // timeline, so the model year stays unknown; a row for the wrong
            // engine is still refused, which is what is being tested.
            &KnowledgeQuery::for_vehicle(context)
                .with_ecu_family(family)
                .include_indeterminate(true),
        );
        result
            .records
            .iter()
            .filter(|resolved| resolved.record.key == ClaimKey::DiagnosticAddressing)
            .map(|resolved| match &resolved.record.value {
                KnowledgeValue::DiagnosticAddressing {
                    request_id,
                    response_id,
                    ..
                } => (*request_id, *response_id),
                other => panic!("addressing expected, found {other:?}"),
            })
            .collect()
    }

    // The engine decides. Each car gets one address, and it is its own.
    assert_eq!(
        address_for(&store, "QUALMOD", Some("SMALLENGINE"), None, None, &[]),
        vec![(Some(0x7B2), Some(0x7BA))]
    );
    assert_eq!(
        address_for(
            &store,
            "QUALMOD",
            Some("BIGENGINE"),
            Some("5L"),
            Some("VAL_SYNTH_USA"),
            &[]
        ),
        vec![(Some(0x720), Some(0x728))]
    );
    // The bigger engine outside that market, or of another displacement,
    // matches nothing rather than borrowing the other engine's address.
    assert!(address_for(
        &store,
        "QUALMOD",
        Some("BIGENGINE"),
        Some("5L"),
        Some("VAL_SYNTH_ROW"),
        &[]
    )
    .is_empty());
    assert!(address_for(
        &store,
        "QUALMOD",
        Some("BIGENGINE"),
        Some("4_2L"),
        Some("VAL_SYNTH_USA"),
        &[]
    )
    .is_empty());

    // A test SDD has no dimension for keeps SDD's own name, and still selects.
    assert_eq!(
        address_for(
            &store,
            "BUILDMOD",
            None,
            None,
            None,
            &[("sdd_qual_cm_qual_synth_build", "VAL_SYNTH_2014")]
        ),
        vec![(Some(0x760), Some(0x768))]
    );
    assert_eq!(
        address_for(
            &store,
            "BUILDMOD",
            None,
            None,
            None,
            &[("sdd_qual_cm_qual_synth_build", "VAL_SYNTH_2015")]
        ),
        vec![(Some(0x7E6), Some(0x7EE))]
    );

    // A module with no test of its own keeps the id it always had: the fix
    // must not move the unqualified majority.
    let plain = store
        .get_record("f9-qual.module.PLAINMOD.addressing")
        .unwrap();
    assert_eq!(plain.applicability.powertrain, DimensionConstraint::Any);
    assert!(store
        .get_record("f9-qual.module.PLAINMOD.network")
        .is_some());
}

/// ADR-0027: a module's identification identifiers are recorded in the DID
/// catalogue's own shape, from the sets the platform names for it — and
/// only those. The NET set's non-identification members, a member read with
/// another service, a set the document never defines, and a module without a
/// diagnostic address all yield nothing.
#[test]
fn identification_identifiers_are_recorded_per_module_in_the_catalogue_shape() {
    let store = ingest(SourceType::Documented);
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));

    /// identifier, parameter name, encoding, the set's own qualifiers.
    type Row = (String, String, Option<String>, Vec<(String, String)>);
    fn identification(all: &knowledge::KnowledgeQueryResult, family: &str) -> Vec<Row> {
        let mut rows: Vec<_> = all
            .records
            .iter()
            .filter(|resolved| {
                resolved.record.entity.kind == EntityKind::IdentifierParameter
                    && resolved.record.applicability.ecu_family
                        == DimensionConstraint::one_of([family.to_string()]).unwrap()
            })
            .map(|resolved| {
                let (identifier, encoding) = match &resolved.record.value {
                    KnowledgeValue::IdentifierDefinition {
                        identifier,
                        encoding,
                        ..
                    } => (identifier.clone(), encoding.clone()),
                    other => panic!("unexpected value {other:?}"),
                };
                let parameter = match &resolved.record.key {
                    ClaimKey::ParameterDefinition { parameter } => parameter.clone(),
                    other => panic!("unexpected key {other:?}"),
                };
                let quals: Vec<(String, String)> = resolved
                    .record
                    .applicability
                    .other
                    .iter()
                    .filter(|(key, _)| key.starts_with("sdd_qual_"))
                    .map(|(key, value)| {
                        let value = match value {
                            DimensionConstraint::OneOf { values } => values.join("|"),
                            other => format!("{other:?}"),
                        };
                        (key.clone(), value)
                    })
                    .collect();
                (identifier, parameter, encoding, quals)
            })
            .collect();
        rows.sort();
        rows
    }

    let synth = identification(&all, "SYNTHMOD");
    let identifiers: Vec<&str> = synth.iter().map(|row| row.0.as_str()).collect();
    assert_eq!(
        identifiers,
        vec!["0xF111", "0xF124", "0xF125", "0xF188", "0xF188", "0xF188", "0xF18C", "0xF190", "0xF1A0"],
        "NET gives its F1xx members, SWDL its two qualified lists, PDI its two; DD01 and the 0x09 member stay out: {synth:?}"
    );
    for (identifier, parameter, encoding, _) in &synth {
        assert_eq!(
            encoding.as_deref(),
            Some("text=ascii"),
            "{identifier} is a text"
        );
        assert!(!parameter.is_empty());
    }
    // SDD's human text is the parameter's name; the attribute name only
    // when there is no text.
    assert!(synth
        .iter()
        .any(|row| row.0 == "0xF111" && row.1 == "ECU Core Assembly Number"));
    assert!(synth
        .iter()
        .any(|row| row.0 == "0xF1A0" && row.1 == "synth_no_text"));
    // The software list a qualifier chooses is narrowed by that qualifier;
    // the unqualified NET row of the same identifier stays unqualified.
    let f188: Vec<_> = synth.iter().filter(|row| row.0 == "0xF188").collect();
    assert_eq!(f188.len(), 3);
    assert!(
        f188.iter().any(|row| row.3.is_empty()),
        "the NET row carries no set qualifier"
    );
    assert!(f188.iter().any(|row| row.3
        == vec![(
            "sdd_qual_cm_qual_synth_hw".to_string(),
            "VAL_STD".to_string()
        )]));
    assert!(f188.iter().any(|row| row.3
        == vec![(
            "sdd_qual_cm_qual_synth_hw".to_string(),
            "VAL_PLUS".to_string()
        )]));

    // OTHERMOD names the same NET set and an undefined PDI set: the NET
    // identification only.
    let other: Vec<String> = identification(&all, "OTHERMOD")
        .into_iter()
        .map(|row| row.0)
        .collect();
    assert_eq!(other, vec!["0xF111", "0xF188", "0xF190", "0xF1A0"]);

    // A module with no diagnostic address is not seen at all, sets or not.
    assert!(identification(&all, "PROGONLY").is_empty());
    // Every identification record cites this document's own evidence and
    // is documented, like the addressing beside it.
    for resolved in all
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::IdentifierParameter)
    {
        assert_eq!(
            resolved.record.validation_state,
            ValidationState::SourceBacked
        );
        assert!(resolved
            .record
            .evidence_ids
            .iter()
            .all(|id| id.0.starts_with("f9-plat.ev.f9-plat.module.")));
    }
}
