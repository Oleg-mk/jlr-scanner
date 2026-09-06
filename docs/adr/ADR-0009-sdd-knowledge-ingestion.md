# ADR-0009: SDD knowledge ingestion as a separate adapter crate

- Status: Accepted
- Decision: Add a sibling `sdd-ingest` crate that depends only on `knowledge`
  and a read-only XML parser, and that implements the existing public
  `IngestionAdapter` trait. Keep `knowledge` free of any vendor-specific parser.
  SDD-derived material enters the knowledge base only as classified
  `SourceType::Documented` records carrying provenance, never as bundled or
  redistributed source content.
- Reason: F5 already defines `IngestionAdapter` as the extension point for new
  source formats, and `diagnostic-environment` already establishes the pattern
  of a crate that depends on `knowledge` alone. Placing SDD XML parsing inside
  `knowledge` would bind a vendor format and an XML dependency into the crate
  the architecture contract requires to stay generic, and would make every
  future source format accumulate there. A separate adapter keeps `knowledge`
  vendor-neutral, keeps the SDD format in one replaceable place, and needs no
  change to the existing evidence, applicability, validation, or query
  semantics. The F5 model already carries every type F9 needs —
  `EntityKind::DiagnosticAddressing` and `IdentifierParameter`,
  `ClaimKey::DiagnosticAddressing` and `IdentifierDefinition`,
  `KnowledgeValue::DiagnosticAddressing` with `CanIdFormat`, and
  `DiagnosticSafetyClass::ReadOnly` — so no model change is proposed.
- Consequence: `sdd-ingest` may depend on `knowledge` and a read-only XML
  parser only. It must not depend on protocol, transport, simulator, replay,
  execution, environment, backend, OS, Tauri, or frontend packages, and nothing
  may depend on it except future ingestion tooling. `knowledge` remains unaware
  of SDD.

  Ingestion is **fail-closed by data, not by curation**. Only identifiers
  expressed by SDD as readable — the `ReadParameter` element, and in MDX terms
  `ACCESS_PARAMETERS/READABLE` with no populated `SECURITY_REFS` — may become
  available operations. Writeable, controllable, periodic, downloadable,
  uploadable, memory-area, configuration and security-data material is either
  recorded as known-but-unavailable or not ingested at all, and never as an
  available operation. This keeps stage-1 compliance mechanically verifiable
  instead of dependent on reviewer attention.

  Firmware is never ingested. Components whose names contain `FLASH` and every
  `.vbf` payload are excluded from extraction and from the knowledge base, per
  `ADR-0005`. Decryption of `.exml` is out of scope; only material SDD publishes
  in plain form is parsed.

  Extracted SDD trees stay outside the repository. Committed artifacts are the
  derived knowledge records, their provenance, and fixtures small enough to
  serve as goldens. `docs/research/sdd/PROVENANCE.md` remains the provenance
  record of record.

  The architecture checker gains manifest and source guards for the new crate
  on the same pattern as `knowledge` and `diagnostic-environment`.
