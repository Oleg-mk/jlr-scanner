# F10 — Multi-module read-only diagnostics

F10 status: `IN_PROGRESS`. Enumeration, family-level resolution, offline UDS
read execution and the composition root are `IMPLEMENTED / FIXTURE_TESTED`;
the survey has additionally run against the real SDD corpus
(`REAL_SOURCE_SURVEYED`, offline). Nothing in F10 opens a vehicle, transmits,
or reaches a live adapter; live vehicle status is unchanged.

F10 is the first phase in which the application addresses more than one ECU. It
rests entirely on F9: without the platform addressing there was nothing to
address.

## What the F9 addressing made possible

`docs/F9_SDD_KNOWLEDGE_INGESTION.md` records 1,418 module addressing claims over
89 modules and 22 vehicle programs, each with a diagnostic request and response
identifier and the bus it sits on. F10 turns that from a catalogue into an
answer to a question about one car, and then into a prepared, offline-executed
read.

## Slice 1: enumeration

`DiagnosticEnvironmentResolver::enumerate_ecu_families` returns, for a vehicle
context, the ECU families the knowledge base associates with it. Each entry
states its strongest applicability, whether a request and a response identifier
are known, and the logical bus when the source gives one.

This is deliberately a **report, not a permission**. `has_complete_route` means
both directions are known, nothing more. Preparing anything still goes through
the resolver, which checks protocol, bus parameters, capability and provenance;
a route that exists is not a route that may be used.

Families whose applicability rules the vehicle out are excluded rather than
returned with a low rank, and an empty vehicle context selects no module as
applicable. The point of the enumeration is to make "fitted and routable",
"mentioned but unresolved" and "absent" three visibly different answers instead
of one inferred one.

## Slice 2: family-level plans and route bindings (`ADR-0013`)

Enumeration could name a module; the resolver could not plan for it. Every
SDD-sourced plan came back INDETERMINATE, for two reasons that are both about
what F6 counts as *related* to a target:

- the resolver relates a record to a target only if the record mentions the
  target's ECU family and, when named, its implementation. SDD describes module
  **families** on a programme and never names a software build, so requiring an
  implementation ruled out every SDD record by construction;
- the facts a plan needs about a bus — rate, protocol, adapter route, pins — are
  stated once per network, under a network entity, and so never mention the
  module.

`ADR-0013` resolves both without changing what "related" means:

1. `DiagnosticEnvironmentPlan.diagnostic_implementation` is now optional. A
   target that names an implementation still has to evidence it; a target that
   names only a family gets a **family-level plan** that records the
   implementation as absent.
2. The platform adapter records bus facts per module: the bus name and rate as a
   `NetworkRoute` claim and the diagnostic protocol as a `ProtocolFamily`
   claim, under the module's own entity, with evidence citing both the module's
   `<network>` and the network's `<rate>`/`<protocol>` elements. The precedent
   was already there — width and addressing mode were copied this way.
3. For each module whose bus declares `ISO14229`, the adapter records two
   `READ_ONLY` capabilities, `uds.service22.read_data_by_identifier.read_only`
   and `uds.service19.read_dtc_information.read_only`.
4. Which adapter route reaches an SDD bus is **knowledge, not code**:
   `fixtures/knowledge/documented/mongoose_jlr_route_bindings.json`, ingested by
   the existing `JsonManifestAdapter`, binds `CAN_HS` to `hs-can` on J1962 pins
   6/14 at 500 kbit/s and `CAN_MS` to `ms-can` on pins 3/11 at 125 kbit/s. Its
   records sit under `NetworkRoute:<bus>` entities, so a target that names its
   bus through `DiagnosticTarget::family_on_bus` relates to them through the
   resolver's existing `knowledge_entity` rule.
5. `DiagnosticEnvironmentResolver::resolve_ecu_family` does the two hops: the
   family's own claims say which bus it is on; the target then names that bus.
   A family with no stated bus, or with two, is resolved without a bus entity so
   that the missing route or the conflict surfaces from the resolver.
   `readable_identifiers` lists what the DID catalogue makes readable for the
   family, with evidence.

