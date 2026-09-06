# F5 JLR SDD-era Knowledge System

## Purpose and boundary

F5 provides a small, deterministic, evidence-first foundation for research
knowledge across Jaguar, Land Rover, and Range Rover vehicle programs from the
SDD era. X250 is one data point, not an architectural root or a closed program
enumeration.

The primary invariant is:

> **JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI**

The knowledge crate cannot open CAN, operate Mongoose, send diagnostic
requests, depend on Windows/Tauri/React, or implement ISO-TP, UDS, KWP2000, or
ISO 9141. It describes evidence-backed facts which a future application layer
may use to interpret a diagnostic result.

## Evidence-first flow

    raw source
        -> source registry
        -> versioned ingestion adapter
        -> evidence records
        -> normalized knowledge records
        -> applicability resolver
        -> structured query result and exact trace-back

The protocol path remains independent:

    replay / simulator / future live source
        -> CAN / legacy transport
        -> ISO-TP or a future peer
        -> UDS, KWP, or a future peer
        -> diagnostic result

Knowledge may participate only later, at application interpretation. It does
not inject protocol behavior or vehicle operations.

## Source registry

`SourceRegistry` stores records by validated open stable ID in deterministic
`BTreeMap` order. A source contains title, type, origin, locator, optional
acquisition date, declared program scope, provenance, redistribution status,
notes, and a content fingerprint where applicable.

The supported source classes are:

- `documented`: traceable manufacturer, workshop, electrical, or technical
  material;
- `captured`: a real observation or diagnostic capture;
- `synthetic`: test-only material which is never real JLR evidence;
- `unverified_research`: a lead which is not yet validated knowledge.

Documented and captured sources require a valid SHA-256 fingerprint. The
registry can hash a local file directly, so identity is not based on a mutable
filename. Re-registering an identical source is idempotent; reusing its ID for
different metadata is rejected.

## Evidence model

Each `EvidenceRecord` references an existing source and carries a reproducible
locator: description plus optional document page, section, record key, or
capture timestamp. Invalid source references are rejected. Excerpts are
optional and limited to 500 characters so the repository retains only the
minimum needed for traceability.

The proprietary source corpus is not part of the repository. When
redistribution is restricted, only metadata, a stable hash, an exact locator,
and a minimal factual excerpt are committed.

## Normalized knowledge model

The typed foundation supports these independent entity kinds:

- vehicle program and aliases;
- ECU family;
- diagnostic implementation;
- protocol family;
- network route;
- diagnostic addressing;
- diagnostic capability;
- identifier or parameter definition.

Programs, implementations, and ECU families are open string IDs stored as
data. The code has no `X250` branch and does not assume that an ECU name, a
physical module, a software generation, and a diagnostic implementation are
the same thing. Values have typed representations plus an explicit `unknown`
form for unsupported semantics; unknown protocol, addressing, bitrate, or
identifier data is never defaulted or guessed.

Every knowledge record requires one or more sorted, unique evidence IDs.

## Applicability and vehicle context

Applicability models independent constraints for vehicle program, model-year
range, architecture generation, ECU family, powertrain, variant, market,
diagnostic implementation, and version-tolerant additional dimensions.
Vehicle programs are data and one implementation can list several programs.

Each dimension is one of:

- `one_of` or `range`: the source states a bounded constraint;
- `any`: evidence explicitly supports all values for that dimension;
- `unknown`: the source does not establish the dimension.

`UNKNOWN != ANY`. An unknown dimension produces `InsufficientEvidence`, not an
exact match. A missing value in the explicit neutral `VehicleContext` produces
`InsufficientContext`. Proven non-matches are omitted. Queries exclude both
indeterminate states by default; a research caller must opt in with
`include_indeterminate(true)` and still receives the resolution label.

F5 does not decode a VIN or infer a vehicle program.

## Validation states

The discrete states are `unverified`, `source_backed`, `corroborated`,
`capture_validated`, `contradicted`, and `deprecated`. They are assertions
validated against referenced source classes, not confidence percentages.

- Synthetic or unverified research can remain `unverified`.
- `source_backed` requires documented/captured evidence.
- `corroborated` requires at least two distinct real source IDs.
- `capture_validated` requires real captured evidence.
- `contradicted` and `deprecated` require real evidence.

