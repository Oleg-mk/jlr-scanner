# The filter table entry on the bare adapter, 2026-11-14

Why: the tester's third session (2026-11-13) showed every transmit queued
and completed on his Discovery 4 and the firmware handing the host no
received frame on either bus, not even to a listen-only channel. The sister
product's client states the rule for its firmware — a pass filter per
identifier, or you receive nothing — and the vendor library's own
PASS/BLOCK filter path, read statically the same morning, gave the record
that rule is written in. The owner's adapter, in USB with nothing on the
OBD side, was asked whether it takes that record. Nothing was transmitted
on the bus at any point; every command here is table configuration of the
adapter, answered in the firmware's own words.

Tools: `scripts/mongoose-probe/mongoose_table_entry_probe.py`,
`mongoose_table_type_sweep.py`, `mongoose_pass_filter_probe.py`, over the
serial port (`usbser`, COM4), the adapter powering up in its bootloader and
jumped to firmware by the first probe.

## 1. The record the library's builder at VA 0x100375D0 sends — refused

`table_entry_subtype7_hs.txt`. Channel 5 opened without the listen-only
flag, pins 6/14, then seven 27-byte records with table subtype 7, mask 0,
pattern 0 and seven pairs of values for the two trailing fields. Every one
drew `0x800D`, status `0x00000200`, "CanTableAdd: Unsupported or Invalid
table type". So that builder is not the CAN filter's (the library has
several `cTableAddEntry` builders), and the type word at +0x0C is the first
thing the firmware checks.

## 2. The table types the firmware knows — a sweep of the type word

`table_type_sweep_hs.txt`. The same record with the type word 0 to 15:

| Type | The firmware's answer |
| ---: | --- |
| 0, 1 | `0x202` eInvalidValue, "FilterAdd: Invalid Filter type" — the filter table, refusing the next field |
| 4 | `0x203` eInvalidMsgLength, "PeriodicAdd: Invalid message length" — the periodic-message table |
| 2, 3, 5–15 | `0x200`, "CanTableAdd: Unsupported or Invalid table type" |

The errors show that types 0 and 1 reach filter handling and type 4
reaches periodic-message handling. The library maps types 0 and 1 to PASS
and BLOCK respectively. Only the PASS entry below was accepted in these
runs.

## 3. The PASS filter record of the library's own filter path — accepted

The record, from the library's PASS/BLOCK handler (RVA 0xE600, read
statically; see `docs/evidence/MONGOOSE_STATUS_WORDS_2026-11-14.md` for the
morning's method): after the command header, a 32-bit table subtype (0 for
PASS, 1 for BLOCK), a 16-bit flags word (bit 0x0100 for a 29-bit
identifier), the rule type byte (1 PASS, 2 BLOCK), the pattern length byte,
then the mask bytes and the pattern bytes as J2534 carries them, identifier
words big-endian. Twenty-eight bytes of payload for a four-byte mask and
pattern — the sister product's client calls its own pass filter "28-byte"
too.

`pass_filter_hs.txt` (and `pass_filter_hs_first_run.txt`, the first run,
cut short by the probe's own printing after the answer): channel 5 opened
without the listen-only flag, pins 6/14, then

```
TX 1C 00 FA 51  01 05 00 00  0D 00 04 00  00 00 00 00  00 00 00 00  00 00 01 04  00 00 00 00  00 00 00 00
RX             00 00 01 05  0D 80 04 00  00 00 00 00  00 00 00 00  9C 04 34 03  CC 04 00 00
```

Both replies carried **status `0x00000000`**. The four bytes after the
tick were `CC 04 00 00` in the complete run and `90 02 00 00` in the
first run. Their meaning is unknown. The complete run counted zero inbound
frames during three seconds on the disconnected adapter and recorded a
successful close. The first run ended with a Python console encoding error
after the filter acknowledgement. Its transcript has three local path
prefixes replaced with `<repo>` or `<python>` and retains the raw frames.

## 4. The gate of ADR-0043 — the product's own code, the afternoon

After `ADR-0043` was accepted and built, the same adapter, the same way —
USB only, nothing on the OBD side, nothing transmitted — was run through
the product's own code with the probe tool (`tools/mongoose-jlr-probe`,
release build, COM4). Every open is a listen-only one; the adapter's
serial number is replaced with `<serial>`.

| Transcript | What ran | Result |
| --- | --- | --- |
| `adr0043_gate_filter_cycles_hs.txt` | `--filter-cycles hs-can 25` | 25 of 25 opens; the 11-bit and the 29-bit entry accepted with status 0 every time |
| `adr0043_gate_filter_cycles_ms.txt` | `--filter-cycles ms-can 25` | 25 of 25, the same |
| `adr0043_gate_filter_cycles_hs_200_1.txt`, `_2.txt` | `--filter-cycles hs-can 200`, twice | 200 of 200 each, the same; a few hundred opens a second |
| `adr0043_gate_passive_ms.txt` | `--passive-route ms-can`, 20 s | both entries accepted, zero frames (no bus), no drops, a clean close |

450 opens on one adapter, 900 entries, not one refused: entries do not
fill the firmware's table over that many channels, whatever it does with
them at a close. The four bytes after the tick moved between `0x01C0` and
`0x91C4` in steps that are multiples of `0x34`, rising and falling, and
carried on across connections without resetting: not a count of entries
and not memory that is never given back. What they are stays `UNKNOWN`.

## What is settled, and what is not

| Item | State |
| --- | --- |
| The PASS filter record's layout (subtype, flags, type, length, mask, pattern) | `HARDWARE_CONFIRMED` on the bare adapter: accepted with status 0 |
| The firmware's handling of types 0 and 1 as filters and type 4 as periodic | Observed in rejection messages. BLOCK acceptance remains unverified |
| MS-CAN, listen-only mode, the 29-bit flag, two entries on one channel | `HARDWARE_CONFIRMED` on the bare adapter (§4): accepted with status 0 |
| A non-zero mask or pattern | `NOT_YET_VALIDATED`; not sent by the product |
| The word returned with an accepted entry | `UNKNOWN`; its range and steps in §4 |
| Whether a received frame reaches the host once a PASS entry is in place | `NOT_YET_VALIDATED` — the next real-car session |
| Whether entries survive a channel close | `UNKNOWN`; 450 open–entry–close cycles never filled the table (§4) |

See [ADR-0043](../../adr/ADR-0043-one-pass-filter-on-every-can-channel-before-it-listens.md)
for the proposed integration and its validation plan.
