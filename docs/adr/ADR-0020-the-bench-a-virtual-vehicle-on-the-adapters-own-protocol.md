# ADR-0020: The bench — a virtual vehicle on the adapter's own protocol

Status: accepted, 2026-09-09. Owner's decision in discussion; no code yet.

## Context

The tester programme is the only road to `VEHICLE_CONFIRMED`, and its
binding constraint is people: to decide whether to take part, a person
today needs an adapter, a car, an installer, a stamped library and an
hour. Their first session in a car is also their first session ever.
Separately, CI cannot exercise the live path at all — survey, route open,
ISO-TP, UDS, report — because nothing in CI has an adapter; that path is
proven only by transport-fake unit tests.

Two things make a virtual vehicle cheap and honest here. The MongoosePro
JLR protocol is known byte for byte (`ADR-0018`), so a fake adapter can
answer with the same frames the real one does. And the knowledge library
already holds, per vehicle, what a car would answer: every module with its
request and response identifier and bus, its readable identifiers with
their converters, its fault-code wording. The same base that tells the
application what to ask can tell a simulator what to answer, for all 22
programmes at once.

The owner's separate project, Digital Jaguar X250 (2026-08, Python, two
hand-written modules, 245 tests), showed the useful kind of digital model
— externally observable diagnostic behaviour, no firmware — and supplied
three ideas taken here: an evidence class on every value independent of the
value, a candidate that never answers unless a profile mounts it, and a
logical clock for determinism. Its code is not merged: different language,
a second evidence model, a second catalogue.

## Decision

1. **A bench transport on the adapter's protocol.** A new crate,
   `transport-bench`, implements `transport_api::ByteTransport` and speaks
   the MongoosePro JLR framing exactly as the device expects it: board-info
   answering from the bootloader until `cJumpToFirmware`, resources 5 and
   21 opening with their pins, close, and `cOutboundData` answered with
   `cInboundData` frames. It knows no vehicle. It hands each CAN frame to a
   `BenchBus` trait — frame in, frames out, by route — and encodes what
   comes back. Substitution happens at the lowest level so that everything
   above it, in the shell and the crates, runs unchanged and real.
2. **A vehicle built from the library.** A crate `bench-vehicle`, in the
   session layer, implements `BenchBus` from the loaded library: one
   responder per module the survey names, on its bus and identifiers,
   answering ReadDTCInformation and ReadDataByIdentifier for the
   identifiers the library marks readable, with ISO-TP and UDS from the
   existing crates. Values are shaped by the library's converters so units
   and ranges read as on a real screen, and they are synthetic, made from a
   seed, deterministic. A module the library does not mount does not
   answer.
3. **The shell chooses the transport.** `MongooseJlrDevice<SerialTransport>`
   becomes generic over `ByteTransport`; the adapter panel gains one
   choice beside "Detect adapter": connect the bench. Nothing else in the
   shell distinguishes the two.
4. **A session is bench or real, never both.** The mode is a property of
   the session. Connecting an adapter of the other kind starts a new
   session, so a real report can never contain a bench record.
5. **Nothing is written in bench mode.** The shell refuses "save report"
   and "save capture" while the session is bench — a refusal in the
   command, not a hidden button. The report step shows on screen what the
   report would hold, under the bench banner, so a tester learns its shape
   without receiving a file.
6. **The marker stays anyway.** Every snapshot the shell produces in bench
   mode carries the mode; every validation label on a bench value reads
   `SYNTHETIC`; and `report-intake` refuses a bench report by
   construction, should one ever exist. Three layers: no file, a marked
   file, a refused file.
7. **The whole screen says so.** While the bench is connected the ground
   of the application changes from the green to another JLR colour — a
   light sand, after Range Rover's Kaikoura Stone, proposed as
   `#efe6d6` with surfaces and borders tinted to match, the owner's swatch
   to confirm — and a band with the word BENCH, in the interface language,
   stays in the header and the footer. A screenshot cannot pass for a car.
8. **Scope.** The bench answers diagnostics. It has no vehicle dynamics, no
   driver workplace, no ignition, no fault injection beyond what the
   library's fault-code catalogue allows. It stays a bench.
9. **CI drives it.** One end-to-end test runs the shell's services against
   the bench on Linux: survey, network check, module read, capture, report
   preview. The live path is then proven on every commit, as software.

## Consequences

- `VEHICLE_CONFIRMED` is unreachable from the bench, by the project's own
  rule that synthetic fixtures prove protocol behaviour only. The bench is
  a stand and a shop window: recruitment, onboarding, regression, and a
  way to tell a car's fault from the software's, by repeating a failed
  step on the bench.
- Two boundaries to respect at implementation: `transport-bench` holds
  framing only, `bench-vehicle` holds the answers, and neither adds a
  transmit API to the application. `scripts/check-architecture.mjs` is run
  against the design before the first commit, and extended if its rules
  do not yet name the bench crates.
