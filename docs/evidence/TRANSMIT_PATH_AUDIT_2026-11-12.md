# The transmit path, audited against the first two real-car sessions

Date: 2026-11-12. Status: analysis; no code changed, nothing validated on
hardware by this document. Written at the owner's word after the second
session file from the first tester's Discovery 4 (L319, MY10, V8 NA) showed
zero answers for the second time, and after his remark that the ignition had
been on.

Sources, in the order they are used: the two session files the tester sent
(builds 1.2.5 and 1.2.6); the bench transcripts of 2026-09-06 and 2026-09-08
(`mongoose-probe-2026-09-06/`, `mongoose-probe-2026-09-08/`); the static
research on the vendor library (`docs/research/mongoose/`); the sister
product's open client, gocan, at the commit the research pins; and the code
in `crates/mongoose-jlr`. Every statement carries its evidence; what no
source settles is listed as unknown at the end.

## 1. What the second session recorded

Build 1.2.6 (commit 60cfd2b), the cable on SDD's driver (`dtmonpro`), board
communication verified, the adapter already in its firmware (the board-info
body carried a running tick, so no jump was sent). Library loaded, context
L319 MY10 V8NA. Ignition on, engine off — the tester's words.

| Operation | Started (UTC) | Requests | Answers |
| --- | --- | ---: | ---: |
| Live read of the PCM, two identifiers | 11:14:49.045 | 6 | 0 |
| Check all modules (confirmed fault codes) | 11:15:05.975 | 39 | 0 |
| Mileage | 11:15:51.146 | 46 | 0 |

The six live samples, with the time after the run started:

| At | Request | Failure recorded |
| ---: | --- | --- |
| 123 ms | PCM 0xDA02 | `diagnostic request transmission failed: cOutboundData failed with device status 0x00000100` |
| 227 ms | PCM 0xDA48 | `CAN connection failed: unexpected Mongoose command 0x8007; expected 0x8006` |
| 355 ms | PCM 0xDA02 | `CAN connection failed: response sequence mismatch: expected 9, got 8` |
| 461 ms | PCM 0xDA48 | `… expected 10, got 9` |
| 591 ms | PCM 0xDA02 | `… expected 11, got 10` |
| 720 ms | PCM 0xDA48 | `… expected 12, got 11` |

Every later request of the session failed the same way, one step behind,
through `expected 51, got 50` for the last module and `expected 97, got 96`
for the last mileage reading. Nothing was tried on MS-CAN before the chain
broke, and nothing recovered it.

### 1.1 The sequence numbers reconstruct the whole exchange

The device numbers its commands from 1 and expects each answer to carry the
command's number. Counting what the code sends:

| Seq | Command | What the code read back |
| ---: | --- | --- |
| 1 | board-info at connect | its answer, 0x8109 |
| 2 | board-info at the first route open | its answer; the firmware was running |
| 3 | open resource 5 (HS-CAN, 500 kbit/s, no listen flag) | its answer, status 0 |
| 4 | pins 6/14 | its answer, status 0 |
| 5 | the request, 0x7E0 `22 DA 02`, padded | 0x8008 with status `0x100` — taken as a refusal |
| 6 | close the channel | **one extra frame** — the close failed; the channel was remembered as stale |
| 7 | close the stale channel (new in 1.2.6) | the answer to 6 — a mismatch, ignored by design |
| 8 | open resource 5 again | the answer to 7, a close response — `unexpected command 0x8007; expected 0x8006` |
| 9 | open | the answer to 8 — `expected 9, got 8` |
| … | … | … |

From seq 6 on, every wait consumed the previous command's answer, so every
command failed before its own answer was read, and its own answer waited in
the stream for the next command. The adapter executed every one of those
commands; what it answered to an open of a resource it already had open is
unknown, because the sequence check comes before the status is looked at.

Exactly one frame entered the stream between the acknowledgement of the
request (seq 5) and the close response (seq 6). Its identity was not
recorded: the close's error is discarded when the exchange before it has
already failed (`execute_prepared_uds_read`, the `(Err(error), _)` arm), and
the live-read and module-read records hold only the exchange's error.

