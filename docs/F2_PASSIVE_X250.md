# F2 Passive X250 Acceptance

## Phase split and current status

**F2A — MONGOOSE CAN BACKEND: IMPLEMENTED / FIXTURE_TESTED / STATICALLY_CONFIRMED / NOT_VEHICLE_VALIDATED**

**F2B — VEHICLE CAN VALIDATION: DEFERRED_BY_OWNER_DECISION**

The available X250 is operationally critical and is not a development test bench. Deferring its physical test is not an F2A failure. No fake physical evidence is created.

No F2 CAN hardware command, vehicle connection, or passive capture has been performed. Under the corrected backend-scoped criterion, HS-CAN and MS-CAN are the complete physical capture set for MongoosePro JLR:

- X250 vehicle-side `CCP_HS_CAN` on 12/13 is `UNSUPPORTED_BY_MONGOOSE_JLR` and is not an F2 blocker;
- official MongoosePro JLR pin 12 is PS GND and pin 13 is FEPS, so both are prohibited;
- HS-CAN 6/14 at 500 kbit/s and MS-CAN 3/11 at 125 kbit/s are implemented but `NOT_VEHICLE_VALIDATED`;
- Windows Application Control blocks local execution of newly built Rust test binaries; security policy was not changed.

GitHub `Baseline validation` run `33293377659` is green for corrective commit `08495c2`, including workspace Rust tests and the regression that prohibits production CAN routes on pins 12/13.

## Route readiness

### `hs-can`

- Network type: CAN.
- OBD pins: 6/14.
- Bitrate: 500 kbit/s.
- Mongoose route ID: semantic `hs-can`; firmware resource 5 (CAN1), route word `0x0501`, returned unchanged as the channel token (ADR-0018).
- Exact setup: board-info, `cJumpToFirmware` if the bootloader answered; `cOpenChannel` on `0x0501` with `DT_LISTEN_ONLY | 500000`, validate `0x8006` and token `0x0501`; `cSetPin(1,6,14)`, validate `0x8012`.
- Explicit listen-only: YES.
- CAN frame TX possible during implemented setup: NO.
- Vehicle validation: `NOT_VEHICLE_VALIDATED`; command sequence `HARDWARE_CONFIRMED` on the bench 2026-09-06 (adapter, no vehicle, every step status 0); physical RX `NOT_RUN`.

### `ms-can`

- Network type: CAN.
- OBD pins: 3/11.
- Bitrate: 125 kbit/s.
- Mongoose route ID: semantic `ms-can`; firmware resource 21 (CAN2), route word `0x1501`, returned unchanged as the channel token (ADR-0018). Resource 5 refuses pins 3/11 ("board only supports CAN on pins 6 and 14"); resource 21 accepts them and refuses 6/14.
- Exact setup: board-info, `cJumpToFirmware` if the bootloader answered; `cOpenChannel` on `0x1501` with `DT_LISTEN_ONLY | 125000`, validate `0x8006` and token `0x1501`; `cSetPin(1,3,11)`, validate `0x8012`.
- Explicit listen-only: YES.
- CAN frame TX possible during implemented setup: NO.
- Vehicle validation: `NOT_VEHICLE_VALIDATED`; command sequence `HARDWARE_CONFIRMED` on the bench 2026-09-06 (adapter, no vehicle, every step status 0); physical RX `NOT_RUN`.

### `CCP_HS_CAN`

- Logical meaning: X250 vehicle-schematic net label.
- MongoosePro JLR physical CAN route: unsupported/unconfirmed.
- Vehicle DLC drawing labels 12/13 as CCP nets, but adapter pin 12 is PS GND and pin 13 is FEPS.
- Alternate mapping over 6/14 or 3/11: NOT FOUND.
- Status: `UNSUPPORTED_BY_MONGOOSE_JLR`; production route absent.
- Physical RX: PROHIBITED.

### `TCM_COMMS` / ROSCO

- `TCM_COMMS` / pin 8 is not part of F2 CAN acceptance.
- Logical/protocol compatibility: UNKNOWN.
- Production route: REMOVED; not an F2 CAN route.
- Physical status: not tested.

## Application CAN TX

`APPLICATION CAN TX API EXISTS: NO`

The probe accepts only `--enumerate`, `--board-info`, `--routes`, and `--passive-route <route>`. It rejects raw, send/TX, ISO-TP, UDS, and diagnostic modes. A passive capture prints `application_can_tx: 0` before and after the session.

## F2B physical test plan — deferred and not executed

This retained plan requires a future owner decision and separate explicit approval:

1. Confirm no SDD, ELM, or other scanner is connected or running.
2. Connect Windows 11 to MongoosePro JLR, then connect the VCI to X250 OBD.
3. Run `--routes`; it performs zero writes.
4. Run `--passive-route hs-can` for the fixed 20-second production capture; close; preserve raw output.
5. Verify frames, unique IDs, parse errors, dropped frames, close response, and `application_can_tx: 0`.
6. Repeat separately for `ms-can`; never open the two routes concurrently.
7. `ccp-hs-can` and `tcm-comms` are not production route IDs; do not touch pins 12/13.
8. Create one evidence report and one machine-readable fixture per physically confirmed CAN route, calculate SHA-256, and exclude/redact any accidental VIN-bearing content.
9. Confirm the VCI, vehicle, Windows PnP state, and serial port remain normal.

Abort on any nonzero setup status, unexpected command/channel, parser error trend, inability to close, activity suggesting transmission, or change in vehicle state. Do not brute-force bitrate and do not substitute PowerShell/Python for production acceptance.

## F2A completion and F2B pass criterion

F2A completion requires the production receive-only backend, truthful route inventory, static evidence, golden fixtures, regression tests, architecture checks, and green CI. It does not require or claim vehicle confirmation.

F2B retains the physical criterion: all CAN networks physically accessible through the current MongoosePro JLR backend must be physically captured. Vehicle networks known to exist but unsupported by this VCI must be documented as `UNSUPPORTED_BY_BACKEND` and are not an F2B blocker.

If F2B resumes, its required physical captures for the current backend are HS-CAN 6/14 and MS-CAN 3/11. `CCP_HS_CAN` is `UNSUPPORTED_BY_MONGOOSE_JLR`; `TCM_COMMS` / pin 8 is `OUT_OF_SCOPE / UNKNOWN`. Pins 12 and 13 are prohibited.

## Addendum 2026-09-03 — listen-only capture is now an application command

The passive routes above are exercised by `MongooseJlrDevice::capture_route`
and the shell's `capture_bus` command: a bounded listen-only recording of one
route, saved as a `captured` replay fixture with provenance. Physical RX on a
vehicle remains `NOT_RUN`; the command is what a tester will run first.
