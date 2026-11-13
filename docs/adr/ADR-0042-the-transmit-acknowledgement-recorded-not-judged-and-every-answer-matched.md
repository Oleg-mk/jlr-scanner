# ADR-0042: The adapter's transmit acknowledgement is recorded, not judged, and every answer is matched by command and sequence

- Status: **Accepted, 2026-11-13**, on the owner's word («робимо за твоїм
  планом, але відповідальніше»), the morning after the transmit-path audit
  he ordered on the second real-car session file
  (`docs/evidence/TRANSMIT_PATH_AUDIT_2026-11-12.md`).
- Scope: how `mongoose-jlr` waits for the adapter's answers, what it does
  with the acknowledgement of a transmit, and what it records of both. The
  frames the application sends are unchanged. Not added, because their
  records are unknown: a filter table entry, any value/config command after
  the open, `cOpenDevice`. Transport, knowledge, interface and the safety
  classes are untouched; no operation is added.

## Context

Two sessions on the first tester's Discovery 4, the second with the ignition
on, drew zero answers from 91 requests. The audit reconstructed both from the
sequence numbers in the session files and found the code, not the car:

1. The firmware answers every transmit with `0x8008` and status `0x100`,
   with no text — within milliseconds on a live bus, after about three
   seconds on a bench with nothing to acknowledge. The code applied the rule
   it had learned from open, pins and close — a non-zero status is a
   refusal — to that word, on no evidence (ADR-0018 left the word open), and
   abandoned every request a few milliseconds after sending it. Nothing was
   ever waited for. Three signs say the word is the acknowledgement: this
   firmware's refusals carry text and this word carries none; the refusals
   seen are `(family << 8) | code` and `0x100` is the transmit family with
   code zero; the open client of the sister product (MongoosePro GM II, same
   vendor, same framing, same command family) does not read the status of
   the `0x08` acknowledgement at all and treats the `0x0A` status as a fault
   only for `0x105`, `0x10C` and `0x121`.
