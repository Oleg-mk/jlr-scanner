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