### 1.2 The first session, for comparison

Build 1.2.5, same car, same laptop, 07:58 UTC the same day, ignition state
not stated.
Live samples at 122 ms and 267 ms both drew `device status 0x00000100`; the
close after the first transmit went through (the second request reached the
adapter), the close after the second did not, and 1.2.5 then refused every
later operation itself ("a passive receive route is already open"). So in
that session the extra frame came after the second transmit, within the few
milliseconds before the close's answer; in this one after the first. No
close timed out in either session: the gaps between samples are the 100 ms
cadence plus a few milliseconds of round trips.

## 2. The transmit path as built

One module read, end to end, with what each step assumes and the evidence
behind the assumption.

| Step | Bytes sent (payload) | Wait | Assumption | Evidence |
| --- | --- | --- | --- | --- |
| open | route word `0x0501`, `0x0006`, seq, flags `0`, bitrate `500000` | 1 s | status 0 and the route word back as token | bench 2026-09-06/08, `HARDWARE_CONFIRMED` |
| pins | `0x0012`, seq, `1`, `6`, `14` | 1 s | status 0 | bench, `HARDWARE_CONFIRMED` |
| request | `0x0008`, seq, count `1`, TxFlags `0`, per-call word `0`, DataSize `12`, ID big-endian, 8 data bytes | 1 s | **a 0x8008 with status 0** | the layout matches the vendor serializer field for field (research, CONFIRMED); the status-0 expectation has **no evidence** — see §3 |
| answer | — | 2 s (5 s after ResponsePending) | the module's frames arrive as `cInboundData` 0x0009 records on this channel, without any filter | layout from the vendor parser and the sister product's one golden capture; delivery without a filter **never observed** — see §4 |
| close | `0x0007`, seq | 1 s | status 0 | bench, `HARDWARE_CONFIRMED` |

Two rules of the code shape everything that happened:

- **The next command-class frame is the answer.** `wait_for_response` and
  `wait_for_outbound_response` skip 0x0009 records and hand the first other
  frame to the parser; a different command or sequence is an error, and the
  frame is consumed either way. There is no pending map, no skipping of an
  answer to an earlier command, and no handling of any other frame the
  adapter may send on its own.
- **A non-zero status is a refusal, for every command alike.**
  `parse_command_response` turns any non-zero status word into
  `DeviceStatus`. The rule was taken from open, pins and close, where the
  firmware's refusals carry small codes and a text; it was applied to the
  transmit acknowledgement unchanged.

The 1-second command wait is the same for the acknowledgement of a transmit
as for an open.

## 3. What the firmware answers to a transmit

### 3.1 On the bench, with no car

2026-09-06, resource 5, and 2026-09-08, resources 21 and 5: the
application's own record for `0x7E0` / `0x733` drew `0x8008` with status
`0x00000100` and no text — **after 3.12 s and 3.04 s** — and the close sent
right after it was answered with status 0, nothing in between. The 0x8008
body is 12 bytes, `00 01 00 00 · tick · 00 00 00 00`, where an accepted
open, pins or close carries 9: `00 00 00 00 · tick · 00`. The evidence of
that day says, correctly: what `0x100` means, and whether frames arrive in
non-listen mode without a filter, is for the first car (ADR-0018).

### 3.2 In the car, ignition on

The same word, twice in each session, **within the first hundred
milliseconds** of the open — the acknowledgement itself took a few
milliseconds, where a bus with nothing on it made the firmware wait three
seconds. Then one extra command-class frame.

### 3.3 Three signs that `0x100` is not a refusal

