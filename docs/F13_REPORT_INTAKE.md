# F13 — reports become evidence (milestone M2, the intake)

Status: `IMPLEMENTED / FIXTURE_TESTED`. No report from a vehicle exists yet,
so nothing here is `HARDWARE_CONFIRMED` or `VEHICLE_CONFIRMED`; the tool is
what will make those words true when the first report arrives. The rules are
`docs/adr/ADR-0016-session-reports-as-captured-evidence.md`.

## What the intake does

`crates/report-intake` reads one session report — the bundle the application
saves (`jlr-scanner.session-report`, version 1) — together with the data
library, and writes one F5 manifest:

- source: `SourceType::Captured`, fingerprinted with the report's SHA-256,
  dated from the report, declaring the programme(s) it describes; never the
  VIN, never the adapter's serial number;
- evidence: one `DirectObservation` per record, located at the report item
  (`module_reads[3]`, `captures[0]`, …) with the exchange as excerpt and the
  report's timestamp;
- records: `CaptureValidated`, applicable to the reported vehicle — the
  programme, the model year, and the engine and breakpoint marker when the
  report states them — and no wider.

```text
cargo run -p report-intake --example intake -- <report.json> <library dir> [out dir]
```

The manifest is named after its source (`capture-session-<16 hex>.json`) and
written into the library directory by default, where the application's
loader reads it like any other. Removing the file removes the evidence.

## What a report establishes

| Report item | Outcome | Records |
| --- | --- | --- |
| module read answered by the expected responder, positive response | confirmation | the module's diagnostic addressing, its bus, the bus's physical route and adapter route, its protocol family, the capability exercised; plus a `captured_read` observation |
| module read answered by the expected responder, negative response | confirmation, capability excepted | addressing, bus, physical route, adapter route, protocol; plus the observation |
| module read answered from an unexpected identifier | recorded as seen | addressing with the observed responder — the store reports the disagreement and the survey shows `CONFLICT` |
| module read with no answer or an error | observation only | `captured_read_attempt` naming route, request and outcome |
| listen-only capture | observation only | `captured_bus_activity` on the programme: frames, distinct identifiers, top identifiers, route |
| F8 calibration read with a decoded identifier | observation only | `captured_calibration_id` on the target module |

Confirmed values are copied from the library records the plan resolved for
that vehicle, so a confirmation agrees with the data exactly and the store
sees one value with two kinds of evidence, not two values. When the library
holds no single value for a field — none, or several that disagree — nothing
is confirmed for it and the summary says so.

Fault codes are not recorded as knowledge: they are the state of one car on
one day. That the module answered service 19 is.

## What changes in the survey

Two rules of the resolver changed with ADR-0016, and the golden tests pin
them:

- records that agree in value make a field as validated as the best of them
  (`CaptureValidated` beats `Unverified`); the plan as a whole still reports
  the weakest of its fields;
- a route is a hypothesis only while every trace behind its physical and
  backend route is unverified research. One direct observation ends it: the
  survey shows the route `REACHABLE` with `CAPTURE_VALIDATED`, the dashed
  lane on the map becomes solid, and the fleet counts move from "on a
  hypothesised route" to "reachable" for that programme and model year.

## Acceptance

`crates/report-intake/tests/f13_intake.rs`, on synthetic data only: a
synthetic module on `PT_HSCAN` (bound to hs-can only by the ADR-0015
hypothesis) is a hypothesis before any report; an answered read's manifest
makes it reachable and capture-validated with six facts confirmed; silence
and a capture yield two observations and change nothing; an unexpected
responder surfaces as a conflict; a report of another schema, or with nothing
to record, is refused. Two unit tests cover the date and identifier helpers.

## What this does not do yet

- **Read the report inside the application.** The intake is a command-line
  tool for the programme's maintainer; an "import a report" action in the
  application is a later step, once reports exist to import.
- **Aggregate across reports.** Every manifest applies to one vehicle. How
  many confirmations make a route documented for a programme, and how many
  silences refute one, is a judgement the documents will record when there
  are reports to judge.
- **Confirm identifiers.** A positive service 22 answer confirms the read
  capability and the addressing; that the identifier's decoded value is
  right is for F13's tester to compare with a known quantity, which is also
  the path to the map-converter scale (`F11_TESTER_APPLICATION.md`).