Synthetic evidence cannot be mixed with real/research evidence in one record
and cannot be promoted to a real-evidence state. Importing the same source or
the same text twice does not create corroboration.

## Conflict handling

The store never overwrites a claim because a newer source disagrees. Records
are keyed independently, so contradictory evidence-backed values coexist.
Query results group equal entity/key pairs; two or more distinct values produce
a deterministic `ConflictReport` with every record ID and value. Selection or
resolution policy remains outside the knowledge store.

## Ingestion and storage

`IngestionAdapter` is an extensible parser boundary. F5 implements strict JSON
manifest schema version `1` with parser ID `jlr-knowledge-json` and parser
version `1`. Unknown fields, malformed structures, duplicate IDs, unsupported
schema versions, invalid evidence references, and invalid validation claims are
rejected. Ingestion validates a cloned store and commits only after the full
manifest succeeds, preventing partial writes.

Storage is an in-memory typed store backed by `BTreeMap`, with versioned JSON
fixtures as the reproducible interchange format. This is sufficient for the
small F5 dataset and introduces no database, service, network, or OS runtime.
Given the same source content, schema version, and parser version, record order
and ingestion receipts are deterministic and reviewable.

## Query boundary

`KnowledgeQuery` supports an explicit vehicle context plus ECU-family and
diagnostic-implementation filters. `KnowledgeQueryResult` returns normalized
records together with applicability resolution, validation state, evidence,
full source metadata, and conflicts. `trace_back(record_id)` provides the exact
evidence and source path independently of query filtering.

## Real-source golden evidence

F5 uses one lawful user-provided, traceable JLR document to demonstrate the
complete production path. The PDF itself is not committed.

| Field | Value |
| --- | --- |
| Source ID | `jlr-x250-ewd-2008-13-56-10-1e` |
| Type | `documented` |
| Title | X250 Electrical Wiring Diagrams |
| Provenance | JLR publication `13 56 10_1E`, March 2008; PDF metadata title `X250_1PP_SERVICE.book` |
| Source fingerprint | SHA-256 `406e07bfc3a8afc2caaa7384795210d911d44b3e23ca53ed7aab663c2782bee3` |
| File size | 4,789,074 bytes |
| Locator | PDF page 181 / printed page 133, section `418-00`, CAN bus - high speed - Part 4, diagnostic connector `C2DB04B` |
| Record keys | `C2DB04B/12`, `C2DB04B/13` |
| Extracted claim | vehicle-side logical net `CCP_HS_CAN` reaches diagnostic connector pins 12/13; bitrate remains unknown |
| Applicability | program `X250`; all unproven dimensions remain `unknown` |
| Validation | `source_backed` |
| Redistribution | `restricted_metadata_only` |

The golden test executes source registration, strict ingestion, evidence
validation, normalized route storage, fail-closed applicability, query, and
exact hash/page/key trace-back. It does not assert a Mongoose route or authorize
touching pins 12/13; the separate F2 backend restriction remains unchanged.

## Focused acceptance scenarios

- Synthetic: generic PROGRAM-A/PROGRAM-B data reaches an exact
  multi-dimensional query result while remaining `synthetic / unverified`.
- Real documented: the JLR source above reaches a source-backed query result
  with exact trace-back and correctly reports insufficient applicability
  evidence for unknown dimensions.
- Conflict: two sources and contradictory values coexist and produce one
  deterministic conflict report; no last-write-wins behavior occurs.

## Copyright, safety, and limitations

No proprietary manual or database dump is redistributed. No pirated SDD archive
or untraceable community assertion is used. AI output is never authoritative
evidence; extraction is acceptable only when the original source and exact
locator remain checkable.

F5 does not populate a full ECU catalogue, perform VIN decoding, build a search
engine/UI, add a protocol stack, open a vehicle channel, transmit CAN, issue a
diagnostic request, or expose programming, flashing, SecurityAccess, key,
actuator, or write operations. Future growth can add source-specific adapters,
more typed claim/value variants, and a persistent store without breaking the
evidence and applicability contracts.

## Acceptance status

F5 is PASS. GitHub Baseline validation run `33359988532` passed frontend
lint/tests/build and Windows Rust formatting, workspace clippy with warnings
denied, all workspace tests, the F5 golden paths, and Tauri shell check for
implementation commit `1e7890f`. No vehicle, Mongoose channel, CAN TX, live
diagnostic request, signing resource, or native UI validation was used.
