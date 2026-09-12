//! F10 golden tests for the composition layer: manifests in, survey out.
//!
//! The library is built the way the application builds it — built-in
//! documented manifests plus a directory of exported ones — and the survey is
//! checked for what it shows and, as importantly, for what it refuses to hide.

use app_contracts::{
    LibraryState, ModuleApplicability, ModuleReadState, PassportReading, RouteStatus,
    VehicleContextInput,
};
use diagnostic_session::{
    catalogue_comparisons, survey_vehicle, vehicle_context, KnowledgeLibrary, CATALOGUE_AGREES,
    CATALOGUE_DIFFERS, CATALOGUE_NOT_NAMED, CATALOGUE_NO_ASSEMBLY, SDD_MODEL_YEAR_DIMENSION,
    SDD_YEAR_BREAKPOINT_DIMENSION,
};
use knowledge::{
    sha256_bytes, ContentFingerprint, IngestionAdapter, RedistributionStatus, SourceId,
    SourceRecord, SourceType,
};
use sdd_ingest::{
    ConverterCatalogue, DidFormattingAdapter, IvsLineageAdapter, ModelYearTimeline,
    ModuleTextAdapter, OdstInfoAdapter, PlatformAdapter, VinDecodeAdapter,
};
use std::collections::BTreeMap;

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const DIDS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_did_formatting.xml");
const CONVERTER: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_converter.xml");
const CONVERTER_KM: &str =
    include_str!("../../../fixtures/knowledge/synthetic/f9_converter_km.xml");
const MODULE_TEXT: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_module_text.xml");
const VIN_DECODE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_vin_decode.xml");
const ODST: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_odst_info.xml");
const IVS: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_ivs_lineage.xml");

fn synthetic_source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F10 session fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "F10 golden test".into(),
        source_locator: format!("fixtures/knowledge/synthetic/{id}.xml"),
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

/// The manifests an export of the two fixtures would produce, as JSON text —
/// one as a single manifest, one as a bundle, since the loader accepts both.
fn exported_manifests() -> Vec<(String, String)> {
    let platform = PlatformAdapter::new(synthetic_source("f10-session-plat", PLATFORM))
        .unwrap()
        .with_timeline(timeline());
    let mut converters = ConverterCatalogue::new();
    converters.insert_from_xml(CONVERTER).unwrap();
    converters.insert_from_xml(CONVERTER_KM).unwrap();
    let dids = DidFormattingAdapter::new(synthetic_source("f10-session-did", DIDS), converters)
        .unwrap()
        .with_timeline(timeline());

    let platform_batch = platform.parse(PLATFORM).unwrap();
    let did_batch = dids.parse(DIDS).unwrap();
    let text_batch = ModuleTextAdapter::new(synthetic_source("f10-session-text", MODULE_TEXT))
        .unwrap()
        .parse(MODULE_TEXT)
        .unwrap();
    let vin_batch = VinDecodeAdapter::new(synthetic_source("f10-session-vin", VIN_DECODE))
        .unwrap()
        .parse(VIN_DECODE)
        .unwrap();
    let odst_batch = OdstInfoAdapter::new(synthetic_source("f10-session-odst", ODST))
        .unwrap()
        .parse(ODST)
        .unwrap();
    let ivs_batch = IvsLineageAdapter::new(synthetic_source("f10-session-ivs", IVS))
        .unwrap()
        .parse(IVS)
        .unwrap();
    vec![
        (
            "platform.json".to_string(),
            serde_json::to_string(&platform_batch).unwrap(),
        ),
        (
            "bundle.json".to_string(),
            serde_json::to_string(&vec![
                did_batch, text_batch, vin_batch, odst_batch, ivs_batch,
            ])
            .unwrap(),
        ),
    ]
}

fn library() -> KnowledgeLibrary {
    let manifests = exported_manifests();
    KnowledgeLibrary::from_manifests(
        manifests
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    )
}

fn vehicle() -> VehicleContextInput {
    VehicleContextInput {
        vehicle_program: "SYNTHA".into(),
        model_year: Some(2010),
        powertrain: Some("SYNTHENGINE".into()),
        variant: None,
        market: None,
        year_breakpoint: Some("MY10".into()),
    }
}

