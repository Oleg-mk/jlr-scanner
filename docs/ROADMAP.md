# Roadmap

`CURRENT_STATE.md` records what is done. This file records **what is next and why
in that order**. Both are store of record. Neither may live only in a chat
session, an issue tracker, or an assistant's context window.

Update this file whenever a gate opens, a priority changes, or a phase closes.
If an entry has no open gate and no pending owner action, it is ready to start.

## What this product is

**ProwlOne replaces SDD. It is not a scanner.**

That distinction decides almost every downstream question, so state it plainly.
A generic OBD-II scanner is a commodity: any inexpensive Wi-Fi or Bluetooth
dongle paired with a phone already reads standard modes on any car. Competing
there adds nothing. The value of this product is the capability SDD has and a
scanner cannot reach — every module on every bus, manufacturer-specific
diagnostics, and JLR knowledge — delivered without SDD's operational misery.

The owner's own comparison, recorded 2026-09-01: a V-GATE Wi-Fi/Bluetooth OBD-II
interface pairs with any phone in minutes, while the MongoosePro JLR requires
installing SDD on a dedicated Windows 7 laptop. **The hardware was never the
problem; SDD as software is.** The Mongoose is good hardware trapped behind a
bad stack.

The product thesis follows: *professional J2534-class JLR diagnostics that start
as easily as plugging in a cheap dongle.* Modern native application, automatic
adapter discovery, no virtual machine, no legacy Windows, no install ritual.

Generic SAE J1979 remains a baseline capability for vehicles with no JLR
knowledge yet. It is not the product and must not become the centre of a phase
plan.

## Scope, decided 2026-09-01

The owner selected staged scope. Later stages are **declared direction, not
current work**, and none may be started early.

| Stage | Contents | Status |
| --- | --- | --- |
| 1 | Full read-only diagnostics: all modules on all buses, DTC read, live data, ECU and software identification, CCF read | **CURRENT SCOPE** |
| 2 | Service functions: service-interval reset, adaptations, actuator tests, DTC clear | future direction |
| 3 | Configuration and programming: CCF write, module configuration | future direction |

Stage 1 keeps `SAFETY_BOUNDARIES.md` and `ADR-0005` unchanged. Every operation
stays `READ_ONLY`; everything else stays unavailable.

Stage 2 introduces the first write operations and requires new safety classes, a
new ADR, and explicit in-application confirmation per operation.

Stage 3 collides with `ADR-0005`, which prohibits firmware programming. Reaching
it would require an ADR that explicitly supersedes `ADR-0005`, taken as a
deliberate decision. **It must never happen by gradual scope drift.**

Design consequence for stage 1: do not architect in a way that makes stages 2
and 3 impossible, and do not architect as though they are already approved. Keep
write paths absent, not merely disabled.

## Knowledge sources, decided 2026-09-01

All three are in scope and they are not interchangeable. F5 evidence
classification applies to each.

**1. SDD's own diagnostic data — the breadth driver.** JLR already compiled the
module, identifier, and routine definitions this product needs. Extracting them
as `documented` class evidence is the only realistic way to cover the SDD-era
fleet without access to hundreds of cars. `README.md` already permits SDD as a
research reference; it must never become a runtime dependency, a bundled asset,
or a redistribution.

**2. Captures from community testers — the validation layer.** Real responses
from real vehicles confirm that SDD-derived definitions behave as expected. This
is what turns `IMPLEMENTED` into `VEHICLE_CONFIRMED`, and nothing else can.

**3. Public documentation and community sources — supplementary.** Wiring
diagrams, published DTC tables, forum captures. Secondary sources under F5,
never promotable to primary evidence.

The bottleneck is real: the knowledge base holds two artifacts totalling about
5 KB, one of which is a negative golden. Source 1 moves that number by orders of
magnitude; source 2 makes it trustworthy.

## Gates