Because the binding states a rate and the platform states a rate, the resolver
checks them against each other for free: agreement merges into one value with
both traces; disagreement is a `CONFLICT`, not a wrong wire.

### What a vehicle context must carry

SDD knowledge is qualified by programme, by SDD's own model-year breakpoint
marker, and sometimes by engine. A context that resolves it therefore carries
`vehicle_program`, `model_year` (resolved through a `ModelYearTimeline`),
`other["sdd_year_breakpoint"]`, and `powertrain` where the DID catalogue
qualifies by it. Deriving the marker from a year, or all of it from a VIN, is
the deferred "digital car" work, not F10.

### What stays INDETERMINATE, on purpose

Only `CAN_HS` and `CAN_MS` are bound. The L405-era platforms declare several
high-speed buses — `PT_HSCAN`, `CH_HSCAN`, `CO_HSCAN`, `HY_HSCAN` — all at
500 kbit/s, and never state which one the diagnostic connector carries.
`CAN_HS_NVJCOM`, `CAN_HS_ISO14229` and every legacy bus are likewise unbound.
Every module on those buses resolves INDETERMINATE with `BackendRoute` and
`PhysicalRoute` named as unresolved. Binding them needs evidence, not a guess.

## Slice 3: UDS read execution (`ADR-0012`)

`crates/uds-execution` is the sibling of `diagnostic-execution` that `ADR-0012`
decided: the same one-way boundary from an F6 RESOLVED plan into offline
replay or simulator execution, for ISO 14229 reads. It depends on
`diagnostic-environment`, `uds`, `isotp` and the two offline frame sources, and
on nothing else. `ADR-0007` and the existing checker rules are untouched.

Two intents exist and no others:

- `ReadDataByIdentifier { target, identifier }`. The identifier must appear in
  the `ReadableIdentifier` catalogue passed to `prepare_read_only_transaction`;
  anything else is `IdentifierNotReadable` before a byte is encoded. The
  catalogue entries that admitted it travel in the transaction's provenance.
- `ReadDtcInformation { target, status_mask }`, sub-function `0x02`
  reportDTCByStatusMask with a typed `DtcStatusMask` (`TestFailed`, `Pending`,
  `Confirmed`, `AnyStatus`) so no caller composes a raw mask byte.

Preparation requires protocol `ISO14229`, the matching capability, a complete
route, and an addressing mode of `normal` or `normal_fixed`. SDD's `enhanced`
mode — 29-bit identifiers with a network-address prefix, used on L319, L320
and L322 sub-networks — needs different ISO-TP framing and is refused as
`UnsupportedAddressingMode`. A target naming a software build is refused
against a family-level plan (`MissingDiagnosticImplementation`); a family
target accepts either.

Execution mirrors the J1979 bridge and adds two UDS facts of life: a negative
response is a **result** carrying its NRC, not an error of this crate; and NRC
`0x78` ResponsePending restarts the response window and keeps waiting, as
ISO 14229 specifies. DTC reports are decoded by the `uds` crate into SAE J2012
codes with their failure type byte (`P0301-00`), ready to join to F9's DTC
descriptions.

### The two ADR-0012 gates, stated precisely

`ADR-0012` describes refusing identifiers that are not `READABLE` and
identifiers carrying a security reference, "from the data". An earlier draft of
this document claimed F9 had ingested the MDX `ECU_DATA` access parameters
those flags live in. **It had not**, and this document was wrong. The gates as
actually implemented:

- the DID catalogue is readable by construction — the adapter rejects any
  element that is not a `ReadParameter` — so an identifier either exists in the
  store as read-only or does not exist. The bridge refuses anything the
  module's catalogue does not list;
- security references live in MDX `ECU_DATA`, which is not ingested. There is
  no security-reference data to check yet and none is pretended. SecurityAccess
  itself stays outside every current stage.

### Architecture checker

