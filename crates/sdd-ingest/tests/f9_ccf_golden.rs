//! F9 golden tests for SDD's car configuration descriptions (ADR-0028).
//!
//! The load-bearing tests: the write service SDD names beside every block
//! never enters the knowledge base, a source that is not a module yields no
//! identifier, a paged (VDF) document yields no identifier at all, and the
//! layout is recorded as a layout — never as a value.

use knowledge::{
    sha256_bytes, ClaimKey, ContentFingerprint, DimensionConstraint, EntityKind, KnowledgeQuery,
    KnowledgeStore, KnowledgeValue, RedistributionStatus, SourceId, SourceRecord, SourceType,
    ValidationState,
};
use sdd_ingest::{
    CcfAdapter, TextLookup, CCF_BLOCK_ENCODING_PREFIX, CCF_PARAMETER_CLAIM, CCF_SCHEME_CLAIM,
    CCF_SOURCE_CLAIM_PREFIX,
};
use std::sync::Arc;

const FIXTURE: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_ccf_data.xml");
const VDF: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_ccf_data_vdf.xml");

fn source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F9 CCF fixture {id}"),
        source_type: SourceType::Documented,
        origin: "F9 golden test".into(),
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

fn texts() -> Arc<TextLookup> {
    let mut lookup = TextLookup::new();
    lookup.insert("@SYNTH_BRAND", Some("Brand"), Some("Марка"));
    lookup.insert("@SYNTH_UNDEF", Some("Undefined"), Some("Не определено"));
    // A text carrying every separator, to prove the round trip is safe.
    lookup.insert(
        "@SYNTH_ALPHA",
        Some("Alpha; edition=1|first"),
        Some("Альфа"),
    );
    lookup.insert("@SYNTH_FLAGS", Some("Comfort flags"), None);
    lookup.insert(
        "@SYNTH_HEATED",
        Some("Heated seats"),
        Some("Подогрев сидений"),
    );
    lookup.insert("@SYNTH_NO", Some("Not fitted"), Some("Не установлено"));
    lookup.insert("@SYNTH_YES", Some("Fitted"), Some("Установлено"));
    lookup.insert("@SYNTH_CODE", Some("Build code"), None);
    Arc::new(lookup)
}

fn ingest() -> KnowledgeStore {
    let adapter = CcfAdapter::new(source("f9-ccf", FIXTURE))
        .unwrap()
        .with_texts(texts());
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, FIXTURE).unwrap();
    store
}

/// module, identifier, encoding, powertrain, model-year marker.
type IdentifierRow = (String, String, String, Option<String>, String);

