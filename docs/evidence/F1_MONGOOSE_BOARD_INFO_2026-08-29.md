# F1 Mongoose board-info device evidence - 2026-08-29

## Test boundary

- Device connected by USB only.
- No vehicle connection.
- Executable: Rust workspace package `mongoose-jlr-probe`.
- Production crates used: `transport-api`, `transport-serial`, and `mongoose-jlr`.
- No DrewTech DLL/driver, J2534 runtime, SDD, Python probe, or PowerShell probe carried protocol traffic.

## Enumeration

```text
writes_performed: 0
matching_devices: 1
usb_vid: 18E1
usb_pid: 0104
usb_serial: AOLHE00000xxxxxx
port: COM3
```

Windows PnP readback before and after the board-info operation:

```text
Status: OK
Name: USB Serial Device (COM3)
Service: usbser
PNPDeviceID: USB\VID_18E1&PID_0104\AOLHE00000xxxxxx
```

## Board-info operation

- Writes performed: exactly 1.
- Request: `0C 00 EA 51 01 00 00 00 09 01 01 00 00 00 00 00`.
- Response command: `0x8109`.
- Sequence: `1`.
- Route A/B: `0x0000` / `0x0001`.
- Outer frame size: 184 bytes.
- Payload length: 180 bytes (`0x00B4`).
- Verifier: `0x5152 = 0x00B4 XOR 0x51E6`.
- Opaque board-info body: 168 bytes.
- Transport close: completed.
- Device after test: PnP `OK`, `usbser`.

Golden fixture: `crates/mongoose-jlr/tests/fixtures/board_info_response.hex`.
Fixture text SHA-256: `1AEE3E18FA31BC0C0E33263F7D4A81E0847AD11E102CA3CFB548FAD579520245`.