`scripts/check-architecture.mjs` gives `uds-execution` the manifest rule and
source guard `diagnostic-execution` carries, plus refusals of any
session-control or programming constructor and of response-identifier
arithmetic. It also scans the crate for vehicle-program hardcoding.

## Bus coverage (decided 2026-09-03)

Recorded in `docs/ROADMAP.md`. The corpus is 42 CAN-only programmes, 0
K-line-only, 3 mixed (L316 base, L322 MY06, L322 MY07). CAN reaches every
programme; K-line is needed only for BMW-era body electronics over DS2 on pin 7
and two modules on the base L316. The MongoosePro JLR variant provides K-Line
on pin 7 and ROSCO on pin 8 (`docs/F2_JLR_NETWORK_INVENTORY.md`, addendum), so
that is a software gap, scheduled as F14 with its own ADR. Gatewayed
sub-networks (`SUB_MOST`, `SUB_CAN1`, `NGI`) stay stage 2 because gateway access
is `ROUTINE_CONTROL`.

An earlier draft of this document said an early Range Rover was "unreachable
over CAN at all". The platform classification shows it has `CAN_HS` and
`CAN_MS`; only its body modules are on K-line. Corrected.

### Connector pins are standard, not per-model

Four platform files declare a `<connector>` explicitly, and the adapter ingests
it. Every one that carries high-speed CAN states pins 6 and 14, which ISO 15765
mandates; `PLATFORM_L322_200600.xml` declares only K-line pins 7 and 8. What
varies is not the pin for a given bus but which physical layer a bus uses. Pins
are recorded as the document lists them and are **not** assigned to individual
buses by the parser; the binding manifest is where a bus meets its pins, with
its own evidence.

## Slice 4: the composition root (`ADR-0014`)

`crates/diagnostic-session` is what the shell calls. `KnowledgeLibrary` loads
the two documented manifests built into the binary plus every `.json`
manifest or bundle in a directory the user names; `survey_vehicle` enumerates
the families the knowledge associates with the vehicle, resolves each for
identifier reads and for fault-code reads, and returns an
`app-contracts::VehicleSurveySnapshot`: reachable modules with their route,
identifiers and readable-identifier catalogue, unreachable ones with the
resolver's unresolved facts or conflicts in plain words. The shell exposes
`get_data_library`, `load_data_library` and `survey_vehicle`; the UI gains a
data-library panel and a module table. The frontend never names a crate; it
speaks of a data library.

Two changes underneath made this possible at corpus scale:

- `KnowledgeStore::ingest` is atomic without cloning the store (two-phase
  validation), so loading thousands of manifests is linear;
- `IngestionBatch` serialises to the manifest shape, and the `sdd-ingest`
  example `export_manifests` writes the whole corpus as one bundle per adapter
  kind after validating it as one store.

### Run against the real corpus, 2026-09-03

The export took 30 seconds: 45 platform, 6 DID catalogue, 3 DTC index, 6,168
DTC help, 93 ODST and 1 link-monitor documents became 6,316 manifests and
126,706 records, none rejected, in six bundles totalling about 170 MB. The
library loads in about 7.5 seconds. The survey then said, for four vehicles
described in SDD's own terms:

| Vehicle | Result |
| --- | --- |
| X250, MY2010, marker `MY10` | 39 modules known, **28 reachable**: 15 on `CAN_HS` over `hs-can` pins 6/14, 13 on `CAN_MS` over `ms-can` pins 3/11; PCM at `0x7E0`/`0x7E8` with 74 readable identifiers, TCM with 64. The 11 unreachable sit on `SUB_MOST` and `SUB_CAN1` behind a gateway and say so |
| L405, MY2014, marker `MY14` | 52 modules known, **0 reachable**; every row names the reason — `adapter route: no applicable evidence-backed value`, `connector pins: …` — because `PT_HSCAN`, `CH_HSCAN`, `HY_HSCAN`, `BO_MSCAN` and `CO_MSCAN` are unbound |
| L322, MY2006, marker `MY06` | 33 modules known, **0 reachable**, for two different reasons that the rows keep apart: CAN modules such as IPC and FLM are on `hs-can` pins 6/14 but have no request or response identifier, because this platform addresses them by physical address under `normal_fixed` (below); DS2 modules such as `LR_DSM_L322` have no route, bit rate or capability at all, because they are on K-line (`F14`) |
| X250, MY2010, no marker | 39 modules known, 0 reachable, each saying the vehicle description lacks a detail its data is qualified by |

