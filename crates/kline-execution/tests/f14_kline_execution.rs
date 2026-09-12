//! F14 golden tests: from SDD-shaped knowledge to a prepared K-line read.
//!
//! The store is built the way the product builds it — the synthetic platform
//! fixture through the F9 adapter, the K-line route hypotheses through their
//! own manifest — then resolved and prepared. The load-bearing tests are the
//! refusals: a plan that states no framing, a protocol this product does not
//! speak, an addressing mode that is not a node address. Fixtures are
//! synthetic and prove behaviour only.

use diagnostic_environment::{DiagnosticEnvironmentResolution, DiagnosticEnvironmentResolver};
use kline_execution::{
    decode_response, prepare_read_only_kline_read, DecodeError, KlineReadOutcome, KlineRequest,
    KlineTarget, Parity, PreparationError, ProvenanceField, ReadOnlyKlineIntent, SerialFraming,
    TransactionSafetyClass, SERIAL_NODE_ADDRESSING_MODE,
};
use knowledge::{
    sha256_bytes, ContentFingerprint, JsonManifestAdapter, KnowledgeStore, RedistributionStatus,
    SourceId, SourceRecord, SourceType, VehicleContext,
};
use sdd_ingest::{ModelYearTimeline, PlatformAdapter, YEAR_BREAKPOINT_DIMENSION};
use std::collections::BTreeMap;

const PLATFORM: &str = include_str!("../../../fixtures/knowledge/synthetic/f9_platform.xml");
const KLINE_ROUTES: &str =
    include_str!("../../../fixtures/knowledge/research/mongoose_jlr_kline_route_hypotheses.json");

