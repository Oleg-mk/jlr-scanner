# ADR-0013: Family-level diagnostic plans and bus-to-route binding

- Status: Accepted
- Decision: Five changes, together making an F6 plan resolvable for a module
  that SDD describes, without weakening what a plan means.

  1. **A plan may omit the diagnostic implementation.**
     `DiagnosticEnvironmentPlan.diagnostic_implementation` becomes
     `Option<PlanField<String>>`. When the target names an implementation, the
     field stays mandatory and must be evidenced exactly as before. When the
     target names only an ECU family, the plan is a *family-level plan* and
     records the implementation as absent rather than inventing one.

  2. **Bus facts are recorded per module at ingestion.** The platform adapter
     already copies the CAN identifier width and addressing mode from a
     network onto each module that sits on it. It now also records, under the
     module's `EcuFamily` entity, the bus name and rate as a `NetworkRoute`
     claim and the bus's diagnostic protocol as a `ProtocolFamily` claim. Both
     facts come from the same SDD document; the evidence cites the module's
     `<network>` element and the network's `<rate>` and `<protocol>` elements.

  3. **UDS read capabilities are recorded per module.** For each module whose
     bus declares `ISO14229` as its diagnostic protocol, the platform adapter
     records two `READ_ONLY` capabilities,
     `uds.service22.read_data_by_identifier.read_only` and
     `uds.service19.read_dtc_information.read_only`, with the protocol
     declaration as evidence.

  4. **Binding a bus to an adapter route is knowledge, not code.** The fact
     that SDD's `CAN_HS` is what the MongoosePro JLR reaches as `hs-can` on
     J1962 pins 6 and 14 at 500 kbit/s is a documented, evidence-backed
     claim, so it lives in a JSON manifest under
     `fixtures/knowledge/documented/` and is ingested by the existing
     `JsonManifestAdapter`. Its records sit under `NetworkRoute:<bus>`
     entities. Only `CAN_HS` and `CAN_MS` are bound now.

  5. **F10 resolves a family in two hops without changing what "related"
     means.** `DiagnosticEnvironmentResolver::resolve_ecu_family` finds the
     module's bus from its own claims, then resolves with a target that names
     the family, the capability, and the bus entity. The resolver's existing
     rule — records under the target's `knowledge_entity` are related — does
     the second hop. `readable_identifiers` lists what SDD's DID catalogue
     makes readable for that module, so the UDS bridge never touches the
     store.

- Reason: SDD describes module *families* on a programme, not software
  builds. The X250 ECM work in F6–F8 resolved because every record named one
  implementation, `jlr.x250.ecm.cx23-14c204-zad`, observed on one car. No
  SDD record can name an implementation, and the resolver's relation rule
  demands that every related record mention whichever implementation the
  target names. Requiring an implementation therefore makes every SDD-sourced
  plan INDETERMINATE by construction. Making the field optional is the only
  honest fix: the alternative of stamping an SDD-derived identity onto every
  platform record would constrain those records to a context the vehicle
  never states, which breaks enumeration, and treating the family as the
  implementation would silently change the field's meaning.

  Denormalising bus facts per module was chosen over teaching the resolver to
  walk from a module to its network. The precedent already exists in the
  adapter for width and addressing mode, the facts come from one document so
  no cross-source inference is involved, and the resolver stays generic. A
  resolver hop would need to know SDD's `<program>-<bus>` entity naming.

  Bindings are knowledge because they are claims with evidence and a
  validation state, and because putting them in the store lets the resolver
  check them against SDD's own declarations for free: a binding that says
  500 kbit/s meeting a platform that declares 250 kbit/s for the same bus is a
  `CONFLICT`, surfaced with both traces, not a wrong wire.

  Only two buses are bound because only two have evidence.
  `docs/F2_JLR_NETWORK_INVENTORY.md` records the adapter's `hs-can` and
  `ms-can` routes with their pins and rates, and the Drew Technologies user
  guide confirms CAN on pins 6/14 and CAN 2 on pins 3/11 for the JLR variant.
  Platforms with several high-speed buses — `PT_HSCAN`, `CH_HSCAN`,
  `CO_HSCAN`, `HY_HSCAN` on L405 and its contemporaries — declare them all at
  500 kbit/s and never state which one the diagnostic connector carries.
  Binding those would be a guess about a wire, so they stay unbound and every
  module on them stays INDETERMINATE with the missing route named.

- Consequence: `diagnostic-execution` keeps requiring an implementation for
  J1979 calibration identification, now as an explicit
  `MissingDiagnosticImplementation` error rather than a type guarantee; F7 and
  F8 behaviour is unchanged. `jlr-profiles` wraps its literal in `Some`.

  The two ADR-0012 gates are restated precisely. The DID catalogue is
  readable by construction: the adapter rejects any element that is not a
  `ReadParameter`, so an identifier either exists in the store as read-only or
  does not exist. The `uds-execution` bridge refuses any identifier the
  catalogue does not list for the module. Security references live in SDD's
  MDX `ECU_DATA` access parameters, which F9 has **not** ingested; the earlier
  F10 text claiming otherwise was wrong and is corrected. There is therefore
  no security-reference data to check yet, and none is pretended.

  Modules on `<enhanced>` addressing (29-bit, gateway-prefixed, L319/L320/L322
  sub-networks) and on any unbound bus resolve to INDETERMINATE or are refused
  at preparation with the reason stated. Nothing in this decision opens a
  vehicle, transmits, or reaches a live adapter.

- Amendment 2026-09-03 (`ADR-0015`): the per-module bus claim of §2 carries
  the bus name and no longer its rate. A plan's bit rate is the route's, taken
  from the binding; the bus's own rate stays on the per-programme network
  record. `ADR-0015` records why.
