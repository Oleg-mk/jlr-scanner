# ADR-0011: Derive model-year ranges from SDD breakpoints, opt-in

- Status: Accepted
- Decision: Add an opt-in `ModelYearTimeline` to `sdd-ingest`. When a caller
  supplies one, SDD model-year markers are additionally expressed as a
  `YearConstraint::Range`; the verbatim marker is always retained on its
  existing custom dimension. Without a timeline the adapters behave exactly as
  before, leaving `model_year` unknown.
- Reason: Roughly 108,000 ingested records filter exactly by an SDD marker such
  as `MY10` but cannot answer a question asked in calendar model years, which is
  how a vehicle is actually identified. `docs/research/sdd/MODEL_YEAR_BREAKPOINTS.md`
  establishes two readings as `CORROBORATED_NOT_DOCUMENTED`: a breakpoint
  applies from its own model year inclusive until the next breakpoint for that
  program, and `BASE` is the state preceding the first breakpoint. The case rests
  on owner domain knowledge plus a structural regularity in JLR's own data —
  in all 21 vehicle programs the earliest breakpoint coincides with the
  program's launch model year — not on any JLR statement.

  Two-digit years resolve without guesswork because the platform's window is
  closed. SDD covers roughly 1994 to 2017 and was superseded by Pathfinder; the
  extracted corpus is stamped 2019 and contains only `MY94` through `MY17`. The
  implementation therefore **validates** rather than assumes: `94`–`99` resolve
  to `19xx`, `00`–`17` to `20xx`, and anything outside that closed range is
  rejected. A year cannot be silently misread by a century.

- Consequence: The inference is opt-in, so it is a deliberate act by a caller
  rather than a property of the parser. Nothing changes for existing callers.

  The derived range is a re-expression of what the marker already says under a
  recorded interpretation, not an additional claim about the vehicle, so
  validation state is unchanged. The verbatim marker is always retained, which
  keeps the derivation reversible and lets a future correction re-derive rather
  than re-ingest. This reasoning is stated here so a reviewer can disagree with
  it knowingly.

  Timelines are built by observing the corpus, never hardcoded, so vehicle
  programs remain open data as `ADR-0004` and the F5 rules require.

  Half-year markers such as `MY95_5` order correctly but coarsen to a whole
  calendar year, so two markers inside one year yield the same range. That is
  correct at year granularity — both genuinely apply to part of that year — and
  the marker remains the finer discriminator.

  `BASE` yields a range bounded above by the first breakpoint and open below,
  because the source states no launch year. Where a program declares no
  breakpoints at all, `BASE` stays unresolved rather than becoming an unbounded
  range.

  If the readings are ever contradicted by evidence, this ADR is superseded and
  the timeline is rebuilt; no ingested record needs to be discarded, because the
  markers were never overwritten.
