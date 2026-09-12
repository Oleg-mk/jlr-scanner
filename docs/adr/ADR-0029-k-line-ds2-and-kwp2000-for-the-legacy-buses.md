# ADR-0029: K-line — DS2 and KWP2000 for the legacy buses

- Status: Accepted, 2026-09-12 (the owner brought F14 forward on this day:
  "робимо зараз, з внесенням необхідних правок; якщо будуть проблеми —
  вирішимо, коли з'являться на діагностиці саме цих старих авто")
- Decision: the buses the platform documents call `ISO` — the BMW-era
  body electronics of the Range Rover L322 of 2006–2007 over **DS2**, its
  diesel engine and transfer case over **KWP2000\***, the ABS of the base
  Freelander L316 over **KWP2000**, its VIM over **ROSCO** — enter the
  product in three layers with three different standings: the buses, the
  modules and their node addresses as *documented* knowledge from SDD's own
  platform documents; the adapter routes `k-line-7` and `k-line-8` as
  *hypotheses*, because this adapter has never opened a K-line channel with
  a pin selected; and the protocols DS2 and KWP2000 as read-only protocol
  crates *implemented offline* against their public specifications, with
  KWP2000\* and ROSCO recorded as known and not spoken. Every module on
  these buses is shown in the survey with its bus, protocol, pin and node
  address, and with the honest reason it is or is not readable today.
- Reason: 12 body modules of the L322 MY06 (13 on MY07), its TCM, PCM and
  TCCM, and the ABS and VIM of the L316 are the only modules in the SDD-era
  fleet the product cannot reach over CAN. `ROADMAP.md` had F14 after the
  first live tests so that a new transport would not be debugged alongside
  the first multi-module pipeline; the owner weighed that against having
  the code ready for the first such car and chose to build now and correct
  on the car. That is a decision on order, not on scope, and it is his to
  take.
- Consequence: the platform ingest reads `<iso>` buses; a hypothesis
  manifest binds six SDD bus names to two adapter routes; two new crates,
  `ds2` and `kwp2000`, hold the framing and the read-only services; the
  Mongoose device gains the two routes and, in the second slice, a K-line
  execution path whose device commands are marked hypothesis until the
  adapter answers them; a probe script for the adapter alone; the survey
  says what it knows. Nothing is written to any module: the clear-fault
  services of both protocols (`DS2 0x05`, `KWP 0x14`) and every session,
  security, routine and write service are absent, and the architecture
  check enforces it.

## What SDD says, measured

Read from the platform documents on 2026-09-12:

| programme, marker | bus | modules | physical layer | wake-up | pin |
| --- | --- | --- | --- | --- | --- |
| L322 MY06, MY07 | `DS2` | BPM DSM FBH HVAC LCM MFSW PAM RAIN RCM SASM SCLM VDM (MY07: BPM DSM FBH HVAC LCM PAM RAIN SCLM) | ISO 9141, 9600 baud, 8 data bits, even parity, 1 stop | `bmw_ds2` | **not stated** |
| L322 MY06 | `DS2_PIN7` | TCM | as above | `bmw_ds2` | 7 (`K_LINE_PIN_7`) |
| L322 MY06 | `KW2000STAR` | PCM (diesel) at 0x12 | ISO 9141, 9600, 8N1 | `bmw_kw2000_star` | **not stated** |
| L322 MY06 | `KW2000STAR_PIN8` | TCCM at 0x34 | ISO 9141, 9600, 8N1 | `bmw_kw2000_star` | 8 (`K_LINE_PIN_8`) |
| L316 MY07, MY11, MY12 | `KW2000` | ABS at 0x29 | ISO 9141, 10400, 8E1 | `kw2000_fast` | 7 (`ISO_K_KW2000`) |
| L316 MY07, MY11, MY12 | `ROSCO` | VIM at 0x1C | 9600, 8N1 | `rosco` | 8 (`ISO_K_ROSCO`) |

Each module has a one-byte node address in `<address type="phys"
session="diag">` (the DSM is `0x72`); the module documents of the DS2
modules are stubs, the L316's name the protocol (`VKW2000`, `ROSCO`) and
the address. What SDD reads from them, by the identifier sets: DS2 — the
identification (`service 0x00`) and two "AIF" blocks (`0x02`); KWP2000 and
KWP2000\* — `ReadEcuIdentification 0x1A` with local identifiers (`0x8A`,
`0x8B`, `0x8D`, `0xE0` on the ABS; `0x80` on the BMW units); ROSCO — an
identification under `0x9B`. Fault descriptions exist: 57 index entries
and 259 help documents name the DSM, LCM and PAM with 16-bit codes such as
`0x000B`.

**The adapter.** The firmware's own words, recorded in `ADR-0018` and the
2026-09-06 sweep: resources **3 and 4** open and answer a wrong pin pair
with *"supports ISO9141 K line on pin 3, 7 or 8"*; resource 13 *"UEB on pin
7"*; 25 and 26 *"ISO9141_PS K line on pin 15"*. The user guide gives the JLR
variant K-Line on pin 7 and ROSCO on pin 8. In the J2534 numbering the
firmware appears to follow, 3 is ISO 9141 and 4 is ISO 14230. What we have
never seen: the pin-select word for a single line, the wake-up command, and
the outbound record for K-line bytes. Those are the hypotheses of the
second slice, and only the adapter can confirm them — no car is needed for
that.

## Decisions

1. **Three standings, kept apart.** The bus, its physical layer, its
   wake-up, the stated pin and each module's node address are recorded from
   the platform document as documented knowledge: `NetworkRoute` on the
   bus's own name with the pin and the baud rate where the document states
   them, `sdd_iso_settings` and `sdd_iso_wakeup` as text claims, and
   `DiagnosticAddressing` on the module with the node address in both
   identifier fields and the addressing mode `iso9141_node`. The adapter
   route — which Mongoose route carries which bus — is an
   `unverified_research` hypothesis in its own manifest, resolved as
   `Unverified`, shown as a hypothesis, confirmed by the first answer from a
   module on that bus (`ADR-0015`'s rule). The protocol implementation is
   `IMPLEMENTED / FIXTURE_TESTED` against the standards and nothing more.

2. **Two routes, chosen by pin; the resource chosen by protocol.**
   `k-line-7` and `k-line-8` are Mongoose routes on J1962 pin 7 and pin 8,
   with no fixed bit rate — the bus states 9600 or 10400. Opening one
   selects the firmware resource by the protocol's physical layer: 3 for
   ISO 9141 framing (DS2, ROSCO), 4 for ISO 14230 (KWP2000). Neither route
   has a listen-only mode; a capture on them is refused with that reason.

3. **The bindings.** `DS2_PIN7` and `KW2000` → `k-line-7`; `ROSCO` and
   `KW2000STAR_PIN8` → `k-line-8` — the pin from the document, the route the
   hypothesis. `DS2` and `KW2000STAR`, whose documents state no pin, →
   `k-line-7` on a weaker hypothesis: the same documents name `DS2_PIN7` for
   the one module they had to place explicitly, and the user guide gives
   K-Line on pin 7; recorded as such, refuted the day a module answers on
   pin 8.

4. **Two protocols spoken, two named.** `ds2`: the frame is node address,
   length (the whole frame), data, XOR checksum; the requests are the
   identification (`0x00`) and the fault memory (`0x04`); the reply's first
   data byte `0xA0` is acceptance, anything else is the module's own answer
   shown as such. `kwp2000` (ISO 14230): physical addressing with the
   format byte, target, source and a sum checksum; the fast-init
   `StartCommunication 0x81` as the link handshake the standard requires
   before any request — a handshake, not a diagnostic session —
   `ReadEcuIdentification 0x1A` and `ReadDiagnosticTroubleCodesByStatus
   0x18`; nothing else. KWP2000\* — BMW's variant with its own wake-up and
   8N1 framing — and ROSCO — a JLR protocol the corpus names and does not
   describe — are recorded on their modules as protocols with **no read-only
   capability**, so the survey says "protocol KW2000STAR: not spoken by this
   product" rather than trying the wrong one.

5. **What a read returns is shown as read.** A DS2 identification is the
   bytes and their printable text; a DS2 fault memory is the bytes, with the
   module's 16-bit codes joined to SDD's index where the code is in it and
   raw where it is not. A KWP identification is the text of the record; a
   KWP fault list is code, status byte and the index's description where
   there is one. Nothing is interpreted beyond what the frame says.

6. **The second slice and its hypotheses.** A K-line execution path
   prepares a transaction from the resolver's plan — route, pin, baud,
   framing, wake-up, node address, request bytes, capability — and the
   Mongoose device executes it: open the resource, select the pin, wake the
   bus as the bus says, send, wait, close. The pin-select word, the wake-up
   command and the outbound record are written to the J2534 shape the
   firmware has followed so far and are **hypotheses**, each parameterised
   in one place and each tried by `scripts/mongoose-probe/mongoose_kline_probe.py`
   against the adapter alone, which answers with its own status texts as it
   did for the resource sweep. Until that probe has run, the path is
   `IMPLEMENTED / UNVERIFIED` and the shell says so before any read.

7. **Nothing that writes.** No clear-fault service, no security access, no
   session control beyond the link handshake, no routine, no write. The
   architecture check lists the forbidden constructors of both crates by
   name.

## What this does not decide

- The read of the "AIF" blocks and other DS2 data beyond identification and
  fault memory; the decode of module-specific fault-memory layouts.
- KWP2000\* and ROSCO: named, not spoken. Speaking them needs a capture of
  SDD talking to such a module, or a specification.
- SCP (J1850 PWM) and NVJCOM (JAGCAN) of the older Jaguars: different
  physical layers, their own decision.
- Where the `DS2` bus without a stated pin really is: decided by the first
  L322 that answers.

## Built

**Slice A, 2026-09-12 — `IMPLEMENTED / FIXTURE_TESTED`.** Nothing has met
an adapter or a car; no request travels on a K-line.

- *Knowledge.* `PlatformAdapter` reads the `<iso>` buses: the rate as the
  bit rate, the `settings` as `sdd_iso_settings`
  (`data_bits=8;parity=even;stop_bits=1`), the wake-up as `sdd_iso_wakeup`,
  the connection pin's name as a J1962 pin number through the document's own
  `<connector>`; the `NetworkRoute` sits on the bus's own name, where the
  resolver looks for a physical route. A module with a `phys` address on such
  a bus gets `DiagnosticAddressing` with the node address in both identifier
  fields, no CAN identifier format and the mode `iso9141_node` — a word that
  lives in the `knowledge` crate (`ISO9141_NODE_ADDRESSING_MODE`) because the
  resolver reads it too. Capabilities by protocol: DS2 two, KW2000 two,
  KW2000\* and ROSCO none. Real source: 18 K-line bus records over
  14 platform documents, 70 node-addressed
  module records, 44 K-line capability records — and, because the older
  Jaguars' documents describe their `ISO` bus the same way, nine `ISO` bus
  records of the X100–X400 era with 43 NVJCOM modules on them, recorded as
  the documents state them, bound to no route and given no capability,
  exactly as *What this does not decide* leaves them; the library
  re-exported to 353,626 records (from 353,450), nothing rejected; the
  owner's copy re-stamped (code 2CD9-E6D0, valid to 2026-10-12).
- *Hypotheses.* `fixtures/knowledge/research/mongoose_jlr_kline_route_hypotheses.json`,
  the sixth built-in manifest: `k-line-7` for `DS2_PIN7` and `KW2000`,
  `k-line-8` for `KW2000STAR_PIN8` and `ROSCO`; `DS2` and `KW2000STAR` get a
  pinned `NetworkRoute` (pin 7, 9600) beside their backend route, on the
  weaker evidence, as decision 3 says.
- *Resolver.* One rule it did not have: the plan's `can_id_format` is an
  `Option` now — required under every CAN addressing mode, not asked for
  under `iso9141_node`, never invented. `uds-execution` and
  `diagnostic-execution` refuse a plan without one (`MissingCanIdFormat`); a
  UDS or J1979 read never gets that far on a K-line plan, the protocol check
  comes first. F6 test:
  `a_serial_node_addressing_needs_no_can_identifier_format`.
- *Protocols.* `crates/ds2`: the frame, the two requests, echo stripping,
  the payload as bytes and printable text, `words_from` as candidates for
  the fault index with no layout decided — 9 golden tests. `crates/kwp2000`:
  the physical header in both length forms, the sum checksum, the three
  requests, positive and negative responses with the response codes named,
  echo stripping — 11 golden tests. Both encoders private. The architecture
  check names the forbidden constructors, counts the request constructors
  (two and three) and forbids `ds2` and `kwp2000` wherever it forbids `uds`.
- *Adapter.* Routes `k-line-7` and `k-line-8`: pins 7 and 8, no bit rate of
  their own, `NetworkType::Other`, `Blocked` with the reason; the resource
  word is `None` for them and the device refuses every open of them both
  before and after the firmware check. The interface table shows them as a
  *route hypothesis* — a third `InterfaceImplementationState`, `Hypothesis`,
  the ADR had not named — with nothing confirmed.
- *Survey.* A module whose protocol is DS2 or KW2000 is resolved against its
  protocol's own read-only capabilities; every K-line row carries the reason
  the line is not opened yet; a protocol named and not spoken says so in
  those words. On the synthetic platform: `DS2MOD` a hypothesis with pin 7,
  9600, node `0x72`; `STARMOD` indeterminate with *protocol KW2000STAR: named
  in the data and not spoken by this product*.
- *Tests, in all:* the platform golden test for the buses, the addressing
  and the capabilities; the survey tests (six modules, two hypotheses); the
  built-in library counts (six manifests); the route tests; the F6 rule; 20
  protocol golden tests; 88 interface tests with the new interface row.

**Departures from the text above.** Decision 2 says opening a K-line route
selects the firmware resource by protocol; in this slice the route has no
resource word and refuses to open — the selection is the second slice's,
where the probe can confirm it. Decision 4's "the survey says *protocol
KW2000STAR: not spoken by this product*" reads *named in the data and not
spoken by this product*.

**Slice B — not built.** The `kline-execution` crate, the device's K-line
commands as hypotheses, the probe script, the shell read through the shared
record and intake, the bench's DS2 and KWP2000 answers.