#[test]
fn built_in_library_holds_the_documented_and_research_manifests() {
    let library = KnowledgeLibrary::built_in();
    let snapshot = library.snapshot();
    assert_eq!(snapshot.state, LibraryState::NotLoaded);
    assert_eq!(snapshot.directory, None);
    assert_eq!(snapshot.sources, 6);
    assert_eq!(snapshot.manifests_failed, 0);
    assert!(snapshot.message.contains("Built-in data only"));

    // With no vehicle data at all, a survey names the gap rather than a car.
    let survey = library.survey(&vehicle());
    assert!(survey.modules.is_empty());
    assert!(survey.message.contains("No modules are known"));
}

#[test]
fn exported_manifests_and_bundles_load_and_are_counted_honestly() {
    let library = library();
    let snapshot = library.snapshot();
    assert_eq!(snapshot.state, LibraryState::Loaded);
    // Six built-in plus one manifest plus one bundle of five.
    assert_eq!(snapshot.manifests_loaded, 12);
    assert_eq!(snapshot.manifests_failed, 0);
    assert_eq!(snapshot.sources, 12);
    assert!(snapshot.records > 20);
    assert!(snapshot.message.starts_with("Loaded 6 manifests"));
}

#[test]
fn a_broken_manifest_is_reported_and_the_rest_still_load() {
    let mut manifests = exported_manifests();
    manifests.push(("broken.json".to_string(), "{ not json".to_string()));
    manifests.push((
        "wrong-shape.json".to_string(),
        r#"{"schema_version": 1, "unexpected": true}"#.to_string(),
    ));
    let library = KnowledgeLibrary::from_manifests(
        manifests
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    );
    let snapshot = library.snapshot();
    assert_eq!(snapshot.state, LibraryState::PartiallyLoaded);
    assert_eq!(snapshot.manifests_failed, 2);
    let files: Vec<_> = snapshot
        .failures
        .iter()
        .map(|failure| failure.file.as_str())
        .collect();
    assert_eq!(files, vec!["broken.json", "wrong-shape.json"]);
    assert!(snapshot.failures[0].message.contains("not valid JSON"));
    // The good data is still there.
    assert_eq!(snapshot.sources, 12);
}

