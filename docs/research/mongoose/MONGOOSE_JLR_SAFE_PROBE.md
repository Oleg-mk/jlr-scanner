# MongoosePro JLR safe probe

## Current state

**SAFE PROBE READY: YES, for a separately approved one-command experiment.**

No physical write was performed while preparing or testing it. The default CLI
mode is read-only enumeration and byte display.

## Static safety decision

The GM-II information-looking commands are not uniformly safe on JLR:

- `03 01` is GM-II GetVersion but JLR `cJumpToFirmware`: forbidden.
- `11 01` is GM-II GetStatus but JLR `cCheckCRN`: excluded.
- `13 00` is `cGetString` on JLR, but selector/response semantics are not fully
  recovered: excluded from the first probe.
- `09 01` maps to JLR `cGetBoardInfo` and has a dedicated response `09 81`:
  selected.

JLR evidence for `09 01`:

- literal `cGetBoardInfo` in command switch VA 0x1003B080;
- request builder VA 0x1003F160;
- response branch for raw word 0x8109;
- builder sends 12 payload bytes with route word 1, route word 0, raw command
  0x0109, a non-zero sequence, and reserved tail;
- no call from this builder to reset/reflash/unprotect/update routines.

## Exact proposed bytes

For first sequence value 1, the probe constructs:

```text
0c 00 ea 51  01 00 00 00  09 01 01 00  00 00 00 00
```

Field decoding:

| Bytes | Meaning |
|---|---|
| `0c 00` | payload length 12 |
| `ea 51` | `0x000c XOR 0x51e6 = 0x51ea` |
| `01 00` | outbound route A, channel 0 |
| `00 00` | route B zero |
| `09` | opcode 0x09 |
| `01` | request flag 0x01; raw word 0x0109 |
| `01 00` | correlation sequence 1 |
| `00 00` | explicitly zeroed request field |
| `00 00` | deterministic reserved zero |

The old DLL explicitly zeroes payload +0x08 but does not consistently
initialize +0x0A in its stack builder. Sending deterministic zero avoids
leaking stack residue and does not introduce a subcommand.

Expected matching response header:

```text
route words, 09 81, sequence 01 00, ...
```

The first probe does not attempt to interpret undocumented board-info body
fields. It reports route/opcode/flag/sequence and raw body hex.

## Utility behavior

Location: `tools/mongoose-jlr-probe/`.

Default invocation:

```text
python tools/mongoose-jlr-probe/probe.py --show-proposed-bytes
```

Default behavior:

1. query Windows CIM for `VID_18E1&PID_0104` and matching serial ports;
2. if pyusb is already available, read descriptors;
3. print the proposed command bytes;
4. report `writes_performed: 0`;
5. do not open COM or claim a USB interface.

No dependency is installed automatically. Missing pyserial/pyusb is reported
only if that optional path is requested.

Physical execution is deliberately double-gated:

- `--execute-info` must be present;
- `--confirm-write I_ACCEPT_ONE_GET_BOARD_INFO_WRITE` must match exactly;
- `--transport serial|libusb` must be explicit;
- serial additionally requires an explicit port;
- one request is sent, with no retry;
- the OS transport is closed in `finally`;
- no Mongoose `cCloseDevice` command is added.

These switches document a future approved action; they were not used here.

## Transport candidates

### SERIAL

Candidate mode is 115200 8N1, no flow control, DTR/RTS true, 50 ms read
timeout. COM3 stream equivalence is UNKNOWN. Opening the port may assert modem
lines, so even open is deferred until approval.

### LIBUSB

The backend discovers bulk IN/OUT from the active descriptors. It refuses an
interface owned by a kernel driver and never detaches it. It does not set a
configuration, reset, issue control transfers, or change Windows binding.

On the current Windows configuration, libusb may be unable to claim the
interface while dtmonpro is bound. Failure is an expected safe result; the
probe must not work around it automatically.

## Firmware/update reachability

**From the probe utility: NO.**

There is no generic raw-command option and no builder/call for firmware,
bootloader, reset, update, EEPROM, programming voltage, CAN open, filters, or
CAN transmit. EP0 request `0xDA` is absent.

The device firmware itself naturally implements maintenance commands, so this
statement concerns reachability through this probe, not their absence from the
device.

## Approval boundary

Before any physical write, report and wait:

```text
SAFE PROBE READY: YES
Transport candidate: SERIAL / LIBUSB / BOTH (physical access still unverified)
Command proposed: cGetBoardInfo, raw command word 0x0109
Exact bytes proposed: 0c 00 ea 51 01 00 00 00 09 01 01 00 00 00 00 00
Why safe: dedicated read-like JLR command and response; no state-changing path
Firmware/update path reachable from probe: NO
```

No CAN or vehicle action follows from approval of this information probe.
