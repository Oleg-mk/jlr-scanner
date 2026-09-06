# F2 JLR Network Inventory — Corrective Audit

## Phase and validation state

F2A backend implementation is `IMPLEMENTED`, `FIXTURE_TESTED`, `STATICALLY_CONFIRMED`, and `NOT_VEHICLE_VALIDATED`. F2B vehicle validation is `DEFERRED_BY_OWNER_DECISION`. No Mongoose CAN command, hardware connection, or vehicle test was executed during this audit. Pins 12 and 13 are prohibited.

## Conflict and provenance

The previous inventory conflated two different connector-side facts:

1. The X250 factory vehicle wiring diagram labels DLC connector `C2DB04B/12` as `HS_CAN_POS_CCP` and `C2DB04B/13` as `HS_CAN_NEG_CCP` (publication `JLR 13 56 10_1E`, March 2008, PDF page 181 / printed page 133). The earlier `C2D804B` spelling was a transcription error corrected during the F5 exact-source audit.
2. The MongoosePro JLR hardware pinout assigns its own J1962 pin 12 to `PS GND` and pin 13 to `FEPS`, not to a CAN transceiver.

The 12/13 statement originated in the local source file `jaguar elm+/research_sources/x250_electrical_wiring.pdf`. It was summarized in `jaguar elm+/ARCHITECTURE.md:41-43` and `jaguar elm+/x250_research_report_2026-08-22.md:47`, then propagated into F2 commit `28958bc` as `CCP_HS_CAN_PINS = &[12, 13]` and the `ccp-hs-can` inventory row. It did not originate from `monpj432.dll` and is not a MongoosePro JLR hardware capability statement.

## Official MongoosePro JLR pinout