#[test]
fn the_survey_shows_reachable_and_unreachable_modules_with_reasons() {
    let survey = library().survey(&vehicle());
    // SYNTHMOD, OTHERMOD, LEGACYMOD, FIXEDMOD, and the two K-line modules
    // DS2MOD and STARMOD (ADR-0029); only SYNTHMOD has a confirmed route,
    // DS2MOD sits on a hypothesised one.
    assert_eq!(survey.modules.len(), 6);
    assert_eq!(survey.reachable, 1);
    assert_eq!(survey.hypothesis, 1);
    assert_eq!(survey.unreachable, 4);
    assert!(survey.message.starts_with(
        "6 modules known: 1 reachable over the adapter, 1 on a hypothesised route, 4 not"
    ));

    // Sorted by mnemonic: DS2MOD, FIXEDMOD, LEGACYMOD, OTHERMOD, STARMOD, SYNTHMOD.
    let reachable = &survey.modules[5];
    assert_eq!(reachable.ecu_family, "SYNTHMOD");
    assert_eq!(reachable.applicability, ModuleApplicability::Applicable);
    assert_eq!(reachable.identifier_read.status, RouteStatus::Reachable);
    assert_eq!(reachable.dtc_read.status, RouteStatus::Reachable);
    assert!(reachable.identifier_read.reasons.is_empty());
    assert_eq!(reachable.logical_network.as_deref(), Some("CAN_HS"));
    assert_eq!(reachable.backend_route.as_deref(), Some("hs-can"));
    assert_eq!(reachable.pins.as_deref(), Some("6/14"));
    assert_eq!(reachable.bitrate_bps, Some(500_000));
    assert_eq!(reachable.protocol.as_deref(), Some("ISO14229"));
    assert_eq!(reachable.request_id.as_deref(), Some("0x7E0"));
    assert_eq!(reachable.response_id.as_deref(), Some("0x7E8"));
    let identifiers: Vec<_> = reachable
        .readable_identifiers
        .iter()
        .map(|entry| entry.identifier.as_str())
        .collect();
    // 0xDD01 is the distance the mileage survey asks for (ADR-0024);
    // the fixture carries it so the survey has something to read. The
    // 0x40xx rows are the battery monitor the platform names for this
    // module (ADR-0030). The 0xF1xx rows are the module's identification
    // (ADR-0027), from the sets the platform names for it; the two software
    // lists a qualifier chooses are not this car's, which states no such
    // qualifier, so they are absent.
    assert_eq!(
        identifiers,
        vec![
            "0x0347", "0x1945", "0x4020", "0x4025", "0x4028", "0x4090", "0xDD01", "0xF111",
            "0xF188", "0xF18C", "0xF190", "0xF1A0"
        ]
    );

    // DS2MOD speaks DS2 on the K-line bus the document pins to 7; the
    // document's pin and baud reach the plan, the adapter route is the
    // hypothesis, and the read-only capabilities are DS2's own.
    let ds2 = &survey.modules[0];
    assert_eq!(ds2.ecu_family, "DS2MOD");
    assert_eq!(
        ds2.identifier_read.status,
        RouteStatus::Hypothesis,
        "{ds2:?}"
    );
    assert_eq!(ds2.logical_network.as_deref(), Some("DS2_PIN7"));
    assert_eq!(ds2.backend_route.as_deref(), Some("k-line-7"));
    assert_eq!(ds2.pins.as_deref(), Some("7"));
    assert_eq!(ds2.bitrate_bps, Some(9600));
    assert_eq!(ds2.protocol.as_deref(), Some("DS2"));
    assert_eq!(ds2.request_id.as_deref(), Some("0x72"));
    assert_eq!(ds2.response_id.as_deref(), Some("0x72"));
    assert_eq!(ds2.route_validation, "UNVERIFIED");
    assert!(
        ds2.readable_identifiers.is_empty(),
        "no catalogue parameter names a DS2 identifier"
    );

    // STARMOD speaks a protocol this product only names, on a bus whose pin
    // the document does not state: the survey says the capability is missing.
    let star = &survey.modules[4];
    assert_eq!(star.ecu_family, "STARMOD");
    assert_eq!(
        star.identifier_read.status,
        RouteStatus::Indeterminate,
        "{star:?}"
    );
    assert_eq!(star.protocol.as_deref(), Some("KW2000STAR"));
    assert!(
        star.identifier_read
            .reasons
            .iter()
            .any(|reason| reason.starts_with("read-only capability:")),
        "{:?}",
        star.identifier_read.reasons
    );

    assert!(
        star.identifier_read.reasons.iter().any(|reason| reason
            == "protocol KW2000STAR: named in the data and not spoken by this product"),
        "{:?}",
        star.identifier_read.reasons
    );
    // The names the ingest records and the names the protocol crates speak
    // are one and the same, or the survey would look for the wrong thing.
    assert_eq!(ds2::PROTOCOL_FAMILY, sdd_ingest::DS2_DIAGNOSTIC_PROTOCOL);
    assert_eq!(
        ds2::ECU_IDENTIFICATION_CAPABILITY,
        sdd_ingest::DS2_ECU_IDENTIFICATION_CAPABILITY
    );
    assert_eq!(
        ds2::FAULT_MEMORY_CAPABILITY,
        sdd_ingest::DS2_FAULT_MEMORY_CAPABILITY
    );
    assert_eq!(
        kwp2000::PROTOCOL_FAMILY,
        sdd_ingest::KWP2000_DIAGNOSTIC_PROTOCOL
    );
    assert_eq!(
        kwp2000::READ_ECU_IDENTIFICATION_CAPABILITY,
        sdd_ingest::KWP2000_READ_ECU_IDENTIFICATION_CAPABILITY
    );
    assert_eq!(
        kwp2000::READ_DTC_BY_STATUS_CAPABILITY,
        sdd_ingest::KWP2000_READ_DTC_BY_STATUS_CAPABILITY
    );

    // OTHERMOD sits on a bound bus that declares no diagnostic protocol, so
    // the survey says exactly that instead of dropping the row.
    let unreachable = &survey.modules[3];
    assert_eq!(unreachable.ecu_family, "OTHERMOD");
    assert_eq!(
        unreachable.identifier_read.status,
        RouteStatus::Indeterminate
    );
    assert_eq!(unreachable.logical_network.as_deref(), Some("CAN_MS"));
    assert_eq!(unreachable.backend_route.as_deref(), Some("ms-can"));
    assert_eq!(unreachable.request_id.as_deref(), Some("0x760"));
    assert!(unreachable
        .identifier_read
        .reasons
        .iter()
        .any(|reason| reason.starts_with("diagnostic protocol:")));
    assert!(unreachable
        .identifier_read
        .reasons
        .iter()
        .any(|reason| reason.starts_with("read-only capability:")));
    // The data still declares the module's identification (ADR-0027) and the
    // battery monitor it shares with SYNTHMOD through the same set
    // (ADR-0030): the route, not the list, is what says it cannot be read
    // today.
    let declared: Vec<&str> = unreachable
        .readable_identifiers
        .iter()
        .map(|entry| entry.identifier.as_str())
        .collect();
    assert_eq!(
        declared,
        vec!["0x4020", "0x4025", "0x4028", "0xF111", "0xF188", "0xF190", "0xF1A0"]
    );

    // LEGACYMOD has a physical address and no CAN identifiers: seen, placed on
    // its bus, and unreachable for exactly that reason. (FIXEDMOD, first in
    // the order, has a physical address on a normal_fixed bus and is derived
    // only when the ingester is asked to; this library did not ask.)
    let legacy = &survey.modules[2];
    assert_eq!(survey.modules[1].ecu_family, "FIXEDMOD");
    assert_eq!(survey.modules[1].request_id, None);
    assert_eq!(legacy.ecu_family, "LEGACYMOD");
    assert_eq!(legacy.logical_network.as_deref(), Some("CAN_HS"));
    assert_eq!(legacy.request_id, None);
    assert_eq!(legacy.identifier_read.status, RouteStatus::Indeterminate);
    assert!(legacy
        .identifier_read
        .reasons
        .iter()
        .any(|reason| reason.starts_with("request identifier:")));
}

