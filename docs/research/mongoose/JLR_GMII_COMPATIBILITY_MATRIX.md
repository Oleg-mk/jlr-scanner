# MongoosePro JLR vs GM II wire compatibility

## Scope and pinned sources

This is a static, byte-level comparison. No Mongoose transport was opened and
no USB/serial command was sent.

- gocan repository: `https://github.com/roffe/gocan`
- pinned commit: `cb17a9651f8f47f6c3d191c79663b2410ed94bae`
- commit date: 2026-08-14 17:35:16 +0200
- analysed upstream files: `pkg/drewtech/drewtech.go`,
  `pkg/drewtech/j2534.go`, `pkg/drewtech/firmwareinfo.go`,
  `pkg/drewtech/drewtech_test.go`, and
  `adapters/drewtech/drewtech_linux.go`
- first experimental DrewTech commit: `629a88e6e82a6f635b64965f1d9b8a92b59957d6`
- major native-driver revision: `2e26e1c3098ddb8d406e6f0b7a1222c7c83e313f`
- serial dependency: `go.bug.st/serial v1.8.0`, source commit
  `9b9e4a4b829f22f53181c84df19915b858599bbb`
- JLR binary: `monpj432.dll` 1.1.16.0, SHA-256
  `2710726FDD174F32B5E703E310CFCE047A663451EDBB3E35CA0C8EEFF6929413`

The gocan tree contains comments referring to `devel/drewtech/PROTOCOL.md`,
but that file is absent from both current history and the two DrewTech commits
above. The executable source and tests are therefore the primary GM-II
evidence. Only one hardware golden capture is present: an inbound CAN record in
`drewtech_test.go:86-117`.

Confidence terms:

- **CONFIRMED**: direct source, instruction, constant, or golden capture.
- **STRONG**: multiple static facts agree, but no JLR physical capture exists.
- **PARTIAL**: a shared envelope/family exists while one or more fields differ.
- **UNKNOWN**: static evidence cannot close the gap.

## Critical representation correction

The JLR reports originally displayed the field at payload `+0x04` as a single
u16 opcode. Byte comparison with gocan proves it is better represented as:

```text
payload +0x04: u8 opcode
payload +0x05: u8 flag
```

The little-endian raw command word remains useful for matching the JLR command
name switch:

```text
raw_word = opcode | (flag << 8)
```

This is not cosmetic. For example:

| Wire bytes | Raw word | GM-II gocan meaning | JLR command switch |
|---|---:|---|---|
| `03 00` | 0x0003 | opaque `sendMagic` step | `cOpenDevice` |
| `03 01` | 0x0103 | `GetFirmwareVersion` | **`cJumpToFirmware`** |
| `09 01` | 0x0109 | `GetDeviceInfo` | `cGetBoardInfo` |
| `11 01` | 0x0111 | `GetStatus` | `cCheckCRN` |
| `12 00` | 0x0012 | `SetProtocol` | **`cSetPin`** |

Therefore gocan's GM-II `PassThruOpen` and `PassThruConnect` must never be run
unchanged against JLR. In particular `03 01` reaches a firmware-state command.

## A-H: common envelope

### Exact outer frame

| Offset | Size | GM II | JLR | Result | Evidence |
|---:|---:|---|---|---|---|
| 0x00 | u16 LE | payload length | payload length | SAME | gocan `Packet.ToBytes`; JLR writer VA 0x1006E180 |
| 0x02 | u16 LE | length XOR 0x51E6 | length XOR 0x51E6 | SAME | gocan lines 94-104; JLR VA 0x1006E1E7 |
| 0x04 | variable | command payload | command payload | SAME envelope | both parsers |

`0x51E6` is a full u16 guard, not a payload CRC. The initial gocan version only
checked the low byte and constant `0x51`; current code fixes this and has a
large-frame regression test (`drewtech_test.go:53-84`).

### Common payload prefix

