use crate::KnowledgeError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum DimensionConstraint {
    #[default]
    Unknown,
    Any,
    OneOf {
        values: Vec<String>,
    },
}

impl DimensionConstraint {
    pub fn one_of(values: impl IntoIterator<Item = String>) -> Result<Self, KnowledgeError> {
        let mut values: Vec<_> = values.into_iter().collect();
        values.sort();
        values.dedup();
        let constraint = Self::OneOf { values };
        constraint.validate()?;
        Ok(constraint)
    }

    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if let Self::OneOf { values } = self {
            if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
                return Err(KnowledgeError::InvalidApplicability(
                    "one_of requires non-empty values".into(),
                ));
            }
            if values.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(KnowledgeError::InvalidApplicability(
                    "one_of values must be sorted and unique".into(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn mentions(&self, expected: &str) -> bool {
        matches!(self, Self::OneOf { values } if values.iter().any(|value| value == expected))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum YearConstraint {
    #[default]
    Unknown,
    Any,
    Range {
        model_year_from: Option<u16>,
        model_year_to: Option<u16>,
    },
}

impl YearConstraint {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if let Self::Range {
            model_year_from,
            model_year_to,
        } = self
        {
            if model_year_from.is_none() && model_year_to.is_none() {
                return Err(KnowledgeError::InvalidApplicability(
                    "year range requires at least one bound".into(),
                ));
            }
            if let (Some(from), Some(to)) = (model_year_from, model_year_to) {
                if from > to {
                    return Err(KnowledgeError::InvalidApplicability(
                        "model_year_from exceeds model_year_to".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Applicability {
    #[serde(default)]
    pub vehicle_program: DimensionConstraint,
    #[serde(default)]
    pub model_year: YearConstraint,
    #[serde(default)]
    pub architecture_generation: DimensionConstraint,
    #[serde(default)]
    pub ecu_family: DimensionConstraint,
    #[serde(default)]
    pub powertrain: DimensionConstraint,
    #[serde(default)]
    pub variant: DimensionConstraint,
    #[serde(default)]
    pub market: DimensionConstraint,
    #[serde(default)]
    pub diagnostic_implementation: DimensionConstraint,
    #[serde(default)]
    pub other: BTreeMap<String, DimensionConstraint>,
}

impl Applicability {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        self.model_year.validate()?;
        for constraint in [
            &self.vehicle_program,
            &self.architecture_generation,
            &self.ecu_family,
            &self.powertrain,
            &self.variant,
            &self.market,
            &self.diagnostic_implementation,
        ] {
            constraint.validate()?;
        }
        for (name, constraint) in &self.other {
            if name.trim().is_empty() {
                return Err(KnowledgeError::InvalidApplicability(
                    "custom dimension name must not be empty".into(),
                ));
            }
            constraint.validate()?;
        }
        Ok(())
    }

    pub fn resolve(&self, context: &VehicleContext) -> ApplicabilityResolution {
        let mut insufficient_context = false;
        let mut insufficient_evidence = false;
        let mut observe = |result| match result {
            DimensionResolution::Matched => true,
            DimensionResolution::NotMatched => false,
            DimensionResolution::MissingContext => {
                insufficient_context = true;
                true
            }
            DimensionResolution::UnknownEvidence => {
                insufficient_evidence = true;
                true
            }
        };

        if !observe(resolve_text(
            &self.vehicle_program,
            context.vehicle_program.as_deref(),
        )) || !observe(resolve_year(&self.model_year, context.model_year))
            || !observe(resolve_text(
                &self.architecture_generation,
                context.architecture_generation.as_deref(),
            ))
            || !observe(resolve_text(
                &self.ecu_family,
                context.ecu_family.as_deref(),
            ))
            || !observe(resolve_text(
                &self.powertrain,
                context.powertrain.as_deref(),
            ))
            || !observe(resolve_text(&self.variant, context.variant.as_deref()))
            || !observe(resolve_text(&self.market, context.market.as_deref()))
            || !observe(resolve_text(
                &self.diagnostic_implementation,
                context.diagnostic_implementation.as_deref(),
            ))
        {
            return ApplicabilityResolution::NotApplicable;
        }

        for (name, constraint) in &self.other {
            if !observe(resolve_text(
                constraint,
                context.other.get(name).map(String::as_str),
            )) {
                return ApplicabilityResolution::NotApplicable;
            }
        }

        if insufficient_evidence {
            ApplicabilityResolution::InsufficientEvidence
        } else if insufficient_context {
            ApplicabilityResolution::InsufficientContext
        } else {
            ApplicabilityResolution::Applicable
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VehicleContext {
    pub vehicle_program: Option<String>,
    pub model_year: Option<u16>,
    pub architecture_generation: Option<String>,
    pub ecu_family: Option<String>,
    pub powertrain: Option<String>,
    pub variant: Option<String>,
    pub market: Option<String>,
    pub diagnostic_implementation: Option<String>,
    pub other: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityResolution {
    Applicable,
    NotApplicable,
    InsufficientContext,
    InsufficientEvidence,
}

#[derive(Clone, Copy)]
enum DimensionResolution {
    Matched,
    NotMatched,
    MissingContext,
    UnknownEvidence,
}

fn resolve_text(constraint: &DimensionConstraint, context: Option<&str>) -> DimensionResolution {
    match constraint {
        DimensionConstraint::Unknown => DimensionResolution::UnknownEvidence,
        DimensionConstraint::Any => DimensionResolution::Matched,
        DimensionConstraint::OneOf { values } => match context {
            Some(value) if values.iter().any(|expected| expected == value) => {
                DimensionResolution::Matched
            }
            Some(_) => DimensionResolution::NotMatched,
            None => DimensionResolution::MissingContext,
        },
    }
}

fn resolve_year(constraint: &YearConstraint, year: Option<u16>) -> DimensionResolution {
    match constraint {
        YearConstraint::Unknown => DimensionResolution::UnknownEvidence,
        YearConstraint::Any => DimensionResolution::Matched,
        YearConstraint::Range {
            model_year_from,
            model_year_to,
        } => match year {
            Some(year)
                if model_year_from.map_or(true, |from| year >= from)
                    && model_year_to.map_or(true, |to| year <= to) =>
            {
                DimensionResolution::Matched
            }
            Some(_) => DimensionResolution::NotMatched,
            None => DimensionResolution::MissingContext,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_applicability() -> Applicability {
        Applicability {
            vehicle_program: DimensionConstraint::one_of(vec!["L320".into(), "X351".into()])
                .unwrap(),
            model_year: YearConstraint::Range {
                model_year_from: Some(2010),
                model_year_to: Some(2013),
            },
            architecture_generation: DimensionConstraint::Any,
            ecu_family: DimensionConstraint::one_of(vec!["ECU-A".into()]).unwrap(),
            powertrain: DimensionConstraint::Any,
            variant: DimensionConstraint::one_of(vec!["V8".into()]).unwrap(),
            market: DimensionConstraint::Any,
            diagnostic_implementation: DimensionConstraint::one_of(vec!["IMPL-A".into()]).unwrap(),
            other: BTreeMap::new(),
        }
    }

    fn context(program: &str, year: u16) -> VehicleContext {
        VehicleContext {
            vehicle_program: Some(program.into()),
            model_year: Some(year),
            architecture_generation: None,
            ecu_family: Some("ECU-A".into()),
            powertrain: None,
            variant: Some("V8".into()),
            market: None,
            diagnostic_implementation: Some("IMPL-A".into()),
            other: BTreeMap::new(),
        }
    }

    #[test]
    fn resolves_year_multiple_programs_ecu_variant_and_one_implementation() {
        let applicability = exact_applicability();
        assert_eq!(
            applicability.resolve(&context("X351", 2011)),
            ApplicabilityResolution::Applicable
        );
        assert_eq!(
            applicability.resolve(&context("L320", 2012)),
            ApplicabilityResolution::Applicable
        );
        assert_eq!(
            applicability.resolve(&context("X351", 2014)),
            ApplicabilityResolution::NotApplicable
        );
        let mut wrong_ecu = context("X351", 2011);
        wrong_ecu.ecu_family = Some("ECU-B".into());
        assert_eq!(
            applicability.resolve(&wrong_ecu),
            ApplicabilityResolution::NotApplicable
        );
    }

    #[test]
    fn unknown_is_not_any_and_missing_context_is_not_exact_match() {
        let mut applicability = exact_applicability();
        applicability.market = DimensionConstraint::Unknown;
        assert_eq!(
            applicability.resolve(&context("X351", 2011)),
            ApplicabilityResolution::InsufficientEvidence
        );
        applicability.market = DimensionConstraint::one_of(vec!["EU".into()]).unwrap();
        assert_eq!(
            applicability.resolve(&context("X351", 2011)),
            ApplicabilityResolution::InsufficientContext
        );
        applicability.market = DimensionConstraint::Any;
        assert_eq!(
            applicability.resolve(&context("X351", 2011)),
            ApplicabilityResolution::Applicable
        );
    }

    #[test]
    fn invalid_ranges_and_unsorted_values_are_rejected() {
        assert!(YearConstraint::Range {
            model_year_from: Some(2015),
            model_year_to: Some(2010)
        }
        .validate()
        .is_err());
        assert!(DimensionConstraint::OneOf {
            values: vec!["B".into(), "A".into()]
        }
        .validate()
        .is_err());
    }
}
