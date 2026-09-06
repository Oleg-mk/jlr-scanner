# ADR-0017: Derived normal-fixed CAN identifiers

Status: accepted, 2026-09-04. Milestone M3.

## Context

Five SDD platform documents — L322 MY04.5, MY06 and MY07, L319 MY05, L320
MY06 — address every module by a one-byte physical address under
`normal_fixed` on 29-bit CAN, and declare the identifier prefixes
`0x18DA` (physical) and `0x18DB` (functional). They declare no CAN request or
response identifier and no tester address. F9 records the physical address
verbatim (`sdd_physical_address`) and derives nothing, so 148 modules across
those programme-years — 360 module rows in the fleet survey — have no
identifier and cannot be read, although their buses are the same pairs the
adapter already opens.

ISO 15765-2 defines normal fixed addressing: the 29-bit identifier is the
prefix, the target address and the source address — `0x18DA TA SA` for a
physical request, and the response swaps them, `0x18DA SA TA`. ISO 15765-4
assigns the external test equipment source address `0xF1` for legislated
OBD. Whether JLR's own enhanced diagnostics on these buses accept `0xF1` as
the tester's address is not stated anywhere in the corpus; SDD's tooling
uses it in practice, but practice is not a document.

## Decision

1. **The identifiers are derived at ingest, on request.** The platform
   adapter, when asked (`with_derived_normal_fixed_identifiers`), derives for
   every physically addressed module on a `normal_fixed` 29-bit bus:
   request `prefix << 16 | address << 8 | 0xF1`, response
   `prefix << 16 | 0xF1 << 8 | address`, identifier format 29-bit, addressing
   mode `normal_fixed`. No functional identifier is derived: the functional
   target address for these buses is not documented. The exporter asks; a
   caller that does not ask gets the physical address text and nothing more.

2. **Three pieces of evidence, three classes, one honest state.** The derived
   record cites the module's own physical-address evidence (SDD, OEM
   documentation), the layout as standards evidence from a built-in
   documented manifest (`iso15765_normal_fixed_addressing.json`, ISO
   15765-2 and ISO 15765-4), and the tester address `0xF1` as an unverified
   research hypothesis from a built-in research manifest
   (`normal_fixed_tester_address_hypothesis.json`). Because one of its
   sources is research, the record is `Unverified` — the store's own rule —
   and the survey shows the module reachable with `UNVERIFIED` addressing.
   The tester's first answered read confirms it through the F13 intake
   (ADR-0016), which raises exactly that addressing to `CaptureValidated`.

3. **The live path sends 29-bit frames.** `execute_prepared_uds_read` accepts
   `normal_fixed` with 29-bit identifiers. The MongoosePro's outbound data
   frame carries the 29-bit flag in its status words the way its inbound
   frames carry it (`CAN_29BIT_ID`, observed in captures); which of the two
   words the device reads on transmit is not documented, so the flag is set
   in both. The path is implemented and transport-fake tested, not
   hardware-confirmed; a mis-encoded read-only request costs an unanswered
   request and nothing else.

4. **Built-in manifests are part of the platform data's evidence.** The two
   manifests join the application's built-in set and the exporter ingests
   them before the platform documents, because the derived records reference
   their evidence by identifier.

## Consequences

- L322 MY04.5–MY07 and the L319/L320 base gain derived identifiers for
  every CAN module on their main buses; the survey moves them from "no
  identifiers" to reachable with unverified addressing where the bus is
  bound, and the fleet document records the counts.
- The DS2/K-line body modules of the early Range Rover stay out of reach:
  no CAN identifier can be derived for a K-line module (F14).
- A wrong tester address would show as silence on the first read, recorded
  as an attempt; the hypothesis manifest is the single place to change it.

## Addendum, 2026-09-05

SDD's own per-module protocol configuration was examined for the tester
address it might state (`docs/research/sdd/COMMUNITY_NOTES.md`, the
tekonline entry): every 29-bit module file carries
`<CAN srcId= targetId=/>` with `srcId` = `targetId` + 8 in all 152 cases,
the Ford 11-bit response convention carried over, and no tester source
address anywhere. The `0xF1` hypothesis therefore stands unchanged, and
so does the derivation. Recorded here so that nobody reads `srcId="0x33"`
as SDD's tester address: it is the RLM's own address plus eight. If the
first read on such a car is met with silence, the alternative worth one
attempt is a request `prefix << 16 | address << 8 | (address + 8)` with the
response addresses swapped — the literal reading of that configuration —
and the report intake records which of the two, if either, answered.
