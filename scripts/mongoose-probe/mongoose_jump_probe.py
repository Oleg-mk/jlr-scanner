"""Leave the bootloader: cJumpToFirmware, then see whether channel commands live.

    python mongoose_jump_probe.py COM3

Steps, each printed byte for byte:
  1. board-info (bootloader answers; versions 1.1.8 / 1.1.16 visible)
  2. cJumpToFirmware — command word 0x0103, wire bytes `03 01`, no body,
     the simple 12-byte builder the driver uses
  3. wait; list serial ports again (the device may re-enumerate)
  4. reopen; board-info again; cOpenChannel listen-only 500 kbit/s;
     if a channel token comes back, cCloseChannel on it
Nothing is written to the adapter's flash: no reflash, no unprotect, no
serial-number command exists in this script.
"""
import sys
import time

import serial
from serial.tools import list_ports

from mongoose_probe import BAUD, describe, exchange, frame, hexs


def ports():
    return [(p.device, p.hwid) for p in list_ports.comports()]


def find_mongoose():
    for device, hwid in ports():
        if "18E1" in hwid.upper():
            return device
    return None


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else find_mongoose()
    if not port_name:
        print("no Mongoose (VID 18E1) port found")
        sys.exit(1)
    print("ports before:", ports())
    seq = 1
    with serial.Serial(port_name, BAUD, timeout=0.1) as port:
        print(f"port {port_name} open at {BAUD}")
        exchange(port, "cGetBoardInfo (0x0109) in the current state", frame(0x0001, 0x0109, seq, b""))
        seq += 1
        exchange(port, "cJumpToFirmware (0x0103, wire 03 01), no body", frame(0x0001, 0x0103, seq, b""), wait=2.0)
        seq += 1

    for i in range(10):
        time.sleep(0.5)
        current = ports()
        found = find_mongoose()
        if found:
            print(f"after {0.5 * (i + 1):.1f}s ports: {current}")
            break
    else:
        print("adapter did not come back within 5 s; ports:", ports())
        sys.exit(1)

    time.sleep(1.0)
    with serial.Serial(found, BAUD, timeout=0.1) as port:
        print(f"\nreopened {found}")
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
            print("\nchannel still refused after the jump")
    print("\ndone")


if __name__ == "__main__":
    main()