| Gate | State | Blocks | What closes it |
| --- | --- | --- | --- |
| `G2` signing identity | `DECIDED 2026-09-03 — Ukraine, individual entrepreneur (ФОП)`; certificate not obtained and, since 2026-09-09, not being sought at all — the free route included | nothing since 2026-09-05: the tester programme runs on the unsigned build, the certificate improves it later | the owner obtains an RSA code-signing certificate from a Microsoft Trusted Root CA — OV under the sole-proprietor procedure or IV in his own name — then F12 wires it into CI; see `F3_1_WINDOWS_SIGNING.md` |
| `G1` owner vehicle access | `DECLINED_BY_OWNER` | nothing; superseded by the tester programme | closed by decision |
| `G3` adapter class | `DECIDED — J2534 / MongoosePro JLR class` | nothing | closed by the SDD-replacement thesis |
| `G4` distributing SDD-derived knowledge | `DECIDED 2026-09-03 — named testers only, separate from the installer`; every copy stamped since 2026-09-05 | nothing; the tester programme hands the library to each named tester | closed by decision, see below |

### `G1` — owner vehicle, closed

The owner will not use his own X250 as a development test vehicle. The car is
in daily use and operationally critical; recovering a disabled vehicle by tow truck to
Ukraine costs more than buying another Jaguar at home. A successful test yields
one data point; a failed one strands the owner in a foreign country.

Do not re-propose owner-vehicle validation. Validation comes from community
testers instead.

### `G3` — adapter class, closed

Target J2534-class hardware, MongoosePro JLR first. ELM327-class adapters cannot
reach modules outside standard OBD-II addressing, so they cannot serve an SDD
replacement regardless of how common they are. Adapter *ease of use* is a
first-class product requirement; adapter *breadth downward* is not.

### `G2` — signing identity, decided

The owner is an individual entrepreneur (ФОП) registered in Ukraine. That
settles the path: Azure Artifact Signing Public Trust (Option A) is closed,
because its organisation list does not include Ukraine and its individual
path is limited to the United States and Canada; Option B stands. A
Microsoft Trusted Root CA issues either an OV certificate under the CA/B
Forum sole-proprietor procedure, validated against the state business
register, or an IV certificate validated on the person; either is RSA and
satisfies Smart App Control. Which of the two a given CA applies to a
Ukrainian ФОП is the first question F12 asks that CA. Accounts, identity
verification and the purchase are the owner's own actions; this project
prepares the pipeline and never performs them.

### `G4` — distributing SDD-derived knowledge, decided

The owner said yes and asked for a reasoned form. The decision: **the
exported library is handed to named testers only, as a separate folder, and
is never bundled into the installer.** The manifests are derived records, but
they carry SDD's parameter names and fault descriptions as excerpts, so a
public installer containing them would redistribute JLR's text to anyone who
downloads it. A folder given to a named tester under the tester programme's
terms keeps the public artifact clean, limits exposure to people who have
agreed to those terms, and costs no code: the application already loads a
directory. `RedistributionStatus::RestrictedMetadataOnly` on every SDD-derived
source stays as it is. This is a product decision, not legal advice, and the
owner can tighten it at any time.

## Path to the first live vehicle test (written 2026-09-03)

The finish line, in the owner's words: a finished application on a computer
that a MongoosePro JLR connects to a live car for tests. This section is the
plan to that line, ordered, with what is done, what each step needs, and the
two decisions only the owner can take. Sizes are relative (S, M, L), not
dates.

### Where the product stands today

Everything below is implemented and tested offline; nothing has touched a
vehicle.

- **Adapter.** Discovery, connect, board identity over the inbox `usbser`
  driver on Windows 11: hardware-confirmed. Both CAN routes (`hs-can` 6/14 at
  500 kbit/s, `ms-can` 3/11 at 125 kbit/s) implemented; never opened on a car.
- **Knowledge.** The whole SDD 169 corpus exported: 45 programme documents,
  126,706 records — addressing, buses, 15,281 identifier definitions with
  units, 93,521 fault-code descriptions. The application loads it as a data
  library in seconds.
