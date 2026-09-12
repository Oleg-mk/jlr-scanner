# F2 live finding — the adapter boots in the bootloader; the channel-open path is not what the firmware expects (2026-09-06)

First real exchange with the MongoosePro JLR beyond board-info. Provoked
by the owner's first live test: every operation — capture, calibration
read, module read — failed at channel open with `cOpenChannel failed with
device status 0x00000001`, on macOS (Apple silicon) and then reproduced on
Windows. The adapter is the genuine device already on file (USB
`VID_18E1 & PID_0104`, serial `AOLHE00000xxxxxx`, `usbser`, COM3).

Probed directly over the serial port with a standalone Python script
(`scratchpad/mongoose_probe.py`, `mongoose_jump_probe.py`), printing every
byte. The application was disconnected first. No flash-write, reflash,
unprotect, or serial-number command was ever sent.

## What the bytes show

**The adapter powers up in its bootloader.** `cGetBoardInfo` (0x0109)
answers with status 0 and a 168-byte body that carries two version blocks:
`00 08 01 01` and `00 10 01 01`. These are exactly the header version
bytes of the two firmware images the driver research found inside
`monpj432.dll`: bootloader **1.1.8** (RCDATA 5006) and firmware **1.1.16**
(RCDATA 5005). The device is sitting in the 1.1.8 bootloader.

**In the bootloader, every flag-0x00 command is unknown.** `cOpenDevice`
(0x0003), `cOpenChannel` (0x0006) and `cCloseDevice` (0x0005) each return
status `0x00000001` and the body text:

```
Invalid or Unhandled command type
```

So all of F1/F3's "hardware-confirmed" traffic was bootloader board-info
only; the bootloader answers board/status commands (flag 0x01) and refuses
the working command set.

**`cJumpToFirmware` leaves the bootloader cleanly.** Command word 0x0103
(wire `03 01`), no body, returns status 0. The adapter stays on COM3 (no
re-enumeration), board-info still answers, and its body now carries a
running counter (`E0 90 69 02` where the bootloader had zeros): the 1.1.16
firmware is live. Nothing was written; a USB replug returns the device to
the bootloader.

**In the firmware, our channel-open encoding is wrong.** The same
`cOpenChannel` (listen-only flag `0x10000000`, bitrate 500000, route word
`0xFF01` "unassigned CAN channel") now returns a *different* error, status
`0x00000003`:

```
cOpenChannel: Unsupported or Invalid Resource ID 31
```

The firmware knows the command but rejects how we build it: it wants a
valid resource/channel identifier, not the `0xFF01` word taken from the
static disassembly. The research already said as much —
`PassThruConnect` = "`cOpenChannel` (0x0006), followed by value/config
commands" (STRONG INFERENCE, `MONGOOSE_REVERSE_ENGINEERING.md`) — so the
real connect does more than the single frame we implemented.

## What this means

The entire live CAN path (F2 open/capture, F7/F8 read) was built from
static reverse-engineering of `monpj432.dll` and **never met the firmware**.
Two gaps, both newly evidenced here:

1. **The application never leaves the bootloader.** It must send
   `cJumpToFirmware` (0x0103, flag 0x01, no body) after board-info and
   before any channel command. This is a device-state transition, not a
   flash write; confirmed harmless here (status 0, replug-reversible).
2. **The firmware's channel-open resource sequence is unknown to us.**
   The `0xFF01` open word is rejected as "Invalid Resource ID 31". The real
   `PassThruConnect` selects a resource and issues value/config commands we
   do not have.

Data ingestion, VIN decode, the survey, the library and the whole UI are
unaffected; they need no adapter. The read functions cannot work against a
car until the two gaps above are closed and validated on this device.

## Corrections to the record

- F1/F3 `HARDWARE_CONFIRMED` scope is **bootloader board-info only**, not
  the working command set. Reworded in `CURRENT_STATE.md`.
- F2 `cOpenChannel(DT_LISTEN_ONLY, …)` route `hs-can`/`ms-can` was
  `STATICALLY_CONFIRMED`, and is now **contradicted on hardware**: the
  firmware rejects the `0xFF01` open word. The open encoding is reopened.

## The way to the real sequence

The cheapest exact answer is a USB capture of a genuine SDD installation
opening a channel on the same Windows PC with this adapter: it yields
the real jump, open-device, open-channel resource id, and filter sequence,
byte for byte, and validates them at once. Failing that, more disassembly
of `PassThruConnect` (RVA 0x525D0) — the analysis artifacts are on the
owner's disk, not in the repository. Further blind probing of the firmware
is host-only and vehicle-safe but is guessing and is not the way to a
trustworthy fix.