- The shell refactor to a generic transport is the one change to code that
  already works; it is covered by the existing shell tests.
- The tester guides gain a step: try the bench before the car.
- Digital Jaguar X250 remains an idea source; nothing of it is copied.

## Amendment at implementation (2026-09-09)

The bench was built the same day, and three points of the decision met the
code differently from the text above; the decision stands, the text is
corrected here rather than rewritten.

1. **The bench transport lives in `mongoose-jlr`, not in a crate of its
   own.** Decision 1 named a `transport-bench` crate. The framing it must
   speak is the MongoosePro JLR protocol, and the architecture forbids a
   transport crate to know an adapter protocol — that knowledge belongs to
   `mongoose-jlr` alone. So the bench transport is
   `mongoose_jlr::bench::BenchTransport`, a `ByteTransport` that answers
   the device code with the firmware's own records (board-info from the
   bootloader until the jump, resources 5 and 21 with their pins, close,
   outbound answered with inbound frames), and hands each CAN frame to a
   `transport_api::BenchBus` — the trait is in `transport-api`, where a
   protocol-neutral contract belongs. `bench-vehicle` implements the trait
   from the library and depends on no transport crate and no adapter
   crate; `scripts/check-architecture.mjs` names it.
2. **The shell keeps one device type per transport.** Decision 3 spoke of
   the device becoming generic; it already was. The shell holds a `Link`
   of either `MongooseJlrDevice<SerialTransport>` or
   `MongooseJlrDevice<BenchTransport>` and calls the same methods on both;
   the bench is connected by one command, `connect_bench`, with no port
   and no discovery, and appears in the adapter panel as transport `bench`,
   backend `bench-vehicle`. The vehicle on the bench follows the session's
   survey: every survey rebuilds it from the loaded library.
3. **The colour is the owner's choice B, not the proposed sand.** The
   ground while the bench is connected is a warm Namib Orange tint,
   `#f6e3d3`, with surfaces `#f0d6c2` and borders `#e2c1a6` / `#cfa585`,
   and the band is `#c8763a` with the words «СТЕНД · віртуальне авто ·
   дані синтетичні» in the interface language, in the header and at the
   foot.

The session rule of decision 4 is enforced twice: the interface asks for a
new session before connecting the other kind while records exist, and the
shell's `connect_adapter` and `connect_bench` refuse the switch on their
own (`SESSION_MODE_MISMATCH`), keeping whatever is connected. Decision 5
holds in the one command that writes, `save_text_file`, which returns an
error for a bench session; the report step shows the bundle on screen
instead. Decision 6 holds in three places: captures become `Synthetic`
fixtures named `bench-…` with the validation
`synthetic_bench_capture_not_vehicle_evidence`, module reads carry
`route_validation: SYNTHETIC`, calibration reads carry the execution source
`BENCH_SYNTHETIC`, the session bundle carries `session_mode: bench`, and
`report-intake` refuses that bundle. Decision 9 is the shell test
`bench_e2e`: connect, survey, an identifier read and a fault-code read
through the real UDS stack, a listen, the bundle, the intake's refusal and
the refused switch — 22 shell tests in all, plus four interface tests and
the crates' own.


## Amendment, 2026-09-09: the fault codes follow a scenario number

The first bench reported one or two codes per module from a list of eight
written into the crate, filtered by whether the library described them. That
proved the read path and nothing else: one picture, every time, out of eight
codes. The owner asked for variety — draw two or three at random from the
whole library — and named the reason: a bench that never reports anything
leaves you unable to tell a working bench from a broken one.

Random alone would have cost two things. A drawn code that the library does
not describe for that module shows as a code with no wording, which reads on
screen as a hole in the application rather than as variety. And nothing would
be reproducible: a tester's «I saw this» could not be repeated, and the
shell's end-to-end test could not assert anything.

So the draw is seeded, and the seed is a number the tester chooses when
connecting: the **scenario**.

- `0` is the healthy vehicle. Every module answers and none reports a code.
  This is the case a car in good order presents, and the application has to
  show it as plainly as a broken one; nothing else in the product could
  produce it before.
- Any other number gives each module none, one or two codes, drawn from
  those the library describes **for that module's own family** — module-scoped
  wording first, the programme-independent wording after. Every code reported
  therefore has wording, because that is what it was drawn from.
- The seed is the number mixed with the family name, so two modules of one
  scenario draw differently, and one module draws the same on every run, on
  every machine, from the same library.
- The number is shown in the BENCH band and on the adapter card, and a tester
  who quotes it can be shown the same screen again.

`KnowledgeLibrary::dtc_codes_for` is the new seam: the list of codes the
loaded data describes for a family, in a fixed order. It is the library
answering what it holds, which is where that question belongs; the bench only
draws from it.

What does not change: the codes are still not a car's, every value is still
marked synthetic, a bench session is still refused by `report-intake`, and the
picture is still illustration, never evidence.
