# ADR-0012: UDS read execution as a sibling bridge crate

- Status: Accepted
- Decision: Add a sibling `uds-execution` crate that depends on
  `diagnostic-environment`, `uds`, `isotp` and the existing read-only frame
  sources, and that compiles typed read-only UDS intent from an F6 RESOLVED
  plan. Leave `diagnostic-execution`, `ADR-0007` and the existing checker rules
  untouched. Only `0x22` ReadDataByIdentifier and `0x19` ReadDTCInformation are
  reachable.

- Reason: F10 must address more than one ECU, which needs UDS reads. Both
  services already exist in the `uds` crate from F4, but
  `diagnostic-execution` may not reach them: `ADR-0007` deliberately kept that
  crate free of UDS, and `scripts/check-architecture.mjs` enforces it through
  both a manifest rule and a source scan. That guarantee is load-bearing and is
  not widened here.

  This project has already faced the same choice. `ADR-0007` could have
  generalised `diagnostics-core` to carry SAE J1979 and instead added siblings,
  on the reasoning that mixing application protocols into shared request and
  result types is the more expensive mistake. The situation now is symmetric, so
  the answer is the same one, and the resulting shape is symmetric too:

  ```text
  diagnostic-environment + obd-j1979 + isotp -> diagnostic-execution  (J1979)
  diagnostic-environment + uds       + isotp -> uds-execution         (UDS)
  ```

  The alternative of extracting the protocol-neutral half of
  `PreparedDiagnosticTransaction` into a third crate would remove some
  duplication, but it refactors a type that working, committed F7 and F8 code
  depends on, in exchange for a benefit that is not yet proven. The duplicated
  part is route validation; the parts that differ are not incidental, because
  UDS reads carry session and security preconditions that J1979 Mode 09 has
  none of. If the shared half later proves genuinely identical and burdensome,
  extracting it is a smaller, separate decision that this ADR does not
  foreclose.

  Widening `diagnostic-execution` itself was rejected outright. It is the only
  option that weakens an existing enforced guarantee, and it would do so for
  convenience rather than necessity.

- Consequence: `uds-execution` may depend on `diagnostic-environment`, `uds`,
  `isotp`, `core-types` and the read-only frame sources. It must not depend on
  `diagnostics-core`, `mongoose-jlr`, `transport-serial`, `knowledge` directly,
  Windows, Tauri, or any frontend package. Nothing depends on it except the
  application composition root.

  **Read services only.** The crate exposes no constructor for any other UDS
  service. The architecture checker gains, for this crate, the guards
  `diagnostic-execution` already carries — no `execute_raw`, `send_frame`,
  `send_can`, `execute_live`, no `WriteDataByIdentifier`, `SecurityAccess`,
  `RoutineControl` or `EcuReset`, and no response-identifier arithmetic — plus
  a check that no session-control or programming service constructor appears.

  **Two preconditions gate every identifier, from the data rather than from
  review.** F9 ingested SDD's own access parameters, so the crate refuses any
  identifier that is not marked `READABLE`, and refuses any identifier whose
  record carries a security reference, because that one needs SecurityAccess and
  SecurityAccess is outside every current stage. A caller cannot opt out of
  either check.

  Preparation stays offline and total: an INDETERMINATE or CONFLICT plan, a
  missing route, an unstated protocol, a capability the target does not declare,
  or an identifier failing either gate produces an error, never a partial
  transaction. Execution remains restricted to the replay and simulator frame
  sources; no live adapter is reachable from this crate.

  The naming is asymmetric — `diagnostic-execution` handles J1979 despite its
  general name. Renaming it would churn working F7 and F8 code for cosmetic
  gain, so the asymmetry is recorded here instead of fixed.

- Amendment 2026-09-03: `uds-execution` was written under this ADR. The two
  identifier gates are restated precisely in `ADR-0013`: the DID catalogue is
  readable by construction, so the gate is presence in the module's
  readable-identifier catalogue; security references live in MDX `ECU_DATA`,
  which F9 has not ingested, so no security check exists yet and none is
  pretended.

- Amendment 2026-09-03 (`ADR-0015`): `mongoose-jlr` depends on this crate for
  the `PreparedUdsTransaction` type, as it depends on `diagnostic-execution`
  for the J1979 one; the live UDS path accepts nothing else, and the checker
  enforces it. Decoding a live answer is `decode_response`, shared with the
  offline paths.
