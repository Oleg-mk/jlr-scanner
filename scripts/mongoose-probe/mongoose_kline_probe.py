"""Can this adapter open a K-line, select one pin, and set even parity?

Every word the K-line read path sends is a hypothesis (ADR-0029, decision 6
and `crates/mongoose-jlr/src/kline.rs`). This asks the adapter itself, on the
bench, with **no vehicle**: nothing is transmitted on a vehicle bus, nothing
is written to the adapter, and the only outbound record it can send is one
DS2 identification request — sent only when `--send` is given, and refused
otherwise.

What it tries, in order, for each of resources 3 (ISO 9141) and 4 (ISO 14230):

  1. cOpenChannel diagnostic (flags 0) at the bus's baud — 9600 for DS2,
     10400 for KWP2000;
  2. cSetPin with one pin: (1, 7, 0), then (1, 7, 7), then (1, 0, 7) — the
     CAN path sends a count and two pins, and which of them a single line
     takes is exactly the open question;
  3. the parity command: command word 0x0010 with J2534's own SCONFIG shape
     (count 1, parameter 0x16, value 2 = even);
  4. the same commands with values that cannot be right — pin 99, pin 0,
     parity 99, an unknown parameter — because a firmware that takes those
     as readily as the real ones has not confirmed anything;
  5. cCloseChannel.

Every status and every status text the firmware answers with is printed, and
the summary at the end is what ADR-0029 needs to stop calling these guesses.

    python mongoose_kline_probe.py COM3
    python mongoose_kline_probe.py COM3 --send      # also sends one DS2 request

The adapter must be plugged into USB. It does not have to be plugged into a
car, and it should not be: a wrong pin selection is a wrong pin selection.
"""
import sys

import serial

from mongoose_probe import BAUD, exchange, frame, hexs

# The words the application would use, all of them hypotheses today.
RESOURCES = ((3, 9_600, "ISO 9141 / DS2"), (4, 10_400, "ISO 14230 / KWP2000"))
PIN_SHAPES = ((1, 7, 0), (1, 7, 7), (1, 0, 7), (1, 8, 0))
# 0x0010 is a handled channel command that takes a config-shaped body.
# Its neighbours are known and are not to be poked: 0x0011 is Fast Init,
# 0x0013 is cGetString, and **0x0014 is SetData, a write to the adapter** —
# this product does not write to the adapter, so nothing here sends it.
CONFIG_WORD = 0x0010
PARITY_PARAMETER = 0x16
PARITY_EVEN = 2
# One DS2 identification for node 0x72: address, length, command, XOR.
DS2_IDENTIFICATION = bytes((0x72, 0x04, 0x00, 0x76))


def reply(frames, command):
    """(status, text) of the first reply carrying `command`, or (None, '')."""
    for kind, payload in frames:
        if (
            kind == "FRAME"
            and len(payload) >= 16
            and int.from_bytes(payload[4:6], "little") == command
        ):
            status = int.from_bytes(payload[12:16], "little")
            rest = payload[20:]
            text = rest[: rest.index(0)] if 0 in rest else rest
            return status, text.decode("ascii", "replace") if status else ""
    return None, ""


def jump_if_bootloader(port, seq):
    frames = exchange(port, "cGetBoardInfo", frame(0x0001, 0x0109, seq, b""))
    seq += 1
    for kind, payload in frames:
        if kind == "FRAME" and payload[16:20] == b"\x00\x00\x00\x00":
            exchange(
                port,
                "cJumpToFirmware (bootloader answered)",
                frame(0x0001, 0x0103, seq, b""),
                wait=2.0,
            )
            seq += 1
            exchange(port, "cGetBoardInfo after the jump", frame(0x0001, 0x0109, seq, b""))
            seq += 1
            break
    return seq


