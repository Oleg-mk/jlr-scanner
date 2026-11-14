# ADR-0043: One pass filter on every CAN channel before it listens — the record the firmware waits for before it hands the host a frame

- Status: **Accepted, 2026-11-14**, on the owner's word («Приймаю,
  будуй»), the day it was written at his word («прочитай … а потім напиши
  ADR на наступний крок»), the day the firmware's status words were named
  and the bare adapter accepted a pass filter entry.
- Scope: what `mongoose-jlr` sends between opening a CAN channel and using
  it, what it records of that, how the bench answers it, and the
  architecture guard's rule for the one table command it adds. The frames
  sent to modules, the read and service operations, their safety classes,
  knowledge, the interface and the transports are not touched; no
  operation is added. The K-line routes are not CAN and are not touched.

## Context

### What the car showed

The tester's third session (build 1.2.7, his Discovery 4, ignition on,
2026-11-13) accounted for every frame
(`docs/evidence/TRANSMIT_PATH_AUDIT_2026-11-12.md`, §11): 45 transmits, each acknowledged with `0x100` and followed by one
`cIndication` with status `0x106`; zero inbound records in every wait; zero
frames in a ten-second listen-only capture of HS-CAN. The vendor library's
own table of status words, read statically the next morning
(`docs/evidence/MONGOOSE_STATUS_WORDS_2026-11-14.md`), names `0x100`
eMsgQueued and `0x106` iMsgTxDone. A CAN frame completes only when another
node acknowledges it, so all 45 frames went out on both buses at the rates
the data gives. The cable, the driver, the routes, the bit rates and this
application's transmit path are confirmed. One thing is left: the firmware
handed the host no received frame, on either bus, to a diagnostic channel
or a listen-only one.

### Why a filter

