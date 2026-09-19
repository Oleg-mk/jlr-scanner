# ADR-0037: A diagnostic system belongs to its module

- Status: **Accepted, 2026-09-19** — drafted from what the owner's screen
  showed that morning, the configuration read on his X250 refusing every
  holder as "cannot be resolved from the loaded data", and what the library
  said when asked why; accepted on his word the same hour («ок, робимо»).
- Decision: a record whose module constraint names `<FAMILY>_SYSTEM_<letter>`
  is the family's. It is matched by a vehicle context that names the family,
  found by a query that names the family, listed under the family in the
  store's index, and the configuration's sources are read as the family they
  name. Nothing in the library changes; the rule is applied where the
  library is read, once, in the layer that owns names.
- Reason: SDD's configuration documents name an ECU's *diagnostic system*
  where the rest of SDD names the ECU. The product resolves routes by the
  ECU, so the two never meet: the route is `RSJB`'s, the blocks are
  `RSJB_SYSTEM_A`'s, and the configuration read of a real car plans nothing.
- Consequence: the configuration read works on the real programmes of this
  library, the bench proves it through a system-named holder, and every
  query that names a module also sees the module's system-scoped records —
  which, measured, are the configuration's block addresses and nothing else.

## What the library says, measured

Asked on 2026-09-19, the owner's copy, 517,595 records, through the same
store the application reads:

| records whose module constraint names a system | by kind |
| --- | --- |
| `IdentifierParameter` | **335** scoped to a system, 50,136 to a bare module |
| `DiagnosticTroubleCode` | 0 to a system, 171,235 to a bare module |
| `ModuleAssembly` | 0, 67,755 |
| `DiagnosticCapability` | 0, 11,658 |

The 335 are the configuration's block addresses, from SDD's CCF documents
(`<address module="RSJB_SYSTEM_A" …>`), and the configuration's holders are
named the same way (`<sync>RSJB_SYSTEM_A</sync>`). The only suffix in the
library is `_SYSTEM_A`. Six families carry it — `BCM`, `FSJB`, `GWM`, `IPC`,
`PCM`, `RSJB` — and every one of the six also exists as a bare module, with
its routes, its codes and its catalogue under the bare name.

`ADR-0028` measured these same sources on 2026-09-12 and wrote them as
`GWM` (19), `BCM` (14), `IPC` (7), `RSJB` (6), `PCM` (18), `FSJB` (6): the
family was meant, and the ingest kept the system's name. The bench never
noticed because the synthetic fixture names its holders bare.

On the owner's X250: sources `RSJB_SYSTEM_A` (sync), `FSJB_SYSTEM_A`,
`PCM_SYSTEM_A`; blocks under `RSJB_SYSTEM_A` on four identifiers and under
`PCM_SYSTEM_A` on two; blocks under `RSJB`, `FSJB`, `PCM`: none. Layout: 629
parameters. Scheme: `did`. So everything the read needs is in the library,
under a name the read does not ask for.

## Decisions

1. **The rule lives in the knowledge layer**, beside the catalogue, as the
   mileage rule (`ADR-0024`) and the battery rule (`ADR-0030`, decision 1)
   do: `knowledge::system_family(name)` gives the family of a system name
   and the name itself otherwise. What it accepts is exactly `_SYSTEM_`
   followed by one uppercase letter, at the end of the name. Anything else
   is not a system name and is left as it is: fail closed.

2. **The rule is applied at the three places a module name is matched**, so
   that they cannot disagree: the applicability of a record against a
   vehicle context, for the module dimension only; the store's filter and
   index, for the module dimension only; and the environment resolver's own
   `record_mentions_ecu`. A query naming `RSJB` returns the records scoped to
   `RSJB` and to `RSJB_SYSTEM_A`; a context naming `RSJB` finds a record
   scoped to `RSJB_SYSTEM_A` applicable. The diagnostic-implementation
   dimension is not touched: nothing in the library names one that way.