- **Survey.** For a described vehicle, every module the data knows, with its
  route and the reason when it cannot be reached. Fleet-wide: 617 modules on
  documented routes, 454 on a hypothesised route awaiting a tester's first
  read, 360 waiting on `normal_fixed` identifier derivation, 274 on buses not
  yet bound.
- **Reads.** UDS ReadDataByIdentifier and ReadDTCInformation prepared from
  the plan and executed offline; the live Mongoose path implemented and
  tested against a scripted adapter, never against a module. The F8 J1979
  calibration read on the X250 profile, likewise.
- **Capture.** Listen-only recording of a J1962 pair, saved as a `captured`
  fixture: the tester's zero-risk first contact.
- **Installer.** CI builds an unsigned Windows NSIS installer on every push
  and, since 2026-09-05, an unsigned universal macOS disk image. The unsigned build runs after
  SmartScreen's per-file "Run anyway" on Windows 10, and on Windows 11 without
  Smart App Control; since 2026-09-05 that is what testers get, with the
  prompt explained in `TESTER_GUIDE.md`.

### Milestones, in order

Status on 2026-09-05: M1 done, M2 done, M3 first half done; M4 deferred
(signing is an improvement, not a gate, by owner decision of 2026-09-05);
M5 and M6 next — the working unsigned build and the library go to named
testers, and the first step that needs a car. `CURRENT_STATE.md` has the
resume notes.

**M1 — an application a tester can use without knowing SDD (F11, size L).**
Today the vehicle is described by typing SDD's programme name and breakpoint
marker; the module list shows identifiers, not values; fault codes show
without their descriptions. M1 closes that: a programme and model-year picker
built from the loaded library instead of free text; fault codes joined to the
descriptions F9 already holds; identifier values decoded with units from the
catalogue's encodings and converters; one session flow — connect, describe
the car, survey, capture, read, save — with one report bundle; every
unsupported case shown with its reason; finished presentation. VIN decoding
stays deferred: the picker does its job for testers. *Done 2026-09-04
(`F11_TESTER_APPLICATION.md`): the picker, the wording join, the value
decoding with named states, the guided session flow, the single session
report, and the vehicle network drawn as SDD draws it — buses as lanes,
modules as nodes — with every state explained and a read-only check of every
module; then VIN decoding with SDD's own tables, module names from SDD's text
database, and an interface in English, Russian and Ukrainian with SDD's
data text following into Russian. Open: the map-converter scale.*

**M2 — reports become evidence (F13 intake, size M).** A tester's saved
report — survey, captures, module reads — is turned into `Captured` knowledge
records by a small tool: a route hypothesis confirmed or refuted, a response
recorded, a validation state raised. This is the mechanism that turns the
454 hypothesised modules into reachable ones, and the only thing that ever
raises a claim to `VEHICLE_CONFIRMED`. *Done 2026-09-04
(`F13_REPORT_INTAKE.md`, ADR-0016): the intake confirms what an answer
proved, records what it did not, and never refutes on silence alone; a
confirmed route shows as reachable and capture-validated. Awaiting the first
report.*

**M3 — coverage that needs no car (size M).** Derive `normal_fixed` 29-bit
identifiers from the platform's prefixes and physical addresses with
ISO 15765-2 recorded as standards evidence; that returns L322 MY04.5–07 and
L319/L320 base to CAN reach, 360 modules. Bind the 2014-and-later medium-speed
buses once a tester's capture or read shows where they are reached. *First
half done 2026-09-04 (ADR-0017): 73 derived identifiers, reachable rows
617 → 688, each `Unverified` until a tester's answer confirms the
tester address. The medium-speed buses were hypothesised on
2026-09-05 from the owner's community statement (ADR-0015 addendum): 716
module rows now sit on hypothesised routes, 14 on unbound sub-networks;
confirmation waits on a tester's read (M6).*