Authoritative adapter source: [MongoosePro JLR User Guide](https://opusivs.com/wp-content/uploads/2023/12/2019_MGP_User_Guide_JLR.pdf), JLR column on displayed page 18.

| J1962 pin(s) | MongoosePro JLR hardware function | F2 classification |
|---:|---|---|
| 6/14 | CAN+ / CAN- | Physical CAN route |
| 3/11 | CAN2+ / CAN2-; pin 3 is also marked K-Line | Physical CAN2 route |
| 12 | PS GND — Pin Switched to Ground | Programmable non-network function; prohibited |
| 13 | FEPS | Programming-voltage function; prohibited |
| 8 | ROSCO | Non-CAN JLR-specific function; compatibility with the X250 `TCM_COMMS` label is unproven |

The GM II column shows CAN3 on 12/13, but the GM II column is not evidence for the JLR variant and is excluded from this inventory.

## Logical protocol versus physical pins

Static source: `monpj432.dll` from the MongoosePro JLR driver package.

- `CAN_PS` is logical J2534 protocol ID `0x8005`; `CAN_PS_J1962` is a logical string at VA `0x1008AA5C`. Neither value encodes physical pins.
- The string `J1962_PINS only valid for *_PS ProtocolIDs` is at VA `0x10082CE0`. Additional validation strings include `J1962_PINS unknown value for pins` at `0x100860E0`, `J1962_PINS pins not supported %d` at `0x10086128`, and `Pin %d, %d not supported by this device` at `0x1008C6A8`.
- The generic pin-name formatter at VA `0x1003C620` maps numbers to names. Its `j1962_pin12` and `j1962_pin13` return sites are `0x1003C678` and `0x1003C67E`; the strings are at `0x1008C1B4` and `0x1008C1CC`. This proves only that the DLL can name those pin numbers.
- The internal `cSetPin` serializer begins at VA `0x1003FC80`, writes opcode `0x0012`, a connect/disconnect boolean, and two caller-supplied pin fields. Protocol-class call sites use it to attach/detach supported J1962 pin selections. `PassThruSetProgrammingVoltage` at VA `0x10057FE0` also calls it at `0x10058749`, `0x1005878E`, `0x100587E8`, and `0x100589F5`.

Therefore `cSetPin` is a shared low-level programmable pin-routing primitive. For `*_PS` protocol objects it can select a hardware-supported protocol pin pair; it is also used by programming-voltage control. It does not prove that an arbitrary pair is electrically connected to a CAN transceiver. Hardware capability remains bounded by the JLR pinout.

## Revised F2 routes

| F2 logical route | Logical protocol/channel | Physical OBD pins | Vehicle-side meaning | MongoosePro JLR hardware support | Physical route status | Passive status |
|---|---|---:|---|---|---|---|
| `hs-can` | CAN-class / `CAN_PS` logical mode | 6/14 | X250 HS-CAN, 500 kbit/s | CAN+ / CAN- | CONFIRMED | Backend implemented; `NOT_VEHICLE_VALIDATED` |
| `ms-can` | CAN-class / `CAN_PS` logical mode | 3/11 | X250 MS-CAN, 125 kbit/s | CAN2+ / CAN2- | CONFIRMED | Backend implemented; `NOT_VEHICLE_VALIDATED` |
| `CCP_HS_CAN` | X250 schematic net label; no confirmed Mongoose protocol/channel | Vehicle-side 12/13 only | X250 wiring confirms the CCP differential nets | No MongoosePro JLR CAN route is supported/confirmed | `UNSUPPORTED_BY_MONGOOSE_JLR` | Not an F2 blocker; do not touch 12/13 |
| `TCM_COMMS` / ROSCO | Logical relationship unproven | 8 | X250 uses `TCM_COMMS`; guide calls adapter function `ROSCO` | Compatibility UNKNOWN | OUT_OF_SCOPE | Not part of F2 CAN acceptance |

Only `hs-can` and `ms-can` remain production F2 routes. Production route descriptors contain no pins 12 or 13, and the `cSetPin` serializer now accepts a supported route descriptor rather than arbitrary pin arguments.

## F2A completion and F2B vehicle criterion

F2A completion is based on implementation, static evidence, fixtures, regression tests, architecture checks, and CI. It is not blocked by deferred vehicle validation and does not establish `VEHICLE_CONFIRMED`.

F2B retains the physical criterion: all CAN networks physically accessible through the current MongoosePro JLR backend must be physically captured. Vehicle networks known to exist but unsupported by this VCI must be documented as `UNSUPPORTED_BY_BACKEND` and are not an F2B blocker.

For this backend, `UNSUPPORTED_BY_MONGOOSE_JLR` is the concrete `UNSUPPORTED_BY_BACKEND` status. If F2B resumes, its capture set is `hs-can` on 6/14 and `ms-can` on 3/11 only. `CCP_HS_CAN` does not expand the capture set, and `TCM_COMMS` is `OUT_OF_SCOPE / UNKNOWN`.

## Corrective conclusion

The vehicle's CCP labels on DLC 12/13 are real vehicle-side wiring evidence, but they are not a usable MongoosePro JLR CAN route. Its status is `UNSUPPORTED_BY_MONGOOSE_JLR`; pins 12 and 13 must not be touched in F2.

## Addendum 2026-09-03 — full JLR-column pinout, verified from the user guide

The table above lists only the pins the original audit needed. The full JLR
column of the MongoosePro Vehicle Connector Pin Assignments (user guide page 17,
extracted from the PDF cited above) is:

| J1962 pin | JLR function |
|---:|---|
| 2 | J1850+ |
| 3 | CAN 2+ / K-Line |
| 5 | GND |
| 6 | CAN+ |
| 7 | **K-Line** |
| 8 | **ROSCO** |
| 10 | J1850- |
| 11 | CAN 2- |
| 12 | PS GND |
| 13 | FEPS |
| 14 | CAN- |
| 15 | L-Line |
| 16 | VBatt |

This settles a question the earlier audit left `UNKNOWN`. The SDD platform
corpus declares legacy buses by pin — `DS2_PIN7`, `KW2000STAR_PIN8`, `ROSCO`,
`SCP` — and the JLR variant provides K-Line on pin 7, ROSCO on pin 8, and
J1850 on pins 2/10. Every bus the corpus declares is reachable by this adapter's
hardware. K-line and J1850 remain `NOT IMPLEMENTED` in software; that is now a
known software gap, not a hardware limit.

Pins 12 and 13 remain prohibited. Nothing here changes that.
