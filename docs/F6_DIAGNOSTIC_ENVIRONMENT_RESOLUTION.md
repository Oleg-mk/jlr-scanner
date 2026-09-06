# F6 Evidence-Backed Diagnostic Environment Resolution

## Status and boundary

F6 status: **PASS / IMPLEMENTED / REAL_X250_POSITIVE_GOLDEN_TESTED /
CI_TESTED / NO_VEHICLE_INTERACTION**.

The offline resolver, typed resolution states, real X250 positive and CCP
negative goldens, synthetic conflict and multi-program coverage, field-level
provenance, and architecture guards are implemented. The positive golden uses
a reproducibly fingerprinted, field-specific source chain; its public-index
observation limitation is retained explicitly.

F6 produces data only. It cannot open Mongoose or CAN, transmit a frame,
construct a UDS request, call diagnostics-core, or execute a vehicle operation.
No vehicle or native Tauri application is required.

## Architecture

    caller
       |
       v
    diagnostic-environment
       |
       v
    knowledge

The new crate depends only on knowledge. Knowledge, diagnostics-core, ISO-TP,
UDS, transport, Mongoose, and the frontend do not depend on it. F4 remains a
vehicle-independent protocol path accepting caller-supplied route, response ID,
and correlation request.

The primary invariant remains:

> **JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI**

## Query model

DiagnosticEnvironmentQuery contains an explicit F5 VehicleContext and a typed
DiagnosticTarget. A target can identify an ECU family plus diagnostic
implementation and read-only capability, or directly identify one knowledge
entity for research/incomplete-resolution queries. VIN decoding is not part of
F6.

Vehicle-program IDs remain open strings. The resolver has no X250 production
branch or closed list of JLR programs.

## Plan model

A complete CAN/ISO-TP DiagnosticEnvironmentPlan carries individually traced:

- vehicle applicability;
- ECU family and diagnostic implementation;
- logical network;
- physical connector and pins;
- evidence-backed backend route descriptor;
- bitrate;
- protocol family;
- addressing mode;
- 11-bit or 29-bit CAN ID format;
- physical request and response IDs;
- optional functional request ID;
- a supported READ_ONLY capability;
- optional typed implementation markers such as calibration ID;
- the weakest actual validation state across the selected facts; within one
  fact, records that agree in value make it as validated as the best of them
  (ADR-0016).

The CAN ID format is explicit and validated. An 11-bit value cannot exceed
0x7FF and a 29-bit value cannot exceed 0x1FFFFFFF. No bitrate, address, protocol,
or route is defaulted.

Future non-CAN environments are not forced through this plan. They require a
separate protocol-appropriate plan model rather than nullable CAN fields
misrepresented as universal diagnostics.

## Resolution states

- RESOLVED: every mandatory field has one mutually compatible, applicable,
  evidence-backed value and explicit evidence classification.
- INDETERMINATE: one or more mandatory facts, applicability dimensions, or
  evidence classifications are missing. Exact UnresolvedFact values are
  returned with a partial data view.
- CONFLICT: two or more applicable values disagree for the same plan field.
  All candidate values and their evidence are returned. No value is selected.

Callers never infer state from a nullable plan.

## Field-level provenance

Every PlanField contains its value, validation state, and sorted
PlanEvidenceTrace entries. A trace identifies the knowledge record, evidence
record, source, source class, evidence class, exact locator, and validation
state.

Evidence classes preserve:

- direct observation;
- OEM documentation;
- standard documentation;
- source-code evidence;
- secondary corroboration;
- synthetic test evidence;
- unverified research.

Source registration remains an F5 responsibility. Documented and captured
sources require SHA-256 identity. Synthetic evidence cannot be classified as
observed, and captured evidence must be explicitly classified as direct
observation.

## Multi-source joins

Several sources may populate one plan only when every contributing record
shares the explicit ECU-family and diagnostic-implementation applicability
chain. Similar module acronyms or conventional addresses are not a join key.
An otherwise plausible route scoped to another implementation is excluded and
leaves the requested environment INDETERMINATE.

UNKNOWN remains different from ANY. A missing market, model-year range, ECU
implementation, response address, or other applicability dimension cannot be
converted into universal support.

## Real positive X250 golden

The accepted target is an X250 ECM/PCM environment identified by calibration
`CX23-14C204-ZAD`, observed response CAN ID `0x7E8`, and read-only OBD
Service 09 InfoType 04 Calibration ID capability.

