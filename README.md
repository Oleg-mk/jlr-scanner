# ProwlOne

**Multi-platform vehicle diagnostics.** Independent, read-only diagnostics for Jaguar and Land Rover of the SDD era —
every module on every bus, manufacturer-specific identifiers and fault codes
with the manufacturer's own wording — through a MongoosePro JLR adapter,
without SDD's Windows 7, virtual machine and install misery.

It is not a generic scanner: a cheap OBD-II dongle already covers that. It is
not an SDD clone or frontend either. It replaces what SDD does for reading a
car, on a modern stack: a Rust diagnostic core, a Tauri 2 shell, one React
interface in English, Russian and Ukrainian, on Windows and macOS.

## Status

Stage 1, read-only. Honest validation states, per phase, are in
[docs/CURRENT_STATE.md](docs/CURRENT_STATE.md); the short version:

- the application, the vehicle survey, VIN decoding, module reads, bus
  capture and session reports are implemented and fixture-tested;
- the adapter path — jump from the bootloader, HS-CAN and MS-CAN channel
  open, listen, close — is confirmed on real hardware on the bench;
- nothing is `VEHICLE_CONFIRMED` yet. A community tester programme with a
  handful of named testers is the validation route; see
  [docs/TESTER_GUIDE.md](docs/TESTER_GUIDE.md) and
  [docs/ROADMAP.md](docs/ROADMAP.md).

Stage 2 (the first write operations, starting with clearing fault codes) is
future direction, each step behind its own safety class and decision record.
See [docs/SAFETY_BOUNDARIES.md](docs/SAFETY_BOUNDARIES.md).

## What is and is not in this repository

- **In:** the whole application, the protocol work for the MongoosePro JLR
  (documented from the firmware's own replies, byte for byte), the knowledge
  model, the ingestion tooling, the decision records and the evidence.
- **Not in:** the diagnostic data library. It is derived from SDD's own
  data, is never committed or bundled with the installer, and is issued to
  named testers as a personal, signed, dated copy
  ([ADR-0019](docs/adr/ADR-0019-signed-expiring-library-issues.md)).
  Without a library the application still connects to the adapter, captures
  buses and shows its built-in data; the survey, VIN decoding and fault-code
  wording need the library. The browser demo runs on a synthetic vehicle.
- **Never:** ECU firmware, flashing, key or immobiliser programming, or any
  arbitrary CAN transmit API. The architecture checker fails the build on
  them.

## Hardware

A J2534 MongoosePro JLR (or a faithful clone that answers the same
board-info) on the vehicle's OBD-II socket. ELM327-class dongles cannot reach
modules outside standard OBD-II addressing and are not supported.

## Building and testing

```bash
node scripts/check-architecture.mjs
pnpm install --frozen-lockfile
pnpm lint
pnpm test
cargo test --workspace --exclude prowlone-shell --exclude transport-serial
```

The two excluded crates need Windows; everything else builds and tests on
Linux, macOS or Windows. Installers are built by the CI workflow in
`.github/workflows/ci.yml`. Start reading at [CLAUDE.md](CLAUDE.md), then
[docs/CURRENT_STATE.md](docs/CURRENT_STATE.md), [docs/ROADMAP.md](docs/ROADMAP.md),
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/adr/](docs/adr/).

## The invariant

> JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI

Vehicle knowledge never leaks into protocol code, protocol never into
transport, and the interface reaches none of them directly. Unknown stays
`UNKNOWN`; a synthetic fixture is never presented as vehicle evidence; an
unvalidated procedure is unavailable, not guessed.

## Contributing

Issues and pull requests are welcome, in English or Ukrainian. Before an
architectural change, write a decision record in `docs/adr/`. Every physical
finding gets a reproducible fixture or a byte-level transcript. Do not send
SDD files, and do not send captures of vehicles you do not own.

## License and authorship

Copyright © 2026 Oleg-mk. Licensed under the GNU Affero General Public
License, version 3 ([LICENSE](LICENSE)): use, study, change and share freely;
a changed version that others use, including over a network, must be shared
under the same terms with its source. The name "ProwlOne" and the
project's marks are not covered by the license. Jaguar, Land Rover, Range
Rover and SDD are trademarks of Jaguar Land Rover Limited; this project is
not affiliated with or endorsed by it.

Written by Oleg-mk with Claude (Anthropic) as the coding assistant; the
decision records and evidence say who decided what, and when.