#[test]
fn an_incomplete_vehicle_description_is_named_not_guessed() {
    let library = library();

    let blank = library.survey(&VehicleContextInput::default());
    assert!(blank.modules.is_empty());
    assert!(blank.message.contains("State the vehicle programme"));

    // Without the breakpoint marker the SDD-qualified data cannot apply, so
    // every module is listed as lacking context rather than as reachable.
    let mut partial = vehicle();
    partial.year_breakpoint = None;
    let survey = library.survey(&partial);
    assert_eq!(survey.reachable, 0);
    assert!(survey
        .modules
        .iter()
        .all(|module| module.applicability == ModuleApplicability::InsufficientContext));
    assert!(survey.modules[0].identifier_read.reasons[0].contains("lacks a detail"));
}

#[test]
fn vehicle_context_dimension_names_match_the_ingester() {
    assert_eq!(
        SDD_YEAR_BREAKPOINT_DIMENSION,
        sdd_ingest::YEAR_BREAKPOINT_DIMENSION
    );
    // And a store-level survey sees the same thing the library does.
    let library = library();
    assert_eq!(
        survey_vehicle(library.store(), &vehicle(), &|mnemonic| library
            .module_names(mnemonic)),
        library.survey(&vehicle())
    );
}

/// One row of a passport run, as the shell builds it.
fn reading(family: &str, identifier: &str, value: Option<&str>) -> PassportReading {
    PassportReading {
        ecu_family: family.into(),
        identifier: identifier.into(),
        parameter: identifier.into(),
        state: ModuleReadState::Succeeded,
        value: value.map(str::to_string),
        route_id: "hs-can".into(),
        route_validation: "SOURCE_BACKED".into(),
        raw_response_hex: None,
        negative_response: None,
        note: None,
        reason: None,
        catalogue: None,
    }
}