**M4 — a signed installer (F12, gate `G2` decided; set aside 2026-09-09).** The owner obtains the
certificate as a Ukrainian ФОП — OV under the sole-proprietor procedure or IV
in his own name, from a Microsoft Trusted Root CA — following
`F3_1_WINDOWS_SIGNING.md`; CI signs. What that earns, stated correctly since
2026-09-09: Smart App Control gets an identity to trust and the uninstaller
can run. It does not silence SmartScreen — for an OV certificate, for Azure
Artifact Signing and, since 2024, for EV alike, reputation accumulates over
downloads and the first prompts are expected. macOS packaging follows the same
pipeline later. Since 2026-09-05 M4 no longer gates the tester programme, and
since 2026-09-09 it is not being pursued: the owner set the certificate aside,
free route included, and `F3_1_WINDOWS_SIGNING.md` records what would reopen
it.

**M5 — the library reaches testers (gate `G4` decided).** Each named tester
receives the exported library as a folder under the programme's terms; the
installer never contains it. The application already loads a directory, so
this is a distribution step, not a code one.

**Positioning, corrected 2026-09-05.** A community reply to the first
announcement (`docs/research/sdd/COMMUNITY_NOTES.md`) shows that for people
who accept an unofficial SDD build, SDD installs on Windows 11 in an hour and
does everything, writes included. The product is therefore not "a
replacement for SDD" to that audience today but a native diagnostic
application for Windows 10/11 and macOS including Apple Silicon — SDD's
depth per module, every module explained, evidence from tester reports —
read-only in this stage with the write stages declared above, and it must
say plainly what it does not do yet and what it will never do
(`FORBIDDEN_PROGRAMMING` only). MongoosePro clones are in use; their USB
identifiers and drivers are unknown to F3 and are the first thing to collect
from a tester.

**M6 — the tester programme and the first live tests (F13, size M).**
Safety and consent text, a plain statement of what the application does and
does not do, report submission. Two first cars are enough to validate the two
architectures: one two-bus-era car — X250, X150, X351 MY10–13, L319 or L320
MY10 onward, L405 MY13 — to confirm the documented routes, and one
2014-and-later car — L405 MY14+, L494, L538 MY14+, X760 — to confirm or
refute the gateway-relay hypothesis. Each first read yields the first
`VEHICLE_CONFIRMED` records the project has ever had.

After M6: F14 K-line for the early Range Rover's body modules; then, only by
explicit owner decision, stage 2.

### What "ready to connect to a live car" means, concretely

A signed installer that starts on Windows 10 or 11 without a virtual
machine, Docker or driver installation; a MongoosePro JLR recognised over the
inbox driver; the data library loaded; the car chosen from a list; the
survey shown; a capture and a fault-code read from at least the PCM completed
and saved as a report; every action read-only; every unsupported module
shown with its reason. M1, M4 and M5 make that true; M2 and M6 make it
worth doing.

### Interim runtime until the certificate exists (decided 2026-09-03, revised 2026-09-05)

**Revised 2026-09-05 by the owner.** The tester programme does not wait for
the certificate. The community should receive a working product first and
explanations second; its members already run SDD of their own as a matter of course,
so an unsigned build is not the obstacle it was assumed to be. Testers
install the unsigned CI build, `TESTER_GUIDE.md` names the exact Windows
prompt and what it means, and nobody is asked to switch off anything the
guide does not name. What follows in this section is the earlier, narrower
plan, kept for the record.

The certificate is deferred until the first real vehicle results. Until
then the unsigned installer that CI already builds runs on the owner's own
hardware only, and the first live sessions are a tester's car with the
owner's laptop, the owner operating. The library stays on that laptop, which
also defers `M5`.

