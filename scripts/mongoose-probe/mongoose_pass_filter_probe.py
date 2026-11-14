"""One pass-everything filter on the bare adapter, in the record the vendor
library's own PASS/BLOCK filter path builds (read statically, 2026-11-14):

    route (the channel token), 0, 0x000D, sequence, 0, 0,
    u32 table subtype (0 = PASS, 1 = BLOCK),
    u16 flags (bit 0x0100 for 29-bit identifiers),
    u8 rule type (1 = PASS, 2 = BLOCK),
    u8 pattern length,
    mask bytes, then pattern bytes (big-endian identifier words, as J2534 Data[0..3])

Twenty-eight bytes of payload for a four-byte mask and pattern. The bench of
the day before had reached this handler with table subtypes 0 and 1 and been
refused on the rule-type byte ("FilterAdd: Invalid Filter type"), which is
where this layout puts it.

    python mongoose_pass_filter_probe.py COM4 [hs|ms] [seconds]

Board-info, the jump if the bootloader answered, cOpenChannel without the
listen-only flag, cSetPin, exactly one cTableAddEntry (PASS, mask 0, pattern
0, 11-bit), a listen of `seconds`, cCloseChannel. Nothing is transmitted on
the bus.
"""
import sys
import time

import serial

from mongoose_probe import BAUD, describe, exchange, frame, hexs, read_frames
from mongoose_table_entry_probe import RESOURCES, TABLE_ADD_ENTRY, reply_status, status_name

PASS_SUBTYPE = 0
PASS_RULE = 1


def pass_filter_body(mask: bytes, pattern: bytes, extended: bool) -> bytes:
    flags = 0x0100 if extended else 0
    return (
        PASS_SUBTYPE.to_bytes(4, "little")
        + flags.to_bytes(2, "little")
        + PASS_RULE.to_bytes(1, "little")
        + len(pattern).to_bytes(1, "little")
        + mask
        + pattern
    )


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM4"
    bus = sys.argv[2] if len(sys.argv) > 2 else "hs"
    seconds = float(sys.argv[3]) if len(sys.argv) > 3 else 3.0
    bitrate, pins = (500_000, (6, 14)) if bus == "hs" else (125_000, (3, 11))
    resource = RESOURCES[bus]
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}; bus {bus}: {bitrate} bit/s, pins {pins}")
        seq = 1
        frames = exchange(port, "cGetBoardInfo", frame(0x0001, 0x0109, seq, b""))
        seq += 1
        in_bootloader = any(
            kind == "FRAME" and payload[12 + 4 : 12 + 8] == b"\x00\x00\x00\x00" for kind, payload in frames
        )
        if in_bootloader:
            exchange(port, "cJumpToFirmware (bootloader answered)", frame(0x0001, 0x0103, seq, b""), wait=2.0)
            seq += 1
        route = (resource << 8) | 0x01
        body = (0).to_bytes(4, "little") + bitrate.to_bytes(4, "little")
        frames = exchange(port, f"cOpenChannel (no listen-only flag), route 0x{route:04X}, {bitrate} bit/s", frame(route, 0x0006, seq, body))
        seq += 1
        token = None
        for kind, payload in frames:
            if kind == "FRAME" and len(payload) >= 16 and int.from_bytes(payload[4:6], "little") == 0x8006 and int.from_bytes(payload[12:16], "little") == 0:
                token = int.from_bytes(payload[2:4], "little")
        if token is None:
            print("channel did not open; stopping")
            return
        pin_body = (1).to_bytes(4, "little") + pins[0].to_bytes(4, "little") + pins[1].to_bytes(4, "little")
        exchange(port, f"cSetPin(connect=1, {pins[0]}, {pins[1]}) on token 0x{token:04X}", frame(token, 0x0012, seq, pin_body))
        seq += 1

        request = frame(token, TABLE_ADD_ENTRY, seq, pass_filter_body(bytes(4), bytes(4), extended=False))
        frames = exchange(port, f"cTableAddEntry PASS, 11-bit, mask 00 00 00 00, pattern 00 00 00 00 (seq {seq})", request)
        status, text = reply_status(frames, 0x800D, seq)
        if status is None:
            print("   -> no answer to the entry")
        else:
            print(f"   -> status 0x{status:08X} {status_name(status)}{(' : ' + text) if text else ''}")
            # Whatever follows the status and the tick in an accepted answer:
            # the entry's own word, by the look of it.
            for kind, payload in frames:
                if kind == "FRAME" and len(payload) > 20 and int.from_bytes(payload[6:8], "little") == seq:
                    print(f"   -> answer body after status and tick: {hexs(payload[20:])}")
        seq += 1

        print(f"\n>>> listening {seconds:.0f}s for cInboundData (0x0009) on token 0x{token:04X}")
        deadline = time.monotonic() + seconds
        count = 0
        others = 0
        while time.monotonic() < deadline:
            for kind, payload in read_frames(port, 0.5):
                if kind != "FRAME":
                    continue
                if int.from_bytes(payload[4:6], "little") == 0x0009:
                    count += 1
                    if count <= 10:
                        print(f"    RX {hexs(payload[:40])}")
                        print(f"       {describe(payload)}")
                else:
                    others += 1
                    print(f"    RX other {hexs(payload[:40])}")
                    print(f"       {describe(payload)}")
        print(f"    frames received: {count}; other records: {others}")
        exchange(port, f"cCloseChannel on 0x{token:04X}", frame(token, 0x0007, seq, b""))
    print("\ndone")


if __name__ == "__main__":
    main()
