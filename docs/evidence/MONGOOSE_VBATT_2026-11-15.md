# How the vendor library reads the battery voltage at the connector, 2026-11-15

Static, read-only: the installed vendor library (`monpj432.dll`, 1.1.16, the
file the research of 2026-08 and the status-word table of 2026-11-14 were
read from) was read as bytes and disassembled; nothing was executed, no
adapter was touched, no frame was sent. Written the day the first tester,
after two sessions on his Discovery 4, asked for a battery voltage indicator
"similar to SDD" — SDD shows the voltage at the connector all the time, and
`ADR-0030` had recorded on 2026-09-19 that "the adapter itself measures
nothing" without looking.

## What was read

| Where (VA, image base `0x10000000`) | What it is |
| --- | --- |
| `0x10059D90` | `PassThruIoctl`, the export (ordinal 14) |
| `0x1005A932`–`0x1005A97F` | the branch that logs `   Battery Voltage %f V` |
| `0x1005AB68`–`0x1005ABB5` | the branch that logs `   Prog Voltage %f V` |
| `0x1003FED0` | the function both branches call, with one argument differing |
| `0x1006D9B0` | the wait for its answer, labelled `Waiting for get value response` |
| `0x10059D60` | the conversion applied to the answer |

The library's own names for the J2534 IOCTLs are in its string table
(`READ_VBATT`, `READ_PROG_VOLTAGE`, `READ_CH1_VOLTAGE`…`READ_CH6_VOLTAGE`,
`DT_READ_V_VEHICLE`, `READ_V_J1962PIN1`), and the firmware's status table
names `eVbattLoss` (`0x123`, `0x20A`, `MONGOOSE_STATUS_WORDS_2026-11-14.md`):
the firmware knows the battery rail.

## The command

The battery branch calls `0x1003FED0` with the arguments `1` and `3`; the
programming-voltage branch with `1` and `2`. That function builds a sixteen-
byte payload — the product's own twelve-byte command header
(`crates/mongoose-jlr/src/codec.rs`) and a four-byte body — and sends it:

| Offset | Field | Value | Source in the code |
| ---: | --- | --- | --- |
| +0x00 | route A | `01 00` | the first argument, `1` (the device; a channel's open carries its number in the high byte, `01 05` for HS-CAN) |
| +0x02 | route B | `00 00` | the constant at `0x100806C4`, zero |
| +0x04 | command | `0C 00` | `cGetValue` (`0x000C`) |
| +0x06 | sequence | `SS 00` | the library's running counter |
| +0x08 | field 08 | `00 00` | zeroed |
| +0x0A | field 0A | not written | `UNKNOWN`; the product sends zero, as it does for every command |
| +0x0C | body: value id | `03 00 00 00` | the second argument: `3` battery, `2` programming voltage |

On the wire, with the product's frame prefix: `10 00 F6 51 01 00 00 00 0C 00
SS 00 00 00 00 00 03 00 00 00` (length `0x10`, verifier `0x10 ^ 0x51E6`).
That is the record a `PassThruIoctl(READ_VBATT)` becomes. It reads a value
the adapter holds; it opens no channel and sends nothing on a bus.

## The answer

The wait matches the answer to the request by route and sequence — the
answer's route A against the request's route B, the answer's sequence
against the request's — as the product already matches every answer
(`ADR-0042`); the answer is the firmware's frame payload without the frame
prefix, in the product's own layout. Then:

| Offset | Field | Read as |
| ---: | --- | --- |
| +0x04 | command | `0C 80` (`cGetValueResp`, by the family rule) |
| +0x0C | body +0: status | `0` the value follows; `1` the library says "Parameter %s not supported by this device"; anything else is the library's generic failure |
| +0x10 | body +4 | the tick, as in every answer seen |
| +0x14 | body +8 | not read by this function; `UNKNOWN` (an echo of the value id is the obvious guess, and only a guess) |
| +0x18 | body +12: value | `u32`, millivolts |

The conversion at `0x10059D60` rounds the millivolts to the nearest 100 mV
(`(v / 100 + (v % 100 > 49)) * 100`) and that is what `PassThruIoctl`
returns; the log line divides by 1000 and prints volts. The unit is the
J2534 one for `READ_VBATT` (millivolts), and the division by 1000 for a
line that says "V" agrees.

## What is settled, and what is not

| Item | State |
| --- | --- |
| `READ_VBATT` is `cGetValue` with value id 3 | `STRONG INFERENCE`, static: the branch, its log line and the builder's command word |
| The request record, offsets above | `STRONG INFERENCE`, static |
| The answer's status at body +0 and value at body +12, in millivolts | `STRONG INFERENCE`, static |
| Body +8 of the answer; field 0A of the request | `UNKNOWN` |
| What the bare adapter (USB only, nothing on the OBD side) answers | `NOT_YET_VALIDATED` — the next step, on the owner's adapter; zero or near zero is expected |
| The voltage on a car | `NOT_YET_VALIDATED` — a tester's session |

Value id 2, the programming voltage, is named here because the same function
builds it; the product has no use for it and never sends it. Nothing here
writes a value: `cSetValue` (`0x000B`) is not touched.