| Payload offset | Size | GM-II interpretation | JLR static use | Result |
|---:|---:|---|---|---|
| +0x00 | u16 | outbound bytes `01, channel`; inbound route A | route/object word A | SAME bytes, semantic name PARTIAL |
| +0x02 | u16 | outbound zero; inbound bytes `01, channel` | route/object word B | SAME bytes, semantic name PARTIAL |
| +0x04 | u8 | opcode | low byte of JLR command word | SAME |
| +0x05 | u8 | request/response flag | high byte of JLR command word | SAME |
| +0x06 | u16 LE | sequence | correlation sequence | SAME field |
| +0x08 | u16 | command-specific count/zero | command-specific count/zero | PARTIAL |
| +0x0A | u16 | reserved/zero | often not initialized by old DLL | PARTIAL |
| +0x0C | variable | command body | command body | command-dependent |

For outbound channel N, both encodings produce `route_a = 0xNN01` and
`route_b = 0`. The captured inbound GM-II frame instead has route A `0x0000`
and route B `0x0501`; this agrees with the JLR dispatcher using the second word
to find the receiving object at VA 0x10043650.

### Length, direction, channel, flags, sequence and matching

| Property | GM II | JLR | Result |
|---|---|---|---|
| length includes | route words + command + body | same | SAME |
| direction/channel | byte-oriented view of two route words | u16 route/object words | SAME bytes / PARTIAL names |
| request flags | 0x00 or 0x01 | raw words 0x00xx/0x01xx | SAME encoding, command semantics can differ |
| response flags | 0x80 or 0x81 | raw words 0x80xx/0x81xx | SAME |
| sequence width | monotonically increasing u16 | non-zero 8-bit counter copied into u16 | DIFFERENT wrap policy |
| response match | sequence only in gocan | opcode family + correlation in dispatcher | PARTIAL; JLR codec matches opcode, flag, and sequence |
| parser resync | one byte after invalid header | one byte at VA 0x1006B3CB | SAME |
| maximum payload | no protocol maximum enforced by gocan | 0x1800 | DIFFERENT guard |

## Command-family matrix

`xx 00` below means opcode byte `xx`, flag byte `00`; it is not an abstract
function number.

| Opcode | GM-II format/meaning | JLR format/meaning | Compatibility | Confidence/evidence |
|---:|---|---|---|---|
| 0x03 | `03 00`, seq, reserved, `DA 6B`: opaque open step; `03 01`: version | `03 00` `cOpenDevice`; `03 01` `cJumpToFirmware` | **DIFFERENT across flags** | CONFIRMED names at JLR VA 0x1003B080; builder VA 0x1003E820 |
| 0x05 | `05 00`, seq, reserved, `49 4C`: close | `05 00` `cCloseDevice` | PARTIAL/SAME family | JLR literal command; exact magic interpretation unknown |
| 0x06 | 20-byte payload; channel route, `06 00`, seq, flags at +0x0C, bitrate at +0x10 | same 20-byte builder at VA 0x1000B3C0 | SAME for flags=0, routing still JLR-specific | STRONG |
| 0x07 | 12-byte close-channel record | 12-byte builder at VA 0x1000B690 | SAME shape | STRONG |
| 0x08 | CAN TX, gocan always emits 32 command bytes after routing | JLR `cOutboundData`; variable payload `DataSize+0x18` | PARTIAL | JLR serializer VA 0x1006B090; see layout below |
| 0x09 | `09 00` async CAN RX; `09 01` device info | `09 00` `cInboundData`; `09 01` `cGetBoardInfo` | SAME family, RX body unverified | CONFIRMED command mapping; one GM capture only |
| 0x0A | asynchronous TX status/completion | JLR `cIndication` | PARTIAL | dispatcher VA 0x1004369E; exact JLR statuses incomplete |
| 0x0C | `0C 00`, config/value identifier at +0x0C | JLR `cGetValue` | PARTIAL | shared family; JLR value IDs not fully mapped |
| 0x0D | 28-byte pass filter with kind/len/mask/pattern | JLR `cTableAddEntry`; observed builder is 27 bytes with table subtype 7 | DIFFERENT/PARTIAL | gocan `j2534.go:290-313`; JLR VA 0x100375E0 |
| 0x10 | stop one filter or subtype 2 clear-all | JLR `cTableClear` | PARTIAL | shared family, JLR subtype/ID semantics incomplete |
| 0x11 | `11 00` multiplexed control; `11 01` status | JLR `11 00` `cIoctl`; `11 01` `cCheckCRN` | PARTIAL for flag 0, DIFFERENT for flag 1 | CONFIRMED command names |
| 0x12 | `12 00` set CAN protocol | JLR `12 00` `cSetPin` | **DIFFERENT** | CONFIRMED JLR literal and builder family |
| 0x13 | `13 00` get serial/string | JLR `cGetString` | PARTIAL | selector and response string offset unresolved for JLR |

