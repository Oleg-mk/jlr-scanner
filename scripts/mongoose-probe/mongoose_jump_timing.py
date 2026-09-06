"""How long after cJumpToFirmware does the firmware answer?

board-info -> (if the bootloader answered) cJumpToFirmware -> board-info
every 50 ms until a running tick shows at body offset 4 -> then one
listen-only open on resource 5, pins 6/14, close. Prints the elapsed time.
Nothing is written to the device. Run it right after plugging the adapter
in, while it is still in the bootloader.

    python mongoose_jump_timing.py COM3
"""
import sys
import time

import serial

from mongoose_probe import BAUD, frame, read_frames

def status_of(frames, command):
    for kind, payload in frames:
        if kind == "FRAME" and len(payload) >= 16 and int.from_bytes(payload[4:6], "little") == command:
            return int.from_bytes(payload[12:16], "little"), payload
    return None, None

with serial.Serial(sys.argv[1] if len(sys.argv) > 1 else "COM3", BAUD, timeout=0.05) as port:
    seq = 1
    port.write(frame(0x0001, 0x0109, seq, b"")); seq += 1
    status, payload = status_of(read_frames(port, 0.5), 0x8109)
    if status is None:
        sys.exit("no board-info answer (port busy or adapter silent)")
    boot = payload[16:20] == b"\x00\x00\x00\x00"
    print(f"board-info status {status}, tick bytes {payload[16:20].hex()} -> {'BOOTLOADER' if boot else 'firmware already running'}")
    if boot:
        t0 = time.perf_counter()
        port.write(frame(0x0001, 0x0103, seq, b"")); seq += 1
        status, _ = status_of(read_frames(port, 1.0), 0x8103)
        t_resp = time.perf_counter() - t0
        print(f"jump response status {status} after {t_resp*1000:.0f} ms")
        for attempt in range(60):
            port.write(frame(0x0001, 0x0109, seq, b"")); seq += 1
            status, payload = status_of(read_frames(port, 0.1), 0x8109)
            if status == 0 and payload[16:20] != b"\x00\x00\x00\x00":
                print(f"firmware board-info (tick {payload[16:20].hex()}) after {(time.perf_counter()-t0)*1000:.0f} ms, attempt {attempt+1}")
                break
            time.sleep(0.05)
        else:
            sys.exit("firmware never answered board-info within ~9 s")
    route = 0x0501
    port.write(frame(route, 0x0006, seq, (0x10000000).to_bytes(4, "little") + (500000).to_bytes(4, "little"))); seq += 1
    status, payload = status_of(read_frames(port, 0.5), 0x8006)
    token = int.from_bytes(payload[2:4], "little") if payload else None
    print(f"open resource 5 listen-only: status {status}, token {token:#06x}" if token is not None else f"open: no answer")
    port.write(frame(token, 0x0012, seq, (1).to_bytes(4, "little") + (6).to_bytes(4, "little") + (14).to_bytes(4, "little"))); seq += 1
    status, _ = status_of(read_frames(port, 0.5), 0x8012)
    print(f"setpin 6/14: status {status}")
    port.write(frame(token, 0x0007, seq, b"")); seq += 1
    status, _ = status_of(read_frames(port, 0.5), 0x8007)
    print(f"close: status {status}")