2. One frame the code did not expect entered the stream after the
   acknowledgement — in the second session after the first transmit, in the
   first after the second — and the next wait took it for its answer.
   `wait_for_response` handed the first command-class frame to the parser
   and consumed it either way, so from that frame on every command read the
   previous command's answer: `expected 9, got 8` through `expected 97, got
   96`. The vendor library routes answers by opcode and correlation field
   and hands `cInboundData` (0x0009) and `cIndication` (0x000A) to separate
   handlers; this code knew 0x0009 only. The frame's identity was not
   recorded (the close's error is discarded when the exchange before it has
   failed). That it was a 0x000A is the one reading the sources offer, and a
   hypothesis.
3. The one-second acknowledgement wait is shorter than the firmware's own
   three-second give-up on a silent bus, so on the owner's bare adapter the
   same chain starts by another door.
4. Found while building this record: every wait relied on whatever read
   timeout the transport was last given. A close after a read that timed out
   inherited the last `receive_frame` remainder — milliseconds, on a bus
   with other traffic — and the acknowledgement of a flow control sent in
   the middle of a read inherited the same. Either can end a session the
   way finding 2 does.
5. The close's error and the adapter's counters were never written into a
   read's record, so the one frame that mattered was consumed and forgotten
   twice, and the number of inbound frames the adapter delivered during a
   read — which would answer the filter question on its own — was counted
   for captures only.
6. The bench and the test transports answered a transmit with status 0 and
   never sent an indication, so the fixture tests proved the code against
   its own guess of the firmware.

## Decision

1. **The acknowledgement of a transmit is recorded, not judged.** A `0x8008`
   with the transmit's sequence ends the wait whatever its status word; the
   word, the firmware's words after it when there are any, and the time
   from the write to the acknowledgement go into the operation's trace. The
   module's answer is then waited for, as the read was always meant to. One
   judgment is kept and stated: text after the status is a refusal, because
   every refusal this firmware has given carried text and no
   acknowledgement has.
2. **Every answer is matched, not assumed.** A wait names the response word
   and the sequence it expects. A `0x0009` record is kept for the open route
   (parsed as that route's frame) or counted when no route is open; a
   `0x000A` is recorded with its sequence and status word and the wait goes
   on; a command-class frame with any other sequence is a stale answer,
   skipped and recorded with its command, sequence and status; a frame with
   the expected sequence and another command is an error; a timeout is an
   error. Board-info, the jump to firmware, open, pins, close, the line
   commands and the transmit all wait this way. Bytes the codec cannot
   frame are dropped one at a time and counted, so a stream left half-read
   by an earlier session resynchronises on the next frame instead of
   failing the connection.
3. **Each wait sets its own timeout from its own deadline**, before every
   read: one second for board-info, open, pins, close, the jump and the line
   commands; **five seconds for the transmit acknowledgement**, longer than
   the firmware's own give-up. Skipped frames shorten the remaining time;
   they never extend it.
4. **One trace per operation**, counted where the frames pass, from the open
   to the close: transmits and the last acknowledgement (status, words, wait
   in milliseconds); inbound records delivered; indications and the last
   one; stale frames and the last one; frames and bytes dropped; the close's
   error in words. The trace is part of every result and readable after
   every failure. The shell writes it into every read record — module read,
   live read, mileage, passport, configuration, battery, standard OBD — as
   `adapterTrace`, and its words into the error's technical details, so the
   next session file says what the first two could not.
5. **An answer that was received stands.** When the exchange brought the
   module's answer and the close after it failed, the result is returned
   with the close's error in the trace; the channel is remembered as stale
   and closed again before the next open (1.2.6's rule). A clear or routine
   that the module performed is no longer reported as failed because of the
   close.
6. **The receive path fails closed on the record, not on the session.**
   `pending_frames` is cleared at every open; an inbound record on another
   channel is counted and skipped, not an error for the read.
7. **The bench answers as the adapter has.** A transmit draws `0x8008` with
   status `0x100` and the recorded twelve-byte body (status, tick, four zero
   bytes), then one `0x000A` indication carrying the transmit's sequence —
   the hypothesis of finding 2 made executable and labelled as such — and
   then the vehicle's frames. Every test above the transport runs on that
   shape.

Not decided here, for want of evidence: a filter table entry (the vendor
builder's record is only partly reconstructed), any value/config command
after the open (which ones, unknown), `cOpenDevice` (no sign it is needed).
The audit's §6 says what settles each.

## Validation

- `IMPLEMENTED` / `FIXTURE_TESTED`: the device tests replay the second
  session's shape (acknowledgement `0x100` at once, one indication, the
  close that must skip it, the next read clean) and the first session's
  (the indication after the second transmit, during the close); a stale
  answer skipped and recorded; a late acknowledgement waited for within the
  five seconds; an acknowledgement with the firmware's words refused; an
  answer kept when the close fails; a frame on another channel counted;
  frames before board-info skipped; and the bench end to end on the new
  shape, in the crate and in the shell.
- `HARDWARE_CONFIRMED` — **the gate before the tester gets the build**: on
  the owner's bare adapter, no car, a module read must record an
  acknowledgement with status `0x100` after about three seconds, time out
  with zero inbound frames, close cleanly, and the next read must run clean
  — no `expected k, got k-1` anywhere in the session file. What the bench
  cannot show: the indication (none has been seen without a bus) and
  reception.
- `VEHICLE_CONFIRMED`: the tester's car. Either a module answers after the
  acknowledgement — and `0x100` is confirmed the acknowledgement — or the
  trace shows an acknowledged transmit, a bus the ignition has on, and zero
  inbound frames in two seconds, which is the filter question answered and
  the next work named.

## Consequences

- The meaning of `0x100` stays `HYPOTHESIS` until a module answers after
  it; the capabilities keep `NOT_YET_VALIDATED`. Nothing here is JLR proof.
- A module that does not answer now costs the read's two seconds instead of
  a few milliseconds: "Check all modules" on a car where few modules answer
  takes minutes. That is the timeout the read was designed with; it had
  never run.
- `response sequence mismatch` and `unexpected Mongoose command` no longer
  end a session; a frame that is not the answer is recorded and the wait
  goes on, bounded by its deadline.
- A read whose close failed is a success with a close error in its trace,
  where 1.2.6 reported `CAN channel close failed`.
- The probe tool and the capture read their counts from the trace.

## Built, 2026-11-13

Decisions 1 to 7 were built the day the record was accepted: 74 tests in
`mongoose-jlr`, the shell's bench run on the new shape, the workspace,
clippy, the formatting and the architecture guard clean; the preflight
passed on `fa430f3`, CI built the three installers (run 43367485173).

**The gate ran the same day**, on the owner's bare adapter: the PCM's
fault codes read twice with nothing on the OBD side. The record carries the
acknowledgement (`0x00000100`, no words), zero inbound frames, zero
indications, zero stale or dropped frames and a clean close, and the second
read ran clean. The build does what this record says.

**One claim of this record is wrong**, and the gate showed it: the
acknowledgement came after **0 ms**, not "about three seconds". The three
seconds of the bench transcript of 2026-09-08 were the probe's own
collection window; the firmware's ticks in that transcript put the
acknowledgement tens of milliseconds after the transmit, in September as
today (the audit's §10 has the arithmetic). So: the firmware acknowledges a
transmit at once, bus or no bus; `0x100` at once says nothing about the bus;
finding 3 of the context falls, and the five-second wait of decision 3
stays only as an upper bound a late acknowledgement cannot miss. What the
bare adapter did not send — any indication — the car did, once after every
acknowledgement: the one difference is a bus that completes the transmit,
which sharpens decision 2's reading of that frame as `cIndication` without
confirming it. The next session file records it and settles it.
