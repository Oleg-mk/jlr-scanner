# F1 MongoosePro JLR Device Transport

## Scope

F1 closes one capability:

```text
discover -> open -> one cGetBoardInfo -> validate response -> close
```

The implementation uses production Rust crates and the Microsoft serial driver. It does not use DrewTech runtime components.

## Confirmed hardware identity

- USB VID/PID: `18E1:0104`.
- Physical USB serial during acceptance: `AOLHE00000xxxxxx`.
- Windows service: `usbser`.
- Physical port during acceptance: `COM3`; this value is evidence, never a production constant.
- Serial mode: 115200 baud, 8 data bits, no parity, 1 stop bit, no flow control.

## Windows discovery and transport

`transport-serial` reads USB identities from the Windows PnP Enum registry, requires each instance to resolve as present through `CM_Locate_DevNodeW`, and intersects its `PortName` with ports currently reported by `GetCommPorts`. It selects by VID/PID and fails closed:

- zero matches: `DeviceNotFound`;
- one match: use it;
- multiple matches: return all candidates as `AmbiguousDevices`.

`SerialTransport::open` only opens the OS byte stream. It sends no protocol command. `close` drops that stream and does not send `cCloseDevice`.

## Framing and correlation

The transport-independent `mongoose-jlr` codec implements:

- `u16` little-endian payload length;
- verifier `length XOR 0x51E6`;
- maximum payload `0x1800`;
- fragmented reads and multiple frames per stream;
- verifier and length rejection;
- non-zero 8-bit sequence values stored in a `u16` field;
- rollover from `0xFF` to `1`;
- opcode, response flag, and sequence correlation.

## F1 command allowlist

The only application command exposed is `cGetBoardInfo`:

- request command: `0x0109`;
- response command: `0x8109`;
- exact sequence-1 request: `0C 00 EA 51 01 00 00 00 09 01 01 00 00 00 00 00`.

There is no raw opcode API. The safe probe exposes only `--enumerate` and `--board-info`, and it never retries the application command.

## Physical acceptance

On 2026-08-29, with the Mongoose connected by USB only:

- production `--enumerate` found one `18E1:0104` device on `COM3` and performed zero writes;
- production `--board-info` performed one write;
- response command was `0x8109`, sequence was `1`;
- frame length was 184 bytes, with a 180-byte payload and verifier `0x5152`;
- the 168 command-body bytes remain opaque;
- the port was closed and Windows continued to report PnP `OK` with service `usbser`.

The exact physical frame is stored at `crates/mongoose-jlr/tests/fixtures/board_info_response.hex`.

## Exact limitations

- Board-info bytes are not labelled as firmware version, hardware revision, or capability flags.
- Only one outstanding request is supported.
- No automatic retry is performed.
- No diagnostic UI or Tauri command was added.
- The physical acceptance covers USB serial device identity only, not a vehicle.

## Explicitly not implemented

CAN, HS-CAN, MS-CAN, channel open/close, filters, CAN TX/RX, ISO-TP, UDS, KWP, ECU discovery, X250 logic, mobile/network transport, raw engineering commands, reset, bootloader, firmware update, programming voltage, and pin control are absent.