The output lives outside the repository, on the owner's machine, pending `G4`.

### Fleet coverage, 2026-09-03

`fleet_coverage` surveys every programme in the library in one run, using
each platform document's own programme and breakpoint marker. Across all 45
programme documents the library holds 1,705 module rows, of which 617 are
reachable today, on 23 programmes. The two blockers, counted once per module:

| Blocker | Modules | Where |
| --- | --- | --- |
| bus unbound | 728, 14 since 2026-09-05 | every 2014-and-later architecture — L405 MY14/16, L494, L538 MY14–17, L538C/JV, L550, X152 MY16, X260, X351 MY16, X760, X761 — whose buses SDD names `PT_HSCAN`, `CH_HSCAN`, `CO_HSCAN`, `HY_HSCAN`, `BO_MSCAN`, `CO_MSCAN` and for which no evidence yet says which one the connector carries | — the medium-speed `BO_MSCAN` and `CO_MSCAN` were hypothesised through `ms-can` on 2026-09-05 (ADR-0015 addendum), leaving only the sub-networks `NGI`, `SUB_MOST` and `SUB_CAN1` unbound |
| no identifiers | 360, 287 since M3 | physically addressed modules: L322 MY04.5–MY07, L319 MY05 and L320 MY06 on their main buses under `normal_fixed` (derived since M3, ADR-0017), plus the gatewayed sub-network modules of every programme, which remain |

Reachable in full or in large part: X150, X250 MY08–13, X351 MY10/13, L319
and L320 MY10 onward, L322 MY10, L359, L405 MY13, L538 MY12, X152 MY14 and
L316 — the two-bus era with explicit CAN identifiers. The same L405 surveys as
45 reachable modules for MY13 and 0 for MY14, because SDD renames and splits
the buses at MY14; the wiring did not become unknowable, the evidence for
which bus meets pins 6/14 simply has not been recorded yet. That binding, not
any single programme, is the largest lever the fleet has.

### Physical addresses: seen, not derived

Running the survey against the real corpus exposed a schema the platform
adapter had silently skipped. 744 module addresses in the corpus are not CAN
identifiers but **physical (node) addresses** — `<address type="phys"
session="diag">0x10</address>` — used wherever the bus addressing scheme
derives identifiers from a prefix and that address: L322 up to MY07 and L319
and L320 base on their main buses under `normal_fixed` (29-bit,
`can_id_prefix phys=0x18DA`, `func=0x18DB`), and every gatewayed sub-network
module on every platform. Before this slice such a module produced no record
at all and an L322 MY2006 surveyed as "no modules known", which is false.

The adapter records the physical address verbatim under the
`sdd_physical_address` claim, together with the module's bus. Since M3
(ADR-0017) it also derives, on request, the 29-bit identifiers for modules
on `normal_fixed` buses: `prefix << 16 | address << 8 | 0xF1` for the
request and the addresses swapped for the response, with the platform's own
prefix and address as OEM evidence, ISO 15765-2's layout as standards
evidence from a built-in documented manifest, and the tester address `0xF1`
as a built-in research hypothesis — so the record is `Unverified` by the
store's own rule and the survey shows the module reachable with
`UNVERIFIED` addressing until a tester's answered read confirms it through
the F13 intake. No functional identifier is derived. The export holds
73 derived addressing records; fleet-wide the "no identifiers" count
falls from 360 to 287 — what remains are the gatewayed sub-network
modules — and reachable rows rise from 617 to 688. Surveyed after re-export: L319 MY05 27 modules, 19 reachable; L320 MY06 28, 20; L322 MY04.5 10, 2; L322 MY06 33, 10 (ACM on ms-can at 0x18DA80F1/0x18DAF180, the rest gatewayed sub-network or K-line modules); L322 MY07 36, 20. Every one of the 45 programme-years now has at least one reachable module (40 before).

