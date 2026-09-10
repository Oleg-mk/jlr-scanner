"""How fast is the link to the adapter itself? (ADR-0022, decision 3)

The bench measures the software's ceiling; this measures the floor the
hardware adds, with no car attached. Two things, both device-local and
transmitting nothing on any vehicle bus:

  1. board-info round trips (0x0109 -> 0x8109), one after another, timed
     individually: the USB/serial cost of one request and one answer, which
     every read pays at least twice (request out, answer in);
  2. open resource 5 listen-only -> set pins 6/14 -> close, timed as a unit:
     what a read pays if the route is opened and closed around it, as the
     product does today for a single read.

Nothing is written to the device; a listen-only open sends no frame on the
bus. Run with the adapter plugged in:

    python mongoose_link_cadence.py COM3
"""
import statistics
import sys
import time

import serial

from mongoose_probe import BAUD, frame

VERIFIER_XOR = 0x51E6


def wait_frame(port: serial.Serial, command: int, timeout: float):
    """Read until one complete frame carrying `command` arrives, or time out.
    Returns the payload, or None."""
    deadline = time.perf_counter() + timeout
    buffer = b""
    while time.perf_counter() < deadline:
        chunk = port.read(512)
        if chunk:
            buffer += chunk
        while len(buffer) >= 4:
            length = int.from_bytes(buffer[0:2], "little")
            verifier = int.from_bytes(buffer[2:4], "little")
            if verifier != (length ^ VERIFIER_XOR):
                buffer = buffer[1:]
                continue
            if len(buffer) < 4 + length:
                break
            payload = buffer[4 : 4 + length]
            buffer = buffer[4 + length :]
            if len(payload) >= 6 and int.from_bytes(payload[4:6], "little") == command:
                return payload
    return None


def status_of(payload):
    return int.from_bytes(payload[12:16], "little") if payload and len(payload) >= 16 else None


def summarise(label, samples_ms):
    if not samples_ms:
        print(f"{label}: no samples")
        return
    s = sorted(samples_ms)
    p95 = s[min(len(s) - 1, int(round(0.95 * (len(s) - 1))))]
    print(
        f"{label}: n={len(s)}  min {s[0]:.1f}  median {statistics.median(s):.1f}  "
        f"mean {statistics.fmean(s):.1f}  p95 {p95:.1f}  max {s[-1]:.1f} ms  "
        f"-> {1000.0 / statistics.fmean(s):.1f} per second"
    )


def main():
    port_name = sys.argv[1] if len(sys.argv) > 1 else "COM3"
    with serial.Serial(port_name, BAUD, timeout=0.01) as port:
        seq = 1

        def next_seq():
            nonlocal seq
            value = seq
            seq = seq % 250 + 1
            return value

        # Firmware first, as the application does (ADR-0018).
        port.reset_input_buffer()
        port.write(frame(0x0001, 0x0109, next_seq(), b""))
        payload = wait_frame(port, 0x8109, 1.0)
        if payload is None:
            sys.exit("no board-info answer: port busy, or the adapter is silent")
        in_bootloader = payload[16:20] == b"\x00\x00\x00\x00"
        print(f"board-info status {status_of(payload)}: {'bootloader' if in_bootloader else 'firmware running'}")
        if in_bootloader:
            t0 = time.perf_counter()
            port.write(frame(0x0001, 0x0103, next_seq(), b""))
            wait_frame(port, 0x8103, 1.0)
            for _ in range(60):
                port.write(frame(0x0001, 0x0109, next_seq(), b""))
                payload = wait_frame(port, 0x8109, 0.2)
                if payload is not None and status_of(payload) == 0 and payload[16:20] != b"\x00\x00\x00\x00":
                    print(f"firmware answered {1000 * (time.perf_counter() - t0):.0f} ms after the jump")
                    break
                time.sleep(0.05)
            else:
                sys.exit("the firmware never answered board-info")

        # 1. Board-info round trips, one at a time, for five seconds.
        samples = []
        misses = 0
        end = time.perf_counter() + 5.0
        while time.perf_counter() < end:
            t0 = time.perf_counter()
            port.write(frame(0x0001, 0x0109, next_seq(), b""))
            payload = wait_frame(port, 0x8109, 1.0)
            if payload is None:
                misses += 1
                continue
            samples.append(1000 * (time.perf_counter() - t0))
        summarise("board-info round trip", samples)
        if misses:
            print(f"  ({misses} request(s) went unanswered within 1 s)")

        # 2. Open listen-only on resource 5, set pins 6/14, close — twenty times.
        cycles = []
        failures = 0
        for _ in range(20):
            t0 = time.perf_counter()
            port.write(frame(0x0501, 0x0006, next_seq(), (0x10000000).to_bytes(4, "little") + (500000).to_bytes(4, "little")))
            opened = wait_frame(port, 0x8006, 1.0)
            token = int.from_bytes(opened[2:4], "little") if opened else None
            if token is None or status_of(opened) != 0:
                failures += 1
                continue
            port.write(frame(token, 0x0012, next_seq(), (1).to_bytes(4, "little") + (6).to_bytes(4, "little") + (14).to_bytes(4, "little")))
            pins = wait_frame(port, 0x8012, 1.0)
            port.write(frame(token, 0x0007, next_seq(), b""))
            closed = wait_frame(port, 0x8007, 1.0)
            if pins is None or closed is None:
                failures += 1
                continue
            cycles.append(1000 * (time.perf_counter() - t0))
        summarise("open + pins + close (listen-only, resource 5)", cycles)
        if failures:
            print(f"  ({failures} cycle(s) failed)")


if __name__ == "__main__":
    main()
