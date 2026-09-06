# CLAUDE.md

Independent diagnostic application for Jaguar / Land Rover / Range Rover of the
SDD era. Rust core, Tauri 2 shell, one adaptive React UI, MongoosePro JLR adapter.

**This product replaces SDD; it is not a scanner.** A cheap OBD-II dongle plus a
phone already covers generic scanning, so that is not the goal. The goal is the
capability SDD has and a scanner cannot reach — every module on every bus,
manufacturer-specific diagnostics, JLR knowledge — without SDD's Windows 7,
virtual-machine, and install misery. Good hardware, modern stack. Current scope
is stage 1: full read-only diagnostics. See `docs/ROADMAP.md`.

## Start here

The repository is the store of record, not any chat session. Resume work by
reading, in this order:

1. `docs/CURRENT_STATE.md` — where the project is, per phase, with honest
   validation states.
2. `docs/ROADMAP.md` — what is next and why in that order, plus the owner-only
   gates.
3. The relevant `docs/F*.md` phase document.
4. `docs/adr/` — why each architectural decision was taken.

Never ask the owner to re-explain state that these documents already hold. If you
learn something that matters tomorrow, write it into them today.

## The invariant

> **JLR KNOWLEDGE != PROTOCOL != TRANSPORT != UI**

This is machine-enforced by `scripts/check-architecture.mjs`, which fails on
vehicle-program hardcoding in protocol code, knowledge depending on transport or
protocol, a public application CAN TX API, active or programming UDS request
constructors, and new frontend access to execution crates. Read
`docs/ARCHITECTURE.md` before touching crate boundaries.

## Commands

```bash
node scripts/check-architecture.mjs
```

```bash
pnpm lint
```

```bash
pnpm test
```

Rust tests run cross-platform except for two crates. Smart App Control on the
Windows host blocks freshly linked unsigned test binaries with error `4551`, so
prefer running the portable majority under WSL2 or Linux:

```bash
cargo test --workspace --exclude jlr-scanner-shell --exclude transport-serial
```

That covers every F4–F8 golden test, including the F8 transport-fake Mongoose
exchange. Only `transport-serial` and `jlr-scanner-shell` genuinely require
Windows. See `docs/F3_1_WINDOWS_SIGNING.md` for the Smart App Control details.

## Working discipline

Taken from `docs/DEVELOPMENT_PRINCIPLES.md`; these are not style preferences.

- Write an ADR **before** an architectural change, never after. Do not redesign
  architecture silently.
- Unknown stays `UNKNOWN`. Never let an assumption become a recorded fact, and
  never generalize vehicle knowledge speculatively.
- Validation states are independent and not interchangeable: `IMPLEMENTED`,
  `FIXTURE_TESTED`, `HARDWARE_CONFIRMED`, `VEHICLE_CONFIRMED`. Report exactly the
  one that was earned.
- Synthetic fixtures prove protocol behavior only. They must never be presented
  as JLR evidence, and they cannot be promoted to real evidence.
- Every physical milestone gets a reproducible fixture or test.
- Fail closed. Unknown or unvalidated procedures are unavailable, not guessed.

## Safety boundaries

Every diagnostic operation carries exactly one safety class; see
`docs/SAFETY_BOUNDARIES.md`. Currently only `READ_ONLY` operations exist.

Never add: a public `send_raw` / `send_can` / `send_bytes` / `send_payload` or
any arbitrary CAN transmit API; ECU firmware, VBF, bootloader, or recovery
flashing; key, immobilizer, or security programming; arbitrary EEPROM writes;
or use of X250 diagnostic connector pins 12/13.

The first write operation in the product will be DTC clear (F12). It requires its
own ADR, an explicit safety class, and explicit user confirmation — it must not
be folded into a read phase.

## Validation strategy and owner-only gates

Validation is a **community tester programme**, not owner-vehicle testing. On
2026-09-01 the owner declined using his own X250: the car is in daily use and
operationally critical, and recovering a disabled vehicle costs more than
replacing it. That decision is closed — do not re-propose it.

Consequences worth knowing before planning any work:

- code signing is an improvement, not a gate. On 2026-09-05 the owner decided
  that the tester programme runs on the unsigned CI build: the testers are
  people who already run SDD on machines of their own, `docs/TESTER_GUIDE.md` states
  exactly which Windows prompt to expect and what it means, and a working
  application matters more than a signature. The signed installer (M4) follows
  once real results justify the certificate;
- knowledge, not Rust, is the binding constraint: the base holds about 5 KB of
  real evidence, so SDD's own diagnostic data is the breadth driver and tester
  captures are the validation layer;
- adapter class is settled — J2534 / MongoosePro JLR. ELM327-class devices cannot
  reach modules outside standard OBD-II addressing, so they cannot serve an SDD
  replacement. Adapter *ease of use* is still a first-class requirement.

All three are tracked in `docs/ROADMAP.md`. Do not create accounts, purchase
certificates, or connect to a vehicle on the owner's behalf.