## Part 2 — the working listen sequence, found by probing (2026-09-06, same day)

Continued probing over the serial port, adapter on `usbser`/COM3, no
vehicle. Findings, each a status-0 reply unless said otherwise:

- **The channel opens on a "resource", not the `0xFF01` word.** The
  firmware read our `0xFF01` route word as resource id 31 (`0xFF01 >> 11`).
  Sweeping route word A as `(resource << 8) | 0x01`: resources **2, 3, 4, 5,
  6 all open** (status 0) and the firmware returns a channel token equal to
  the route word; resources 1 and 7 are rejected ("Invalid Resource ID").
  So route word A is a resource selector and becomes the channel token.
- **`cSetPin` accepts 6/14 and refuses 3/11.** On an opened channel
  (token 0x0501), `cSetPin(connect=1, 6, 14)` returns status 0.
  `cSetPin(connect=1, 3, 11)` returns status `0x00000206` with the text:
  `SetPins: MongoosePro JLR board only supports CAN on pins 6 and 14`.
- **HS-CAN listen-only runs end to end.** board-info → `cOpenChannel`
  (resource 5, listen-only flag `0x10000000`, 500000) → token 0x0501 →
  `cSetPin(1,6,14)` → listen → `cCloseChannel`, every step status 0, zero
  frames (the bus is silent with no car).

### What this changes

- **A working listen-only capture path now exists**, hardware-validated on
  HS-CAN: jump to firmware, open a CAN resource, select pins 6/14, listen.
  It needs a car only to see frames; the command sequence is confirmed.
- **MS-CAN on pins 3/11 is in doubt.** This board's firmware says it only
  does CAN on 6/14. Whether MS-CAN is reached another way (a different
  resource, or not at all on this adapter) is unresolved and matters: our
  F2 treated `ms-can` on 3/11 as a production route. Do not trust that until
  a genuine capture or a documented resource settles it.
- **The resource id is not yet canonical.** Resource 5 works, but so do
  2–6; which one the vendor library uses for CAN and for ISO 15765, and the
  full connect/filter/transmit sequence for *reads*, are best taken from a
  capture of the genuine vendor J2534 library (`monpj432.dll`) rather than
  guessed. The probe tools are in `scripts/mongoose-probe/`.

### The vendor driver did not load on this Windows 11

`pnputil /add-driver monpjaguar.inf /install` staged the 2012 Drew driver,
but `dtmonpro.sys` failed to start: `CM_PROB_DRIVER_FAILED_LOAD` (code 14),
and Code Integrity logged event 3077 "did not meet the Authenticode signing
level requirements" (memory integrity/HVCI is off; the block is the old
signature itself). Reverted to `usbser`, adapter back on COM3. To run the
vendor library for a genuine capture, driver signature enforcement must be
relaxed by the owner, or the driver replaced with a currently-signed build.

## Part 3 — the resource map, MS-CAN found, a transmit accepted (2026-09-06, evening)

The owner objected that SDD reaches MS-CAN through this very adapter, so
"pins 6 and 14 only" could not be the whole truth. It was not. Sweeping
every resource id the route word can carry (0–31), and asking each open
one for pins 3/11 (`ms_can_sweep.txt`, `ms_can_listen_and_tx.txt`):

| Resource | Opens | `cSetPin` answer | Reading |
| --- | --- | --- | --- |
| 1, 7–12, 14–20, 23, 27–30 | no: "Invalid Resource ID n" | — | absent |
| 2 | yes | "board only supports PWM on pins 2 and 10" | J1850 PWM |
| 3, 4 | yes | 3/11: "supports L line on pin 15 only"; 6/14: "supports ISO9141 K line on pin 3, 7 or 8" | ISO 9141 / ISO 14230 K-line |
| 6 | yes | 6/14 status 0; 3/11 refused: "only supports ISO15765 on pins 6 and 14" | ISO 15765 on CAN1 |
| **5** | yes | 6/14 status 0; 3/11 refused: "board only supports CAN on pins 6 and 14" | **CAN1, HS-CAN** |
| 13 | yes | "board supports UEB on pin 7 only" | a K-line-class resource on pin 7 |
| **21** | yes | 3/11 status 0; 6/14 refused: "board only supports CAN2 on pin 3 and 11" | **CAN2, MS-CAN** |
| 22 | yes | 3/11 status 0 | ISO 15765 on CAN2 |
| 25, 26 | yes | "MFC board supports ISO9141_PS K line on pin 15 only" | K-line on pin 15 |

**MS-CAN listen-only runs end to end on resource 21**: open (listen-only,
125000) → token 0x1501 → `cSetPin(1, 3, 11)` → listen → close, all status
0. The first field of `cSetPin` is not a controller index: 0, 2 and 3 on
resource 5 all drew the same refusal.