def outbound_line_record(channel, sequence, payload):
    """The K-line outbound record of `crates/mongoose-jlr/src/kline.rs`: the
    CAN record without its four identifier bytes."""
    body = (
        channel.to_bytes(2, "little")
        + (0).to_bytes(2, "little")
        + (0x0008).to_bytes(2, "little")
        + sequence.to_bytes(2, "little")
        + (1).to_bytes(2, "little")
        + (0).to_bytes(2, "little")
        + (0).to_bytes(4, "little")
        + (0).to_bytes(4, "little")
        + len(payload).to_bytes(2, "little")
        + (0).to_bytes(2, "little")
        + payload
    )
    length = len(body)
    return length.to_bytes(2, "little") + (length ^ 0x51E6).to_bytes(2, "little") + body


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM3"
    send_request = "--send" in sys.argv[1:]
    summary = []
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}")
        print("no vehicle is needed for any of this; nothing is written to the adapter\n")
        seq = jump_if_bootloader(port, 1)

        for resource, baud, what in RESOURCES:
            route = (resource << 8) | 0x01
            print(f"\n=== resource {resource} (route 0x{route:04X}) — {what} at {baud} baud")
            body = (0).to_bytes(4, "little") + baud.to_bytes(4, "little")
            frames = exchange(
                port,
                f"cOpenChannel diagnostic on 0x{route:04X} at {baud}",
                frame(route, 0x0006, seq, body),
            )
            seq += 1
            status, text = reply(frames, 0x8006)
            print(
                f"   -> open status {status:#010x} text={text!r}"
                if status is not None
                else "   -> no reply"
            )
            if status != 0:
                summary.append((resource, "open refused", text))
                continue

            taken_pins = None
            for shape in PIN_SHAPES:
                pin_body = b"".join(value.to_bytes(4, "little") for value in shape)
                frames = exchange(
                    port,
                    f"cSetPin{shape} on 0x{route:04X}",
                    frame(route, 0x0012, seq, pin_body),
                )
                seq += 1
                status, text = reply(frames, 0x8012)
                print(
                    f"   -> setpin{shape} status {status:#010x} text={text!r}"
                    if status is not None
                    else "   -> no reply"
                )
                if status == 0:
                    taken_pins = shape
                    break
            summary.append((resource, "pin shape", taken_pins))

            config_body = b"".join(
                value.to_bytes(4, "little")
                for value in (1, PARITY_PARAMETER, PARITY_EVEN)
            )
            frames = exchange(
                port,
                f"parity=even through command 0x{CONFIG_WORD:04X} on 0x{route:04X}",
                frame(route, CONFIG_WORD, seq, config_body),
            )
            seq += 1
            status, text = reply(frames, CONFIG_WORD | 0x8000)
            print(
                f"   -> 0x{CONFIG_WORD:04X} status {status:#010x} text={text!r}"
                if status is not None
                else f"   -> 0x{CONFIG_WORD:04X}: no reply"
            )
            summary.append((resource, "parity command accepted", status == 0))

            # The same commands with values that cannot be right. A refusal
            # here is what makes an acceptance above mean something.
            for label, word, values in (
                ("pin 99 — no such pin", 0x0012, (1, 99, 0)),
                ("pin 0 — no pin at all", 0x0012, (1, 0, 0)),
                ("parity 99 — no such parity", CONFIG_WORD, (1, PARITY_PARAMETER, 99)),
                ("parameter 0xFFFF — no such parameter", CONFIG_WORD, (1, 0xFFFF, PARITY_EVEN)),
            ):
                nonsense = b"".join(value.to_bytes(4, "little") for value in values)
                frames = exchange(
                    port, f"refusal check: {label}", frame(route, word, seq, nonsense), wait=0.6
                )
                seq += 1
                status, text = reply(frames, word | 0x8000)
                verdict = (
                    "no reply"
                    if status is None
                    else ("TAKEN — so an acceptance proves nothing here" if status == 0 else f"refused: {text!r}")
                )
                print(f"   -> {label}: {verdict}")
                summary.append((resource, f"refusal check: {label}", verdict))

            if send_request and resource == 3:
                if not taken_pins:
                    print("   -> not sending: no pin selection was accepted")
                else:
                    print(f"   -> sending one DS2 identification: {hexs(DS2_IDENTIFICATION)}")
                    port.write(outbound_line_record(route, seq, DS2_IDENTIFICATION))
                    seq += 1
                    from mongoose_probe import read_frames

                    for kind, payload in read_frames(port, 1.0):
                        print(f"      {kind} {hexs(payload)}")

            exchange(port, f"cCloseChannel 0x{route:04X}", frame(route, 0x0007, seq, b""))
            seq += 1

    print("\n=== summary — what the adapter answered")
    for row in summary:
        print("   ", row)
    print(
        "\nA resource that opens, a pin shape that is accepted and a parity command that answers"
        "\nare the three things ADR-0029 needs before a K-line read stops being a guess."
    )


if __name__ == "__main__":
    main()
