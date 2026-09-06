# ADR-0018 — The MongoosePro JLR firmware handshake and resource routes

Date: 2026-09-06. Status: accepted. Evidence:
`docs/evidence/F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`.

## Context

The live CAN path — capture (F2), calibration read (F7/F8), module reads
(F10) — was built from static analysis of the vendor library and had never
met the adapter's firmware. The owner's first live test failed at channel
open on macOS and on Windows. Direct probing of the genuine adapter over
its serial port, on the bench with no vehicle, established:

- the adapter powers up in its **bootloader**, which answers only board
  commands (flag byte `0x01`) and refuses the working command set with
  "Invalid or Unhandled command type";
- `cJumpToFirmware` (command word `0x0103`, no body) starts the stored
  1.1.16 firmware in place, status 0, no re-enumeration, nothing written;
  a power cycle returns the adapter to the bootloader;
- the firmware opens a channel on a **resource selector** carried in the
  request's route word A, `(resource << 8) | 0x01`, and hands back that
  same word as the channel token; the `0xFF01` "unassigned channel" word
  from the static analysis is rejected as "Invalid Resource ID 31";
- resource **5** is CAN1 and takes `cSetPin(1, 6, 14)` only ("board only
  supports CAN on pins 6 and 14"); resource **21** is CAN2 and takes
  `cSetPin(1, 3, 11)` only ("board only supports CAN2 on pin 3 and 11");
  resources 2, 3, 4, 6, 13, 22, 25, 26 also open and belong to other
  protocols (J1850, K-line on pin 7 or 15, ISO 15765 on the device);
- the listen-only open (`0x10000000`, bitrate), the pin selection, the
  close, and the `cOutboundData` record as the application already builds
  it are all accepted by the firmware.

## Decision

1. **Firmware handshake before any channel command.** At the first route
   open of a device session the transport reads board-info; if the
   response body shows the bootloader (the tick counter at body offset 4
   is zero), it sends `cJumpToFirmware`, waits for its response, then
   polls board-info until the firmware answers with a running tick (five
   seconds at most, after which the open fails with a plain message and no
   channel command is sent). The command word is `0x0103` — opcode `0x03` with flag `0x01`
   — sent from the device route `0x0001` with no body. It is the only
   flag-`0x01` command the application sends besides board-info; the
   reflash, unprotect, reset and serial-number commands of the same family
   stay absent from the code.
2. **Resource routes replace the unassigned-channel word.** `hs-can`
   opens resource 5 (route word `0x0501`) and selects pins 6/14; `ms-can`
   opens resource 21 (route word `0x1501`) and selects pins 3/11. The
   token the device returns must equal the route word sent.
3. **The firmware's own words reach the tester.** A non-zero device
   status now carries the text the firmware puts after the status word,
   so a refusal reads "SetPins: … only supports CAN on pins 6 and 14"
   instead of a bare number.

## Consequences

- F2's `hs-can`/`ms-can` route descriptors gain a device resource word;
  the `0xFF01` constant goes. The transport-fake tests script the
  board-info exchange and, in one test, the bootloader jump.
- Validation states: the listen-only path on both buses and the outbound
  record are `HARDWARE_CONFIRMED` on the bench (adapter, no vehicle);
  whether frames arrive in non-listen mode without a filter, and what
  status `0x100` on an unacknowledged transmit means, are for the first
  car. `VEHICLE_CONFIRMED` remains empty.
- The vendor driver is not needed: the whole sequence was obtained from
  the firmware's replies over the standard serial port, which is also
  what macOS and Linux have.
- Nothing here changes the safety boundary: no vehicle write, no flash
  command, no security service.