**A transmit is accepted.** On resource 5 opened without the listen flag,
after `cSetPin(1, 6, 14)`, the application's own `cOutboundData` record
(0x7DF, `02 01 00 …`) drew `0x8008` with status `0x00000100` and no text —
on a bench with no other CAN node, where no frame can be acknowledged.
The record's format is therefore accepted; what `0x100` means, and whether
inbound frames arrive without a filter in non-listen mode, the first car
will show.

Reproduced later the same evening with the committed tools
(`scripts/mongoose-probe/mongoose_resource_sweep.py`,
`mongoose_listen_probe.py hs|ms`); the transcripts, every byte, are in
`docs/evidence/mongoose-probe-2026-09-06/` (`resource_sweep.txt`,
`listen_hs.txt`, `listen_ms.txt`). The pin rows above quote that run.

Decision recorded as ADR-0018 and implemented the same evening in
`crates/mongoose-jlr` (handshake, resource routes, firmware text in
refusals), with the transport-fake tests scripting these exchanges.

**Confirmed in the application, same evening.** The owner ran build
`5d9840e` on his Windows PC with the adapter and no car: capture on
HS-CAN and on MS-CAN each finished silent, zero frames, no error, both
with the adapter already in firmware and again after unplugging and
replugging it (bootloader → jump → firmware, inside the application).
Screenshots in the session; the command sequence of ADR-0018 is thereby
`HARDWARE_CONFIRMED` in the product on both buses.

## Part 4 — the diagnostic open and an outbound record on CAN2, and how long the jump takes (2026-09-08)

Bench, adapter on COM3, no vehicle. Asked before any tester reads a
medium-speed module: does the firmware accept on resource 21 what it accepted
on resource 5 on 2026-09-06 — an open *without* the listen-only flag, the pin
selection, and the application's own `cOutboundData` record?

**It does, identically.** Resource 21 opened with flags 0 at 125000 (status
0), took `cSetPin(1, 3, 11)` (status 0), and answered the application's
record for `0x733` `03 19 02 FF 00 00 00 00` — ReadDTCInformation to the X250
HVAC, as the module read would send it — with `0x8008`, status `0x00000100`,
the same word resource 5 gave on 2026-09-06 and gave again in the same
session as the control (`0x7E0`, same data). On a bench with no other CAN
node nothing can acknowledge the frame, so `0x100` is the firmware's answer
to an unacknowledged transmit; what it says with a car listening is still
for the first car. Transcript:
`mongoose-probe-2026-09-08/diagnostic_open_and_outbound_resources_21_and_5.txt`.

**A protocol fact found on the way.** The record's header word after the
sequence number must be `1`, as `passive::outbound_data_request` writes it.
A first attempt today built the record with the generic probe helper, which
zeroes that word, and the firmware answered nothing at all — not an error,
silence — on both resources, for six seconds, twice, while the channel
stayed alive and closed normally afterwards. The application has always
written the `1`; the probe had not. Recorded so nobody mistakes that silence
for a bus problem again.

**The jump takes about a quarter of a second.** The adapter had been
replugged and answered board-info from the bootloader; after
`cJumpToFirmware` the first board-info poll, 241 ms later, already carried
the firmware's running tick. The transport polls every 100 ms with a
five-second cap (`ADR-0018`), which leaves room.

Validation: the medium-speed diagnostic path — resource 21, pins 3/11, the
outbound record — is `HARDWARE_CONFIRMED` on the bench to the same extent
as the high-speed one. `VEHICLE_CONFIRMED` remains empty for both.

## Part 4 — resources 3 and 4 bound as hypotheses (2026-09-12, `ADR-0029`)

Nothing new was measured; the sweep above is what the K-line decision rests
on. Resources 3 and 4 open and name pins 3, 7 and 8 for ISO 9141 K-line; in
the J2534 numbering the firmware appears to follow, 3 is ISO 9141 and 4 is
ISO 14230. The adapter now carries two routes, `k-line-7` and `k-line-8`,
with no resource word of their own: which of 3 and 4 a line is opened on is
chosen by the protocol's physical layer (3 for DS2 and ROSCO, 4 for
KWP2000), and until the second slice's probe
(`scripts/mongoose-probe/mongoose_kline_probe.py`) has asked the firmware,
the pin-select word for a single line, the wake-up command and the outbound
record for K-line bytes stay **unknown** — the routes refuse every open with
that reason. `HARDWARE_CONFIRMED` for K-line means, and will mean, the
firmware's own status texts to those three commands, adapter alone, no car.