#[test]
fn a_part_number_is_set_against_jlrs_catalogue_and_never_judged() {
    // The claim literal is the ingest's.
    let library = library();
    let context = vehicle_context(&vehicle());

    let known = library.catalogue_assemblies(&context, "SYNTHMOD");
    assert_eq!(
        known
            .iter()
            .map(|entry| entry.assembly.as_str())
            .collect::<Vec<_>>(),
        ["8X23-18C808-CE", "8X23-18C808-DA"]
    );
    let carried = &known[0];
    assert_eq!(carried.as_delivered.as_deref(), Some("F113"));
    assert_eq!(carried.dated.as_deref(), Some("Oct-17-2022 22:16:38"));
    assert_eq!(
        carried.parts,
        vec![
            (
                "F191".to_string(),
                "6H52-14C524-CB".to_string(),
                "Hardware".to_string()
            ),
            (
                "F188".to_string(),
                "6H52-14C526-CD".to_string(),
                "Strategy".to_string()
            ),
            (
                "F124".to_string(),
                "8X23-14C527-CE".to_string(),
                "Calibration".to_string()
            ),
        ]
    );
    // A module the catalogue does not carry has nothing to compare with.
    assert!(library
        .catalogue_assemblies(&context, "LEGACYMOD")
        .is_empty());

    let mut assemblies = BTreeMap::new();
    assemblies.insert("SYNTHMOD".to_string(), known);
    let readings = vec![
        // The assembly itself, answered without the hyphens a catalogue
        // writes: the same number (ADR-0033, decision 5).
        reading("SYNTHMOD", "0xF113", Some("8X2318C808CE")),
        // The software the catalogue names for it.
        reading("SYNTHMOD", "0xF188", Some("6H52-14C526-CD")),
        // A calibration that is not the one the catalogue names.
        reading("SYNTHMOD", "0xF124", Some("8X23-14C527-AA")),
        // An identifier the catalogue names no part for.
        reading("SYNTHMOD", "0xF18C", Some("SERIAL-0001")),
        // A module the catalogue does not carry at all.
        reading("OTHERMOD", "0xF188", Some("anything")),
        // And a silence, which is nothing to compare.
        reading("SYNTHMOD", "0xF120", None),
    ];
    let compared = catalogue_comparisons(&assemblies, &readings);
    let states: Vec<Option<&str>> = compared
        .iter()
        .map(|entry| entry.as_ref().map(|value| value.state.as_str()))
        .collect();
    assert_eq!(
        states,
        [
            Some(CATALOGUE_AGREES),
            Some(CATALOGUE_AGREES),
            Some(CATALOGUE_DIFFERS),
            Some(CATALOGUE_NOT_NAMED),
            None,
            None,
        ]
    );
    let differs = compared[2].as_ref().unwrap();
    assert_eq!(differs.expected.as_deref(), Some("8X23-14C527-CE"));
    assert_eq!(differs.part_type.as_deref(), Some("Calibration"));
    assert_eq!(differs.assembly.as_deref(), Some("8X23-18C808-CE"));
    // The catalogue's own date travels with every comparison, so its age is
    // never hidden.
    assert_eq!(differs.dated.as_deref(), Some("Oct-17-2022 22:16:38"));

    // A module whose assembly the catalogue does not carry says so, rather
    // than comparing against an assembly it never reported.
    let mut wrong_unit = BTreeMap::new();
    wrong_unit.insert(
        "SYNTHMOD".to_string(),
        library.catalogue_assemblies(&context, "SYNTHMOD"),
    );
    let replaced = vec![
        reading("SYNTHMOD", "0xF113", Some("9Z99-00A000-ZZ")),
        reading("SYNTHMOD", "0xF188", Some("6H52-14C526-CD")),
    ];
    for entry in catalogue_comparisons(&wrong_unit, &replaced) {
        let entry = entry.expect("the catalogue carries the module");
        assert_eq!(entry.state, CATALOGUE_NO_ASSEMBLY);
        assert!(entry.expected.is_none());
    }

    // Nothing anywhere in this feature reads as a verdict.
    let words = [
        "outdated",
        "out of date",
        "update",
        "flash",
        "programme this",
    ];
    for entry in compared.into_iter().flatten() {
        let text = format!("{entry:?}").to_ascii_lowercase();
        for word in words {
            assert!(!text.contains(word), "{word} in {text}");
        }
    }
}

#[test]
fn the_catalogue_claim_name_matches_the_ingester() {
    assert_eq!(
        catalogue_claim_name(),
        sdd_ingest::IVS_ASSEMBLY_CLAIM,
        "the session reads the claim the ingest writes"
    );
}

/// The claim the session looks for, read back out of a record the ingest
/// wrote, so the two literals cannot drift apart unnoticed.
fn catalogue_claim_name() -> String {
    let library = library();
    let context = vehicle_context(&vehicle());
    assert!(!library
        .catalogue_assemblies(&context, "SYNTHMOD")
        .is_empty());
    let store = library.store();
    let result = store.query(&knowledge::KnowledgeQuery::default().include_indeterminate(true));
    result
        .records
        .iter()
        .find_map(
            |entry| match (&entry.record.entity.kind, &entry.record.key) {
                (knowledge::EntityKind::ModuleAssembly, knowledge::ClaimKey::Custom { name }) => {
                    Some(name.clone())
                }
                _ => None,
            },
        )
        .expect("a lineage record exists")
}

