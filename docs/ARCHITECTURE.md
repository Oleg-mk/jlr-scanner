# Architecture Contract

## Primary invariant

> **JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI**

F5 adds evidence-backed knowledge, F6 adds a data-only application resolver,
and F7 adds a one-way offline execution bridge without reversing protocol or
knowledge dependencies:

    React UI
       | serialized app contracts only
    Tauri / application composition
       |                              |
    diagnostics-core       diagnostic-environment
                                  |
                              knowledge query
       |                              |
       +---- read-only source      applicability + evidence
       |        +---- future live adapter
       |        +---- transport-replay
       |        +---- diagnostic-simulator
       |
       +---- raw classic CAN frames
                    |
                  isotp
                    |
                   uds

The two core paths are deliberately separate. The diagram describes
composition, not permission for live transmission. F4 exposes only read-only
frame-source plumbing. F5 adds no live adapter and no application CAN TX.

    evidence
       |
    knowledge
       |
    applicability
       |
    diagnostic-environment
       |
    data-only application interpretation

This must never become `knowledge -> protocol hacks`.

## Package responsibilities

- core-types: pure domain types.
- app-contracts: serializable DTO/event contracts between core and UI.
- transport-api: protocol-neutral byte stream plus validated raw CAN frame and read-only source contracts.
- transport-serial: Windows/serial device adapter.
- mongoose-jlr: MongoosePro JLR identity and passive raw-CAN receive subset; no diagnostic knowledge. Its `bench` module (ADR-0020) is the bench transport: the same framing, answered from a `transport_api::BenchBus` instead of a device.
- transport-replay: machine-readable CAN fixture validation and deterministic or real-time playback; no ISO-TP or UDS logic.
- diagnostic-simulator: deterministic offline UDS behavior plus a protocol-neutral fixture-payload seam rendered through the existing ISO-TP implementation; no vehicle knowledge or live transport.
- isotp: vehicle-independent ISO 15765-2 framing, segmentation, flow control, timing, and reassembly.
- uds: vehicle-independent request/response correlation and the F4 safe typed service subset.
- knowledge: source registry, ingestion, JLR semantics, applicability,
  evidence, validation, conflict reporting, and query; no protocol, transport,
  OS, database, or UI access.
- diagnostic-environment: typed offline assembly of applicable knowledge into
  RESOLVED, INDETERMINATE, or CONFLICT data; depends only on knowledge and
  exposes no transaction or execution mapping.
- obd-j1979: vehicle-independent typed SAE J1979 Mode 09 InfoType 04 codec; no
  knowledge, transport, UDS, backend, OS, or UI dependency.
- diagnostic-execution: offline application bridge from an F6 RESOLVED plan and
  typed read-only intent to J1979 over the existing ISO-TP reassembler; public
  execution accepts only simulator and replay sources and retains provenance.
- uds-execution: the ADR-0012 sibling of diagnostic-execution for ISO 14229
  reads — ReadDataByIdentifier gated by the readable-identifier catalogue and
  ReadDTCInformation by typed status mask — from a family-level or
  implementation-level F6 plan over the same ISO-TP reassembler; simulator and
  replay sources only; retains provenance.
- bench-vehicle: the bench's vehicle (ADR-0020) — a `BenchBus` built from the loaded library and the session's context, answering the reads the library makes readable with synthetic values; depends on the session layer, isotp and transport-api, never on an adapter, a serial port or the UI.
- diagnostic-session: the F10 composition layer (ADR-0014) — loads the
  knowledge library from F5 manifests and surveys a vehicle into app-contracts
  snapshots; since F11 it also indexes the vehicle catalogue and fault-code
  wording at load and decodes identifier payloads with the catalogue's own
  encodings (`decode`); depends on app-contracts, diagnostic-environment,
  knowledge, uds-execution and serde_json only; opens no transport.
- report-intake: the F13 intake (ADR-0016) — reads a tester's session report
  and the library, writes one `Captured` manifest; depends on app-contracts,
  diagnostic-session, knowledge and serde only; never on a transport or a
  protocol crate, and never writes a `Documented` source.
- diagnostics-core: source-agnostic CAN to ISO-TP to UDS orchestration; no backend, OS, Tauri, React, or knowledge dependency in F4.
- Tauri: composition root and platform adapter only.
- React: one adaptive presentation layer.

## F4 dependency rules

ISO-TP and UDS may not depend on a vehicle program, model year, Mongoose,
Windows, Tauri, React, SDD files, or JLR knowledge. Replay metadata is an
opaque evidence/applicability envelope and must never control CAN or protocol
logic. Vehicle-program identifiers remain open strings in future knowledge
records rather than a closed protocol enum.

The architecture checker enforces:

- no vehicle-program hardcoding in F4 transport/protocol/orchestration source;
- no protocol dependency in replay;
- no backend, UI, OS, or knowledge dependency in the protocol core;
- no application CAN TX API;
- no active/programming UDS request constructors.

## F5 dependency rules

Vehicle programs, ECU families, and diagnostic implementations are open data,
not closed Rust enums or branches. Knowledge may describe a protocol family or
route as evidence-backed data, but it cannot import protocol, transport,
simulator, backend, OS, Tauri, or React crates. Conversely, transport, replay,
simulator, ISO-TP, UDS, and diagnostics-core cannot depend on knowledge.

