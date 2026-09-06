"""Read a USBPcap capture (.pcap, link type 249) and print the Mongoose
protocol frames that crossed the bulk pipes, host->device and device->host,
decoded the same way the probe decodes them.

    python usbpcap_frames.py capture.pcap [device-address]

USBPcap per-packet header (little-endian):
  u16 headerLen, u64 irpId, u32 status, u16 function, u8 info,
  u16 bus, u16 device, u8 endpoint, u8 transfer, u32 dataLength, [...]
transfer: 0 isochronous, 1 interrupt, 2 control, 3 bulk.
info bit 0: 1 = PDO->FDO (device to host), 0 = host to device.
"""
import struct
import sys

VERIFIER_XOR = 0x51E6


def hexs(b: bytes) -> str:
    return " ".join(f"{x:02X}" for x in b)


def describe(payload: bytes) -> str:
    if len(payload) < 12:
        return f"short payload ({len(payload)} bytes): {hexs(payload)}"
    route_a = int.from_bytes(payload[0:2], "little")
    route_b = int.from_bytes(payload[2:4], "little")
    command = int.from_bytes(payload[4:6], "little")
    sequence = int.from_bytes(payload[6:8], "little")
    reserved = payload[8:12]
    body = payload[12:]
    text = ""
    printable = bytes(c for c in body if 32 <= c < 127)
    if len(printable) >= 8 and any(bytes([c]) in body for c in printable):
        try:
            candidate = body.split(b"\x00")[0]
            if len(candidate) >= 8 and all(32 <= c < 127 for c in candidate):
                text = f' text="{candidate.decode()}"'
        except Exception:
            pass
    return (
        f"route_a=0x{route_a:04X} route_b=0x{route_b:04X} cmd=0x{command:04X} seq={sequence} "
        f"res={hexs(reserved)} body[{len(body)}]={hexs(body[:40])}{'…' if len(body) > 40 else ''}{text}"
    )


def split_frames(stream: bytes):
    """Split a direction's byte stream into outer frames; return (frames, leftover)."""
    frames = []
    i = 0
    while i + 4 <= len(stream):
        length = int.from_bytes(stream[i : i + 2], "little")
        verifier = int.from_bytes(stream[i + 2 : i + 4], "little")
        if verifier != (length ^ VERIFIER_XOR) or length == 0 or length > 0x1800:
            i += 1  # resynchronise like the DLL's parser
            continue
        if i + 4 + length > len(stream):
            break
        frames.append(stream[i + 4 : i + 4 + length])
        i += 4 + length
    return frames, stream[i:]


def main():
    path = sys.argv[1]
    wanted_device = int(sys.argv[2]) if len(sys.argv) > 2 else None
    with open(path, "rb") as handle:
        data = handle.read()
    magic = data[:4]
    if magic == b"\xd4\xc3\xb2\xa1":
        endian = "<"
    elif magic == b"\xa1\xb2\xc3\xd4":
        endian = ">"
    else:
        sys.exit(f"not a pcap file (magic {hexs(magic)}); pcapng is not supported by this reader")
    _, _, _, _, _, linktype = struct.unpack(endian + "HHiIII", data[4:24])
    if linktype != 249:
        print(f"warning: link type {linktype}, expected 249 (USBPcap)")
    pos = 24
    records = []  # (ts, direction, device, endpoint, payload)
    while pos + 16 <= len(data):
        ts_sec, ts_usec, incl_len, _ = struct.unpack(endian + "IIII", data[pos : pos + 16])
        pos += 16
        packet = data[pos : pos + incl_len]
        pos += incl_len
        if len(packet) < 27:
            continue
        header_len, _irp, _status, _function, info, _bus, device, endpoint, transfer, data_len = struct.unpack(
            "<HQIHBHHBBI", packet[:27]
        )
        if transfer != 3:  # bulk only
            continue
        payload = packet[header_len : header_len + data_len]
        if not payload:
            continue
        direction = "IN " if info & 1 else "OUT"
        records.append((ts_sec + ts_usec / 1e6, direction, device, endpoint & 0x0F, payload))
    if not records:
        sys.exit("no bulk transfers with data in the capture")
    devices = sorted({r[2] for r in records})
    print(f"bulk transfers with data: {len(records)}; USB device addresses: {devices}")
    if wanted_device is None and len(devices) == 1:
        wanted_device = devices[0]
    if wanted_device is None:
        sys.exit("several devices captured; pass the device address as the second argument")

    streams = {"OUT": b"", "IN ": b""}
    t0 = records[0][0]
    print(f"\n--- protocol frames for device {wanted_device}, in capture order ---")
    for ts, direction, device, endpoint, payload in records:
        if device != wanted_device:
            continue
        streams[direction] += payload
        frames, leftover = split_frames(streams[direction])
        streams[direction] = leftover
        for frame in frames:
            arrow = ">>>" if direction == "OUT" else "<<<"
            print(f"{ts - t0:9.3f}s {arrow} ep{endpoint} {hexs(frame[:20])}{'…' if len(frame) > 20 else ''}")
            print(f"            {describe(frame)}")


if __name__ == "__main__":
    main()