fn synthetic_source(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        id: SourceId::new(id).unwrap(),
        title: format!("F14 synthetic fixture {id}"),
        source_type: SourceType::Synthetic,
        origin: "F14 golden test".into(),
        source_locator: "fixtures/knowledge/synthetic/f9_platform.xml".into(),
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

fn store() -> KnowledgeStore {
    let platform = PlatformAdapter::new(synthetic_source("f14-plat", PLATFORM))
        .unwrap()
        .with_timeline(timeline());
    let mut store = KnowledgeStore::new();
    store.ingest(&platform, PLATFORM).unwrap();
    store.ingest(&JsonManifestAdapter, KLINE_ROUTES).unwrap();
    store
}

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

fn resolved(family: &str, capability: &str) -> DiagnosticEnvironmentResolution {
    DiagnosticEnvironmentResolver::resolve_ecu_family(&store(), &context(), family, capability)
        .unwrap()
}

fn ds2_target() -> KlineTarget {
    KlineTarget::family("DS2MOD").unwrap()
}

fn prepared_identification() -> kline_execution::PreparedKlineTransaction {
    prepare_read_only_kline_read(
        &resolved("DS2MOD", ds2::ECU_IDENTIFICATION_CAPABILITY),
        ReadOnlyKlineIntent::Ds2Identification {
            target: ds2_target(),
        },
    )
    .expect("the fixture's DS2 module resolves")
}

/// A module's reply on the wire: our own request echoed back, then the
/// module's frame. A K-line always echoes.
fn on_the_wire(request: &[u8], reply: &[u8]) -> Vec<u8> {
    let mut stream = request.to_vec();
    stream.extend_from_slice(reply);
    stream
}

fn ds2_reply(node: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![node, 0, ds2::REPLY_ACCEPTED];
    frame.extend_from_slice(payload);
    frame[1] = u8::try_from(frame.len() + 1).unwrap();
    let checksum = ds2::xor_checksum(&frame);
    frame.push(checksum);
    frame
}

#[test]
fn a_ds2_module_resolves_into_a_read_only_transaction_with_the_line_it_needs() {
    let transaction = prepared_identification();

    assert_eq!(transaction.safety_class(), TransactionSafetyClass::ReadOnly);
    assert_eq!(transaction.target().ecu_family(), "DS2MOD");
    assert_eq!(transaction.protocol_family(), "DS2");
    assert_eq!(transaction.logical_network(), "DS2_PIN7");
    assert_eq!(transaction.physical_pins(), [7]);
    assert_eq!(transaction.backend_route(), "k-line-7");
    assert_eq!(transaction.bitrate_bps(), 9_600);
    // The node address is the whole of the addressing (ADR-0029).
    assert_eq!(transaction.node_address(), 0x72);
    // The line's own framing and wake-up, as the document states them.
    assert_eq!(
        transaction.framing(),
        SerialFraming {
            data_bits: 8,
            parity: Parity::Even,
            stop_bits: 1
        }
    );
    assert_eq!(transaction.wakeup(), "bmw_ds2");
    assert_eq!(
        transaction.capability_id(),
        ds2::ECU_IDENTIFICATION_CAPABILITY
    );
    // The bytes are DS2's own frame: address, length, command, checksum.
    assert_eq!(transaction.encoded_payload(), [0x72, 0x04, 0x00, 0x76]);
    assert!(matches!(transaction.request(), KlineRequest::Ds2(_)));

    // Every fact the read rests on can be traced back to a record.
    for field in [
        ProvenanceField::EcuFamily,
        ProvenanceField::LogicalNetwork,
        ProvenanceField::PhysicalRoute,
        ProvenanceField::BackendRoute,
        ProvenanceField::Bitrate,
        ProvenanceField::ProtocolFamily,
        ProvenanceField::AddressingMode,
        ProvenanceField::SerialFraming,
        ProvenanceField::SerialWakeup,
        ProvenanceField::NodeAddress,
        ProvenanceField::Capability,
    ] {
        assert!(
            !transaction.provenance().traces_for(field).is_empty(),
            "{field:?} has no evidence"
        );
    }
}

#[test]
fn the_fault_memory_is_a_second_intent_and_a_different_capability() {
    let transaction = prepare_read_only_kline_read(
        &resolved("DS2MOD", ds2::FAULT_MEMORY_CAPABILITY),
        ReadOnlyKlineIntent::Ds2FaultMemory {
            target: ds2_target(),
        },
    )
    .expect("the fault memory resolves too");
    assert_eq!(transaction.encoded_payload(), [0x72, 0x04, 0x04, 0x72]);
    assert_eq!(transaction.capability_id(), ds2::FAULT_MEMORY_CAPABILITY);

    // A plan resolved for one capability does not carry another read.
    let error = prepare_read_only_kline_read(
        &resolved("DS2MOD", ds2::FAULT_MEMORY_CAPABILITY),
        ReadOnlyKlineIntent::Ds2Identification {
            target: ds2_target(),
        },
    )
    .expect_err("the capability must match the intent");
    assert!(matches!(error, PreparationError::UnsupportedCapability(_)));
}

#[test]
fn a_protocol_this_product_does_not_speak_is_refused_rather_than_tried() {
    // STARMOD sits on KW2000STAR, which ADR-0029 records as named and not
    // spoken: it has no read-only capability at all, so the plan does not
    // resolve and no request is ever built for it.
    let resolution = DiagnosticEnvironmentResolver::resolve_ecu_family(
        &store(),
        &context(),
        "STARMOD",
        kwp2000::READ_ECU_IDENTIFICATION_CAPABILITY,
    )
    .unwrap();
    let error = prepare_read_only_kline_read(
        &resolution,
        ReadOnlyKlineIntent::KwpEcuIdentification {
            target: KlineTarget::family("STARMOD").unwrap(),
            option: 0x80,
        },
    )
    .expect_err("a protocol we do not speak cannot be prepared");
    assert!(matches!(
        error,
        PreparationError::EnvironmentIndeterminate(_)
    ));

    // And a CAN module is refused on the addressing, not attempted.
    let can = DiagnosticEnvironmentResolver::resolve_ecu_family(
        &store(),
        &context(),
        "SYNTHMOD",
        ds2::ECU_IDENTIFICATION_CAPABILITY,
    )
    .unwrap();
    let error = prepare_read_only_kline_read(
        &can,
        ReadOnlyKlineIntent::Ds2Identification {
            target: KlineTarget::family("SYNTHMOD").unwrap(),
        },
    )
    .expect_err("a CAN module is not read over a K-line");
    assert!(matches!(
        error,
        PreparationError::EnvironmentIndeterminate(_) | PreparationError::UnsupportedProtocol(_)
    ));
}

#[test]
fn the_intent_and_the_plan_must_be_about_the_same_module() {
    let error = prepare_read_only_kline_read(
        &resolved("DS2MOD", ds2::ECU_IDENTIFICATION_CAPABILITY),
        ReadOnlyKlineIntent::Ds2Identification {
            target: KlineTarget::family("OTHERMOD").unwrap(),
        },
    )
    .expect_err("the target must be the plan's module");
    assert!(matches!(error, PreparationError::TargetEcuMismatch { .. }));
    assert!(KlineTarget::family("  ").is_err());
}

#[test]
fn a_framing_the_document_does_not_state_is_a_refusal_not_a_default() {
    assert_eq!(
        SerialFraming::parse("data_bits=8;parity=even;stop_bits=1").unwrap(),
        SerialFraming {
            data_bits: 8,
            parity: Parity::Even,
            stop_bits: 1
        }
    );
    assert_eq!(
        SerialFraming::parse("data_bits=8;parity=none;stop_bits=1")
            .unwrap()
            .parity,
        Parity::None
    );
    // Each missing field is named, and nothing is assumed in its place.
    assert!(matches!(
        SerialFraming::parse("data_bits=8;stop_bits=1"),
        Err(PreparationError::UnreadableFraming("parity"))
    ));
    assert!(matches!(
        SerialFraming::parse("parity=even;stop_bits=1"),
        Err(PreparationError::UnreadableFraming("data_bits"))
    ));
    assert!(matches!(
        SerialFraming::parse(""),
        Err(PreparationError::UnreadableFraming(_))
    ));
}

#[test]
fn the_addressing_mode_is_the_knowledge_models_own_word() {
    assert_eq!(
        SERIAL_NODE_ADDRESSING_MODE,
        knowledge::ISO9141_NODE_ADDRESSING_MODE
    );
    assert_eq!(
        SERIAL_NODE_ADDRESSING_MODE,
        sdd_ingest::ISO9141_NODE_ADDRESSING_MODE
    );
}

#[test]
fn a_ds2_answer_is_read_back_as_the_module_sent_it() {
    let transaction = prepared_identification();
    let payload = b"SYNTH-DS2-0001";
    let stream = on_the_wire(transaction.encoded_payload(), &ds2_reply(0x72, payload));

    let outcome = decode_response(&transaction, &stream).expect("the frame reads back");
    let KlineReadOutcome::Ds2Identification {
        node_address,
        bytes,
        text,
    } = outcome
    else {
        panic!("expected an identification, got {outcome:?}");
    };
    assert_eq!(node_address, 0x72);
    assert_eq!(bytes, payload);
    assert_eq!(text, "SYNTH-DS2-0001");

    // A module that did not accept the request says so in its own byte, and
    // nothing is invented in its place.
    let mut refused = vec![0x72, 0x04, 0x1F];
    refused.push(ds2::xor_checksum(&refused));
    let outcome = decode_response(
        &transaction,
        &on_the_wire(transaction.encoded_payload(), &refused),
    )
    .expect("a refusal is an answer");
    assert_eq!(
        outcome,
        KlineReadOutcome::Ds2NotAccepted {
            node_address: 0x72,
            status: 0x1F
        }
    );

    // Silence is silence.
    assert!(matches!(
        decode_response(&transaction, transaction.encoded_payload()),
        Err(DecodeError::NoAnswer)
    ));
}

#[test]
fn a_ds2_fault_memory_offers_its_words_and_decides_no_layout() {
    let transaction = prepare_read_only_kline_read(
        &resolved("DS2MOD", ds2::FAULT_MEMORY_CAPABILITY),
        ReadOnlyKlineIntent::Ds2FaultMemory {
            target: ds2_target(),
        },
    )
    .unwrap();
    let stream = on_the_wire(
        transaction.encoded_payload(),
        &ds2_reply(0x72, &[0x00, 0x0B, 0x00, 0x2A]),
    );
    let outcome = decode_response(&transaction, &stream).unwrap();
    let KlineReadOutcome::Ds2FaultMemory { bytes, words, .. } = outcome else {
        panic!("expected a fault memory, got {outcome:?}");
    };
    assert_eq!(bytes, [0x00, 0x0B, 0x00, 0x2A]);
    assert_eq!(words, [0x000B, 0x002A]);
}