- The sister product's open client (MongoosePro GM II, same vendor, same
  framing, same command family) states the rule for its firmware: a pass
  filter per identifier, or nothing is received; a filter with mask zero
  passes everything (the audit's §4).
- The vendor library implements `PassThruStartMsgFilter` with
  `cTableAddEntry`, command `0x000D` (`MONGOOSE_USB_PROTOCOL.md`,
  `MONGOOSE_REVERSE_ENGINEERING.md`, strong inference), and this
  application has never sent it: `ADR-0042` left the entry undecided for
  want of its record.
- No frame has reached the host on any channel this application has ever
  opened on a car, and every channel it opened had no entry. Nothing
  observed contradicts the rule. That reading is a `HYPOTHESIS` until a
  frame arrives.

### What the bare adapter said, 2026-11-14

`docs/evidence/mongoose-probe-2026-11-14/` (research scripts in
`scripts/mongoose-probe/`, the owner's adapter in USB, nothing on the OBD
side, nothing transmitted on a bus):

1. The 27-byte record the research had attributed to the filter builder,
   table subtype 7, was refused: `0x200`, "CanTableAdd: Unsupported or
   Invalid table type".
2. A sweep of the type word: types 0 and 1 reach the filter handler
   (`0x202` eInvalidValue, "FilterAdd: Invalid Filter type", refusing the
   next field); type 4 reaches the periodic-message table (`0x203`,
   "PeriodicAdd: Invalid message length"); the others are unsupported.
3. The record of the library's own PASS/BLOCK path, read statically, was
   **accepted with status 0**, twice (`pass_filter_hs.txt`,
   `pass_filter_hs_first_run.txt`). After the 12-byte command header:
   a 32-bit table subtype (0, PASS), a 16-bit flags word (0x0100 for a
   29-bit identifier), the rule byte (1, PASS), the pattern length (4),
   four mask bytes, four pattern bytes:

   ```
   TX 1C 00 FA 51  01 05 00 00  0D 00 04 00  00 00 00 00  00 00 00 00  00 00 01 04  00 00 00 00  00 00 00 00
   RX             00 00 01 05  0D 80 04 00  00 00 00 00  00 00 00 00  9C 04 34 03  CC 04 00 00
   ```

   The offsets agree with the sister product's open client byte for byte —
   rule at +0x12, length at +0x13, mask at +0x14, pattern at +0x18 (the
   research's compatibility matrix) — with the JLR subtype at +0x0C and the
   flags at +0x10. The answer carries four bytes after the tick, `CC 04 00
   00` in one run and `90 02 00 00` in the other; their meaning is
   `UNKNOWN`.
4. A third run (`pass_filter_rust_usb_hs.txt`) went through this
   project's serial transport and codec: a capture without the entry, one
   with it (status 0), and one on a reopened channel, twenty seconds each,
   zero frames each and a clean close each. On a bare adapter zero frames
   is expected either way; the run shows the entry and the close after it
   on the product's own path. The program that printed it is a modified
   copy of `tools/mongoose-jlr-probe` that is not in the tree.

Not seen on the bench: an entry on MS-CAN, on a listen-only channel, with
the 29-bit flag, with a non-zero mask or pattern, a second entry on one
channel, and whether entries outlive the channel.

`ADR-0042` said the entry would get a record of its own before any byte of
it is sent. These probes were research tools on the bare adapter and ran
before this record; the product sends nothing of it until this record is
accepted.

### Why not the library's own session first

The strongest evidence would be the vendor library's own session, captured
on the wire: its filter and whatever else it sends after the open. It
needs the library running against the cable through the package's kernel
driver, and on the owner's Windows 11 PC that driver did not load on
2026-11-14 even with memory integrity and Smart App Control off (Code
Integrity events 3076, 3077 and 3004; the device reported that its driver
failed to load). It needs a Windows 10 machine: a virtual machine with USB
pass-through on the owner's PC, or the tester's SDD laptop with the capture
scripts the repository holds. The entry the bare adapter accepted costs one
build and one session on the tester's car, and that session answers the
question either way. The capture is kept as the next step if it does not
(decision 8).

## Decision

1. **Every CAN channel gets one pass entry per identifier format, after its
   pins and before anything is sent or listened for.** The entry is the
   record the bare adapter accepted, unchanged: subtype 0 (PASS), rule 1
   (PASS), pattern length 4, mask `00 00 00 00`, pattern `00 00 00 00` —
   one with flags 0 for 11-bit identifiers, one with flags `0x0100` for
   29-bit. A zero mask passes every frame; the reads keep choosing their
   module's frames by identifier and format, as they already do
   (`uds_live.rs`, `j1979_live.rs`). It is added in the one place every CAN
   channel is opened (`open_route_internal`), so module reads, live reads,
   services, J1979, the calibration read and the listen-only capture all
   get it. The 29-bit entry is in the build only if the bench gate below
   shows the firmware accepting it beside the 11-bit one; otherwise the
   build carries the 11-bit entry alone and its record says so.
2. **Nothing else from the table family.** No BLOCK entry, no flow-control
   entry, no remove, modify or clear (`0x000E`–`0x0010`), and never table
   type 4: the periodic-message table is a transmit the firmware would
   repeat on its own, and the product has no periodic transmit. The entry's
   subtype, rule, mask and pattern are constants in one function, not
   parameters.
3. **The entry lives and dies with its channel.** It is added after every
   open; there is no removal command; the close is its end. Whether the
   firmware frees it at the close is `UNKNOWN`, so the bench gate opens,
   adds and closes fifty times and must see every entry accepted.
4. **A refused entry fails the open, in the firmware's words.** A non-zero
   status or any text after it closes the channel and ends the operation
   with the status, its name and the firmware's text: "the adapter refused
   the receive filter", not a silent two-second timeout. Without an entry
   nothing can be heard, so going on would only hide the cause.
5. **Recorded, as `ADR-0042` records the acknowledgement.** Each entry's
   format, status, name and the four bytes returned go into the operation's
   trace and into every read record's `adapterTrace`. Indications are
   counted by status word, not only the last one, so a lost-frame
   indication (`0x119` iMsgRxQueueOvflw) is visible once frames flow on a
   busy bus.
6. **Every status word carries its name.** Traces, error words and the
   printed report show the word with its name from the library's table —
   `0x00000106 iMsgTxDone` — and "unnamed" for a word the table does not
   have. The table is that library build's (1.1.16); it is data in the
   adapter crate, not knowledge of a vehicle.
7. **The bench answers as the firmware does.** A PASS entry is accepted
   with status 0 and a four-byte word; a malformed rule byte draws `0x202`
   with "FilterAdd: Invalid Filter type"; an unknown table type draws
   `0x200` with "CanTableAdd: Unsupported or Invalid table type". The bench
   vehicle hands a channel its frames only once a PASS entry is on it — the
   hypothesis of this record made executable and labelled as such, as
   `ADR-0042` did with the indication. The entry and its answer of
   `pass_filter_hs.txt` are a golden test byte for byte.
8. **If the car still sends nothing, no more guesses.** Zero frames in a
   listen-only capture with the ignition on and the entry accepted takes
   the hypothesis out. The next step is then the vendor library's own
   session captured on Windows 10 (the virtual machine or the tester's
   laptop, the owner's call), not another command tried on the car.
9. **The guard is loosened for exactly this.** `scripts/check-architecture.mjs`
   gets a rule over `mongoose-jlr` and `bench-vehicle`: `0x000D` is built
   in one function; no constructor names `0x000E`, `0x000F`, `0x0010`, a
   BLOCK rule or the periodic table; the entry's fields are constants. It
   lands in the commit that adds the entry.

Narrow filters — the module's response identifier as the pattern, mask
`0x7FF` — are not decided here. Whether the firmware compares a non-zero
pattern in the byte order the record carries can only be seen on a bus
with frames; they get an amendment once frames flow and a busy bus is
shown to cost anything.

## Validation

- `IMPLEMENTED` / `FIXTURE_TESTED`: the golden entry and answer; the bench
  silent on a channel without an entry and speaking on one with it; a
  refused entry ending the open in the firmware's words with the channel
  closed; the third session's shape replayed with the entry accepted; the
  trace and `adapterTrace` carrying the entries, the counted indications
  and the names; every opener getting the entry.
- `HARDWARE_CONFIRMED` — **the gate before the tester gets the build**, on
  the owner's bare adapter in USB, nothing on the OBD side, when he says
  so: on `hs-can` and `ms-can`, on a diagnostic and a listen-only channel,
  the 11-bit and the 29-bit entries accepted with status 0; fifty
  open–entry–close cycles on one connection, every entry accepted; a module
  read and a capture whose records carry the entries and a clean close.
  What the bare adapter cannot show: reception.
- `VEHICLE_CONFIRMED`: the tester's car, ignition on — a listen-only
  capture of HS-CAN and of MS-CAN, then "Check all modules". Frames in the
  captures confirm the entry as the missing piece; a module's answer
  confirms the whole path. Zero frames with the entry accepted is decision
  8. Either way the session settles the question.

## Consequences

- If the hypothesis holds, this is the first build in which a module can be
  heard, and the capture becomes a capture.
- On a busy bus every frame reaches the host while a channel is open. The
  reads already discard other identifiers; the trace's inbound count will
  run into thousands per read, and a lost-frame indication says when
  narrowing has become worth an amendment.
- Every CAN open costs one or two more command exchanges, a few
  milliseconds each.
- The bench changes behaviour: without the entry its vehicle is silent, so
  every test that opens a channel goes through the entry.
- The research's line "No filter/control command is enabled in the JLR
  runtime implementation" is out of date once this is built, and is
  corrected in the same change.
- No safety class changes and `SAFETY_BOUNDARIES.md` gets no row: the entry
  selects what the adapter hands up and transmits nothing; it is part of
  opening a channel, not an operation.
- What `ADR-0042` left undecided for want of evidence — a filter table
  entry — is decided here. Still undecided, for the same reason: any value
  or configuration command after the open, `cOpenDevice`, flow-control
  entries, narrow filters.

## Built, 2026-11-14

Decisions 1 to 7 and 9 were built the day the record was accepted, with
both entries, 11-bit and 29-bit, in the build until the gate says
otherwise:

- `passive.rs` builds the entry in one function, its subtype, rule, mask
  and pattern constants, and reads the firmware's answer without judging
  it; the recorded entry and answer of `pass_filter_hs.txt` are a golden
  test byte for byte, the type sweep's refusal another.
- `device.rs` adds the entries after the pins in the one function every
  CAN channel is opened by, records each in the trace with its word, and
  closes the channel on a refused one with the firmware's words
  (`ProtocolError::ReceiveFilterRefused`). The trace counts indications by
  status word.
- `status.rs` holds the library's table; every status word in a trace or
  an error is said with its name, and a word the table does not have is
  "unnamed".
- The bench answers the entry as the firmware did and hands a channel its
  vehicle's frames only through a pass entry.
- The shell writes the entries, the acknowledgement's name and the
  indications by status into every read record's `adapterTrace` (new fields,
  read as empty from older session files), and the entries into a
  capture's provenance.
- The probe tool prints the entries of a passive capture and has a gate
  mode, `--filter-cycles <route> <count>`: listen-only opens and closes on
  one connection, nothing listened for, nothing transmitted.
- The architecture guard has decision 9's rule; it was shown to fail on a
  periodic subtype put in the PASS constant's place.

84 tests in `mongoose-jlr` (74 before this change), 511 in the workspace, clippy and the formatting clean, the architecture guard clean.
`IMPLEMENTED` / `FIXTURE_TESTED`.

## The gate on the bare adapter, 2026-11-14

The owner's adapter, USB only, nothing on the OBD side, the product's own
code through the probe tool (`docs/evidence/mongoose-probe-2026-11-14/`,
§4 of its README): 25 listen-only opens on `hs-can` and 25 on `ms-can`,
then 200 and 200 more on `hs-can`, every one with both entries accepted
with status 0; a 20-second listen-only capture of `ms-can` with both
entries, zero frames, a clean close. The 29-bit entry stays in the build.
450 opens did not fill the firmware's table; the word an accepted entry
returns rose and fell between `0x01C0` and `0x91C4` and is still unknown.
Listen-only channels on both buses, the 29-bit flag and two entries on one
channel are `HARDWARE_CONFIRMED`. Not yet run on the product's own build:
the diagnostic channel with both entries — the morning's probe showed the
11-bit one accepted on such a channel — which the installer's module read
on the bare adapter shows next, with the record's `adapterTrace`.