## Slice 5: listen-only capture, and what it can and cannot prove

The fleet coverage above put 728 modules behind one question: which of the
buses SDD names on a 2014-and-later car does the diagnostic connector carry?
Answering it needs evidence from a real vehicle, and the tester programme's
first contact with a car must be risk-free. Slice 5 gives the tester that
first contact: **a listen-only capture**.

`MongooseJlrDevice::capture_route` opens a route with the adapter's
listen-only flag — the same `DT_LISTEN_ONLY` open F2 validated — records what
the vehicle broadcasts for a bounded time, and closes the route. The only
bytes written to the adapter are the listen-only open, the pin selection and
the close; there is no transmit primitive to reach. The shell's `capture_bus`
command turns the result into counts (frames per second, distinct
identifiers, 11- and 29-bit split, the most frequent identifiers) and into a
`captured` replay fixture carrying its provenance: route, pins, rate, adapter
identity, how the vehicle was described, and the validation tag
`captured_listen_only_bus_activity_not_module_confirmed`. The UI's "Bus
capture" panel drives it and saves the fixture.

What a capture establishes is bounded, and the verdict says so in words:

- frames heard → a live bus is on that pair at that rate. Which of the
  vehicle's buses it is **cannot be told from listening**; ordinary traffic
  carries no diagnostic identifiers, and this project holds no catalogue of
  broadcast identifiers per module to match against — `CANLinkMonitorData.xml`
  turned out to hold only addressing widths and module mnemonics;
- no frames heard → the pair is silent at that rate, or the rate does not
  match, or nothing is connected; listening cannot tell these apart.

The silent case is nevertheless informative for the architecture question: a
2014-and-later J1962 pair that carries only a diagnostic CAN into the gateway
is silent until a tester speaks, while a two-bus-era pair is a live vehicle
bus. That contrast is what a tester's first capture on an L405 or X760 would
show, and it is recorded as an observation, not as a binding.

### The 2014-and-later binding: what the search for evidence found

SDD's own platform documents for L538 MY14, L494 MY14, X760 MY16 and their
kin declare `PT_HSCAN`, `HY_HSCAN`, `CH_HSCAN` at 500 kbit/s and `BO_MSCAN`,
`CO_MSCAN` at 125 kbit/s, all 11-bit `normal`, with **no gateway** declared
for any of them; the same L405 declares plain `CAN_HS`/`CAN_MS` for MY13. SDD
therefore addresses modules on all five buses directly and expects the
gateway to relay diagnostic frames transparently — the bus name is where the
module lives, not where the tester connects.

Public documentation checked on 2026-09-03: the Range Rover Evoque L538
(2011–2018) workshop manual pages mirrored at rrevoque.org show the diagnostic
socket on the high-speed CAN, medium-speed CAN and MOST diagrams with the CJB
"acting as a gateway between the 2 networks", and give no pins; three JLR
bulletins hosted by NHTSA (LTB01261NAS1, SSM 75531, H142NAS1) mention the
gateway module only in the context of software updates. The sentence that
would settle it — that the J1962 connector reaches the BCM/GWM through a
dedicated HS CAN diagnostic bus and DoIP — exists only as a search-engine
summary of an L405 workshop excerpt on a document-sharing site, which is not a
source this project can fingerprint or cite. **No binding is recorded.**

The honest path is therefore: an ADR for a *gateway-relayed route* — a
binding that says "modules on bus X are reached through route `hs-can`", with
the route's rate distinguished from the bus's own rate so that a 125 kbit/s
body bus reached through a 500 kbit/s diagnostic CAN is not a false conflict —
recorded as `UnverifiedResearch` until a tester's first read-only request on
such a car answers, which turns it into captured evidence. Live UDS execution
in `mongoose-jlr` is the prerequisite for that read.

## Slice 6: route hypotheses and live UDS reads (`ADR-0015`)

Two decisions taken together, because the second turns the first from a
guess into evidence.