fn identifier_rows(store: &KnowledgeStore) -> Vec<IdentifierRow> {
    let mut rows: Vec<_> = store
        .query(&KnowledgeQuery::default().include_indeterminate(true))
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::IdentifierParameter)
        .map(|resolved| {
            let (identifier, encoding) = match &resolved.record.value {
                KnowledgeValue::IdentifierDefinition {
                    identifier,
                    encoding,
                    ..
                } => (identifier.clone(), encoding.clone()),
                other => panic!("unexpected value {other:?}"),
            };
            let one = |constraint: &DimensionConstraint| match constraint {
                DimensionConstraint::OneOf { values } => Some(values.join("|")),
                _ => None,
            };
            let family = one(&resolved.record.applicability.ecu_family).unwrap_or_default();
            let powertrain = one(&resolved.record.applicability.powertrain);
            let marker = resolved
                .record
                .applicability
                .other
                .get("sdd_year_breakpoint")
                .and_then(one)
                .unwrap_or_default();
            (
                family,
                identifier,
                encoding.unwrap_or_default(),
                powertrain,
                marker,
            )
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn every_readable_block_is_an_identifier_of_its_module_and_nothing_else_is() {
    let store = ingest();
    let rows = identifier_rows(&store);
    let row =
        |module: &str, identifier: &str, encoding: &str, powertrain: Option<&str>, marker: &str| {
            (
                module.to_string(),
                identifier.to_string(),
                encoding.to_string(),
                powertrain.map(str::to_string),
                marker.to_string(),
            )
        };
    assert_eq!(
        rows,
        vec![
            // An address stamped with a neighbouring year keeps that year.
            row(
                "OTHERMOD",
                "0xF105",
                "ccf=RES;offset=4;length=2",
                None,
                "MY12"
            ),
            // The copy's address is narrowed by the engine it states.
            row(
                "OTHERMOD",
                "0xF106",
                "ccf=CCF;offset=0;length=8",
                Some("SYNTHENGINE"),
                "MY10"
            ),
            // One identifier, two blocks at two offsets.
            row(
                "SYNTHMOD",
                "0xF105",
                "ccf=RES;offset=4;length=2",
                None,
                "MY10"
            ),
            row(
                "SYNTHMOD",
                "0xF105",
                "ccf=VB;offset=0;length=4",
                None,
                "MY10"
            ),
            row(
                "SYNTHMOD",
                "0xF106",
                "ccf=CCF;offset=0;length=8",
                None,
                "MY10"
            ),
        ],
        "AS_BUILT is not a module and PAD is read with no service: neither is an identifier"
    );
    for row in &rows {
        assert!(row.2.starts_with(CCF_BLOCK_ENCODING_PREFIX));
    }
    // The write service, the VBF names and the padding never appear.
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let text = serde_json::to_string(
        &all.records
            .iter()
            .map(|resolved| &resolved.record)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(!text.contains("0x2E"), "the write service is not recorded");
    assert!(
        !text.contains(".vbf"),
        "as-built file names are not recorded"
    );
    assert!(!text.contains("\"PAD\""), "padding is not a block");
    for resolved in &all.records {
        assert_eq!(
            resolved.record.validation_state,
            ValidationState::SourceBacked
        );
    }
}

#[test]
fn the_header_says_who_keeps_the_configuration_and_the_scheme_says_how_it_is_read() {
    let store = ingest();
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let programme: Vec<(String, String)> = all
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::VehicleProgram)
        .map(|resolved| {
            let name = match &resolved.record.key {
                ClaimKey::Custom { name } => name.clone(),
                other => panic!("unexpected key {other:?}"),
            };
            let value = match &resolved.record.value {
                KnowledgeValue::Text { value } => value.clone(),
                other => panic!("unexpected value {other:?}"),
            };
            (name, value)
        })
        .collect();
    assert!(programme.contains(&(CCF_SCHEME_CLAIM.to_string(), "did".to_string())));
    assert!(programme.contains(&(
        format!("{CCF_SOURCE_CLAIM_PREFIX}SYNTHMOD"),
        "sync".to_string()
    )));
    assert!(programme.contains(&(
        format!("{CCF_SOURCE_CLAIM_PREFIX}OTHERMOD"),
        "copy".to_string()
    )));
    // AS_BUILT and OTHER are not modules.
    assert!(!programme
        .iter()
        .any(|(name, _)| name.ends_with("AS_BUILT") || name.ends_with("OTHER")));
}

#[test]
fn the_layout_is_recorded_as_a_layout_with_texts_and_nothing_is_decoded() {
    let store = ingest();
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    let mut layout: Vec<(String, String)> = all
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::ConfigurationParameter)
        .map(|resolved| {
            assert_eq!(
                resolved.record.key,
                ClaimKey::Custom {
                    name: CCF_PARAMETER_CLAIM.into()
                }
            );
            let value = match &resolved.record.value {
                KnowledgeValue::Text { value } => value.clone(),
                other => panic!("unexpected value {other:?}"),
            };
            (resolved.record.entity.id.clone(), value)
        })
        .collect();
    layout.sort();
    let ids: Vec<&str> = layout.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "CCF-CCF-PARAM_SYNTH_BRAND",
            "CCF-CCF-PARAM_SYNTH_CODE",
            "CCF-CCF-PARAM_SYNTH_DAY",
            "CCF-CCF-PARAM_SYNTH_HEATED_SEATS",
            "CCF-CCF-PARAM_SYNTH_RADIUS",
            "CCF-CCF-PARAM_SYNTH_TRIM_LEVEL",
            "CCF-RES-SYNTH_RES",
            "CCF-VB-SYNTH_RESERVED",
        ]
    );
    let brand = &layout[0].1;
    assert!(brand.starts_with("block=CCF;bytes=0..0;bits=0..7;mask=0xFF;type=ENUM;display=true;edit=false;scope=base;group=GROUP_SYNTH_BRAND;group_title=Brand;group_title_ru=Марка;title=;title_ru=;options="), "{brand}");
    // Options carry value, name, code and both texts; the separators inside
    // a text are escaped; a text SDD has none for stays empty and the name
    // speaks.
    assert!(
        brand.contains("0x00=UNDEF==Undefined=Не определено|0x01=ALPHA=VS_A=Alpha%3B edition%3D1%7Cfirst=Альфа|0x02=BETA==="),
        "{brand}"
    );
    let heated = &layout[3].1;
    assert!(heated.contains(";bits=0..0;mask=0x01;type=BOOL;display=true;edit=true;scope=personalisation;group=GROUP_SYNTH_FLAGS;group_title=Comfort flags;group_title_ru=;title=Heated seats;title_ru=Подогрев сидений;options=0x00=FALSE==Not fitted=Не установлено|0x01=TRUE==Fitted=Установлено"), "{heated}");
    let trim = &layout[5].1;
    assert!(
        trim.contains("bytes=1..1;bits=1..2;mask=0x06;type=ENUM;display=false"),
        "{trim}"
    );
    let code = &layout[1].1;
    assert!(
        code.contains("bytes=2..4;bits=0..23;mask=0xFF;type=ASCII;display=true"),
        "{code}"
    );
    assert!(code.ends_with("options="), "no options for a text: {code}");
    let reserved = &layout[7].1;
    assert!(
        reserved.starts_with("block=VB;bytes=0..3;bits=0..31;mask=0xFF;type=UNDEF"),
        "{reserved}"
    );
    // The parameter whose span is written stop-before-start is not placeable
    // and is not recorded; the seven that can be placed stand.
    assert!(!ids.iter().any(|id| id.ends_with("SYNTH_RES_ODD")));
    // A layout applies to the programme and marker, not to a module.
    for resolved in all
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::ConfigurationParameter)
    {
        assert_eq!(
            resolved.record.applicability.vehicle_program,
            DimensionConstraint::one_of(["SYNTHA".to_string()]).unwrap()
        );
        assert_eq!(
            resolved.record.applicability.ecu_family,
            DimensionConstraint::Any
        );
    }
}