The source roles are deliberately field-specific:

- a minimal normalized snapshot of the publicly indexed OBD Fusion report
  directly observes 2010 Jaguar XF Supercharged, VIN
  `SAJWA0HE4AMR59890`, response `0x7E8`, and calibration
  `CX23-14C204-ZAD`;
- Jaguar TSB `JTB00244NAS1` and the exact 2010 X250 5.0L SC service manual
  establish vehicle/ECM applicability;
- the service manual establishes HS-CAN, J1962/C2DB04B, pins 6/14, and
  500000 bit/s;
- ELM327 standard documentation establishes ISO 15765-4 11-bit address roles:
  functional `0x7DF` and physical `0x7E0` for the independently observed
  `0x7E8` responder;
- the California BAR specification establishes SAE J1979 Mode 09 InfoType 04
  Calibration Identification semantics;
- the production `mongoose-jlr` descriptor establishes the `hs-can`
  backend route.

The original forum attachment was unavailable. The committed artifact is a
minimal normalized public-index snapshot, not the original thread or
attachment. It is classified as direct observation only for the exact report
fields above and does not prove request IDs, protocol, bitrate, physical pins,
or backend support.

The complete evidence matrix, exact locators, SHA-256 fingerprints, join, and
exclusions are recorded in
`docs/evidence/F6_EXTERNAL_EVIDENCE_2026-08-31.md`.

## Why 0x7E0 is not an observed frame

The proposed 0x7E0 physical PCM request must be supported by applicable
standards or diagnostic documentation. It must use
STANDARD_DOCUMENTATION provenance unless an actual retained capture
independently observes it. A general addressing convention is not an X250
capture.

## Why 0x7E8 must be independently observed

The response ID must come from the real X250 report or capture. F6 never derives
0x7E8 from 0x7E0 by adding eight. The generic mixed-evidence test proves that a
documented request and an observed response keep distinct field traces.

## Real negative X250 CCP golden

The existing JLR X250 wiring manifest remains the real negative case. It proves
only:

- connector C2DB04B;
- pins 12/13;
- HS_CAN_POS_CCP and HS_CAN_NEG_CCP;
- X250 vehicle-side topology at the documented precision.

It does not prove bitrate, protocol, request ID, response ID, a supported
read-only capability, or an executable Mongoose JLR backend route. Resolution
is INDETERMINATE with those facts explicitly missing. It does not fall back to
pins 6/14, 500 kbit/s, ISO-TP/UDS, 0x7E0/0x7E8, or a Mongoose route.

## Conflict and deterministic behavior

The synthetic conflict golden supplies two request IDs for the same applicable
implementation. Resolution returns CONFLICT with both values and both traces.
No source wins by order, recency, confidence, or insertion.

All source, evidence, record, unresolved-fact, trace, and conflict ordering is
deterministic. Generic PROGRAM-A/PROGRAM-B fixtures verify that the same
implementation can apply to several open vehicle-program identifiers.

## Tests and current limitation

Focused F5/F6 tests cover complete resolution, missing bitrate, UNKNOWN != ANY,
conflict propagation, valid multi-source assembly, invalid applicability join,
field-level evidence, evidence-class distinction, deterministic output, CAN-ID
width, multi-program resolution, the real X250 positive source join, and the
real CCP negative case. The positive test asserts distinct
STANDARD_DOCUMENTATION traces for `0x7DF` and `0x7E0`, DIRECT_OBSERVATION
traces for `0x7E8` and `CX23-14C204-ZAD`, and SOURCE_CODE provenance for the
backend route.

Windows Application Control may block newly rebuilt unsigned local test
executables with the already documented error 4551. This does not change F3 or
F3.1 and does not justify modifying signing. The implementation lineage passed
frontend lint/tests/build, Rust formatting, workspace clippy with warnings
denied, all workspace tests including the F5 and F6 golden suites, architecture
checks, and the Tauri shell check. After rebasing F6 onto the current main
baseline, the checks attached to PR #6 are the current CI source of record.

## Future mapping

A future explicitly authorized phase may map a RESOLVED plan into an offline or
live diagnostic transaction. F6 deliberately provides no conversion to
UdsRequest, diagnostics-core, ISO-TP, CAN frames, or backend commands.