## OPEN and CLOSE

The current GM-II open sequence is:

1. `00 01` init/echo with `0x0B48`;
2. `09 01` device info `0x003C`;
3. `0C 00` config `0x2F`;
4. `11 01` status;
5. `03 01` firmware version;
6. `03 00` opaque `0x6ADB` step;
7. `09 01` device info `0xFFFF`;
8. `13 00` serial.

Against JLR, steps 4 and 5 become `cCheckCRN` and
**`cJumpToFirmware`**. Direct reuse is unsafe. The JLR implementation therefore
does not expose this aggregate handshake. Transport `open()` opens only the OS
handle; the sole proposed probe command is `09 01`.

`05 00` is a common close family, but the probe closes the OS transport without
sending it. This avoids adding an unnecessary command to an information-only
test.

## CAN TX binary layout

JLR instruction-level layout from VA 0x1006B09B-0x1006B0FE:

| Absolute | Payload | Size | JLR value | GM-II gocan |
|---:|---:|---:|---|---|
| 0x00 | — | u16 | `DataSize + 0x18` | fixed `0x24` payload length |
| 0x02 | — | u16 | length XOR 0x51E6 | same calculation |
| 0x04 | +0x00 | u16 | route/channel word | bytes `01, channel` |
| 0x06 | +0x02 | u16 | zero | zero |
| 0x08 | +0x04 | u8,u8 | `08 00` | `08 00` |
| 0x0A | +0x06 | u16 | sequence/correlation | sequence |
| 0x0C | +0x08 | u16 | batch message count | 1 |
| 0x0E | +0x0A | u16 | unwritten/reserved | zero |
| 0x10 | +0x0C | u32 | J2534 TxFlags | zero in current API |
| 0x14 | +0x10 | u32 | per-call value, likely timeout | zero |
| 0x18 | +0x14 | u16 | DataSize | `4 + DLC` |
| 0x1A | +0x16 | u16 | ExtraDataIndex | zero |
| 0x1C | +0x18 | DataSize | 4-byte ID + CAN bytes | same content |

CAN ID is big-endian in the message data in both implementations. The JLR DLL
copies the J2534 `Data[]` bytes unchanged; its validation reconstructs the
identifier in big-endian order. DLC is not a standalone byte: it is
`DataSize-4` for classical CAN.

The field positions are substantially shared, but current gocan pads the
command portion to 32 bytes regardless of DLC. JLR's official serializer emits
a variable-sized frame. Compatibility is **PARTIAL**, not SAME.

Extended ID handling is also incomplete in gocan's narrow API: it accepts a
u32 ID but does not expose J2534 `CAN_29BIT_ID` TxFlags. The new serializer keeps
TxFlags explicit and does not infer unproven firmware behavior.

## CAN RX and TX status

The only upstream golden RX capture is:

```text
1e 00 f8 51 00 00 01 05
09 00 00 00 00 00 48 0b
00 00 00 00 fc 37 0d 00
06 00 06 00 00 00 07 e8
01 50
```

GM-II parsing gives channel 5, timestamp `0x000D37FC`, declared data size 6,
CAN ID `0x7E8`, and data `01 50`. JLR confirms `0x0009` dispatch to the channel
receive handler (VA 0x10043685) and uses host-side CAN/ISO-TP parsing, but the
complete JLR `cInboundData` record has not been physically captured. The codec
therefore exposes this as `CanRxCandidate` with evidence metadata.

