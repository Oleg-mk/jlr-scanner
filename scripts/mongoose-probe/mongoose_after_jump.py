"""After cJumpToFirmware: board-info, then the listen-only channel open, and
a close of whatever channel the device hands back.

    python mongoose_after_jump.py COM3
"""
import sys

import serial

from mongoose_probe import BAUD, exchange, frame


def main():
    port_name = sys.argv[1]
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}")
        seq = 1
        exchange(port, "cGetBoardInfo (0x0109) after the jump", frame(0x0001, 0x0109, seq, b""))
        seq += 1
        frames = exchange(
            port,
            "cOpenChannel (0x0006) listen-only, 500 kbit/s, unassigned channel 0xFF01",
            frame(0xFF01, 0x0006, seq, (0x1000_0000).to_bytes(4, "little") + (500_000).to_bytes(4, "little")),
        )
        seq += 1
        token = None
        for kind, payload in frames:
            if kind == "FRAME" and len(payload) >= 16:
                command = int.from_bytes(payload[4:6], "little")
                status = int.from_bytes(payload[12:16], "little")
                if command == 0x8006 and status == 0:
                    token = int.from_bytes(payload[2:4], "little")
        if token is not None:
            print(f"\n*** channel opened, token 0x{token:04X} ***")
            exchange(port, f"cCloseChannel (0x0007) on token 0x{token:04X}", frame(token, 0x0007, seq, b""))
        else:
            print("\nchannel still refused")
    print("\ndone")


if __name__ == "__main__":
    main()