**Hypotheses.** `fixtures/knowledge/research/mongoose_jlr_relayed_route_hypotheses.json`
is an F5 source of type `UnverifiedResearch`, built into the application
beside the documented bindings. It binds `PT_HSCAN`, `CH_HSCAN`, `HY_HSCAN`
and `CO_HSCAN` to route `hs-can` on pins 6/14 at 500 kbit/s, in validation
state `Unverified`, with the reasoning as its only evidence: ISO 15765-4
puts the legislated bus on those pins, SDD places the PCM on `PT_HSCAN` and
addresses every module on these buses directly with no gateway routine, and
the adapter has two CAN pairs. The medium-speed buses are not hypothesised,
because two candidates compete. The survey shows such a module as a
**hypothesis**, a status of its own, with `UNVERIFIED` beside its bus; the
plan carries `Unverified`. *Since 2026-09-05 the medium-speed buses are
hypothesised as well — `BO_MSCAN` and `CO_MSCAN` through `ms-can` on pins
3/11 at 125 kbit/s — on a community statement relayed by the owner; see the
addendum to ADR-0015 and `research/sdd/COMMUNITY_NOTES.md`.* A plan's bit
rate is now the route's, from the
binding; the per-module bus claim names the bus only (`ADR-0013` amended).

**Live reads.** `MongooseJlrDevice::execute_prepared_uds_read` accepts only a
`PreparedUdsTransaction`, which the architecture checker enforces, validates
it against the adapter's own route descriptor, opens the route for
transmission, sends the single-frame request padded to eight bytes, answers a
First Frame with one padded Flow Control, waits through ResponsePending with
the enhanced timer, and closes the route whatever happens. Decoding is
`uds_execution::decode_response`, the one function the offline paths use
too. `normal` 11-bit and, since M3, `normal_fixed` 29-bit addressing are
executed — the 29-bit flag is set in both status words of the outbound frame
because which one the device reads on transmit is not documented — and
`enhanced` is refused before anything is written, as are routes whose pins
or rate differ from the descriptor.

The shell's `read_module` command resolves the plan from the loaded library,
prepares the typed transaction, executes it, decodes, and keeps a report;
the UI's "Module read" panel offers the surveyed modules — a hypothesised
route marked as such — and shows the answer as it came, negative responses
included, with "Save read report". A tester's first read on a 2014-and-later
car is exactly that report.

### Fleet coverage with hypotheses, 2026-09-03

| | modules | of 1,705 |
| --- | ---: | ---: |
| reachable on a documented route | 617 | 36% |
| on a hypothesised route, readable once a tester confirms | 454 | 27% |
| no identifiers (`normal_fixed` derivation pending) | 360 | 21% |
| bus unbound (sub-networks; the medium-speed buses of 2014+ until 2026-09-05, hypothesised since) | 274 | 16% |

40 of 45 programme documents now have at least one module the application
can attempt; the five that do not are L319 MY05, L320 MY06 and L322 MY04.5–07,
all waiting on `normal_fixed`.

### What a live read does not change

Live vehicle status stays `NOT_YET_EXTERNALLY_VALIDATED`. Nothing here adds
a write, a control, a session change or a security service; the checker
refuses session-control and programming constructors in the live path. The
F8 calibration path is untouched and still sends unpadded frames.

## What is still missing in F10

- **Confirming or refuting the route hypotheses** from testers' read reports,
  and the F13 intake that turns a report into a `Captured` record.
- **Binding the medium-speed buses of the 2014-and-later architectures**,
  once a tester's capture or read shows where they are reached. F8 executes J1979 live behind
  `execute_prepared_calibration_identification`, which the checker requires to
  take a `PreparedDiagnosticTransaction`. The UDS equivalent needs the same
  shape and the same checker guard, as its own slice.
- ~~Decoding DID payloads~~ and ~~joining decoded DTCs to descriptions~~ —
  done in F11 (`F11_TESTER_APPLICATION.md`): linear converters applied, map
  converters withheld with the reason, wording joined by code and module.
