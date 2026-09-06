"""Which firmware resources open, and which J1962 pins each one takes.

For every resource id the route word can carry (0..31): cOpenChannel
listen-only at 125 kbit/s on route (id << 8) | 0x01; on an opened
resource, cSetPin(connect=1, 3, 11) and, if that is refused, cSetPin(1, 6,
14); then cCloseChannel. Jumps to the firmware first if the bootloader
answers board-info. Nothing is transmitted on any bus, and nothing is
written to the adapter. Bench use, no vehicle needed.

    python mongoose_resource_sweep.py COM3

The 2026-09-06 run is docs/evidence/mongoose-probe-2026-09-06/resource_sweep.txt:
5 = CAN1 (pins 6/14), 21 = CAN2 (pins 3/11), ADR-0018.
"""
import sys

import serial

from mongoose_probe import BAUD, exchange, frame

LISTEN_ONLY = 0x1000_0000


def reply(frames, command):
    """(status, text) of the first reply carrying `command`, or (None, '')."""
    for kind, payload in frames:
        if kind == "FRAME" and len(payload) >= 16 and int.from_bytes(payload[4:6], "little") == command:
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
            exchange(port, "cJumpToFirmware (bootloader answered)", frame(0x0001, 0x0103, seq, b""), wait=2.0)
            seq += 1
            exchange(port, "cGetBoardInfo after the jump", frame(0x0001, 0x0109, seq, b""))
            seq += 1
            break
    return seq


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM3"
    summary = []
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}")
        seq = jump_if_bootloader(port, 1)
        body = LISTEN_ONLY.to_bytes(4, "little") + (125_000).to_bytes(4, "little")
        for resource in range(32):
            route = (resource << 8) | 0x01
            frames = exchange(port, f"open resource {resource} (route 0x{route:04X}) listen-only 125k", frame(route, 0x0006, seq, body))
            seq += 1
            status, text = reply(frames, 0x8006)
            print(f"   -> open status {status:#010x} text={text!r}" if status is not None else "   -> no reply")
            if status != 0:
                summary.append((resource, "no", text))
                continue
            pins_taken = []
            for pins in ((3, 11), (6, 14)):
                pin_body = b"".join(value.to_bytes(4, "little") for value in (1, *pins))
                frames = exchange(port, f"cSetPin(1, {pins[0]}, {pins[1]}) on 0x{route:04X}", frame(route, 0x0012, seq, pin_body))
                seq += 1
                status, text = reply(frames, 0x8012)
                print(f"   -> setpin status {status:#010x} text={text!r}" if status is not None else "   -> no reply")
                if status == 0:
                    pins_taken.append(pins)
                    break
                pins_taken.append((pins, text))
            exchange(port, f"cCloseChannel 0x{route:04X}", frame(route, 0x0007, seq, b""))
            seq += 1
            summary.append((resource, "yes", pins_taken))
    print("\n=== summary (resource, opens, pins taken / refusal text)")
    for row in summary:
        print("   ", row)


if __name__ == "__main__":
    main()
