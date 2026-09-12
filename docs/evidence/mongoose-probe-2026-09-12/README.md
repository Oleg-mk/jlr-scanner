# The adapter answers about its K-line — 2026-09-12

`scripts/mongoose-probe/mongoose_kline_probe.py COM3` against a MongoosePro
JLR on USB, **no vehicle attached**. Transcript: `kline_probe.txt`.

What was being asked: `ADR-0029` decision 6 wrote four words of the K-line
path as hypotheses — the resource, the pin selection, the byte framing and
the outbound record — and said only the adapter could answer them.

## What the adapter said

| what | answer |
| --- | --- |
| resource 3 (route `0x0301`) opened at 9600 baud | **yes**, status 0 |
| resource 4 (route `0x0401`) opened at 10400 baud | **yes**, status 0 |
| `cSetPin(1, 7, 0)` on both | **yes**, status 0 |
| the same with pin 99, and with pin 0 | **refused**: *"MongoosePro JLR board supports ISO9141 K line on pin 3, 7 or 8"* |
| parity through command `0x0010` (count 1, parameter `0x16`, value 2) | status 0 |
| the same with parity 99, and with parameter `0xFFFF` | **also status 0** |

The refusals are what make the acceptances worth something. The firmware
validates a pin — it names the three pins it will take — so **pin 7 on
resource 3 and on resource 4 is confirmed**, and the shape of the command is
a count and the pin in the first of the two pin fields. It does **not**
validate a config parameter: `0x0010` takes a parity of 99 and a parameter
that does not exist as readily as the real ones, so its acceptance says only
that the command is handled, not that it set anything.

## Command words this run named, from the firmware's own texts

| word | what the firmware called it |
| --- | --- |
| `0x0010` | handled, takes a config-shaped body without validating it |
| `0x0011` | **Fast Init** — *"Fast Init: Timeout on response"*, which is what an ISO 14230 fast init does with no vehicle on the line |
| `0x0013` | `cGetString` — *"cGetString: Unknown String Identifier 771"* |
| `0x0014` | `SetData` — *"SetData: Write FAILED"*; **a write to the adapter, and this product does not send it** |
| `0x0016`, `0x0017`, `0x0018`, `0x00FF` | *"ProcessCommand: Unknown/Unhandled Command N"* — no such command |

`0x0011` is the wake-up ISO 14230 asks for, and the firmware does it itself:
`kw2000_fast` in the platform documents is a command this adapter has, not
something this product has to time on a wire.

**A note on how this was found, for the record.** The sweep that named these
words sent `0x0014` once, before anything knew what it was; the firmware
answered *"SetData: Write FAILED"* and nothing was written. The adapter was
checked afterwards — board info, CAN1 open on pins 6/14, close, all normal.
`0x0014` is now named in the probe script as a word never to send.

## The outbound record

The record `crates/mongoose-jlr/src/kline.rs` writes — the CAN record without
its four identifier bytes — was accepted structurally by the firmware and
failed at the line (`0x8008` status `0x100`, then an event `0x000A` status
`0x105`), which is what a line with nothing on it should do. Whether the
bytes reach a module in the right framing is a question only a module can
answer.

## What this leaves open

- **The parity command.** Both protocols this product speaks on a K-line ask
  for even parity — DS2 at 9600 8E1, KWP2000 at 10400 8E1 — and `0x0010` is
  the best candidate: a handled channel command that takes J2534's own
  `SCONFIG` shape. It is not confirmed, and cannot be from the bench: the
  firmware accepts nonsense on it. The first L322 body module that answers
  confirms it; silence with parity set and silence without it refutes it.
- **The inbound record.** Nothing has come back on a K-line yet, so the shape
  this product reads is still the CAN record's, minus the identifier.
