# Community notes on SDD in practice

Reports from enthusiasts, recorded verbatim in substance and dated. They
are `UnverifiedResearch`-class observations about how SDD is used today,
kept here to keep the product's positioning honest; nothing in this file is
vehicle knowledge and nothing enters the knowledge base.

## 2026-09-05 — reply to the first announcement (owner's community group)

Paraphrased from a member's reply to the intent announcement:

- An unofficial SDD build installs on Windows 11 in thirty minutes to an
  hour; the browser problem is worked around by running Edge in Internet
  Explorer mode with `http://localhost:8080` (not https) in the launch
  settings.
- Drivers for a Chinese-made MongoosePro clone are pulled in by that
  installation.
- On macOS with Apple Silicon it does not run in a virtual machine (VMware,
  Parallels); on older Intel Macs it does not run from a virtual machine
  either, but it did run from Boot Camp on a 2017 MacBook Pro (Intel i7).
- Every SDD feature is available except TOPIx.

### What this changes

The announcement's premise — that SDD means Windows 7, virtual machines and
installation misery — is out of date for the part of the audience that
accepts an unofficial build. For them SDD is an hour's work, and it does
everything, writes included. The project's honest position for that
audience is therefore not "a replacement for SDD" but:

- native on Windows 10/11 **and macOS including Apple Silicon**, which the
  member himself lists as SDD's weakness; mobile later;
- no unofficial distribution, no TOPIx dependency;
- read-only in its first stage, by construction — with the write stages
  declared and planned (`ROADMAP.md`: stage 2 service functions and DTC
  clear, stage 3 configuration), each behind its own safety class, ADR and
  explicit confirmation;
- every module explained — reachable, why not, or a marked hypothesis —
  and tester reports turning into evidence for everyone;
- SDD 169 is frozen; this project grows.

What it does not offer today, and must say so plainly: any write — that is
the next stage, not the current one. What stays outside the product is only
what `SAFETY_BOUNDARIES.md` names `FORBIDDEN_PROGRAMMING`: firmware,
bootloader and recovery flashing, key, immobiliser and security
programming. SDD's symptom-driven flows are not planned.

### What it asks next

The member runs a MongoosePro **clone**. Adapter discovery is
`HARDWARE_CONFIRMED` for one device only (USB VID 18E1, PID 0104, inbox
`usbser` driver, Windows 11). A clone may present a different VID/PID or
driver; the first thing to learn from such a tester is the device's USB
identifiers and driver name from Device Manager, before any session.

## 2026-09-05 — bus topology of the 2014-and-later cars (owner's message)

The owner relayed a statement, source not yet named, in three points:

- **BO and CO are two buses.** The car physically has two separate MS-CAN
  buses: `BO_MSCAN` (body — body electronics, climate, doors) and
  `CO_MSCAN` (comfort — multimedia, seats, panel).
- **The gateway switches.** Neither reaches the OBD connector's pins 3 and
  11 directly at the same time. The central gateway — GWM, or the central
  junction box CJB/BCM — joins the buses and relays or switches the
  scanner's (SDD's) diagnostic requests to the BO or CO branch.
- **HS-CAN is on pins 6 and 14**, the classic factory pins, for engine,
  gearbox and brakes.

### What this changes

It is the sentence ADR-0015 lacked. The platform documents of seventeen
programme-years (L405 MY14/16, L494 MY14/16, L538 MY14/16/17, L538C and
L538JV MY16, L550 MY15/17, X152 MY16, X260 MY16, X351 MY16, X760 MY16/17,
X761 MY17) declare `BO_MSCAN` and `CO_MSCAN` at 125 kbit/s, 11-bit, ISO
14229 over ISO 15765, every module addressed directly and no gateway
declared; they say nothing about pins. ADR-0015 hypothesised the high-speed
buses through `hs-can` on pins 6/14 and left the medium-speed ones
unbound, "because two candidates compete": relay through the second pair,
or through the diagnostic HS CAN. The statement chooses the second pair.

