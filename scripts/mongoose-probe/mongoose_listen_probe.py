"""The listen-only capture path end to end, as the application should do it:

  board-info -> (cJumpToFirmware if the bootloader answers) ->
  cOpenChannel on the bus's resource (hs: resource 5 / route 0x0501, 500k,
  pins 6/14; ms: resource 21 / route 0x1501, 125k, pins 3/11; ADR-0018),
  listen-only -> cSetPin(connect=1, pins) on the returned token -> read for
  a few seconds -> cCloseChannel.

    python mongoose_listen_probe.py COM3 [hs|ms] [seconds]

With no car attached the bus is silent and no cInboundData arrives; what
matters is the status of every command.
"""
import sys
import time

import serial

from mongoose_probe import BAUD, describe, exchange, frame, hexs, read_frames

RESOURCES = {"hs": 5, "ms": 21}


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM3"
    bus = sys.argv[2] if len(sys.argv) > 2 else "hs"
    seconds = float(sys.argv[3]) if len(sys.argv) > 3 else 3.0
    bitrate, pins = (500_000, (6, 14)) if bus == "hs" else (125_000, (3, 11))
    resource = RESOURCES[bus]
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}; bus {bus}: {bitrate} bit/s, pins {pins}")
        seq = 1
        frames = exchange(port, "cGetBoardInfo", frame(0x0001, 0x0109, seq, b""))
        seq += 1
        in_bootloader = any(kind == "FRAME" and payload[12 + 4 : 12 + 8] == b"\x00\x00\x00\x00" for kind, payload in frames)
        if in_bootloader:
            exchange(port, "cJumpToFirmware (bootloader answered)", frame(0x0001, 0x0103, seq, b""), wait=2.0)
            seq += 1
        route = (resource << 8) | 0x01
        body = (0x1000_0000).to_bytes(4, "little") + bitrate.to_bytes(4, "little")
        frames = exchange(port, f"cOpenChannel listen-only, route 0x{route:04X}, {bitrate} bit/s", frame(route, 0x0006, seq, body))
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
        print(f"\n>>> listening {seconds:.0f}s for cInboundData (0x0009) on token 0x{token:04X}")
        deadline = time.monotonic() + seconds
        count = 0
        while time.monotonic() < deadline:
            for kind, payload in read_frames(port, 0.5):
                if kind == "FRAME":
                    count += 1
                    if count <= 10:
                        print(f"    RX {hexs(payload[:40])}")
                        print(f"       {describe(payload)}")
        print(f"    frames received: {count}")
        exchange(port, f"cCloseChannel on 0x{token:04X}", frame(token, 0x0007, seq, b""))
    print("\ndone")


if __name__ == "__main__":
    main()
