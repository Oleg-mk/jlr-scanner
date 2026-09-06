//! Resolution of SDD model-year markers into calendar year ranges.
//!
//! See `ADR-0011` and `docs/research/sdd/MODEL_YEAR_BREAKPOINTS.md`. The
//! readings implemented here are `CORROBORATED_NOT_DOCUMENTED`: they rest on
//! owner domain knowledge plus a structural regularity in JLR's own data, not
//! on a JLR statement. Use is therefore opt-in.

use knowledge::{KnowledgeError, YearConstraint};
use std::collections::{BTreeMap, BTreeSet};

/// Marker denoting the state before a program's first breakpoint.
pub const BASE_MARKER: &str = "BASE";

/// First and last model year SDD covers.
///
/// SDD spans roughly 1994 to 2017 and was superseded by Pathfinder; the
/// extracted corpus is stamped 2019 and contains only `MY94` through `MY17`.
/// Because the window is closed, a two-digit year is validated against it
/// rather than being mapped by an assumed century rule.
pub const FIRST_MODEL_YEAR: u16 = 1994;
pub const LAST_MODEL_YEAR: u16 = 2017;

/// An SDD model-year marker parsed into a sortable point.
///
/// `MY95_5` denotes a mid-model-year introduction, so it orders after `MY95`
/// and before `MY96` while resolving to the same calendar year as `MY95`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelYearPoint {
    pub year: u16,
    /// Hundredths of a model year, so `MY95_75` is `75`.
    pub fraction: u16,
}

/// Parse a marker such as `MY10`, `MY06_5`, `Pre MY10` or `Post MY10`.
///
/// Returns `None` for `BASE` and for prose markers whose span is a boundary
/// rather than a point; those are handled by the timeline, not here.
pub fn parse_marker(marker: &str) -> Result<Option<ModelYearPoint>, KnowledgeError> {
    let trimmed = marker.trim();
    if trimmed.eq_ignore_ascii_case(BASE_MARKER) {
        return Ok(None);
    }
    // "Pre MY10" and "Post MY10" describe a side of a boundary, not a point.
    let Some(rest) = trimmed
        .strip_prefix("MY")
        .or_else(|| trimmed.strip_prefix("my"))
    else {
        return Ok(None);
    };

    let (digits, fraction) = match rest.split_once('_') {
        Some((digits, fraction)) => {
            let parsed = fraction.parse::<u16>().map_err(|_| {
                KnowledgeError::Parse(format!("model-year marker '{marker}' has a bad fraction"))
            })?;
            // SDD writes quarters as 25, 5 and 75; normalise 5 to 50 so the
            // ordering of MY95_5 against MY95_75 is arithmetic, not textual.
            let normalised = if fraction.len() == 1 {
                parsed * 10
            } else {
                parsed
            };
            if normalised >= 100 {
                return Err(KnowledgeError::Parse(format!(
                    "model-year marker '{marker}' has an out-of-range fraction"
                )));
            }
            (digits, normalised)
        }
        None => (rest, 0),
    };

    let two_digit = digits.parse::<u16>().map_err(|_| {
        KnowledgeError::Parse(format!(
            "model-year marker '{marker}' has no two-digit year"
        ))
    })?;
    if digits.len() != 2 {
        return Err(KnowledgeError::Parse(format!(
            "model-year marker '{marker}' must carry exactly two year digits"
        )));
    }

    // The window is closed, so the century is resolved by validation rather
    // than by an assumed rule. A year outside it is rejected, not guessed.
    let year = if two_digit >= FIRST_MODEL_YEAR % 100 {
        1900 + two_digit
    } else {
        2000 + two_digit
    };
    if !(FIRST_MODEL_YEAR..=LAST_MODEL_YEAR).contains(&year) {
        return Err(KnowledgeError::Parse(format!(
            "model-year marker '{marker}' resolves to {year}, outside the SDD window {FIRST_MODEL_YEAR}-{LAST_MODEL_YEAR}"
        )));
    }
    Ok(Some(ModelYearPoint { year, fraction }))
}

/// A marker naming a side of a boundary rather than a point on the timeline.
///
/// SDD's addressing table uses these to record a genuine engineering
/// transition: L319, L320 and L322 all read `Pre MY10` as 29-bit CAN addressing
/// and `Post MY10` as 11-bit. Getting the boundary wrong would mean using the
/// wrong addressing width on a real vehicle, so the two sides must partition
/// the range exactly, with no gap and no overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeMarker {
    /// `Pre MY10` — model years strictly before the boundary.
    Before(ModelYearPoint),
    /// `Post MY10` — the boundary's own model year and later.
    ///
    /// Inclusive for the same reason a breakpoint is: if `Post MY10` began in
    /// 2011, a 2010 vehicle would have no declared addressing width at all.
    From(ModelYearPoint),
}