#[test]
fn a_paged_document_records_its_scheme_and_layout_and_no_identifier() {
    let adapter = CcfAdapter::new(source("f9-ccf-vdf", VDF)).unwrap();
    let mut store = KnowledgeStore::new();
    store.ingest(&adapter, VDF).unwrap();
    let all = store.query(&KnowledgeQuery::default().include_indeterminate(true));
    assert!(all
        .records
        .iter()
        .all(|resolved| resolved.record.entity.kind != EntityKind::IdentifierParameter));
    assert!(all.records.iter().any(|resolved| {
        resolved.record.key
            == ClaimKey::Custom {
                name: CCF_SCHEME_CLAIM.into(),
            }
            && resolved.record.value
                == KnowledgeValue::Text {
                    value: "vdf".into(),
                }
    }));
    let layout: Vec<_> = all
        .records
        .iter()
        .filter(|resolved| resolved.record.entity.kind == EntityKind::ConfigurationParameter)
        .collect();
    assert_eq!(layout.len(), 1);
    // Without a text lookup every text is empty and the names speak.
    if let KnowledgeValue::Text { value } = &layout[0].record.value {
        assert!(value.contains("group_title=;group_title_ru=;"), "{value}");
        assert!(value.ends_with("options=0x01=ALPHA==="), "{value}");
    }
}

#[test]
fn a_document_naming_two_programmes_or_no_leading_marker_is_rejected_whole() {
    let two = FIXTURE.replacen("model=\"SYNTHA\"", "model=\"SYNTHB\"", 1);
    let adapter = CcfAdapter::new(source("f9-ccf-two", &two)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store.ingest(&adapter, &two).expect_err("two programmes");
    assert!(error.to_string().contains("exactly one program"), "{error}");
    assert_eq!(store.sources().iter().count(), 0, "no partial write");

    // The document's marker is the one most qualifiers name; when two are
    // named equally often nothing leads, and the document is refused rather
    // than guessed. The fixture names MY10 six times and MY12 once; three
    // of the MY10 stamps turned MY08 leave MY10 and MY08 level.
    let mut tied = FIXTURE.to_string();
    for _ in 0..3 {
        tied = tied.replacen("year=\"MY10\"", "year=\"MY08\"", 1);
    }
    let adapter = CcfAdapter::new(source("f9-ccf-tied", &tied)).unwrap();
    let mut store = KnowledgeStore::new();
    let error = store.ingest(&adapter, &tied).expect_err("no marker leads");
    assert!(error.to_string().contains("no marker leads"), "{error}");
}