The right machine for this is the owner's Windows 10 laptop: Windows 10 has
no Smart App Control, so the unsigned build runs after SmartScreen's per-file
"Run anyway", with no protection switched off. Turning Smart App Control off
on the Windows 11 desktop is the fallback, and only after confirming on that
build that the switch is reversible, as `F3_1_WINDOWS_SIGNING.md` records it
became in 2026. Two things must be confirmed on the laptop before a session:
that the MongoosePro JLR enumerates over the inbox `usbser` driver on Windows
10 as it does on Windows 11, and that the installer starts there.

The earlier line — no stranger installs an unsigned build — was withdrawn by
the owner on 2026-09-05; see the revision above.

### The two decisions that gate everything

| Decision | Blocks | What is needed from the owner |
| --- | --- | --- |
| `G2` signing identity | nothing since 2026-09-05 (was: any tester installing the application) | the entity type (individual or organisation) and the registered country |
| `G4` library distribution | any tester surveying a real car | whether and how SDD-derived manifests may be given to testers |

Both were taken on 2026-09-03: `G2` Ukraine, individual entrepreneur; `G4`
named testers only, separate from the installer. Later the same day the
owner deferred the certificate itself — account, identity verification,
purchase — until real vehicle results exist; see "Interim runtime" below. Neither needs a vehicle, an
account created by this project, or a purchase made on the owner's behalf;
the certificate itself is the owner's own purchase, for which F12 prepares
everything else.

## Proposed sequence

Owner confirmation required before starting. Nothing below needs a vehicle.

### F9 — SDD knowledge extraction and ingestion at scale (ungated)

**Started 2026-09-02. The surveyed SDD corpus is now fully ingested.** The
addressing, DID-catalogue, DTC, ODST, and DTC-index slices are implemented,
golden-tested, and have run against the real source. See
`docs/F9_SDD_KNOWLEDGE_INGESTION.md`, `ADR-0009`, and `ADR-0010`.

| Slice | Records |
| --- | --- |
| DTC descriptions, per code and qualifier | 93,521 |
| DID parameter definitions, all with units | 15,281 |
| DTC indexes and failure type bytes | 6,046 |
| On-demand self tests, stage 2, never available | 1,153 |
| Addressing | 161 |
| Platform addressing and networks | 3,056 |

Model-year resolution is implemented under `ADR-0011` as an opt-in derivation;
see `docs/research/sdd/MODEL_YEAR_BREAKPOINTS.md` for the reading it rests on,
which is corroborated rather than documented. Further breadth would come from
SDD components not yet surveyed, or from tester captures.


Turn SDD's diagnostic definitions into F5 `documented` evidence: module
inventories per platform, bus topology and addressing, identifier catalogues,
DTC definitions. Extend the F5 ingestion path from hand-placed fixtures to a
repeatable pipeline, since two artifacts do not exercise it meaningfully.

This is the highest-leverage phase in the project. Every stage-1 capability is
gated on knowledge rather than on Rust, and this is the only source that supplies
breadth. Provenance and classification are recorded per record; unknown stays
`UNKNOWN`.

#### F9 access path — how SDD data is actually obtained

Established 2026-09-01. SDD is **official JLR software**, not a grey-market
package: independent operators obtain it from JLR TOPIx at
`topix.jaguar.jlrext.com` under LINKS -> "SDD Manual Software Download".

**F9 needs only local installation and inspection** — the installer, its local
databases, EXML definition files, and configuration files. It needs no vehicle
connection, no programming, and no working J2534 driver. Diagnostic data is
deployed by installation, so the adapter never has to work for data to be read.

##### Resolved 2026-09-02: the installer download needs no subscription

The owner retrieved the full installer directly from JLR's own delivery host,
`diagnosticdelivery.jlrext.com/idscentral/SDD_169.00.001_FULL.exe`, without
purchasing anything. The two tiers are confirmed distinct:

- **installer download** — no subscription required;
- **vehicle connection and programming via official JLR services** — requires an
  active Diagnostic Programming subscription.

F9 needs only the first tier, so **F9 has no cost gate**. Artifact details,
hash, and the unsigned-distribution caveat are recorded in
`docs/research/sdd/PROVENANCE.md`.