gocan treats `08 80/81` as queued acknowledgement and `0A 00/80/81` as final TX
status and recognizes error values `0x105`, `0x10C`, and `0x121`. JLR confirms
the same command families (`cOutboundDataResp`, `cIndication`) but not every
status offset/value. Result: **PARTIAL**.

## Filters, buffers and configuration

- GM-II filter add uses `0D 00`, payload length 28, kind at +0x12, pattern
  length at +0x13, mask at +0x14, pattern at +0x18.
- The observed JLR `cTableAddEntry` builder at VA 0x100375E0 sends length 27,
  a table subtype 7 at +0x0C, and a 15-byte entry. This is not byte-identical.
- GM-II `10 00` uses either a u16 filter ID or u32 subtype 2. JLR confirms the
  `cTableClear` family, but the exact scanner-safe table selection is unknown.
- GM-II `11 00` multiplexes clear-TX=2, clear-RX=3, become-master=4. JLR calls
  this family `cIoctl`; individual JLR subtypes remain incomplete.
- GM-II `0C 00` value reads align with JLR `cGetValue`, but JLR channel/value IDs
  for HS/MS routing are not fully reconstructed.

No filter/control command is enabled in the JLR runtime implementation.

## Serial initialization and COM3

gocan calls `serial.Open(port, &Mode{BaudRate:115200})` and sets a 50 ms read
timeout. With `go.bug.st/serial v1.8.0`, zero-valued fields mean:

| Setting | Effective value |
|---|---|
| baud | 115200 |
| data bits | 8 |
| parity | none |
| stop bits | 1 |
| RTS/CTS | disabled |
| software flow control | disabled |
| DTR initial | true |
| RTS initial | true |
| read timeout | 50 ms |

gocan sends no separate DTR/RTS transition and does not purge buffers before
the init sequence.

The JLR official stack imports no serial APIs and opens the
`{F99F6FCE-03F4-4817-8A03-8CFDE4743B92}` KMDF interface. Static binaries do not
confirm that COM3 carries the same stream. Serial remains a test candidate, not
a compatibility claim.

## JLR-specific feature gaps

| Feature | GM-II code | JLR requirement | Needed for scanner | Status |
|---|---|---|---|---|
| HS-CAN 500 kbit/s | hard-coded channel 5 plus bitrate | correct route/channel and J1962 pins | yes | UNKNOWN routing |
| MS-CAN 125 kbit/s | no separate verified bus path | distinct route/pin selection | yes | UNKNOWN |
| CAN1/CAN2 identity | one `canChannel=5` | at least two logical/physical routes | yes | UNKNOWN |
| standard CAN ID | implemented | needed | yes | TX STRONG; RX candidate |
| extended CAN ID | incomplete flags API | may be needed | sometimes | PARTIAL |
| filters | GM-specific layout | JLR table entry layout | yes | DIFFERENT/PARTIAL |
| K-line | adapter advertises capability but native methods are CAN-only | JLR ISO9141/KWP path | optional | NOT IMPLEMENTED |
| pin selection | GM calls opcode 0x12 set protocol | JLR opcode 0x12 is `cSetPin` | yes for routing | UNKNOWN and blocked |
| programming voltage | deliberately unsupported | not needed | no | EXCLUDED |
| firmware update | deliberately unsupported | must never be used | no | EXCLUDED |
| ISO-TP | not complete in this wire package | host-side JLR engine required | yes above CAN | separate future layer |

## Reuse decision

Reusable with little or no conceptual change:

- length/XOR envelope;
- incremental one-byte resynchronization;
- two route words and opcode/flag/sequence header representation;
- asynchronous response correlation pattern;
- high-level separation of byte transport from CAN/ISO-TP.

Must be JLR-specific:

- non-zero 8-bit sequence behavior;
- safe open/identity sequence;
- bus/channel/pin routing;
- CAN TX variable length;
- filters and control/config subtypes;
- CAN RX validation and status mapping.

**Conclusion: reuse WITH A SMALL CORE ADAPTER for framing, but SUBSTANTIAL
CHANGES for a functional dual-bus JLR scanner. The current GM-II Device and
PassThruOpen/Connect paths are not directly reusable.**
