# ADR-0010: Diagnostic trouble codes are a first-class knowledge entity

- Status: Accepted
- Decision: Add `EntityKind::DiagnosticTroubleCode` to the F5 knowledge model.
  No other model type changes.
- Reason: F9 ingests per-module DTC definitions, and none of the existing
  entity kinds describes a fault code. `DiagnosticCapability` denotes something
  the vehicle can do, `IdentifierParameter` denotes a readable data item, and
  reusing either would make queries mean something different from what they say.
  A DTC is a distinct domain entity that carries its own description,
  applicability, and evidence, so it earns its own kind rather than an overloaded
  one or a `ClaimKey::Custom` escape hatch.
- Consequence: The addition is non-breaking. `EntityKind` is compared for
  equality throughout the workspace and is never matched exhaustively, so no
  existing crate changes behaviour and no migration is required. F5 validation,
  applicability, conflict reporting, and query semantics are untouched.

  This ADR authorises one enum variant, not a new subsystem. DTC ingestion
  continues to obey `ADR-0009`: derived records are documented evidence,
  classification follows the source type, and nothing becomes an available
  operation that the source does not state.