Two access notes recorded so they are not rediscovered later:

- the CD bundled with a MongoosePro adapter is **not** a reliable SDD source. A
  CD holds about 700 MB while the current SDD download exceeds 2 GB, so a bundled
  disc is most likely the J2534 driver media, and any SDD on it would be a decade
  old. Do not buy an optical drive to chase it;
- the owner's hardware is an ASUS K56CB on Windows 10. Installing SDD there is a
  solvable prerequisites problem (.NET, database engine, compatibility shims) and
  is worth attacking once current media exists.

### F10 — multi-module read-only diagnostics (started 2026-09-02)

**Enumeration is implemented and golden-tested**; see
`docs/F10_MULTI_MODULE_DIAGNOSTICS.md`. `ADR-0012` decides how UDS read
execution reaches the application layer: a sibling `uds-execution` crate,
written and golden-tested offline on 2026-09-03. `ADR-0013` makes a module SDD
describes resolvable at all: family-level plans, per-module bus facts, and
adapter route bindings kept as evidence-backed knowledge rather than code.

**The composition root landed 2026-09-03 (`ADR-0014`).** The shell loads a
data library of exported manifests and surveys a vehicle: every module the
knowledge associates with it, reachable or not, with the resolver's reasons.
Run against the real corpus, an X250 MY2010 surveys as 39 modules, 28
reachable and 11 gatewayed ones named; an L405 MY2014 as 52 modules, none
reachable, each naming the unbound bus; an L322 MY2006 as 33 modules, none
reachable, split between the pending `normal_fixed` derivation and K-line.
See `docs/F10_MULTI_MODULE_DIAGNOSTICS.md`.

A listen-only capture landed the same day as the tester's zero-risk first
contact: it proves a live bus is on a J1962 pair and saves a `captured`
fixture, and it says plainly that it cannot name which bus.

`ADR-0015` then bound the 2014-and-later high-speed buses to `hs-can` as an
unverified research hypothesis and gave `mongoose-jlr` a live UDS read behind
the same prepared-transaction gate F8 uses for J1979. A tester's first
read-only request on such a car confirms or refutes the hypothesis; the
application shows it as a hypothesis until then. Fleet-wide, 617 modules are
reachable, 454 more await that confirmation, 360 wait on `normal_fixed`
identifier derivation and 274 on the medium-speed buses and sub-networks.

Remaining in F10, in order: the F13 intake that turns a tester's read report
into a `Captured` record; deriving `normal_fixed`
29-bit identifiers from the platform's prefixes and physical addresses with
ISO 15765-2 as `StandardDocumentation` evidence, which is what unlocks L322
MY04–MY07 and L319/L320 base over CAN; DID payload decoding from the
catalogue's encodings; joining decoded DTCs to the F9 descriptions; and
binding further buses, each with evidence. Describing the vehicle from a VIN
instead of typing SDD's programme and breakpoint marker is the deferred
"digital car" work.

**Bus coverage, decided 2026-09-03.** CAN first, K-line second, both in scope.

The SDD platform corpus classifies as 42 CAN-only programs, 0 K-line-only, and 3
mixed (L316 base, L322 MY06, L322 MY07). CAN therefore reaches every program;
K-line is needed only for the BMW-era body electronics of the early Range
Rover — DSM, HVAC, LCM, PAM, RCM and kin over DS2 on pin 7 — plus ABS and VIM
on the base L316. The MongoosePro JLR variant provides K-Line on pin 7 and
ROSCO on pin 8, verified from the user guide (`F2_JLR_NETWORK_INVENTORY.md`
addendum), so this is a software gap rather than a hardware limit.

Sequence: F10 completes multi-module diagnostics over CAN, which serves 93% of
programs fully and the powertrain and chassis of the other three; F14 adds
ISO9141/ISO14230 transport over pins 7/8 as its own phase with its own ADR,
because it is a new transport implementation and must not be debugged at the
same time as the first multi-module pipeline. Gatewayed sub-networks (MOST,
SUB_CAN1, NGI) stay stage 2 because gateway access is `ROUTINE_CONTROL`.

