# SDD model-year breakpoints

Research note, 2026-09-02. **Acted on in code under `ADR-0011`**, as an opt-in
derivation. F9 still records every marker verbatim; supplying a
`ModelYearTimeline` additionally expresses it as a calendar year range.

## The question

SDD qualifies diagnostic data by a model-year marker rather than a range:
`CM_YEAR_BREAKPOINT` values such as `MY10` in the DID catalogue, `year="MY06"`
designations in the DTC corpus, and `Post MY10` / `Pre MY10` / `BASE` in the
addressing table. Nothing in the extracted corpus states what a marker spans, so
roughly 108,000 records filter exactly by marker but cannot resolve for a
calendar model year.

## Owner domain knowledge

The owner explained on 2026-09-02 that a marker such as `MY10` denotes a known
pre-facelift transition: the vehicle is no longer the base specification but not
yet the facelift, and the marker is the date that transition falls on.

This is owner-stated domain knowledge, not a citable document. On its own it
would classify as `UnverifiedResearch` under F5 and could not justify
reinterpreting the corpus.

## Structural corroboration from the source itself

The explanation predicts that markers are lifecycle transition points, so the
earliest marker for a program should coincide with that program's launch. That
is checkable against the already-ingested `documented` source.

Extracted from `Fully Qualified DID Formatting.xml`, one line per program:

| Program | Breakpoints | Known history |
| --- | --- | --- |
| X250 Jaguar XF | MY08, MY10, MY12, MY13 | launched 2008; facelift 2012 |
| X351 Jaguar XJ | MY10, MY13, MY16 | launched 2010 |
| X150 Jaguar XK | MY06_5, MY10 | launched mid-2006 |
| X152 F-Type | MY14, MY16 | launched 2013/14 |
| X761 F-Pace | MY17 | launched 2016/17 |
| L319 Discovery | MY05, MY10, MY12, MY14 | Discovery 3 in 2005, Discovery 4 in 2010 |
| L320 RR Sport | MY06, MY10, MY12 | launched 2006 |
| L322 Range Rover | MY06, MY07, MY10 | 2006 powertrain update |
| L405 Range Rover | MY13, MY14, MY16 | launched 2013 |
| L494 RR Sport | MY14, MY16 | launched 2013/14 |
| L538 Evoque | MY12, MY14, MY16, MY17 | launched 2012 |
| L550 Discovery Sport | MY15, MY17 | launched 2015 |

**In all 21 programs the earliest breakpoint coincides with the program's launch
model year.** That is not plausible as coincidence, and it is derived from the
`documented` SDD source rather than from the explanation being tested.

## The boundary follows from the same observation

The explanation alone does not say whether a marker includes its own model year.
The launch-year coincidence settles it: if `MY08` meant "from 2009 onwards",
then a 2008 Jaguar XF — the launch year — would have no diagnostic data at all
in a corpus that plainly covers it.

So a breakpoint applies **from its own model year inclusive, until the next
breakpoint for that program**. Half-year markers such as `MY06_5` and `MY04_5`
are mid-model-year introductions and order between whole years.

## Status and what is still missing

`BREAKPOINT_SEMANTICS = CORROBORATED_NOT_DOCUMENTED`.

The case now rests on owner domain knowledge plus a structural regularity in the
manufacturer's own data, which is considerably stronger than either alone. It is
still not a JLR statement of intent, and two things remain unresolved:

- **`BASE` now has a supported reading, tested 2026-09-02.** See below. It is no
  longer the open question it was, though it remains inferred rather than
  published.
- **Marker-to-calendar-year mapping is assumed, not stated.** `MY08` reading as
  model year 2008 is unambiguous in context, but the corpus spans MY94 to MY17,
  so any implementation must handle two-digit years across a century boundary
  explicitly rather than by adding 2000.

## If this is acted on

Reinterpreting the corpus would change the meaning of roughly 108,000 records,
so it needs its own ADR rather than a quiet parser change. That ADR should:

