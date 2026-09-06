# MongoosePro JLR user-space transport design

## Decision

Use one JLR-specific protocol core above interchangeable byte transports. Keep
ISO-TP/UDS/KWP and Jaguar diagnostic knowledge above the CAN transport API.
Do not expose firmware, EEPROM, programming voltage, or generic unknown control
operations.

```text
JLR diagnostics
  -> ISO-TP / UDS / KWP (separate layer)
  -> canonical CAN API
  -> MongoosePro JLR codec/state machine
  -> SerialTransport OR LibUsbTransport
  -> Mongoose firmware
```

The implementation is in `digital_jaguar/mongoose_jlr/`. It imports neither
`monpj432.dll` nor `dtmonpro.sys`.

## Layers

### Codec

`codec.py` owns only deterministic binary transformations:

- `MongoosePacket`: two route words, opcode, flag, sequence and body;
- length/XOR envelope;
- bounded incremental parsing and one-byte resynchronization;
- JLR non-zero 8-bit sequence counter;
- exact `cGetBoardInfo` builder;
- exact static JLR `cOutboundData` serializer;
- GM-II inbound CAN candidate parser, explicitly labelled unvalidated for JLR;
- opcode/flag/sequence response matching.

The codec performs no I/O and is covered by golden arrays.

### Device protocol

`device.py` exposes the intended API shape:

```text
open()
close()
get_device_info()
get_serial()
open_can_channel(bus, bitrate)
close_can_channel()
add_can_filter(...)
clear_can_filters()
send_can(...)
receive_can(...)
control(...)
```

Current support is intentionally narrow:

- `open()` and `close()` operate on the OS transport only and send no Mongoose
  command;
- `get_device_info()` sends one statically audited `cGetBoardInfo` only when
  the caller explicitly constructs the device with writes enabled;
- every CAN, filter, serial-string and generic control method raises
  `ProtocolNotValidatedError` before I/O.

This fail-closed shape prevents an application from accidentally falling back
to the unsafe GM-II aggregate open sequence.

### ISO-TP and diagnostics

ISO-TP must remain above the canonical CAN transport. The official JLR DLL
contains host-side segmentation, reassembly, flow-control-filter checks,
BS/STmin/WFT, address modes and padding. None belongs in the serial/libusb
backend.

The future ISO-TP layer must be tested independently against deterministic CAN
fixtures before any vehicle use. This phase does not implement it.

## SERIAL backend

`SerialTransport` loads pyserial only when requested. No dependency was
installed as part of this work.

Effective mode copied from gocan and its pinned serial dependency:

```text
115200 baud
8 data bits
no parity
1 stop bit
no RTS/CTS
no XON/XOFF
DTR=true
RTS=true
read timeout=50 ms
```

Windows port names are `COMx`; macOS candidates are `/dev/cu.*`. Port selection
must be based on VID/PID/serial metadata when available, not merely the first
serial port.

Opening a serial port can assert DTR/RTS. Therefore the probe's default mode
does not open COM at all. A later physical test must treat transport open as an
explicit operation even before the first command byte.

Static limitation: COM3 was observed under usbser, but neither monpj432 nor the
JLR INF proves that COM3 carries the Mongoose length/XOR stream. Serial remains
UNKNOWN until a physical read/write test.

## RAW USB backend

`LibUsbTransport` dynamically examines the active descriptors and chooses an
interface containing both bulk IN and bulk OUT. It does not hard-code endpoint
addresses because static dtmonpro analysis showed runtime pipe enumeration.

Safety constraints implemented in the backend:

- no kernel-driver detach;
- no `set_configuration`;
- no Windows driver rebinding;
- no device reset;
- no EP0 vendor request `0xDA`;
- no automatic fallback to another interface;
- release only the interface it successfully claimed.

The optional interrupt pipe is ignored for the first probe. The official bulk
IN/OUT path is sufficient for framed ReadFile/WriteFile traffic; whether an
interrupt endpoint is required for a particular firmware state remains an
open question.

On Windows, access may be blocked while dtmonpro owns the interface. The
backend stops with a clear error and does not detach or alter binding. On
macOS, libusb is the preferred common-denominator candidate.

### Optional WinUSB

A future native WinUSB backend can implement the same `ByteTransport` protocol:

```text
open device interface
read active descriptors
locate bulk IN/OUT dynamically
overlapped bulk read/write
close handle
```

It should not be added until deployment/binding is decided separately. The
wire core requires no WinUSB-specific logic.

## Concurrency and correlation

The first probe permits exactly one outstanding request. A full implementation
will need:

- one reader task per physical transport;
- a bounded input buffer (JLR old DLL: 0x4000);
- parser maximum payload 0x1800;
- a pending map keyed by `(opcode, response flag, sequence)`;
- a non-zero sequence allocator that avoids collision with outstanding work;
- bounded RX and indication queues with explicit overflow reporting;
- separate unsolicited `0x0009`/`0x000A` dispatch;
- monotonic deadlines and no unbounded retry.

Matching by sequence alone, as current gocan does, is insufficiently strict for
JLR because asynchronous traffic shares the stream.

## Canonical CAN API contract (future)

The scanner-facing API should use explicit fields:

```text
open_can_channel(bus=HS_CAN|MS_CAN, bitrate, id_mode, listen_only)
add_can_filter(kind, mask, pattern, flow_control=None)
send_can(id, data, extended, timeout)
receive_can(max_frames, timeout)
```

It must not expose raw channel number 5 as a stable JLR fact. Logical bus to
route/channel/pin mappings belong in a JLR hardware profile proven by capture.

Before enabling CAN, require all of:

1. proven HS/MS route words and pin commands;
2. proven open-channel response and error mapping;
3. JLR filter golden captures;
4. JLR TX acknowledgement/status captures;
5. JLR RX records for DLC 0, 1, 8 and standard/extended IDs;
6. confirmation that variable-length cOutboundData is accepted;
7. a separate transmit interlock at the diagnostic layer.

## Explicit exclusions

No API or command builder is provided for:

- `cResetBoard` (`02 01`);
- `cJumpToFirmware` (`03 01`);
- `cReflashBoard` (`0A 01`);
- `cWriteSerialNumber` (`0B 01`);
- `cUnprotectBootloader` (`0C 01`);
- `cUpdateBTModule` (`12 01`);
- EP0 vendor request `0xDA`;
- PassThruFirmwareUpdate;
- programming voltage;
- EEPROM or persistent configuration;
- ECU programming/flashing.

The probe also omits raw generic-command entry points so a caller cannot supply
one of those words indirectly.

## Platform choice

| Platform | First choice | Fallback | Reason |
|---|---|---|---|
| Windows 11 | libusb/WinUSB after separately approved access strategy | serial only if COM stream is proven | exact match to confirmed bulk transport |
| macOS | libusb | serial if CDC stream is proven | shared code and dynamic descriptors |

No driver change or installation is part of this implementation.