Whatever is unsupported is **shown**, never omitted: a module the application
cannot reach is listed with the reason, so a CAN-only build on an early Range
Rover states plainly which modules await K-line.

The first phase where the application addresses more than one ECU. Read-only UDS
services — `0x19` ReadDTCInformation and `0x22` ReadDataByIdentifier — plus
knowledge-driven module addressing and routing across buses. Extends F7's
prepared-transaction model from one target to many.

The architecture check must keep rejecting active and programming request
constructors. Read services only.

### F11 — application shell for real use (parallelisable with F9 and F10)

M1 done 2026-09-04; see `F11_TESTER_APPLICATION.md` for what is in and what
stays open.

The owner's explicit request: a finished desktop application rather than a test
harness. Module tree, DTC view with descriptions, live-data screens, session
flow, report screens, honest unsupported and error states, finished
presentation. The existing five components and 745-line stylesheet are a base,
not a rewrite.

Frontend work does not depend on F9's data landing, so it can proceed in
parallel — but it must not ship screens implying capability the knowledge base
does not have.

### F12 — signing and distribution (gated on `G2`)

Formerly F3.1, now critical path because it gates every tester. Strangers must
never be asked to bypass Windows security to run software that talks to their
car. The pipeline is committed and needs adapting to whichever CA the region
gate selects.

### F13 — tester programme (gated on F11 and F12)

*2026-09-09, ADR-0020:* the bench — a virtual vehicle answering on the adapter's own protocol from the loaded library — joins this phase, before the hand-out: it lowers the cost of deciding to take part to one installation, lets a tester walk the flow before the car, and gives CI an end-to-end test of the live path. It proves software only; VEHICLE_CONFIRMED still comes from cars. *Built the same day:* `IMPLEMENTED / FIXTURE_TESTED`, see `CURRENT_STATE.md`.

Tester-facing safety documentation and consent, a plain statement of what the
application does and does not do, report submission, and the intake path that
turns a returned report into F5 evidence. Testers operate their own vehicles at
their own discretion; the application must never imply otherwise.

### F14 — K-line transport for the legacy buses (after F10; not gating F13)

ISO 9141 / ISO 14230 over J1962 pins 7 and 8, which the MongoosePro JLR variant
provides as K-Line and ROSCO. Needed only for the BMW-era body electronics of
L322 MY06–MY07 over `DS2_PIN7` and for ABS and VIM on the base L316; every
other programme is fully served over CAN. A new transport implementation, so it
gets its own ADR and its own fixtures, and it must not be debugged alongside
the first multi-module pipeline. Until it lands, modules on those buses are
shown with the reason they are unreachable, never omitted.

### F15 — live reading: the same reads, repeated (decided 2026-09-10, `ADR-0022`)

The owner asked for visualisation — gauges, the car seen from above, cards
for fault codes — and the library was read for what it holds before
answering: four of his six blocks are served, per module, by 15,281
identifier parameters; restraints and the traction battery are not served
at all. `ADR-0022` decides the operation: `LIVE_READ`, class `READ_ONLY`, a
loop of the reads the product already makes, in the default session, one
request in flight, at most sixteen pairs, a floor of 100 ms between
requests, a ten-minute cap, a stop that gets through between any two
requests, every sample recorded in the report with the marks a single read
carries. The cadence is measured before any gauge is drawn. Standard J1979
mode 01 and 02 follow as their own slice. Order: the ADR, the cadence on
the bench, the service with a plain table, then the visual layer — and the
first real tester report outranks the last two, because they build over a
read path that has never met a vehicle. The owner names this the start of
the 1.1 line. *Measured 2026-09-10:* the link costs ~16 ms a round trip, the
software 0.01 ms, so the cadence is the link and the module. *Decision 7
built the same day:* the legislated services, codec to screen, on the
standard's own addressing, `IMPLEMENTED / FIXTURE_TESTED`. *Decision 9(c)
built the same day:* `LiveReadService` and the plain table — the set from
the library, the floor and the cap enforced in the shell, the samples
recorded whole, `IMPLEMENTED / FIXTURE_TESTED`. What remains of F15 is the
visual layer, and the first real tester report still outranks it.