Recorded as what it is: an `UnverifiedResearch` evidence item in the
built-in hypothesis manifest, binding `BO_MSCAN` and `CO_MSCAN` to route
`ms-can` (J1962 pins 3/11, 125 kbit/s) in validation state `Unverified`.
The survey now shows a module on either bus as a *hypothesis* with
`UNVERIFIED` beside it instead of "bus unbound", and the first answered
read-only request on such a car turns the binding into captured evidence
through the F13 intake; its absence refutes it for that car.

### What it does not settle

- Whether the gateway relays transparently (a request on pins 3/11 for
  `0x740` reaches the DDM on `BO_MSCAN` unaided) or switches per request in
  a way that needs a routine SDD does not declare. Only a read tells.
- Whether every one of the seventeen programme-years is wired this way; the
  hypothesis is recorded for all of them and confirmed or refuted per car.
- The statement's provenance. If it comes from a wiring diagram or workshop
  manual page, that page can be cited and the evidence becomes documented;
  if from someone's measurement on a specific car, it is a capture-class
  observation for that car. Asked of the owner on 2026-09-05.

### The link that followed, 2026-09-05

The owner then pointed to a Facebook post by the page "Torque Craft"
(3 March, "Dive into the world of vehicle diagnostics with this clear OBD2
pinout diagram"). Read on 2026-09-05 without logging in: a generic OBD-II
diagram and a four-row table — pin 4 chassis ground, pin 6 CAN high, pin 14
CAN low, pin 16 battery supply. It says nothing about pins 3 and 11,
medium-speed CAN, a gateway, or Jaguar and Land Rover. It therefore neither
supports nor contradicts the statement above; the 6/14 fact it restates is
already held from ISO 15765-4, and the 3/11 pair as JLR's second CAN is
already held from the adapter's own documentation (`F2_JLR_NETWORK_INVENTORY.md`).
Not recorded as evidence. The provenance question stands.

## 2026-09-05 — a published Discovery 3 reverse-engineering series (tekonline.com.au)

The owner pointed to part 11 of "Discovery 3 CAN" on tekonline.com.au
(Brisbane; eleven parts, 5 May to 26 June 2026; author unnamed on the
pages). One person, one Land Rover Discovery 3 (L319), a LilyGO ESP32 CAN
board, and an SDD 130 virtual machine. Read on 2026-09-05 through a
summariser, not verbatim; nothing below is recorded as evidence.

What it confirms, from the outside, about a car we hold only from SDD's
documents: on the Discovery 3 the J1962 pair 6/14 at 500 kbit/s carries a
live bus with 29-bit identifiers (part 4: 4,644 frames, 27 identifiers in
the first capture); the air-suspension heights ride in broadcast frame
`0x12E9E6A0` (part 5, "candidate, scaling unknown"). Both agree with
`PLATFORM_L319.xml` (`CAN_HS` 500 kbit/s, 29-bit). Nothing about pins 3/11.

What it made us check in our own corpus (part 6 quotes SDD's per-module
configuration `<CAN srcId="0x33" targetId="0x2B"/>` for the RLM): every
module file under `CURRENT_JLR_XCL_XML_DATA_XML/Xml/<programme>_<year>/`
carries a `<VERONA bitLen= baudRate=>` block with
`<addressing type="physical" format=...><CAN srcId= targetId=/>`. Across
all 2,229 blocks: 2,077 are 11-bit `Normal` with full identifiers
(`srcId` the response, `targetId` the request; 2,056 of them response =
request + 8, the Ford convention); 152 are 29-bit — 102 `Fixed`, 50
`Enhanced` — and in **every one of the 152** `srcId` = `targetId` + 8
(RLM 0x2B/0x33, PCM 0x10/0x18, ACM 0x80/0x88, TPM 0xDA/0xE2 …). So the
29-bit `srcId` is the same "+8" bookkeeping carried over from the 11-bit
files, not a tester source address; SDD's data still states no tester
address anywhere, and ADR-0017's `0xF1` stays a hypothesis. Part 9's tool
tried seven identifier pairs for the RLM, among them `0x18DA2B33 /
0x18DA332B` (reading `srcId` literally as the tester) and `0x18DA2BF1 /
0x18DAF12B` (ISO 15765-4's tester), auto-detected the one that answered,
and never says which. The first tester read on a Discovery 3, Range Rover
Sport or Range Rover of 2005–2009 settles it; if `0xF1` is met with
silence, the alternative to try is source `0x33`-style, module address + 8.

What stays outside this project's boundaries and is not taken from the
series: part 7 (the Ford security-access algorithm), the decrypted
`Security.exml` (part 10) and the RLM calibration write (IOControl
`0x2F`, part 10) — security access and writes are stage 2 and beyond, each
behind its own ADR (`SAFETY_BOUNDARIES.md`). The EXML decryption route it
describes is not used here: this project's code does no decryption or key
handling.

## 2026-09-05 — IIDTool, the landscape (owner's note)

The owner's note: IIDTool by GAP Diagnostic is the best-known example of
an independent tool — a small device in the OBD socket with a smartphone
application — that does nearly everything SDD does, including full CCF
editing, air-suspension calibration and module flashing, on algorithms
that are "colossal work" of taking JLR's dealer protocols apart.

How this project relates to it, for the record:

- **It proves the thesis.** An independent, without JLR's blessing, reached
  dealer-level capability on these cars. The capability is reachable; the
  question is only the route.
- **The route differs.** IIDTool's knowledge is proprietary, obtained by
  reverse engineering, closed, and sold as a device locked to a vehicle.
  This project's knowledge is SDD's own diagnostic data — the same
  definitions the dealer tool reads — ingested at scale for every programme
  in the corpus, with every value carrying its source and validation
  state, on an open J2534-class adapter and not locked to a VIN. The work
  IIDTool had to do by hand, SDD's data mostly states.
- **What IIDTool has that this project does not yet:** the writes. CCF
  editing, calibrations and flashing are the declared later stages here
  (stage 2 and 3, `ROADMAP.md`), each with its own ADR, safety class and
  confirmation; module flashing, key and security programming stay outside
  the product (`FORBIDDEN_PROGRAMMING`).
- **What this project can have that IIDTool does not:** breadth (one
  application for the whole SDD-era range rather than one device per
  car), transparency of every claim, and a community that grows the
  evidence base with each session report.

## 2026-09-07 — X250 facelift, BCM after a battery sag (owner's observation)

Recorded as a community observation, not evidence; nothing here is
confirmed by a capture or a read.

- On the facelift XF (X250, 2012 on) a battery voltage sag is said to
  leave the body control module in a state where the car will not start:
  the community's wording is that the BCM "loses its firmware".
- The workaround people use is to re-programme their own keys with a
  dealer-class tool; the car then runs until the next battery drop. The
  full repair is said to be a BCM software reflash.
- The owner's point: a tool that could re-pair the owner's existing keys
  would keep such a car drivable without a tow to a specialist.

What this project can do with it, in order of what it costs:

1. Stage 1, now: diagnose the state exactly — the BCM's fault codes, the
   readable key-count and status identifiers, the modules that answer and
   do not — so the person knows what happened and what to ask for.
2. Stage 2 or 3, by its own ADR and safety class: re-pairing keys that
   are already the car's, developed and validated on a bench module with
   its keys, never on a customer's car first, behind identity of the user
   and a written log of every write. This is security programming and
   stays forbidden until that decision is taken.
3. Not at all: reflashing the BCM. That needs JLR's `.vbf` binaries and
   the programming bootloader path (`ADR-0005`).

Whether the failure is really lost firmware or a lost key
synchronisation is itself unknown; the read-only diagnosis of step 1 is
what would tell.

## 2026-09-09 — what third-party tooling reaches, relayed by the owner

The owner's statement, recorded as a statement and not as measurement:

- On the SDD-era Ford-derived cars, professional third-party scanners —
  Launch and Autel with their JLR software — read body and comfort modules
  through the OBD connector, which on those cars means the 3/11 pair, since
  those modules are not on the high-speed bus.
- ELM327-class dongles read the powertrain and nothing else, because they
  speak only pins 6/14.
- His conclusion, and it is sound: the presence of a medium-speed bus on
  3/11 is ordinary trade practice on these cars, not a hypothesis of ours.
  Thousands of people plug arbitrary devices into that connector and get
  from a few generic values to full diagnostics, depending only on what the
  device can speak.

This does not promote the binding to `CaptureValidated` — nothing but a
capture does — but it is why the record no longer calls it a weak point.
It says nothing about the 2014-and-later cars, where the gateway question
stands untouched.