- **Binding more buses**, each with evidence.
- **Deriving `normal_fixed` identifiers** from prefix and physical address,
  with the standard's layout and tester address as evidence.

## Acceptance

- F10 golden tests: `f10_enumeration` (3), `f10_family_resolution` (4),
  `f10_uds_execution` (12, end to end from the F9 fixtures and the documented
  binding manifest to simulator and replay execution). `f9_platform_golden`
  gains 2 tests for the per-module bus facts and capabilities; `uds` gains 3
  unit tests for DTC decoding.
- Composition root: `f10_session` (6, from exported manifests and bundles to
  a survey, including a broken manifest and an under-described vehicle), the
  shell's `session_service` (3), the frontend's `ModuleSurvey.test.tsx` (2).
- Listen-only capture: `mongoose-jlr` `capture` (3, on a scripted transport,
  asserting that only the listen-only open, pin selection and close are ever
  written), the shell's `capture_service` (4, including a `captured` fixture
  that replays), the frontend's `Capture.test.tsx` (2).
- Route hypotheses and live reads: `f10_session` gains a hypothesis test;
  `mongoose-jlr` `uds_live` (5, on a scripted transport: padded single frame,
  flow-controlled multi-frame, ResponsePending, silence, refusals before any
  write); the shell's `module_read_service` (4); the frontend's
  `ModuleRead.test.tsx` (2).
- Workspace: 179 Rust tests across 49 suites plus 19 shell tests under Linux
  and 12 frontend tests; clippy clean, formatted, architecture boundaries pass.


## 2026-09-07 — what medium-speed diagnostics actually rests on

Offline analysis, no code changed. Six vehicles surveyed through the
exported library with `diagnostic-session`'s `survey` example, which is the
same path the application takes.

The medium-speed side is not one question but three populations in very
different states.

**1. The SDD-era cars: bound from SDD's own platform documents, never met a
car.** The route status is `Reachable`, not `Hypothesis`; nothing here is a
guess about which bus the pair carries, because SDD names the bus per module
and the binding of `CAN_MS` is documented.

| Vehicle | modules | on `CAN_HS` via `hs-can` | on `CAN_MS` via `ms-can` | behind a gateway |
| --- | --- | --- | --- | --- |
| X250 MY10 | 39 | 14 | 14 | 11 (`SUB_MOST` 10, `SUB_CAN1` 1) |
| L319 MY10 | 39 | 16 | 14 | 9 (`SUB_MOST`) |
| L322 MY07 | 36 | 13 | 7 | 16 (`SUB_MOST` 8, `DS2` 8) |

X250 MY10's fourteen medium-speed modules all carry a request and response
identifier and a reachable fault-code read: DCSM `0x776/0x77E`, DDM
`0x740/0x748`, DSM `0x744/0x74C`, FSJB `0x726/0x72E`, HVAC `0x733/0x73B`,
ICM `0x784/0x78C`, ICP `0x7A0/0x7A8`, KVM `0x731/0x739`, PAM `0x736/0x73E`,
PDM `0x741/0x749`, RSJB `0x7B7/0x7BF`, SODL `0x7C4/0x7CC`, SODR
`0x7C6/0x7CE`, TPM `0x751/0x759`. Readable identifiers per module: HVAC 21,
DDM and PDM 18 each, ICM and PAM 9, DCSM 8, SODL, SODR and TPM 5 each, DSM
4; FSJB, ICP, KVM and RSJB carry none, so for those the only useful request
is fault codes.

**2. The 2014-and-later cars: half the car is a hypothesis.** L405 MY14 has
59 modules, none `Reachable` and 52 on hypothesised routes, of which 25 are
medium-speed: 13 on `CO_MSCAN` and 12 on `BO_MSCAN`. L494 MY16 has 57, 50
hypothesised, 22 medium-speed: 13 and 9. Both buses were bound to `ms-can`
on the owner's community statement (`ADR-0015` addendum) and stay
`Unverified`.