The architecture checker enforces:

- no vehicle-program hardcoding in production knowledge source;
- no knowledge dependency on Mongoose, serial, replay, simulator, CAN API,
  ISO-TP, UDS, diagnostics-core, Windows, Tauri, or frontend packages;
- no protocol, transport, programming, or vehicle-TX symbols in knowledge;
- the existing vehicle-independent protocol and no-application-CAN-TX guards.

See `docs/F5_JLR_KNOWLEDGE_SYSTEM.md` for evidence, applicability, validation,
conflict, ingestion, and query semantics.

## F6 dependency rules

Diagnostic-environment may depend on knowledge. It may not depend on
diagnostics-core, ISO-TP, UDS, transport, replay, simulator, Mongoose, Windows,
Tauri, React, or frontend packages. Knowledge and every protocol/transport
crate remain unaware of diagnostic-environment.

Every significant resolved field carries its own source/evidence trace and
evidence class. Multi-source joins require a shared explicit ECU-family and
diagnostic-implementation applicability chain. Unknown applicability and
conflicting values fail closed.

F6 ends at DiagnosticEnvironmentPlan. There is no mapping to UdsRequest,
diagnostics-core, CAN frames, or a backend operation.

## F7 dependency rules

ADR-0007 keeps `diagnostics-core` UDS-specific and introduces sibling
`obd-j1979` and `diagnostic-execution` crates. The J1979 codec cannot depend
on knowledge, diagnostic-environment, transport, simulator, replay, ISO-TP,
UDS, JLR, backend, OS, or UI packages. It only encodes and validates its narrow
typed protocol messages.

Diagnostic-execution may consume diagnostic-environment plans, J1979, ISO-TP,
and the two offline frame sources. Knowledge and diagnostic-environment do not
depend on execution. Transport, replay, simulator, ISO-TP, UDS, and
diagnostics-core do not depend on knowledge or execution. Diagnostics-core does
not query F5/F6.

The application execution surface has no live source, raw payload, arbitrary
CAN frame, generic diagnostic service, write/control/security intent, response
address arithmetic, or frontend binding. The simulator's raw payload seam is
fixture-only protocol plumbing and does not add application transmission.

## F10 dependency rules

ADR-0012 adds `uds-execution` beside `diagnostic-execution` rather than
widening it. It may consume diagnostic-environment plans and readable-identifier
catalogues, UDS, ISO-TP, and the two offline frame sources. It must not depend
on knowledge directly, diagnostics-core, obd-j1979, mongoose-jlr,
transport-serial, Windows, Tauri, or any frontend package, and it exposes no
session-control, programming, write, control, or security constructor.

ADR-0015 lets `mongoose-jlr` depend on `uds-execution` for the prepared
transaction type, mirroring its dependency on `diagnostic-execution`; the
backend still exposes no transmit primitive. Route bindings that are
hypotheses are `UnverifiedResearch` knowledge, never code.

ADR-0013 keeps the join between SDD knowledge and adapter routes inside the
knowledge layer: diagnostic-environment gains family-level resolution,
sdd-ingest records per-module bus facts and read-only UDS capabilities, and the
bus-to-route bindings are a documented knowledge manifest rather than code in
any protocol or transport crate.

The architecture checker enforces:

- no knowledge or transport dependency in obd-j1979;
- no backend, UI, OS, diagnostics-core, or J1979 dependency in uds-execution,
  and no session-control, programming, raw, or live method in it;
- no backend, transport, UI, or OS dependency in diagnostic-session;
- the Mongoose live UDS path takes only a `PreparedUdsTransaction`, derives no
  response identifier, and contains no session-control, programming, write,
  control, or security constructor (ADR-0015);
- no backend, UI, OS, UDS, or diagnostics-core dependency in
  diagnostic-execution;
- no vehicle-program hardcoding in either production crate;
- no raw/live/non-read-only public execution methods;
- no F5/F6 query symbols in diagnostics-core;
- no new frontend access to execution or protocol crates.

## SDD-era protocol scope

ProwlOne targets Jaguar, Land Rover, and Range Rover across the JLR SDD era.
That era is not UDS-only. F4 establishes the ISO-TP/UDS foundation without
assuming that future KWP, ISO 9141, or other legacy protocol support will pass
through UDS.

Architectural changes require an ADR before code changes.

## F8 typed live application boundary

The tester-alpha slice adds `jlr-profiles` as a data-only product catalog above
F6 and F7. The Tauri application exposes one argument-free intent and composes:

```text
React: Read Calibration Identification
  -> Tauri application service
  -> F8 supported profile / F6 RESOLVED environment
  -> F7 PreparedDiagnosticTransaction (READ_ONLY)
  -> mongoose-jlr typed prepared-transaction entry point
  -> private active HS-CAN + cOutboundData
  -> shared ISO-TP + obd-j1979
```

`jlr-profiles` has no UI, protocol, transport, OS, or execution dependency.
`mongoose-jlr` exposes no generic live CAN sender; the only active entry point
requires the already validated prepared transaction. Application diagnostic
reports are DTOs at the existing app-contract boundary. See ADR-0008.
