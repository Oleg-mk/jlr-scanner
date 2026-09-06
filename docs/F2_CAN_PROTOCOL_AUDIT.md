# F2 CAN Protocol Audit

## Audit result

The production implementation contains a receive-only Mongoose/JLR CAN subset. It can inventory routes, open a proven CAN route atomically in listen-only mode, select its physical pins, parse inbound raw CAN frames, and close the dynamic channel. It has no public or CLI path for CAN data transmission, ISO-TP, UDS, KWP, or arbitrary opcodes.

F2A implementation status is `IMPLEMENTED / FIXTURE_TESTED / STATICALLY_CONFIRMED`. Its validation status is `NOT_VEHICLE_VALIDATED`. F2B vehicle validation is `DEFERRED_BY_OWNER_DECISION`; this does not block F2A closure.

No F2 hardware or vehicle command was executed while producing this audit.

Corrective finding: the old 12/13 CCP row was vehicle-side X250 wiring evidence, not MongoosePro JLR hardware-route evidence. The official adapter pinout defines pin 12 as PS GND and pin 13 as FEPS.

## Route matrix

| Route | Network | Pins | Bitrate | Open/config commands | RX format | Explicit listen-only | Vehicle TX risk | Evidence/confidence |
|---|---|---:|---:|---|---|---|---|---|
| `hs-can` | X250 HS-CAN | 6/14 | 500000 | `cOpenChannel(DT_LISTEN_ONLY, 500000)`; dynamic token; `cSetPin(6,14)` | `cInboundData`, standard/extended, DLC 0..8 | YES | NO in implemented path | Mongoose route CONFIRMED; backend implemented; `NOT_VEHICLE_VALIDATED` |
| `ms-can` | X250 MS-CAN | 3/11 | 125000 | `cOpenChannel(DT_LISTEN_ONLY, 125000)`; dynamic token; `cSetPin(3,11)` | same | YES | NO in implemented path | Mongoose route CONFIRMED; backend implemented; `NOT_VEHICLE_VALIDATED` |
| `CCP_HS_CAN` | X250 schematic net label only | vehicle-side 12/13 | UNKNOWN | none; not a production route | not enabled | Mongoose CAN route unsupported/unconfirmed | UNKNOWN | `UNSUPPORTED_BY_MONGOOSE_JLR`; not an F2 blocker |

Pin 8 is intentionally absent from the CAN table. `TCM_COMMS` / ROSCO is not part of F2 CAN acceptance.

## F2A completion and F2B pass criterion

F2A completion requires the receive-only implementation and its static/fixture/CI evidence; it neither requires nor claims vehicle confirmation. F2B retains the physical criterion: all CAN networks physically accessible through the current MongoosePro JLR backend must be physically captured. Vehicle networks known to exist but unsupported by this VCI must be documented as `UNSUPPORTED_BY_BACKEND` and are not an F2B blocker. `CCP_HS_CAN` is `UNSUPPORTED_BY_MONGOOSE_JLR`; if F2B resumes, only HS-CAN 6/14 and MS-CAN 3/11 require captures.

## Outer frame

All multi-byte integers below are little-endian unless stated otherwise.

| Offset | Size | Meaning |
|---:|---:|---|
| `0x00` | 2 | payload length |
| `0x02` | 2 | payload length XOR `0x51E6` |
| `0x04` | variable | command payload |

The production streaming decoder accepts fragmented and coalesced reads, rejects zero/oversize length and invalid verifier, and resynchronizes one byte at a time after an invalid header.

## Exact request records

Common command payload offsets are route word A at `+0x00`, route word B at `+0x02`, command at `+0x04`, sequence at `+0x06`, and reserved words at `+0x08/+0x0A`.

Sequence-1 HS-CAN listen-only open at 500 kbit/s:

```text
14 00 F2 51 01 05 00 00 06 00 01 00 00 00 00 00 00 00 00 10 20 A1 07 00
```

Sequence-1 MS-CAN listen-only open at 125 kbit/s:

```text
14 00 F2 51 01 15 00 00 06 00 01 00 00 00 00 00 00 00 00 10 48 E8 01 00
```

The body is `DT_LISTEN_ONLY` (`0x10000000`) followed by bitrate as `u32`. The request route word is the firmware's resource selector, `(resource << 8) | 0x01`: `0x0501` opens resource 5 (CAN1, pins 6/14) and `0x1501` opens resource 21 (CAN2, pins 3/11). `cOpenChannelResp` (`0x8006`) must return status zero and a channel token equal to the route word sent. Hardware-confirmed on the bench 2026-09-06 (ADR-0018, `docs/evidence/F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`); the static analysis's `0xFF01` "unassigned channel" word, used until then, is rejected by the firmware as "Invalid Resource ID 31".

Before the first open of a device session the transport reads board-info and, if the adapter answers from its bootloader (zeros at body offset 4, where the firmware keeps a running tick), sends `cJumpToFirmware` — command word `0x0103`, device route `0x0001`, no body — and waits for `0x8103` with status zero (ADR-0018). At sequence 2:

```text
0C 00 EA 51 01 00 00 00 03 01 02 00 00 00 00 00
```

Example sequence-2 `cSetPin(6,14)` on the returned token `0x0501`:

```text
18 00 FE 51 01 05 00 00 12 00 02 00 00 00 00 00 01 00 00 00 06 00 00 00 0E 00 00 00
```

For MS-CAN, only the final pin values become `03 00 00 00 0B 00 00 00`. The connect field is `1`. A nonzero response status or mismatched channel aborts setup and triggers a best-effort close.

Example sequence-3 close for token `0x0501`:

```text
0C 00 EA 51 01 05 00 00 07 00 03 00 00 00 00 00
```

`cSetPin` is a shared low-level pin-routing primitive, not `cOutboundData`. It selects hardware-supported J1962 pins for `*_PS` protocol objects and is also used by programming-voltage handling; it cannot establish unsupported CAN routing. The production crate contains no `cOutboundData` serializer.

## Inbound CAN layout

After the 12-byte common command header, `cInboundData` contains:

| Body offset | Size | Meaning |
|---:|---:|---|
| `0x00` | 4 | `RxStatus` |
| `0x04` | 4 | raw device timestamp |
| `0x08` | 2 | `DataSize`, required range 4..12 |
| `0x0A` | 2 | `ExtraDataIndex`/reserved for this raw-CAN subset |
| `0x0C` | 4 | arbitration ID, **big-endian** |
| `0x10` | 0..8 | CAN data |

The DLC is `DataSize - 4`. The timestamp is preserved as an opaque `u32` device tick; its unit is not claimed. `RxStatus & 0x00000100` selects a 29-bit identifier. Standard identifiers must be `<= 0x7FF`; extended identifiers must be `<= 0x1FFFFFFF`. Channel-token mismatch, bad size, truncation, or out-of-range ID fails closed.

No Jaguar signals, ECU identity, diagnostic payload, ISO-TP semantics, or timestamp unit are inferred.

## Receive/filter behavior

The JLR DLL evidence supports asynchronous inbound delivery on an open CAN channel. F2 does not issue table/filter commands: there is no evidence that a permissive filter is required for this device-side listen-only raw receive path. If a physical test shows that a filter is mandatory, the test stops; a filter command will require independent JLR evidence and new tests before use.

## Production surface and tests

Public receive surface:

- `list_vehicle_routes()`;
- `open_receive_route(route)`;
- `receive_frame(timeout)`;
- `close_route()`.

Tests cover exact open/set-pin/close bytes, DLC 0 through 8, standard and extended IDs, malformed and truncated records, invalid outer framing, fragmented reads, multiple frames in one buffer, route identity, close/reopen, the CLI/API TX denylist, and a regression assertion that production routes contain neither pin 12 nor pin 13.

The architecture check rejects public functions named `send_can_frame`, `raw_can_tx`, `diagnostic_request`, `send_isotp`, or `send_uds` in the production Mongoose receive layer.