#[test]
fn the_self_tests_a_module_declares_are_listed_with_sdds_own_words_and_never_offered_to_run() {
    // The claim names and the dimension are the ingest's; the session reads
    // them back by the same literals.
    assert_eq!(
        SDD_MODEL_YEAR_DIMENSION,
        sdd_ingest::MODEL_YEAR_DESIGNATION_DIMENSION
    );

    let library = library();
    let context = vehicle_context(&vehicle());
    let tests = library.self_tests(&context, "SYNTHMOD");
    assert_eq!(
        tests
            .iter()
            .map(|test| test.test_id.as_str())
            .collect::<Vec<_>>(),
        ["14", "99"],
        "both tests the data does not rule out for this car, in SDD's order"
    );

    let named = &tests[0];
    assert_eq!(named.name, "DR_ODST_14_SYNTHMOD");
    assert_eq!(named.time_ms, Some(40_000));
    assert_eq!(named.timeout_ms, Some(40_000));
    // SDD's own marker, shown rather than matched.
    assert_eq!(named.model_years, ["MY03"]);
    // What SDD tells whoever runs it, its blank line and its line with no
    // text left out by the ingest.
    assert_eq!(
        named.description,
        [
            "The synthetic module runs its own check and logs what it finds.",
            "Make sure the ignition is switched on.",
        ]
    );
    // The class is what keeps every one of them out of this stage.
    assert!(tests
        .iter()
        .all(|test| test.safety_class == "SERVICE_ROUTINE"));

    // A module the data declares no test for gets none, and so does a car
    // the tests are not qualified for.
    assert!(library.self_tests(&context, "SYNTHMOD2").is_empty());
    let other_car = vehicle_context(&VehicleContextInput {
        vehicle_program: "SYNTHZ".into(),
        ..vehicle()
    });
    assert!(library.self_tests(&other_car, "SYNTHMOD").is_empty());

    // And the survey carries them, so the interface needs no second call.
    let survey = library.survey(&vehicle());
    let entry = survey
        .modules
        .iter()
        .find(|entry| entry.ecu_family == "SYNTHMOD")
        .expect("the module is surveyed");
    assert_eq!(entry.self_tests, tests);
}

#[test]
fn a_module_on_a_hypothesised_bus_is_shown_as_a_hypothesis_not_as_reachable() {
    // The fixture's SYNTHMOD sits on CAN_HS, which the documented binding
    // covers. Re-label its bus as PT_HSCAN, which only the research manifest
    // binds, and the survey must say "hypothesis" and carry UNVERIFIED.
    let manifests: Vec<(String, String)> = exported_manifests()
        .into_iter()
        .map(|(name, text)| (name, text.replace("CAN_HS", "PT_HSCAN")))
        .collect();
    let library = KnowledgeLibrary::from_manifests(
        manifests
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    );
    let survey = library.survey(&vehicle());
    let module = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == "SYNTHMOD")
        .unwrap();
    assert_eq!(module.identifier_read.status, RouteStatus::Hypothesis);
    assert_eq!(module.logical_network.as_deref(), Some("PT_HSCAN"));
    assert_eq!(module.backend_route.as_deref(), Some("hs-can"));
    assert_eq!(module.route_validation, "UNVERIFIED");
    assert!(module.identifier_read.reasons[0].contains("unverified hypothesis"));
    // SYNTHMOD on its relayed bus, and DS2MOD on its K-line (ADR-0029): the
    // two hypotheses of this library, and nothing reachable.
    assert_eq!(survey.hypothesis, 2);
    assert_eq!(survey.reachable, 0);
    assert!(survey.message.contains("2 on a hypothesised route"));
}