/// Parse `Pre MY10` or `Post MY10`.
///
/// Returns `None` for anything that is not such a marker, including plain
/// points and `BASE`.
pub fn parse_relative_marker(marker: &str) -> Result<Option<RelativeMarker>, KnowledgeError> {
    let trimmed = marker.trim();
    let lower = trimmed.to_ascii_lowercase();
    let (rest, is_post) = if let Some(rest) = lower.strip_prefix("post") {
        (rest, true)
    } else if let Some(rest) = lower.strip_prefix("pre") {
        (rest, false)
    } else {
        return Ok(None);
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(None);
    }
    // Reuse the point parser so the closed-window validation applies here too.
    let Some(point) = parse_marker(rest)? else {
        return Err(KnowledgeError::Parse(format!(
            "relative model-year marker '{marker}' names no usable boundary"
        )));
    };
    Ok(Some(if is_post {
        RelativeMarker::From(point)
    } else {
        RelativeMarker::Before(point)
    }))
}

/// Ordered model-year markers per vehicle program, built by observing a corpus.
///
/// Programs are never hardcoded. A caller collects markers in a first pass, then
/// supplies the finished timeline to the adapters for a second pass.
#[derive(Clone, Debug, Default)]
pub struct ModelYearTimeline {
    observed: BTreeMap<String, BTreeSet<ModelYearPoint>>,
}

impl ModelYearTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `program` declares `marker`.
    ///
    /// Unparsable markers are an error rather than a silent skip, so a corpus
    /// carrying an unexpected form is noticed instead of quietly losing years.
    pub fn observe(&mut self, program: &str, marker: &str) -> Result<(), KnowledgeError> {
        let program = program.trim();
        if program.is_empty() {
            return Err(KnowledgeError::Parse(
                "a model-year marker was observed without a vehicle program".into(),
            ));
        }
        if marker.trim().eq_ignore_ascii_case(BASE_MARKER) {
            // BASE is not a point on the timeline; it is bounded by the first
            // breakpoint at resolution time.
            return Ok(());
        }
        if let Some(point) = parse_marker(marker)? {
            self.observed
                .entry(program.to_string())
                .or_default()
                .insert(point);
        }
        Ok(())
    }

    pub fn programs(&self) -> impl Iterator<Item = &str> {
        self.observed.keys().map(String::as_str)
    }

    pub fn points(&self, program: &str) -> Option<impl Iterator<Item = &ModelYearPoint>> {
        self.observed.get(program).map(|set| set.iter())
    }

    /// Calendar-year constraint implied by `marker` for `program`.
    ///
    /// Returns `None` when the timeline cannot bound the marker, in which case
    /// the caller leaves `model_year` unknown rather than guessing.
    pub fn range_for(
        &self,
        program: &str,
        marker: &str,
    ) -> Result<Option<YearConstraint>, KnowledgeError> {
        // A relative marker carries its own boundary, so it needs no sequence.
        if let Some(relative) = parse_relative_marker(marker)? {
            return Ok(Some(match relative {
                RelativeMarker::Before(point) if point.year > FIRST_MODEL_YEAR => {
                    YearConstraint::Range {
                        model_year_from: None,
                        model_year_to: Some(point.year - 1),
                    }
                }
                // Nothing precedes the first year SDD covers.
                RelativeMarker::Before(_) => return Ok(None),
                RelativeMarker::From(point) => YearConstraint::Range {
                    model_year_from: Some(point.year),
                    model_year_to: None,
                },
            }));
        }

        let points = match self.observed.get(program.trim()) {
            Some(points) if !points.is_empty() => points,
            // Without a breakpoint sequence there is nothing to bound against.
            _ => return Ok(None),
        };

        if marker.trim().eq_ignore_ascii_case(BASE_MARKER) {
            let first = points.iter().next().expect("checked non-empty");
            // BASE precedes the first breakpoint. The source states no launch
            // year, so the range stays open below.
            if first.year <= FIRST_MODEL_YEAR {
                return Ok(None);
            }
            return Ok(Some(YearConstraint::Range {
                model_year_from: None,
                model_year_to: Some(first.year - 1),
            }));
        }

        let Some(point) = parse_marker(marker)? else {
            return Ok(None);
        };
        if !points.contains(&point) {
            return Ok(None);
        }

        // The next breakpoint in a strictly later calendar year closes the
        // range. A later marker inside the same year does not, because a whole
        // year cannot express a mid-year split.
        let next = points
            .iter()
            .find(|candidate| candidate.year > point.year)
            .map(|candidate| candidate.year - 1);
        Ok(Some(YearConstraint::Range {
            model_year_from: Some(point.year),
            model_year_to: next,
        }))
    }
}
