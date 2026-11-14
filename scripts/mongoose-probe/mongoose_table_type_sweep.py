"""Which table types the firmware's `cTableAddEntry` accepts: the first
probe's seven entries all drew `CanTableAdd: Unsupported or Invalid table
type` (status 0x200) for the type 7 the library's builder was read to send,
so the type word at +0x0C is swept and every answer is printed, the
firmware's own words with it.

    python mongoose_table_type_sweep.py COM4 [hs|ms] [first] [last]

Same path as the first probe: board-info, the jump if needed, cOpenChannel
without the listen-only flag, cSetPin, then one cTableAddEntry per type
with mask 0, pattern 0, word 4, byte 1, then cCloseChannel. Nothing is
transmitted on the bus.
"""
import sys

import serial

from mongoose_probe import BAUD, exchange, frame
from mongoose_table_entry_probe import RESOURCES, TABLE_ADD_ENTRY, reply_status, status_name


def entry_body(table_type: int, mask: int, pattern: int, word: int, byte: int) -> bytes:
    return (
        table_type.to_bytes(4, "little")
        + mask.to_bytes(4, "little")
        + pattern.to_bytes(4, "little")
        + word.to_bytes(2, "little")
        + byte.to_bytes(1, "little")
    )


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM4"
    bus = sys.argv[2] if len(sys.argv) > 2 else "hs"
    first = int(sys.argv[3], 0) if len(sys.argv) > 3 else 0
    last = int(sys.argv[4], 0) if len(sys.argv) > 4 else 15
    bitrate, pins = (500_000, (6, 14)) if bus == "hs" else (125_000, (3, 11))
    resource = RESOURCES[bus]
    results = []
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

        for table_type in range(first, last + 1):
            request = frame(token, TABLE_ADD_ENTRY, seq, entry_body(table_type, 0, 0, 4, 1))
            frames = exchange(port, f"cTableAddEntry type {table_type}, mask 0, pattern 0, word 4, byte 1 (seq {seq})", request, wait=0.6)
            status, text = reply_status(frames, 0x800D, seq)
            if status is None:
                print("   -> no answer")
                results.append((table_type, None, ""))
            else:
                print(f"   -> status 0x{status:08X} {status_name(status)}{(' : ' + text) if text else ''}")
                results.append((table_type, status, text))
            seq += 1

        exchange(port, f"cCloseChannel on 0x{token:04X}", frame(token, 0x0007, seq, b""))

    print("\n== summary ==")
    for table_type, status, text in results:
        shown = "no answer" if status is None else f"0x{status:08X} {status_name(status)} {text}".rstrip()
        print(f"  type {table_type:2d}: {shown}")
    print("\ndone")


if __name__ == "__main__":
    main()
