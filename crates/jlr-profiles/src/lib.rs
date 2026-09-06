//! Evidence-backed product profile catalog.
//!
//! F8 intentionally exposes one live-candidate profile. This crate contains
//! data only: it cannot open a transport or execute a diagnostic operation.

use diagnostic_environment::{
    BackendRoute, CanIdFormat, DiagnosticEnvironmentPlan, DiagnosticEnvironmentResolution,
    ImplementationMarkerKind, PhysicalDiagnosticRoute, PlanEvidenceTrace, PlanField,
    ReadOnlyCapability, ValidationState, VehicleApplicability,
};
use knowledge::{EvidenceClass, SourceLocator, SourceType};
use std::collections::BTreeMap;

pub const CAPABILITY_ID: &str = "obd.service09.infotype04.calibration_id.read_only";
pub const ECU_FAMILY: &str = "jlr.x250.ecm";
pub const DIAGNOSTIC_IMPLEMENTATION: &str = "jlr.x250.ecm.cx23-14c204-zad";
pub const OBSERVED_CALIBRATION_ID: &str = "CX23-14C204-ZAD";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportedVehicleProfile {
    pub make: &'static str,
    pub model: &'static str,
    pub vehicle_program: &'static str,
    pub model_year: u16,
    pub powertrain: &'static str,
    pub variant: &'static str,
    pub module: &'static str,
    pub ecu_family: &'static str,
    pub diagnostic_implementation: &'static str,
}

pub const X250_2010_SUPERCHARGED_ECM: SupportedVehicleProfile = SupportedVehicleProfile {
    make: "Jaguar",
    model: "XF",
    vehicle_program: "X250",
    model_year: 2010,
    powertrain: "5.0L Supercharged",
    variant: "Supercharged",
    module: "ECM / PCM",
    ecu_family: ECU_FAMILY,
    diagnostic_implementation: DIAGNOSTIC_IMPLEMENTATION,
};

#[allow(clippy::too_many_arguments)]
fn trace(
    record_id: &str,
    evidence_id: &str,
    source_id: &str,
    source_type: SourceType,
    evidence_class: EvidenceClass,
    description: &str,
    section: &str,
    key: &str,
    validation_state: ValidationState,
) -> PlanEvidenceTrace {
    PlanEvidenceTrace {
        record_id: record_id.into(),
        evidence_id: evidence_id.into(),
        source_id: source_id.into(),
        source_type,
        evidence_class: Some(evidence_class),
        locator: SourceLocator {
            description: description.into(),
            document_page: None,
            document_section: Some(section.into()),
            record_key: Some(key.into()),
            capture_timestamp_us: None,
        },
        validation_state,
    }
}

fn field<T>(value: T, evidence: Vec<PlanEvidenceTrace>, state: ValidationState) -> PlanField<T> {
    PlanField {
        value,
        evidence,
        validation_state: state,
    }
}

