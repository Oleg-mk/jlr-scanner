//! SDD-era knowledge ingestion adapters.
//!
//! This crate parses material that SDD publishes in plain form and turns it
//! into F5 knowledge batches. It depends only on `knowledge` and a read-only
//! XML parser, and it has no protocol, transport, execution, environment,
//! operating-system, application-shell, or UI dependency.
//!
//! Two rules from ADR-0009 are enforced here rather than left to reviewers:
//!
//! * evidence class and validation state are derived from the registered
//!   source type, so a synthetic fixture can never be presented as JLR
//!   evidence;
//! * only material SDD expresses as readable becomes an available operation.
//!
//! Firmware is never ingested and `.exml` decryption is out of scope.

mod addressing;
mod ccf;
mod converters;
mod did;
mod dtc;
mod dtc_index;
mod ivs;
mod model_year;
mod odst;
mod platform;
mod text;
mod vin;
mod xml;

pub use addressing::{CanLinkMonitorAdapter, CAN_LINK_MONITOR_PARSER_ID};
pub use ccf::{
    CcfAdapter, TextLookup, CCF_BLOCK_ENCODING_PREFIX, CCF_PARAMETER_CLAIM, CCF_PARSER_ID,
    CCF_SCHEME_CLAIM, CCF_SOURCE_CLAIM_PREFIX,
};
pub use converters::{ConverterCatalogue, ConverterInfo, ConverterKind};
pub use did::{DidFormattingAdapter, DID_FORMATTING_PARSER_ID, YEAR_BREAKPOINT_DIMENSION};
pub use dtc::{
    DtcHelpAdapter, DTC_FAULT_TYPE_DIMENSION, DTC_HELP_CLAIM, DTC_HELP_ITEM_SEPARATOR,
    DTC_HELP_LANGUAGE_RUSSIAN, DTC_HELP_PARSER_ID, DTC_HELP_SCREEN_CLAIM_PREFIX,
    DTC_HELP_SCREEN_ITEMS_CLAIM_PREFIX, DTC_TYPE_DIMENSION, MODEL_YEAR_DESIGNATION_DIMENSION,
    MODULE_DATA_NAME_DIMENSION,
};
pub use dtc_index::{
    DtcDescriptionAdapter, DtcFaultTypeAdapter, DTC_DESCRIPTION_PARSER_ID,
    DTC_FAULT_TYPE_PARSER_ID, FAILURE_TYPE_CLAIM,
};
pub use ivs::{IvsLineageAdapter, IVS_ASSEMBLY_CLAIM, IVS_PARSER_ID};
pub use model_year::{
    parse_marker, parse_relative_marker, ModelYearPoint, ModelYearTimeline, RelativeMarker,
    BASE_MARKER, FIRST_MODEL_YEAR, LAST_MODEL_YEAR,
};
pub use odst::{
    OdstInfoAdapter, ODST_HELP_CLAIM_PREFIX, ODST_PARSER_ID, ODST_SCREEN_CLAIM_PREFIX,
    ODST_TEST_CLAIM, QUAL_DIMENSION_PREFIX,
};
pub use platform::{
    BatteryFormatting, BatteryFormattingRow, PlatformAdapter, DS2_DIAGNOSTIC_PROTOCOL,
    DS2_ECU_IDENTIFICATION_CAPABILITY, DS2_FAULT_MEMORY_CAPABILITY, IDENTIFICATION_ENCODING,
    ISO15765_NORMAL_FIXED_LAYOUT_EVIDENCE, ISO9141_NODE_ADDRESSING_MODE, ISO_SETTINGS_CLAIM,
    ISO_WAKEUP_CLAIM, KWP2000_DIAGNOSTIC_PROTOCOL, KWP2000_READ_DTC_BY_STATUS_CAPABILITY,
    KWP2000_READ_ECU_IDENTIFICATION_CAPABILITY, NETWORK_CLAIM, NORMAL_FIXED_TESTER_ADDRESS,
    NORMAL_FIXED_TESTER_ADDRESS_EVIDENCE, PHYSICAL_ADDRESS_CLAIM, PLATFORM_PARSER_ID,
    UDS_DIAGNOSTIC_PROTOCOL, UDS_READ_DATA_BY_IDENTIFIER_CAPABILITY,
    UDS_READ_DTC_INFORMATION_CAPABILITY,
};
pub use text::{
    is_failure_type_id, is_module_description_id, is_text_item_id, ModuleTextAdapter,
    FAILURE_TYPE_NAME_CLAIM_PREFIX, LEGACY_MODULE_NAME_CLAIM_PREFIX, MODULE_NAME_CLAIM_PREFIX,
    MODULE_TEXT_PARSER_ID,
};
pub use vin::{VinDecodeAdapter, VIN_ATTRIBUTE_CLAIM_PREFIX, VIN_DECODE_PARSER_ID, VIN_RULE_CLAIM};
pub(crate) use xml::{child_element, child_text, require_attribute};