1. **This firmware's refusals carry words.** `cOpenChannel: Unsupported or
   Invalid Resource ID 0` with status 3; `MongoosePro JLR board only
   supports PWM on pins 2 and 10` with status `0x206`; `Invalid or
   Unhandled command type` from the bootloader. The transmit's `0x100`
   carries none, on the bench and in the car.
2. **The shape of the status words.** The refusals seen are
   `(family << 8) | code`: `0x206` for a pin refusal, `0x003` for an open.
   `0x100` is the transmit family with code 0.
3. **The sister product reads it that way.** gocan, the open client for the
   MongoosePro GM II (same vendor, same framing, same command family, as the
   research's compatibility matrix lays out), says of the wire: every `0x08`
   transmit gets a `0x08` "queued" acknowledgement and a `0x0A` "sent"
   status, both carrying the transmit's sequence; the client does not
   examine the acknowledgement's status word at all, and treats a `0x0A`
   status as a fault only for `0x105` (bus off), `0x10C` (no flow-control
   filter) and `0x121` (no source address) — codes it attributes to the
   GM II vendor library. All three are the `0x1xx` family with a non-zero
   code.

None of the three is JLR proof. Together they say the application has been
treating the firmware's acknowledgement of a transmit as a refusal, on a rule
it never had evidence for, and abandoning every request a few milliseconds
after sending it. Validation of this reading: `HYPOTHESIS`, until a capture
shows a transmit acknowledged with `0x100` and followed by a module's answer.

### 3.4 The extra frame

The vendor library's dispatcher (research, CONFIRMED structure) routes
synchronous responses by opcode and correlation field and hands two
unsolicited record types to separate handlers: `cInboundData` (0x0009) and
`cIndication` (0x000A). The compatibility matrix names 0x0A "asynchronous TX
status/completion" for both products. gocan's reader consumes a 0x0A after
every transmit.

This code recognises 0x0009 only. A 0x000A arriving between the
acknowledgement and the close response is consumed by the close's wait as
"unexpected command" — which is exactly what the sequence numbers show
happened at seq 6, and in 1.2.5 after the second transmit. That the frame
*was* a 0x000A is the one reading the sources offer; it is not recorded, so
it is `HYPOTHESIS` too. Whatever it was, the code had no place for it.

Why it followed the second transmit in 1.2.5 and the first in 1.2.6, and
nothing is seen after the first transmit of 1.2.5, the records cannot say.
A status that reports a bus-state change rather than each transmit would
fit both; so would a race between the indication and the close response.
The next session file must carry the frame's command and status word.

## 4. What the vendor library does that this code does not

From the static research; the gocan column is the sister product's behaviour
on its hardware.

| Step | Vendor library (JLR, static) | gocan (GM II, hardware-tested) | This code | Does it matter here |
| --- | --- | --- | --- | --- |
| device open | `cOpenDevice` 0x0003 and version/board checks at PassThruOpen (STRONG) | an eight-step handshake | board-info, and the jump when needed | unknown; the bench answered channel commands without it |
| after the channel open | "value/config commands" (STRONG; which ones, not named) | protocol select 0x12; config reads only — "there is no runtime set" | pins 6/14 | unknown; the bitrate goes in the open body in both products and was accepted |
| filters | `cTableAddEntry` 0x000D; builder 27 bytes, table subtype 7, 15-byte entry; fields partly unknown | **"a pass filter per CAN id is required or you receive nothing"**; a mask-0 filter passes everything | none | **decisive for the answer**: if the JLR firmware delivers nothing without a table entry, no module can ever be heard on this channel, however well the transmit goes |
| response matching | opcode + correlation; 0x0009 and 0x000A dispatched apart | pending map by sequence; 0x08/0x0A consumed by the reader | next frame is the answer | the cause of §1.1 |
| transmit acknowledgement | `cOutboundDataResp` 0x8008 (name only) | "queued", status not examined | status must be 0 | the cause of §3 |
| transmit status | `cIndication` 0x000A (name; "TX status/completion") | "sent", faults 0x105/0x10C/0x121 | not recognised | the frame of §3.4 |
| ISO-TP | host side, with a mandatory flow-control filter for *segmented* transmits | not implemented | host side; single frames only | not reached yet |

The transmit record itself needs no change: field for field it is the vendor
serializer's layout, with TxFlags 0 and the per-call word 0 (the research
reads that word as a likely per-call timeout; what SDD passes is unknown).

## 5. Findings

1. **The acknowledgement of a transmit is misread as a refusal.** Every
   request of both sessions was abandoned on `0x100` a few milliseconds
   after it was sent. Nothing in the record says the car did not answer;
   the code never listened. (§3; `HYPOTHESIS` for the meaning of the word,
   `CONFIRMED` for the code's behaviour.)
2. **One frame the code does not expect ends the session.** Strict
   next-frame matching with no handling of the firmware's own indications
   put the stream one answer behind for good, in both sessions. (§1.1;
   `CONFIRMED` from the records.)
3. **The 1-second acknowledgement wait is shorter than the firmware's own
   give-up.** On a bus where nothing acknowledges, the firmware answers
   after about 3 s (bench). The application would time out at 1 s and meet
   the late acknowledgement as a stray frame at the next command — finding
   2 by another door, reproducible on the owner's bench with the adapter
   alone. (`HARDWARE_CONFIRMED` for the 3 s; the sequence consequence
   follows from the code.)
4. **The close's error and the device's counters are not recorded.** Twice
   the one frame that mattered was consumed and forgotten; the number of
   inbound frames the adapter delivered during a read — which would answer
   the filter question on its own — is counted in the device and written
   only for captures.
5. **Reception without a filter is unverified, and the sister product says
   it does not happen.** No frame from a real bus has ever reached this
   code (the capabilities still read `NOT_YET_VALIDATED`); the GM II
   firmware forwards nothing without a table entry; the JLR table record is
   only partly reconstructed. (§4.)
6. **The bench and the transport fake encode the assumptions.** Both answer
   a transmit with status 0 and an inbound frame at once, and never send an
   indication, so the fixture tests prove the code against its own guess
   of the firmware, not against the firmware. The three answers the real
   adapter has given (0x100 after 3 s on the bench, 0x100 at once plus a
   stray frame in the car) are reproduced in no test.
7. Minor: `pending_frames` survives a close, so a frame received during one
   read's close could be handed to the next read as if on its route.

Nothing in the two sessions points at the car, the cable, the laptop or
the driver: the adapter answered every command, at once, with the words the
bench has seen.

## 6. What settles what

| Question | Settled by | Needs |
| --- | --- | --- |
| Does the firmware deliver frames on a channel without a filter, and is the inbound layout right for JLR? | a listen-only capture on HS-CAN with the ignition on: frames, or zero frames on a bus that is certainly busy, or "non-data packets dropped" | the tester, 1.2.6, no build |
| Is `0x100` the acknowledgement of a transmit that went out? | a read with the acknowledgement recorded and the answer waited for: the module's answer, or a timeout with the inbound count | a build (§7), the tester |
| What is the extra frame? | the same build: command and status word of every frame skipped, and the close's error, in the record | the same |
| Does the firmware require a table entry for reception? | the same read: an acknowledged transmit, a busy bus, and zero inbound frames in 2 s says yes | the same |
| The table entry's record | static work on the vendor builder (VA 0x100375E0), then the bench: the firmware refuses a malformed entry with its own words, as it does for pins | research, the owner's adapter on request, no car |
| The stray-frame recovery, on hardware | a module read against the bare adapter: the 3 s acknowledgement after the 1 s wait, then the next read clean | the owner's adapter on request, no car |

## 7. The change set this points to

Design only; nothing of it is written. One build, after fixture tests that
replay the two sessions' streams byte for byte.

1. **A transmit acknowledgement is recorded, not judged.** 0x8008 ends the
   wait whatever its status word; the word goes into the read's record. The
   answer is then waited for as now. A fault the firmware reports in a
   0x000A during that wait ends it with the status in words.
2. **Responses are matched, not assumed.** The wait loop keeps 0x0009
   records for the receive path, records 0x000A indications with their
   status, skips and records any command-class frame whose sequence is
   older than the one expected, returns the frame whose command and
   sequence match, and errors on a frame with the expected sequence but
   another command, or on timeout. The record of every operation carries
   what was skipped: count, last command, last status.
3. **The acknowledgement wait outlasts the firmware's give-up.** Five
   seconds for 0x8008; one second stays for open, pins and close.
4. **The close's error joins the read's error** when both fail, and the
   counters of the read — inbound frames delivered, frames dropped,
   indications, stale frames — are written into the module-read, live and
   mileage records as they are into captures.
5. **The bench answers like the adapter**: 0x100 on a transmit, an
   indication after it, and inbound frames only where the vehicle answers;
   the shell's end-to-end test then runs on the firmware's real shape.
6. **`pending_frames` is cleared at open.**

Not in the change set, because the evidence is not there yet: a filter
table entry (its record is unknown), any value/config command after the
open (which ones, unknown), `cOpenDevice` (no sign it is needed).

## 8. Validation states

| Item | State |
| --- | --- |
| Channel open, pins, close, on both resources | `HARDWARE_CONFIRMED` (bench, 2026-09-06/08; the tester's laptop, 2026-11-12) |
| The transmit record's format | `HARDWARE_CONFIRMED` (accepted, answered) |
| The meaning of status `0x100` | `UNKNOWN`; read as "not a refusal" on the signs of §3.3 (`HYPOTHESIS`) |
| The extra frame after a transmit | observed twice, identity `UNKNOWN`; read as `cIndication` 0x000A (`HYPOTHESIS`) |
| Inbound delivery on a real bus, with or without a filter | `NOT_YET_VALIDATED` |
| The module's answer to any request | none received; whether one was sent, `UNKNOWN` |
| The sequencing defect of §1.1 | `CONFIRMED` from the records; `FIXTURE_TESTED` once the replay test exists |

## 9. Built, 2026-11-13 — `ADR-0042`

The change set of §7 was accepted as `ADR-0042` the next morning, on the
owner's word, and built the same day. What the building added to the
analysis above:

**Finding 8 — the waits inherited the transport's last read timeout.**
`wait_for_response` and `wait_for_outbound_response` never set a timeout of
their own; they read with whatever the transport was last given. A close
after a read that had run its budget down inherited the last
`receive_frame` remainder — milliseconds, on a bus with other traffic — and
the acknowledgement of a flow control sent in the middle of a read inherited
the same. Either ends a session the way finding 2 does. Neither session
reached that code (every request was abandoned on the acknowledgement), and
no fixture caught it, because the test transports ignore the timeout they
are given. Every wait now sets its own timeout from its own deadline before
each read (`ADR-0042`, decision 3); a test asserts that the close after a
3 ms receive is waited for with its second.

**Two judgments kept, and stated.** Text after an acknowledgement's status
is a refusal — every refusal this firmware has given carried text, and no
acknowledgement has (§3.3). And the bench answers a transmit with the
recorded `0x8008` body and then one `0x000A` indication carrying the
transmit's sequence: the hypothesis of §3.4 made executable so the code
above the transport runs on that shape; it is labelled as such in the code
and proves nothing about the adapter.

**What the fixtures now replay.** In `mongoose-jlr`: the second session's
shape (acknowledgement `0x100` at once, nothing from the module, one
indication met by the close's wait, the next read clean); the first
session's (the indication after the second transmit, during its close); a
late acknowledgement arriving while the close is awaited, skipped as a stale
frame and recorded; an acknowledgement with the firmware's words refused in
words; an answer kept when the close after it fails, the stale channel
closed before the next open; a frame on another channel counted, not fatal;
an earlier session's leftover frame and stray bytes skipped before
board-info; indications during a listen-only capture counted as dropped;
and the bench end to end on the new shape — 74 tests in the crate, the
shell's bench run on top. The recorded acknowledgement frame of 2026-09-08
is a golden test, byte for byte.

**What goes into every read's record.** `adapterTrace`: the transmits and
the last acknowledgement (status word, words, wait in milliseconds), the
inbound records delivered, the indications and stale frames with the last of
each, the frames and bytes dropped, the close's error. The error's technical
details carry the same in words, so the printed report shows them in the
answer's place. A record of 1.2.6 read "CAN connection failed: CAN connection
failed: …"; the message is said once now.

| Item | State after the build |
| --- | --- |
| The sequencing defect of §1.1 | `FIXTURE_TESTED` (both session shapes replayed) |
| The late acknowledgement of finding 3 | `FIXTURE_TESTED`; `HARDWARE_CONFIRMED` is the owner's bare adapter, the gate before the tester's build |
| The timeout inheritance of finding 8 | `FIXTURE_TESTED` |
| The meaning of `0x100` | `UNKNOWN` still; read as the acknowledgement (`HYPOTHESIS`); a module's answer after it confirms it |
| The extra frame | `UNKNOWN` still; the next session file carries its command, sequence and status word |
| Inbound delivery on a real bus, with or without a filter | `NOT_YET_VALIDATED`; the next session file carries the inbound count of every read |

## 10. The gate on the bare adapter, 2026-11-13 — and a correction

Build 1.2.7 on the owner's PC, the adapter in USB and nothing on the OBD
side, his X250 context, the PCM's confirmed fault codes read twice. The
second read's record:

| Field | Value |
| --- | --- |
| `acknowledgementStatus` | `0x00000100`, no words |
| `acknowledgementWaitMs` | **0** |
| `inboundFrames` | 0 |
| `indications` | 0 |
| `staleFrames`, `droppedFrames`, `droppedBytes` | 0 |
| `closeError` | none |
| error in words | `No response from ECU: no response from ECU before timeout; adapter: transmit acknowledged with status 0x00000100 after 0 ms; 0 inbound frames` |

So the build does what `ADR-0042` says: the acknowledgement is recorded, the
answer is waited for, the close is clean, and the second read runs clean —
no stale frame, which also says the first read's close was answered.

**The correction.** §3.1, finding 3 and `ADR-0042`'s decision 3 say the
firmware answered a transmit on the bench "after about three seconds". It
did not. The two "after 3.12 s" and "after 3.04 s" lines of the 2026-09-08
transcript are the probe's own collection window — three seconds plus
overhead, both times — and the firmware's own ticks in that transcript say
how long the acknowledgement really took: the tick advances about
1,032,000 between the open's answer and the pin command's answer, and
about 1,110,000 between the pin command's answer and the acknowledgement;
with the probe waiting its fixed second between commands both gaps are one
second plus the firmware's latency, so the acknowledgement followed the
transmit by tens of milliseconds, in September as today. The firmware
acknowledges a transmit at once, bus or no bus, and `0x100` at once says
nothing about the bus. Finding 3 falls (the five-second wait stays as a
harmless upper bound); §3.2's reading of the car's quick acknowledgement as
a sign of a live bus falls with it.

**What stands, sharpened.** On the bare adapter there was no indication at
all within the two-second wait and the close. In the car there was one
extra frame right after each acknowledgement. The one difference between
the two is the bus: a CAN controller keeps retrying a frame nobody
acknowledges and never completes it, where on a live bus any node
acknowledges it and the transmit completes. So the extra frame is the
bus-dependent signal, and the reading of §3.4 — `cIndication`, the
transmit's completion status — gains the bare adapter's silence as
evidence. Still `HYPOTHESIS`: the next session file records the frame's
command, sequence and status word, and settles it.

What the next car session therefore means: an acknowledgement at 0 ms and
one indication say the frame went out on the bus; a module's frame behind
it confirms the whole path; zero inbound frames behind it, with the
ignition on, is the filter question answered.

## 11. The third session, 2026-11-13 — every frame accounted for

Build 1.2.7 on the tester's Discovery 4 (L319, MY10, V8 NA), the cable on
SDD's driver, ignition on and engine off as asked. In order: a live run of
the PCM (six samples, 07:05 UTC), "Check all modules" (39 reads: 16 on
HS-CAN, 23 on MS-CAN, 07:05–07:07), a listen-only capture of HS-CAN
(07:08), the session saved (07:09). The `adapterTrace` of every record:

| Over 45 transmits (6 live, 39 module reads) | |
| --- | --- |
| acknowledged `0x8008`, status `0x100`, no words | 45 of 45; the first after 163 ms, the rest within 0–90 ms |
| indications | **45 of 45: one `0x000A` after every transmit, carrying the transmit's sequence, status `0x00000106`**, on HS-CAN and on MS-CAN alike |
| inbound records during the two-second waits | 0 |
| stale frames, dropped frames, dropped bytes | 0, 0, 0 |
| close errors | 0 |
| the listen-only capture of HS-CAN | 0 frames; the record says no frame was transmitted or acknowledged |

**Settled on the vehicle.**

1. The extra frame of §3.4 is `cIndication`: command `0x000A`, the
   transmit's own sequence, within milliseconds of the acknowledgement,
   after every transmit. `CONFIRMED`.
2. The state machine of `ADR-0042` holds in the car: 45 transmits, 49
   opens and closes, no shift, no stale frame, every record complete.
3. The firmware delivered no inbound record on either bus during any wait,
   and no frame to a listen-only channel on HS-CAN with the ignition on.
   The acknowledgement's timing is the same as on the bare adapter, so it
   says nothing about the bus (§10); the indication is what the bare
   adapter never sent.

**Not settled: the word `0x106`.** Family 1 — the transmit's — code 6. It
is not one of the three faults the sister product's client names (`0x105`
bus off, `0x10C` no flow-control filter, `0x121` no source address), and
the research has no table for this family. Two readings:

- *completion*: the frame went out and the bus acknowledged it; the
  firmware forwards nothing to the host without a filter table entry — the
  rule the sister product's client states for its firmware, and the one
  thing both buses and the listen-only channel have in common;
- *failure*: the transmit did not complete, and the adapter is not on a bus
  at the right rate. Against it: the identical result on both buses, whose
  rates the data gives separately; SDD reading the same car through the
  same cable on the same laptop; and the bare adapter's silence, which says
  an indication comes only when something is on the other side of the
  transceiver.

The reading taken is the first, as `HYPOTHESIS`. What settles it, in
order of cost:

1. Static research on the vendor library, on the owner's own tooling: the
   filter builder's record (the table entry of 27 bytes, subtype 7) and the
   library's table for the transmit family's status words, where `0x106`
   has a name.
2. The bench, the owner's bare adapter: the firmware's acceptance of a
   table entry — it refuses a malformed one with its own words, as it does
   for pins — and whether the acknowledgement or the indication changes
   with one in place.
3. The tester's car, with the entry in a build: inbound frames in a
   capture, or a module's answer.

The strongest evidence of all would be a USB capture of SDD's own session
on the tester's laptop — what the library sends after the open, filters
and configuration included, and what the firmware answers — with the
capture scripts the repository already holds. It is the owner's call: it
asks the tester to install a capture driver and run a script, more than a
tester should be asked lightly.

Nothing in this session points at the car, the cable, the laptop, the
driver or the application's own sequencing. The one question left is what
the firmware needs before it hands a frame to the host.

## 12. The word named, 2026-11-14

The vendor library carries its own table of the firmware's status words,
in the function that names them for its log lines; read back statically
the next morning (`MONGOOSE_STATUS_WORDS_2026-11-14.md`), it names every
status this project has seen, three of them word for word against the
firmware's texts: `0x003` eInvalidProtocolID, `0x206` ePinNotSupported,
`0x208` eInitTimeout, **`0x100` eMsgQueued**, and **`0x106` iMsgTxDone** —
the transmit completed. The sister product's `0x105` and `0x10C` are in it
under the same meanings.

So the first reading of §11 stands and the second is out: on 2026-11-13
all 45 frames went out and were acknowledged on the tester's buses, HS-CAN
and MS-CAN alike, at the rates the data gives; what the firmware never did
was hand a received frame to the host. The filter table entry is the next
work, and the library's own session on the owner's bench — its filter
record and whatever else it sends after the open — is the shortest way to
its bytes.
