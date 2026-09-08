# Mongoose probe and capture tools (research, 2026-09-06)

Standalone tools used to learn what the MongoosePro JLR really answers,
outside the application. None of them is part of the product; none sends a
flash, unprotect, reset or serial-number command. See
`docs/evidence/F2_MONGOOSE_FIRMWARE_STATE_2026-09-06.md`.

- `mongoose_probe.py COM3 baseline|device` — board-info, then the
  application's channel open (and, with `device`, cOpenDevice/cCloseDevice),
  every reply byte printed and decoded. Needs pyserial.
- `mongoose_jump_probe.py COM3` — board-info, `cJumpToFirmware` (0x0103),
  then the channel open again. Leaves the adapter in its firmware until the
  next power cycle.
- `mongoose_after_jump.py COM3` — the "after" half on its own.
- `mongoose_listen_probe.py COM3 hs|ms [seconds]` — the application's
  listen-only path end to end: board-info, the jump if the bootloader
  answered, `cOpenChannel` on the bus's resource (hs: 5, ms: 21), `cSetPin`,
  listen, close (ADR-0018). Prints every byte and each status.
- `mongoose_resource_sweep.py COM3` — opens every resource id 0..31
  listen-only and asks each open one for pins 3/11 then 6/14; prints the
  firmware's refusal texts and a summary. The resource map of ADR-0018.
- `mongoose_jump_timing.py COM3` — right after plugging the adapter in:
  how many milliseconds after `cJumpToFirmware` the firmware answers
  board-info. Measured 2026-09-08: the first poll 241 ms after the jump
  already answered from the firmware.
- `usbpcap_capture.ps1 -Control \.\USBPcap1 -Out file.pcap -Seconds 30`
  — an elevated USBPcap capture of one root hub for a fixed time.
- `usbpcap_frames.py file.pcap [device]` — decodes the Mongoose frames in a
  USBPcap capture, both directions, in order.
- `j2534_session.ps1` — drives the vendor library (`monpj432.dll`, 32-bit,
  so run under `C:\Windows\SysWOW64\...\powershell.exe`) through
  PassThruOpen/Connect/filter/ReadMsgs/Disconnect/Close, for capturing the
  genuine sequence. Never writes to a vehicle.

The USBPcap filter attaches only when a hub starts: after installing it,
restart the root hub (`pnputil /restart-device`, elevated) or reboot.