/// Returns the exact F6 evidence-backed plan accepted for the first F8 alpha.
/// Request and response identifiers are stored independently; no arithmetic is
/// used to derive one from the other.
pub fn x250_2010_supercharged_ecm_environment() -> DiagnosticEnvironmentResolution {
    let oem_applicability = trace(
        "record-x250-positive-ecm-marker",
        "evidence-jtb00244nas1-x250-ecm",
        "jlr-jtb00244nas1-2012-02-24",
        SourceType::Documented,
        EvidenceClass::OemDocumentation,
        "Jaguar Technical Bulletin JTB00244NAS1, PDF pages 1-2",
        "AFFECTED VEHICLE RANGE / REPAIR PROCEDURE",
        "XF X250 5.0L MY2010-2012 / ECM",
        ValidationState::SourceBacked,
    );
    let route = trace(
        "record-x250-positive-hs-can-route",
        "evidence-charm-x250-hs-can-dlc-route",
        "charm-jaguar-2010-x250-v8-5.0l-sc",
        SourceType::Documented,
        EvidenceClass::OemDocumentation,
        "Operation CHARM exact 2010 X250 V8-5.0L SC service bundle",
        "CAN bus - high speed - Part 1",
        "C2DB04B/6,C2DB04B/14",
        ValidationState::SourceBacked,
    );
    let bitrate = trace(
        "record-x250-positive-hs-can-route",
        "evidence-charm-x250-hs-can-bitrate",
        "charm-jaguar-2010-x250-v8-5.0L-sc",
        SourceType::Documented,
        EvidenceClass::OemDocumentation,
        "Operation CHARM exact 2010 X250 V8-5.0L SC service bundle",
        "Communications Network / OVERVIEW",
        "High speed CAN bus / Baud Rate",
        ValidationState::SourceBacked,
    );
    let backend = trace(
        "record-x250-positive-backend-route",
        "evidence-mongoose-jlr-hs-can-route",
        "jlr-scanner-mongoose-jlr-routes",
        SourceType::Documented,
        EvidenceClass::SourceCode,
        "JLR Scanner production Mongoose route descriptor",
        "VEHICLE_ROUTES",
        "VehicleRouteId::HsCan",
        ValidationState::SourceBacked,
    );
    let addressing = trace(
        "record-x250-positive-standard-addressing",
        "evidence-elm-iso15765-11bit-addressing",
        "elm-electronics-elm327dsj",
        SourceType::Documented,
        EvidenceClass::StandardDocumentation,
        "ELM327DSJ PDF page 41 of 94",
        "11 bit ISO15765-4 CAN",
        "7DF/7En/7E0-to-7E8",
        ValidationState::SourceBacked,
    );
    let observed = trace(
        "record-x250-positive-observed-response",
        "evidence-x250-obd-fusion-mode09-7e8",
        "public-jaguarforums-x250-obd-fusion-2016-03",
        SourceType::Captured,
        EvidenceClass::DirectObservation,
        "Mode $09 report dated 2016-03-15 and repeated 2016-03-19",
        "Mode $09 - Vehicle Information",
        "VIN/Calibration ID - $7E8",
        ValidationState::CaptureValidated,
    );
    let capability = trace(
        "record-x250-positive-mode09-calibration-id",
        "evidence-bar-mode09-infotype04",
        "california-bar-dad-2012-v2.5",
        SourceType::Documented,
        EvidenceClass::StandardDocumentation,
        "BAR DAD 2012 V2.5 PDF page 14 / requirement 3.2.46",
        "Mode $09 InfoType $04 Calibration Identification",
        "3.2.46",
        ValidationState::SourceBacked,
    );

    let applicability = VehicleApplicability {
        vehicle_program: Some("X250".into()),
        model_year: Some(2010),
        architecture_generation: Some("X250".into()),
        ecu_family: Some(ECU_FAMILY.into()),
        powertrain: Some("AJ133-5.0L-SC".into()),
        variant: Some("Supercharged".into()),
        market: Some("NAS".into()),
        diagnostic_implementation: Some(DIAGNOSTIC_IMPLEMENTATION.into()),
        other: BTreeMap::new(),
    };
    let mut markers = BTreeMap::new();
    markers.insert(
        ImplementationMarkerKind::ModuleAcronym,
        field(
            "ECM".into(),
            vec![oem_applicability.clone()],
            ValidationState::SourceBacked,
        ),
    );
    markers.insert(
        ImplementationMarkerKind::CalibrationId,
        field(
            OBSERVED_CALIBRATION_ID.into(),
            vec![observed.clone()],
            ValidationState::CaptureValidated,
        ),
    );

    DiagnosticEnvironmentResolution::Resolved(DiagnosticEnvironmentPlan {
        vehicle_applicability: field(
            applicability,
            vec![oem_applicability.clone()],
            ValidationState::SourceBacked,
        ),
        ecu_family: field(
            ECU_FAMILY.into(),
            vec![oem_applicability.clone()],
            ValidationState::SourceBacked,
        ),
        diagnostic_implementation: Some(field(
            DIAGNOSTIC_IMPLEMENTATION.into(),
            vec![oem_applicability],
            ValidationState::SourceBacked,
        )),
        logical_network: field(
            "HS-CAN".into(),
            vec![route.clone()],
            ValidationState::SourceBacked,
        ),
        physical_route: field(
            PhysicalDiagnosticRoute {
                connector: "J1962/C2DB04B".into(),
                pins: vec![6, 14],
            },
            vec![route],
            ValidationState::SourceBacked,
        ),
        backend_route: field(
            BackendRoute {
                backend: "mongoose-jlr".into(),
                route_id: "hs-can".into(),
            },
            vec![backend],
            ValidationState::SourceBacked,
        ),
        bitrate_bps: field(500_000, vec![bitrate], ValidationState::SourceBacked),
        protocol_family: field(
            "ISO15765-4 / SAE J1979".into(),
            vec![addressing.clone()],
            ValidationState::SourceBacked,
        ),
        addressing_mode: field(
            "normal_physical".into(),
            vec![addressing.clone()],
            ValidationState::SourceBacked,
        ),
        can_id_format: field(
            CanIdFormat::Standard11Bit,
            vec![addressing.clone()],
            ValidationState::SourceBacked,
        ),
        physical_request_id: field(
            0x7e0,
            vec![addressing.clone()],
            ValidationState::SourceBacked,
        ),
        physical_response_id: field(
            0x7e8,
            vec![observed.clone()],
            ValidationState::CaptureValidated,
        ),
        functional_request_id: Some(field(
            0x7df,
            vec![addressing],
            ValidationState::SourceBacked,
        )),
        read_only_capability: field(
            ReadOnlyCapability {
                id: CAPABILITY_ID.into(),
            },
            vec![capability],
            ValidationState::SourceBacked,
        ),
        implementation_markers: markers,
        validation_state: ValidationState::SourceBacked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_profile_is_exact_and_never_uses_prohibited_pins() {
        let DiagnosticEnvironmentResolution::Resolved(plan) =
            x250_2010_supercharged_ecm_environment()
        else {
            panic!("profile must be resolved")
        };
        assert_eq!(plan.physical_route.value.pins, [6, 14]);
        assert_eq!(plan.bitrate_bps.value, 500_000);
        assert_eq!(plan.physical_request_id.value, 0x7e0);
        assert_eq!(plan.physical_response_id.value, 0x7e8);
        assert!(plan
            .physical_route
            .value
            .pins
            .iter()
            .all(|pin| !matches!(pin, 12 | 13)));
    }
}