### F16 — the odometer, read from every module (decided 2026-09-11, `ADR-0024`)

A car keeps its mileage in dozens of places, and a rolled-back car is rolled
back only where the tool could reach. The catalogue was read before this was
decided: SDD declares `0xDD01` **Total distance** for **92 module families**
over 21 programmes, the cluster's own `0x61BB` beside it, and — sharper still
— odometer stamps inside event histories: a gearbox stall, a failed gear
selection, a delayed park engagement, the Jaguar flight recorder. An incident
recorded at a mileage the car has not reached is not an ambiguous signal.

`ADR-0024` decides the operation: `MILEAGE_SURVEY`, class `READ_ONLY`, every
reachable module asked once, a table of module, identifier, reading and the
difference from the highest reading on the car, with event stamps shown apart
and compared to it. No verdict is ever printed — a number in a column is the
whole of what is claimed, a module that stayed silent is silent and not a
zero, and the legislated "distance since the lamp came on" is excluded by
name so it can never be mistaken for an odometer. Nothing is written, in this
stage or any other: correcting an odometer is the fraud the feature exists to
expose.

It is the network check with one identifier instead of fault codes, so it
needs no new crate and no new safety class. It is also the first capability
that serves someone who does not own the car yet — which reaches further than
the tester programme has so far.

### F17 — the numbers say their names in the user's language (decided 2026-09-11, `ADR-0025`)

Every reading arrives with a name from SDD's catalogue, in English: 15,281
parameter definitions under 4,977 distinct names, shown raw in all three
interfaces. Two translations turned out to exist and the choice had to be
measured rather than assumed.

SDD publishes its whole catalogue in twelve languages, Russian among them and
Ukrainian not — 15,269 keys in the Russian component against 15,269 in the
English, nothing left in English, no name with two renderings. The owner
translated all 4,977 names himself, into Russian **and** Ukrainian, and his
file won on the numbers: exact coverage including SDD's double spaces, `bank`
rendered one way in 254 names against JLR's 249 `блок` and 5 `ряд`, a mean
name 69.8 characters against 78.3 for a column that shares its row with a
value, and `Total distance` reading `Общий пробег` where JLR reads `Общее
расстояние` — wrong for an odometer, and the very parameter F16 asks every
module for.

`ADR-0025` decides what it is: a dictionary, not knowledge. It never enters
the knowledge store, carries no evidence and no validation state, and can
never move what the product believes about a car, because it says how a
phrase reads and nothing about any vehicle. It lives in the repository beside
`dtc_standard_text.tsv` — the owner's own work, not JLR's text — and the
English stays the identity: the report keeps it whatever the interface
language, because evidence does not change language. A name with no row shows
its English, one name at a time.

Ukrainian becomes a full interface for vehicle data for the first time.

### Beyond stage 1

Service functions and configuration follow only after an explicit owner decision
and the ADRs described under "Scope". Not scheduled here yet.

## Currently prohibited

Under `SAFETY_BOUNDARIES.md` and `ADR-0005`, unchanged by the staged scope above:
ECU firmware flashing, VBF programming, bootloader or recovery flashing, key,
immobilizer, or security programming, arbitrary EEPROM writes, any public
arbitrary CAN transmit API, and use of X250 diagnostic connector pins 12/13.

Stage 2 and stage 3 are declared future direction. Neither is authorisation. Any
move into them requires its own ADR — and for firmware programming, an ADR that
explicitly supersedes `ADR-0005`. Silent scope expansion is the specific failure
mode these documents exist to prevent.