use knowledge::{EvidenceClass, SourceType, ValidationState};

/// Evidence class implied by a registered source type.
///
/// `EvidenceRecord::validate` rejects mismatched pairs, so deriving this
/// removes a class of silent misclassification.
pub(crate) fn evidence_class_for(source_type: SourceType) -> EvidenceClass {
    match source_type {
        SourceType::Documented => EvidenceClass::OemDocumentation,
        SourceType::Captured => EvidenceClass::DirectObservation,
        SourceType::Synthetic => EvidenceClass::SyntheticTest,
        SourceType::UnverifiedResearch => EvidenceClass::UnverifiedResearch,
    }
}

/// Validation state a single-source batch may claim.
///
/// A lone source can never reach `Corroborated`; that requires a second,
/// independent source and is decided by the store, not by a parser.
pub(crate) fn validation_state_for(source_type: SourceType) -> ValidationState {
    if source_type.is_real_evidence() {
        ValidationState::SourceBacked
    } else {
        ValidationState::Unverified
    }
}

/// Lowercase ASCII slug usable as a record-id segment.
///
/// Only used for identifier construction; the untouched original is what gets
/// recorded as evidence.
pub(crate) fn slugify(value: &str, what: &str) -> Result<String, knowledge::KnowledgeError> {
    let mut slug = String::with_capacity(value.len());
    for byte in value.trim().bytes() {
        if byte.is_ascii_alphanumeric() {
            slug.push(byte.to_ascii_lowercase() as char);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err(knowledge::KnowledgeError::Parse(format!(
            "{what} '{value}' has no usable identifier characters"
        )));
    }
    Ok(slug)
}

/// Identifier segment that keeps the source spelling when it is already usable.
///
/// Module acronyms such as `ECM` and `ADHLS_L` are meaningful, so they are
/// preserved rather than lowercased. Only a value that cannot form a stable
/// identifier segment falls back to [`slugify`].
pub(crate) fn preferred_segment(
    value: &str,
    what: &str,
) -> Result<String, knowledge::KnowledgeError> {
    let trimmed = value.trim();
    if !trimmed.is_empty()
        && trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Ok(trimmed.to_string());
    }
    slugify(value, what)
}

/// Applicability for a record built from an SDD qualification expression.
///
/// SDD qualifies with a conjunction of equality tests. A dimension the
/// expression does not test is **not constrained**, which is a positive
/// statement and maps to `Any` — not to `Unknown`, which would mean the
/// applicability is undetermined and would stop the record ever resolving.
///
/// `model_year` is the exception. Every qualification carries a model-year
/// marker, and a marker only becomes a calendar range when a timeline resolves
/// it, so it starts unknown and is filled in by the caller.
pub(crate) fn qualified_applicability() -> knowledge::Applicability {
    use knowledge::DimensionConstraint::Any;
    knowledge::Applicability {
        vehicle_program: Any,
        model_year: knowledge::YearConstraint::Unknown,
        architecture_generation: Any,
        ecu_family: Any,
        powertrain: Any,
        variant: Any,
        market: Any,
        diagnostic_implementation: Any,
        other: std::collections::BTreeMap::new(),
    }
}