#[test]
fn the_catalogue_offers_the_programmes_markers_years_and_engines_the_data_is_qualified_by() {
    let library = library();
    let catalogue = library.catalogue();
    let programme = catalogue
        .programmes
        .iter()
        .find(|entry| entry.program == "SYNTHA")
        .expect("the platform fixture describes SYNTHA");
    let marker = &programme.markers[0];
    assert_eq!(marker.marker, "MY10");
    assert_eq!(marker.model_year_from, Some(2010));
    assert_eq!(marker.model_year_to, Some(2011));
    // The DID catalogue qualifies one entry by engine.
    assert_eq!(programme.powertrains, vec!["SYNTHENGINE".to_string()]);
    // Built-in data alone describes no vehicle.
    assert!(KnowledgeLibrary::built_in()
        .catalogue()
        .programmes
        .is_empty());
}

#[test]
fn fault_code_wording_is_joined_by_code_and_module_and_never_invented() {
    let library = library();
    // The fixtures carry no DTC index, so the library describes nothing.
    let described = library.describe_dtc("P0301", 0, "SYNTHMOD");
    assert!(described.description.is_none());
    assert!(described.description_scope.is_none());
    assert!(described.failure_type_text.is_none());
    assert!(described.failure_type_texts.is_empty());
    // Our own wording for a code the standard defines is not the library's
    // and does not depend on it; it is not an invention either, because the
    // standard says what P0301 means.
    assert!(described.description_texts["ukr"].contains("циліндрі 1"));
    assert!(described.description_texts["rus"].contains("цилиндре 1"));
    // A manufacturer-specific code gets nothing from us: that wording is the
    // manufacturer's, and it lives in the issued library or nowhere.
    let specific = library.describe_dtc("P1234", 0, "SYNTHMOD");
    assert_eq!(specific, diagnostic_session::DtcDescription::default());
}

#[test]
fn module_names_come_from_the_text_database_in_every_language_it_has() {
    let library = library();
    assert_eq!(
        library.module_name("SYNTHMOD", "eng").as_deref(),
        Some("Synthetic control module")
    );
    assert_eq!(
        library.module_name("SYNTHMOD", "deu").as_deref(),
        Some("Synthetisches Steuermodul")
    );
    // No Ukrainian in SDD's text database: absent, not guessed.
    assert_eq!(library.module_name("SYNTHMOD", "ukr"), None);
    // A programme-decorated mnemonic falls back to its base; an unknown one stays unknown.
    assert_eq!(
        library.module_name("LR_SYNTHMOD_L322", "eng").as_deref(),
        Some("Synthetic control module")
    );
    assert_eq!(library.module_name("OTHERMOD", "eng"), None);

    let survey = library.survey(&vehicle());
    let named = survey
        .modules
        .iter()
        .find(|module| module.ecu_family == "SYNTHMOD")
        .expect("SYNTHMOD surveyed");
    assert_eq!(named.name.as_deref(), Some("Synthetic control module"));
    assert_eq!(
        named.names.get("rus").map(String::as_str),
        Some("Синтетический блок управления")
    );
    assert_eq!(named.names.len(), 3);
}

#[test]
fn a_vin_is_decoded_with_the_tables_the_library_holds() {
    let library = library();
    assert!(library.has_vin_tables());
    let decoded = library.decode_vin("syna102vbbac12345");
    assert!(decoded.valid);
    assert_eq!(decoded.decode_model, Some(1));
    assert_eq!(decoded.program.as_deref(), Some("SYNTHA"));
    assert_eq!(decoded.model_year, Some(2011));
    let engine = decoded
        .attributes
        .iter()
        .find(|attribute| attribute.name == "Engine")
        .expect("engine decoded");
    assert_eq!(engine.value, "3.0L V6 synthetic");
    let model_name = decoded
        .attributes
        .iter()
        .find(|attribute| attribute.name == "ModelName")
        .expect("model name decoded");
    assert_eq!(model_name.value, "Synth A estate; long");
    assert_eq!(
        decoded.table_version.as_deref(),
        Some("SDD table version: Synthetic VIN chart issue 1")
    );

    // Position 12 = B routes to the second model, reached by either rule block.
    let other = library.decode_vin("SYNLB02VBBAB12345");
    assert_eq!(other.decode_model, Some(2));
    assert_eq!(other.program.as_deref(), Some("SYNTHB"));
    assert_eq!(other.model_year, None);

    // Built-in data has no tables and says so.
    let built_in = KnowledgeLibrary::built_in();
    assert!(!built_in.has_vin_tables());
    assert!(built_in
        .decode_vin("SYNA102VBBAC12345")
        .message
        .contains("no VIN tables"));
}
