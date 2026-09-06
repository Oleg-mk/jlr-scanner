"""Direct probe of the MongoosePro JLR over its serial port, byte for byte.

Sends exactly the frames the application sends (board-info, then the
listen-only cOpenChannel the capture uses), prints every byte that comes
back, and decodes the command word, sequence and status. Step "device"
sends cOpenDevice (0x0003, flag 0x00) first — only when asked for on the
command line, never by default.

    python mongoose_probe.py COM3 baseline      # board-info + cOpenChannel
    python mongoose_probe.py COM3 device        # board-info + cOpenDevice + cOpenChannel
"""
import sys
import time

import serial

VERIFIER_XOR = 0x51E6
BAUD = 115_200


def frame(route_a: int, command: int, sequence: int, body: bytes, route_b: int = 0) -> bytes:
    payload = (
        route_a.to_bytes(2, "little")
        + route_b.to_bytes(2, "little")
        + command.to_bytes(2, "little")
        + sequence.to_bytes(2, "little")
        + b"\x00\x00\x00\x00"
        + body
    )
    length = len(payload)
    return length.to_bytes(2, "little") + (length ^ VERIFIER_XOR).to_bytes(2, "little") + payload


def hexs(b: bytes) -> str:
    return " ".join(f"{x:02X}" for x in b)


def read_frames(port: serial.Serial, seconds: float):
    """Collect bytes for `seconds`, split into frames by the length header."""
    deadline = time.monotonic() + seconds
    buffer = b""
    while time.monotonic() < deadline:
        chunk = port.read(512)
        if chunk:
            buffer += chunk
        else:
            time.sleep(0.02)
    frames = []
    while len(buffer) >= 4:
        length = int.from_bytes(buffer[0:2], "little")
        verifier = int.from_bytes(buffer[2:4], "little")
        ok = verifier == (length ^ VERIFIER_XOR)
        if not ok or len(buffer) < 4 + length:
            frames.append(("RAW/UNFRAMED", buffer))
            buffer = b""
            break
        payload = buffer[4 : 4 + length]
        buffer = buffer[4 + length :]
        frames.append(("FRAME", payload))
    return frames


def describe(payload: bytes) -> str:
    if len(payload) < 12:
        return f"short payload ({len(payload)} bytes)"
    route_a = int.from_bytes(payload[0:2], "little")
    route_b = int.from_bytes(payload[2:4], "little")
    command = int.from_bytes(payload[4:6], "little")
    sequence = int.from_bytes(payload[6:8], "little")
    body = payload[12:]
    status = int.from_bytes(body[0:4], "little") if len(body) >= 4 else None
    return (
        f"route_a=0x{route_a:04X} route_b=0x{route_b:04X} command=0x{command:04X} "
        f"seq={sequence} body[{len(body)}]={hexs(body[:24])}{'…' if len(body) > 24 else ''}"
        + (f" | status=0x{status:08X}" if status is not None else "")
    )


def exchange(port: serial.Serial, label: str, request: bytes, wait: float = 1.0):
    print(f"\n>>> {label}")
    print(f"    TX {hexs(request)}")
    port.reset_input_buffer()
    port.write(request)
    port.flush()
    frames = read_frames(port, wait)
    if not frames:
        print("    RX (nothing within %.1fs)" % wait)
    for kind, data in frames:
        print(f"    RX {kind} {hexs(data)}")
        if kind == "FRAME":
            print(f"       {describe(data)}")
    return frames


def main():
    if len(sys.argv) < 3 or sys.argv[2] not in ("baseline", "device"):
        print(__doc__)
        sys.exit(2)
    port_name, mode = sys.argv[1], sys.argv[2]
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}")
        seq = 1
        exchange(port, "cGetBoardInfo (0x0109), as the application sends it",
                 frame(0x0001, 0x0109, seq, b""))
        seq += 1
        if mode == "device":
            exchange(port, "cOpenDevice (0x0003, flag 0x00), body DA 6B (the family's known open body)",
                     frame(0x0001, 0x0003, seq, bytes([0xDA, 0x6B])))
            seq += 1
        exchange(port, "cOpenChannel (0x0006) listen-only, 500 kbit/s, unassigned channel 0xFF01",
                 frame(0xFF01, 0x0006, seq, (0x1000_0000).to_bytes(4, "little") + (500_000).to_bytes(4, "little")))
        seq += 1
        if mode == "device":
            exchange(port, "cCloseDevice (0x0005, flag 0x00), body 49 4C",
                     frame(0x0001, 0x0005, seq, bytes([0x49, 0x4C])))
    print("\ndone")


if __name__ == "__main__":
    main()
