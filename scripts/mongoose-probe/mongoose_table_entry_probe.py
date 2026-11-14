"""The filter table entry (`cTableAddEntry`, 0x000D), tried on the bare
adapter, candidate by candidate, until the firmware accepts one.

Why: on the first tester's car (2026-11-13) every transmit was queued and
completed (`0x100` eMsgQueued, `0x106` iMsgTxDone) and the firmware handed
the host no received frame on either bus, not even to a listen-only
channel. The sister product's client states the rule for its firmware: a
pass filter per identifier, or you receive nothing. The record's layout was
read from the vendor library's own builder (2026-11-14): 27 bytes - the
channel word, zero, 0x000D, the sequence, zero, zero, a table subtype of 7,
then four fields of the filter object: two 32-bit words (mask and pattern,
both zero for "pass everything"), a 16-bit word and a byte whose meaning the
code did not give up. The firmware refuses a malformed entry with a named
status (eInvalidEntry 0x201, eInvalidValue 0x202, eInvalidMsgLength 0x203),
so the candidates for those two fields are tried in turn and the first one
answered with status 0 is the record.

    python mongoose_table_entry_probe.py COM4 [hs|ms] [seconds]

What it sends: board-info; the jump to firmware if the bootloader answered;
cOpenChannel on the bus's resource without the listen-only flag (as a read
does); cSetPin; then the candidate entries one at a time, stopping at the
first accepted; then it listens `seconds` for cInboundData (none is
expected with no car attached), and closes the channel. No frame is ever
transmitted on the bus: this is table configuration of the adapter.
"""
import sys
import time

import serial

from mongoose_probe import BAUD, describe, exchange, frame, hexs, read_frames

RESOURCES = {"hs": 5, "ms": 21}

TABLE_ADD_ENTRY = 0x000D
TABLE_SUBTYPE = 7

# The firmware's status words, from the vendor library's own table
# (docs/evidence/MONGOOSE_STATUS_WORDS_2026-11-14.md).
STATUS_NAMES = {
    0x000: "eSuccess", 0x001: "eNotSupported", 0x002: "eInvalidChannelId", 0x003: "eInvalidProtocolID",
    0x004: "eNullParameter", 0x005: "eInvalidIoctlValue", 0x006: "eInvalidFlags", 0x007: "eFailed",
    0x008: "eDeviceNotConnected", 0x009: "eTimeout", 0x00A: "eInvalidMsg", 0x00B: "eInvalidTimeInterval",
    0x00C: "eExceededLimit", 0x00D: "eInvalidMsgId", 0x00E: "eDeviceInUse", 0x00F: "eInvalidIoctlId",
    0x010: "eBufferEmpty", 0x011: "eBufferFull", 0x012: "eBufferOverflow", 0x013: "ePinInvalid",
    0x014: "eChannelInUse", 0x015: "eMsgProtocolId", 0x016: "eInvalidFilterId", 0x017: "eNoFlowControl",
    0x018: "eNotUnique", 0x019: "eInvalidBaudRate", 0x01A: "eInvalidDeviceId",
    0x100: "eMsgQueued", 0x101: "eMsgDiscardedQueueFull", 0x102: "iMsgDiscardedMsgError",
    0x103: "iMsgDiscardedTimeout", 0x104: "iMsgDiscardedNoAck", 0x105: "iMsgDiscardedBusFault",
    0x106: "iMsgTxDone", 0x107: "iMsgRxStart", 0x108: "iMsgRxBreakStart", 0x109: "iMsgRxBreakStop",
    0x10A: "iMsgTxDonePeriodic", 0x10C: "eMsgDiscardedNoFlowFilter", 0x10D: "iMsgTxFCTimeout",
    0x10E: "iMsgTxDoneFC", 0x10F: "iMsgTxDoneSegment", 0x110: "iMsgRxCFTimeout", 0x111: "iMsgRxCFInvalid",
    0x112: "iMsgTxFFTimeout", 0x113: "iMsgTxCFTimeout", 0x114: "iMsgTxSFTimeout", 0x115: "iMsgRxFCTimeout",
    0x116: "iSingleWireChangeHS", 0x117: "iSingleWireChangeNS", 0x118: "iMsgTimeout", 0x119: "iMsgRxQueueOvflw",
    0x11A: "eInvalidEntry", 0x11B: "eInvalidValue", 0x11C: "eInvalidMsgLength", 0x11D: "eOutOfBuffers",
    0x11E: "ePinConflict", 0x11F: "ePinNotSupported", 0x120: "eQueueFull", 0x121: "eInitTimeout",
    0x201: "eInvalidEntry", 0x202: "eInvalidValue", 0x203: "eInvalidMsgLength", 0x204: "eOutOfBuffers",
    0x205: "ePinConflict", 0x206: "ePinNotSupported", 0x207: "eQueueFull", 0x208: "eInitTimeout",
    0x209: "eFiveBaudInitFailed", 0x20A: "eVbattLoss",
}

# (16-bit word, byte) candidates for the two unknown fields of a pass-all
# entry; the library's sister product carries a length and a kind in its
# filter record, and J2534 names PASS_FILTER 1.
CANDIDATES = [(4, 1), (1, 4), (0, 1), (1, 0), (4, 0), (0, 4), (0, 0)]


def status_name(status: int) -> str:
    return STATUS_NAMES.get(status, "unknown")


def entry_body(mask: int, pattern: int, word: int, byte: int) -> bytes:
    return (
        TABLE_SUBTYPE.to_bytes(4, "little")
        + mask.to_bytes(4, "little")
        + pattern.to_bytes(4, "little")
        + word.to_bytes(2, "little")
        + byte.to_bytes(1, "little")
    )


def reply_status(frames, response_command: int, sequence: int):
    """The status word of the answer to `sequence`, and the text after it."""
    for kind, payload in frames:
        if kind != "FRAME" or len(payload) < 16:
            continue
        if int.from_bytes(payload[4:6], "little") != response_command:
            continue
        if int.from_bytes(payload[6:8], "little") != sequence:
            continue
        status = int.from_bytes(payload[12:16], "little")
        # The firmware's words follow the status and the tick; anything
        # else there (an accepted entry's own word, say) is not text.
        tail = payload[20:].split(b"\0", 1)[0] if len(payload) > 20 else b""
        text = tail.decode("ascii") if tail and all(0x20 <= b < 0x7F for b in tail) else ""
        return status, text
    return None, ""


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

        accepted = None
        for word, byte in CANDIDATES:
            request = frame(token, TABLE_ADD_ENTRY, seq, entry_body(0, 0, word, byte))
            frames = exchange(port, f"cTableAddEntry subtype {TABLE_SUBTYPE}, mask 0, pattern 0, word {word}, byte {byte} (seq {seq})", request)
            status, text = reply_status(frames, 0x800D, seq)
            if status is None:
                print("   -> no answer to this entry")
            else:
                print(f"   -> status 0x{status:08X} {status_name(status)}{(' : ' + text) if text else ''}")
            seq += 1
            if status == 0:
                accepted = (word, byte)
                break
        if accepted is None:
            print("\nno candidate was accepted")
        else:
            print(f"\naccepted: word {accepted[0]}, byte {accepted[1]}")

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