3. **The configuration's sources are read as families.** `ccf_sources`
   answers `RSJB` for `RSJB_SYSTEM_A`, so the read plans its route by the
   module and finds the blocks by the same name through decision 2. The
   readings, the master module and the record carry the family, as every
   other read does; SDD's system name stays in the library's own records,
   where it came from.

4. **Nothing is rewritten.** No record changes, no library is exported, no
   copy is re-stamped (2026-09-16: no new stamped libraries now). The rule
   is what a name means when it is read. A later export may write the
   family at ingest time and keep the system name in the record's
   description; the rule then matches nothing and stays harmless.

5. **The bench proves it.** The synthetic configuration fixture gains one
   holder named as SDD names them, `OTHERMOD_SYSTEM_A`, and the end-to-end
   configuration read on the bench reads it as `OTHERMOD`. A run against the
   owner's own library shows X250 planning its reads before this is called
   built.

## What this does not decide

- A second system of one module (`_SYSTEM_B`): none exists in this library.
  By this rule it would belong to the same family, which is what SDD's
  naming says; if one appears, the configuration read reads it as another
  holder of the same module, and this record is amended with what was
  found.
- The fault-code wording's own scopes: measured, none is a system name, so
  `describe_dtc` is not touched and no wording changes.
- Whether the readable-identifier list of a module should show the
  configuration's block identifiers among the rest: after this rule it
  does, because they are the module's; that is correct and is left so.

## To build, on acceptance

`knowledge`: `system_family`, `DimensionConstraint::mentions_module`, the
module dimension of `Applicability::resolve`, the store's filter and index,
with the equivalence test of the index extended by a system-named record.
`diagnostic-environment`: `record_mentions_ecu`. `diagnostic-session`:
`ccf_sources`. `fixtures/knowledge/synthetic/f9_ccf_data.xml`: one holder
named `OTHERMOD_SYSTEM_A`, and the bench's configuration read asserting it
is read as `OTHERMOD`. `CURRENT_STATE.md` records it as built, with the
X250 run.

## Built, 2026-09-19 — `IMPLEMENTED / FIXTURE_TESTED`

Built the hour it was accepted; nothing has met a car.

- *The rule.* `knowledge::system_family`, exported; `DimensionConstraint::mentions_module`
  beside the exact `mentions`, which every other dimension keeps; the
  module dimension of `Applicability::resolve` through `resolve_module`.
  Its own test: `RSJB_SYSTEM_A` and `PCM_SYSTEM_B` are their families,
  `RSJB_SYSTEM_AB`, `RSJB_SYSTEM_a`, `RSJB_SYSTEM_` and `_SYSTEM_A` are left
  as they are; a record scoped to the system is applicable to a context
  naming the module and not to one naming another.
- *The three places.* The store's filter and index (a record scoped to a
  system is listed under its module too), and the environment resolver's
  `record_mentions_ecu`, all through the one method. The store's
  equivalence test gained a record scoped to `ECU-A_SYSTEM_A`: a query
  naming `ECU-A` returns it applicable, a query naming `ECU-B` does not, and
  the narrowed walk still equals the whole one.
- *The configuration.* `ccf_sources` answers the family. The synthetic
  fixture's copy holder is `OTHERMOD_SYSTEM_A`, as SDD names one; the ingest
  golden keeps that name in the ingested rows and claims — the rule is the
  library's, not the ingest's — and the bench's configuration read finds
  `("OTHERMOD", "copy")` among the sources, finds the system's blocks under
  `OTHERMOD`, and its report names the module and never a system, with the
  copy refused for its route and never for lacking blocks.
- *The owner's X250, run through the same library and code:* sources
  `RSJB` (sync), `FSJB`, `PCM`; blocks under `RSJB` on `0xC25F`, `0xD900`,
  `0xF105`, `0xF106`, under `FSJB` the same four, under `PCM` on `0xF105`
  (three offsets) and `0xF106`; layout 629 parameters; scheme `did`. Before
  the rule the blocks under `RSJB`, `FSJB` and `PCM` were none. The read
  plans; whether the car answers is `VEHICLE_CONFIRMED` and unearned.