- derive a per-program ordered breakpoint list from the ingested corpus rather
  than hardcoding one, keeping vehicle programs open data;
- emit `YearConstraint::Range` bounded by the next breakpoint, while **retaining
  the verbatim marker** on its existing dimension so nothing is lost and the
  derivation stays reversible;
- record the derived records at a validation state that reflects their basis,
  never as `Documented`, since the semantics are inferred rather than published;
- treat `BASE` as the state preceding the first breakpoint, per the section
  below, and bound it by that breakpoint rather than leaving it open-ended.

## `BASE`, investigated 2026-09-02

### Web research found nothing

Searches across official JLR material, vendor documentation, GitHub, and the
Jaguar and Land Rover enthusiast forums produced no description of SDD's
qualifier semantics at all. The publicly available material covers what SDD is
and how to run it, never how its diagnostic data is qualified. The reading below
therefore rests entirely on the corpus.

### Only four programs use it

`year="BASE"` appears for exactly four vehicle programs, all from the 1990s:
X300 (800 occurrences), X330, X300H, and X330H (607 each).

### It coexists with specific years rather than replacing them

Grouping every DTC qualifier by module, model, and code:

| Situation | Pairs |
| --- | --- |
| `BASE` alongside specific model years | 57 |
| `BASE` only | 1,637 |
| specific years only, no `BASE` | 15,955 |

A representative case is fault code `B1250` on module `CCM` for X300, which
carries `BASE`, `MY95`, `MY95_5`, `MY95_75`, `MY96`, and `MY97` together.

### Reading

`BASE` is not a wildcard meaning "any model year". If it were, the 57 pairs
where it sits beside `MY95` through `MY97` for the same code would be
self-contradictory. It behaves as **the first state in the sequence**: the base
specification, applying from launch until the first breakpoint, or throughout
where a program declares no breakpoints at all.

That is the same lifecycle the owner described — base, then pre-facelift, then
facelift — seen from its beginning. It also explains why only 1990s programs use
it: SDD models those cars as a base specification with progressive updates
rather than with the breakpoint scheme used for later platforms.

`BASE_SEMANTICS = CORROBORATED_NOT_DOCUMENTED`, on the same footing as the
breakpoint reading. In `helpScreenQualifier` the same token appears as
`model="BASE" year="BASE"`, where it plainly means a default fallback; whether
that is the same concept or a reused token is untested and does not affect the
`dtcQualifier` reading above.

## Related: `.exml` decryption exists publicly

Not acted on, recorded because the research surfaced it.

Public tooling decrypts JLR `.exml` files — `smartgauges/exml` does so by
intercepting `CryptImportKey` through a proxy DLL, and at least one other
project embeds keys extracted from SDD. This is material, because the encrypted
`.exml` instances hold the MDX `PROTOCOL` sections carrying per-ECU bus and
addressing detail that the plain-XML corpus does not supply.

`EXML_DECRYPTION` remains `OUT_OF_SCOPE` per `docs/research/sdd/PROVENANCE.md`.
The technique circumvents a technical protection measure on proprietary data,
its legality varies by jurisdiction and is not something this document assesses,
and nothing in F9 depends on it. Changing that status is an owner decision and
would need its own ADR; no assistant should take it as implied by this note.

## Prose markers carry a real engineering change

`Pre MY10` and `Post MY10` appear only in the addressing table, for L319, L320
and L322. In all three they separate **29-bit CAN addressing before MY10 from
11-bit from MY10**, which is a genuine platform transition rather than a
documentation nicety.

All three programs also declare `MY10` as a breakpoint in the DID catalogue, so
the two files describe the same transition from different sides. That is further
corroboration of the breakpoint reading, obtained without assuming it.

The pair must partition exactly: a vehicle at the boundary needs exactly one
addressing width. `Pre` therefore runs to the year before and `Post` from the
marker year inclusive, which is the same inclusivity the launch-year argument
established. A boundary placed one year out would put the wrong CAN addressing
width on a real car.