**3. The gatewayed sub-networks are a different problem and should not be
called medium-speed.** `SUB_MOST`, `SUB_CAN1`, `NGI` and L322's `DS2` are
reached through a gateway routine, which is `ROUTINE_CONTROL` and therefore
stage 2. They are the bulk of what stays dark on the older cars: ten of
X250 MY10's eleven unreachable modules are `SUB_MOST`.

*What the gateway needs, looked up 2026-09-08 without writing code.* The
X250 platform document names the mechanism and the doorman for each
sub-network: SUB_MOST hangs off CAN_MS behind ICM_SYSTEM_A, SUB_CAN1 off
CAN_HS behind CCM_SYSTEM_A, both with access_method ROUTINE_CONTROL. It
does not name the routine. The MDX documents of ICM and CCM declare only
two routines each, "VIN learn" 0x0404 and "Self Test" 0x0202, neither of
which opens a gate, and SDD's runtime configuration trees mention gateways
only in the module list. So the routine number, its parameters and its
session live in SDD's program logic, not in the data this project holds.
The cleanest source is a listen-only capture, on pins 3/11 and 6/14, taken
while a genuine SDD session reads one MOST module and the radar on a
tester's car: it records the exact RoutineControl request, at no risk,
with the tooling that already exists. Until that is known, no ADR can name
what stage 2 would send, and the map's wording stays exact: the addresses
are known and nothing else is missing on our side, but the door's key is
not in our hands yet.

### What is known, and what is actually open

*Reworded 2026-09-09 at the owner's insistence, and he was right: the
earlier heading called this a weak point, which overstated it.*

For the SDD-era cars on the Ford-derived architecture — X250, X350, X400,
L319, L320, L322 — that a car brings its medium-speed bus to J1962 pins
3/11 at 125 kbit/s is known from the architecture and from the adapter's
own documentation, and it is what the whole trade works on:

- the MongoosePro JLR user guide's own pinout, JLR column, gives pin 3 as
  CAN 2+ and pin 11 as CAN 2- (`F2_JLR_NETWORK_INVENTORY.md`);
- the adapter path is `HARDWARE_CONFIRMED` on the bench: resource 21 opens
  with those pins and takes the application's outbound record exactly as
  resource 5 does (2026-09-08, evidence Part 4);
- third-party tooling reaches body modules on these cars through that pair,
  while ELM-class dongles reach only the powertrain because they speak only
  6/14 — the owner's statement, 2026-09-09, recorded in `COMMUNITY_NOTES.md`.

What is missing is therefore not a doubt but a signature: no capture from a
car is in this record yet. One listen on pins 3/11 of any SDD-era car
supplies it, and it is the first step of the tester guide. Until then the
survey says `Documented`, not `CaptureValidated`, which is exactly what
those two words mean.

For the 2014-and-later cars the question is different and genuinely open,
and it is written down in `COMMUNITY_NOTES.md`: those cars carry two
medium-speed buses, `BO_MSCAN` and `CO_MSCAN`, behind a gateway, and
whether the gateway relays a request from pins 3/11 to either branch
transparently, or switches per request in a way SDD does not declare, is
not known. That is what 25 of L405 MY14's modules and 22 of L494 MY16's
depend on.

### What settles it, cheapest first

0. *Done 2026-09-08 on the bench:* the diagnostic open of resource 21 with
   pins 3/11 and the application's outbound record are accepted by the
   firmware exactly as on resource 5 (evidence, Part 4). Nothing in the
   adapter path distinguishes the two buses any more.
1. A listen-only capture on pins 3/11 of a tester's SDD-era car. Traffic at
   125 kbit/s confirms the pair carries a live medium-speed bus. Zero risk,
   the path is already hardware-confirmed on the bench.
2. One fault-code read from one medium-speed module of that car. HVAC or DDM
   on an X250 are the richest. An answer confirms the binding for that
   vehicle through the F13 intake; silence refutes it, and both are recorded.
3. The same two steps on a 2014-and-later car, where the answer also settles
   the gateway question.

Nothing above needs new code. The survey, the route, the read and the intake
all exist; what is missing is a car.
