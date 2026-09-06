//! F9 golden tests for SDD model-year resolution.
//!
//! See `ADR-0011`. The readings under test are corroborated, not documented, so
//! the tests fix both what is derived and what is deliberately refused.

use knowledge::YearConstraint;
use sdd_ingest::{
    parse_marker, parse_relative_marker, ModelYearTimeline, RelativeMarker, FIRST_MODEL_YEAR,
    LAST_MODEL_YEAR,
};

fn build(program: &str, markers: &[&str]) -> ModelYearTimeline {
    let mut timeline = ModelYearTimeline::new();
    for marker in markers {
        timeline.observe(program, marker).unwrap();
    }
    timeline
}

#[test]
fn two_digit_years_resolve_by_validation_not_by_an_assumed_century() {
    // SDD's window is closed, so both halves of it resolve unambiguously.
    assert_eq!(parse_marker("MY94").unwrap().unwrap().year, 1994);
    assert_eq!(parse_marker("MY99").unwrap().unwrap().year, 1999);
    assert_eq!(parse_marker("MY00").unwrap().unwrap().year, 2000);
    assert_eq!(parse_marker("MY17").unwrap().unwrap().year, 2017);

    // Anything outside the window is rejected rather than guessed. Without this
    // a 1990s Jaguar could silently land in the 2090s.
    for outside in ["MY18", "MY30", "MY93", "MY85"] {
        let error = parse_marker(outside).expect_err("outside the SDD window");
        assert!(
            error.to_string().contains("outside the SDD window"),
            "{outside}: {error}"
        );
    }
    assert_eq!(FIRST_MODEL_YEAR, 1994);
    assert_eq!(LAST_MODEL_YEAR, 2017);
}

#[test]
fn half_year_markers_order_arithmetically_within_their_year() {
    let quarter = parse_marker("MY95_25").unwrap().unwrap();
    let half = parse_marker("MY95_5").unwrap().unwrap();
    let three_quarter = parse_marker("MY95_75").unwrap().unwrap();
    let whole = parse_marker("MY95").unwrap().unwrap();
    let next = parse_marker("MY96").unwrap().unwrap();

    // "_5" is a half, not a five, so it must sort between _25 and _75.
    assert!(whole < quarter);
    assert!(quarter < half);
    assert!(half < three_quarter);
    assert!(three_quarter < next);
    assert_eq!(half.year, 1995);
}

#[test]
fn a_breakpoint_runs_until_the_next_one_and_the_last_stays_open() {
    // X250's real sequence: an XF launched in 2008 and facelifted in 2012.
    let timeline = build("X250", &["MY08", "MY10", "MY12", "MY13"]);

    assert_eq!(
        timeline.range_for("X250", "MY08").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: Some(2008),
            model_year_to: Some(2009)
        })
    );
    assert_eq!(
        timeline.range_for("X250", "MY10").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: Some(2011)
        })
    );
    // The final breakpoint has nothing after it, so it stays open above rather
    // than being closed at an invented year.
    assert_eq!(
        timeline.range_for("X250", "MY13").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: Some(2013),
            model_year_to: None
        })
    );
}

#[test]
fn markers_inside_one_year_share_a_range_because_a_year_cannot_split() {
    let timeline = build("X150", &["MY06_5", "MY10"]);
    assert_eq!(
        timeline.range_for("X150", "MY06_5").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: Some(2006),
            model_year_to: Some(2009)
        })
    );

    // Two markers in the same calendar year coarsen to the same range. That is
    // correct at year granularity; the verbatim marker stays the finer key.
    let dense = build("X300", &["MY95", "MY95_5", "MY96"]);
    let whole = dense.range_for("X300", "MY95").unwrap();
    let half = dense.range_for("X300", "MY95_5").unwrap();
    assert_eq!(whole, half);
    assert_eq!(
        whole,
        Some(YearConstraint::Range {
            model_year_from: Some(1995),
            model_year_to: Some(1995)
        })
    );
}

#[test]
fn base_is_bounded_above_by_the_first_breakpoint_and_open_below() {
    let timeline = build("X300", &["MY95", "MY96", "MY97"]);

    // The source states no launch year, so nothing is invented below.
    assert_eq!(
        timeline.range_for("X300", "BASE").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: None,
            model_year_to: Some(1994)
        })
    );

    // A program whose first breakpoint is the earliest year SDD covers leaves
    // no room beneath it, so BASE stays unresolved instead of empty.
    let earliest = build("XJS", &["MY94"]);
    assert_eq!(earliest.range_for("XJS", "BASE").unwrap(), None);
}

#[test]
fn nothing_is_derived_without_a_sequence_to_bound_against() {
    let timeline = build("X250", &["MY08", "MY10"]);

    // An unobserved program has no timeline, so no range is invented.
    assert_eq!(timeline.range_for("L322", "MY06").unwrap(), None);
    // A marker the program never declares is not placed in its sequence.
    assert_eq!(timeline.range_for("X250", "MY12").unwrap(), None);
    // BASE without any breakpoint has no upper bound to take.
    assert_eq!(
        ModelYearTimeline::new().range_for("X250", "BASE").unwrap(),
        None
    );
}

#[test]
fn prose_markers_partition_the_range_exactly() {
    // L319, L320 and L322 use these to separate 29-bit from 11-bit CAN
    // addressing, so the two sides must meet without a gap or an overlap. A
    // vehicle at the boundary must get exactly one addressing width.
    let timeline = ModelYearTimeline::new();

    assert_eq!(
        timeline.range_for("L319", "Pre MY10").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: None,
            model_year_to: Some(2009)
        })
    );
    assert_eq!(
        timeline.range_for("L319", "Post MY10").unwrap(),
        Some(YearConstraint::Range {
            model_year_from: Some(2010),
            model_year_to: None
        })
    );

    // The boundary year itself belongs to the later side, for the same reason a
    // breakpoint is inclusive: otherwise a 2010 vehicle would have no declared
    // addressing width at all.
    assert_eq!(
        parse_relative_marker("Post MY10").unwrap(),
        Some(RelativeMarker::From(parse_marker("MY10").unwrap().unwrap()))
    );
    assert_eq!(
        parse_relative_marker("Pre MY10").unwrap(),
        Some(RelativeMarker::Before(
            parse_marker("MY10").unwrap().unwrap()
        ))
    );

    // A relative marker needs no program sequence, but its year is still
    // validated against the closed window.
    assert!(timeline.range_for("L319", "Post MY30").is_err());
    // Nothing precedes the first year SDD covers.
    assert_eq!(timeline.range_for("L319", "Pre MY94").unwrap(), None);
}

#[test]
fn malformed_input_is_refused_rather_than_approximated() {
    // A relative marker is not a point, so the point parser declines it.
    assert_eq!(parse_marker("Post MY10").unwrap(), None);
    assert_eq!(parse_marker("Pre MY10").unwrap(), None);
    assert_eq!(parse_marker("BASE").unwrap(), None);
    assert_eq!(parse_relative_marker("MY10").unwrap(), None);
    assert!(parse_relative_marker("Post ").unwrap().is_none());
    assert!(parse_relative_marker("Post nonsense").is_err());

    assert!(parse_marker("MY1").is_err());
    assert!(parse_marker("MY100").is_err());
    assert!(parse_marker("MY10_abc").is_err());

    let mut timeline = ModelYearTimeline::new();
    assert!(timeline.observe("", "MY10").is_err());
    assert!(timeline.observe("X250", "MY99999").is_err());
}
