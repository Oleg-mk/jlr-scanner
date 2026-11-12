# ADR-0041: The cable on SDD's own driver — a second Windows transport, through the MongoosePro package's kernel driver

- Status: **Accepted, 2026-11-12**, on the owner's word the same morning
  («правимо та кодимо по плану»), proposed the day before on his word to
  write it («так, роби ADR-0041»), the day the first tester from the forum
  reported that the application did not detect his MongoosePro on the laptop
  that runs his SDD.
- Scope: how the application finds and opens the MongoosePro JLR on
  Windows. The adapter protocol (`mongoose-jlr`), everything above it, the
  safety classes and the macOS path are not touched.

## Context

### What happened

On 2026-11-11 the first tester ran build 1.2.4 (32-bit) on the 32-bit
Windows 10 laptop his SDD runs on: "won't detect my Mongoose adapter". On
2026-11-12 he confirmed what Device Manager showed with the cable plugged in:
**"MongoosePro JLR J2534 Interface" under "Vehicle PassThru Interfaces"**. He
then handed the cable to Windows's serial driver the way the guide of the day
before describes ("Update driver", "Let me pick", "USB Serial Device"), and
the application detected it. For SDD he has to hand it back the same way.

### Why: a cable belongs to one driver at a time

How the application discovers today — `transport-serial`,
`windows_discovery.rs`: it walks `HKLM\SYSTEM\CurrentControlSet\Enum\USB`,
keeps a device only if it is present, has a `PortName` and that port is one
`GetCommPorts` reports, and opens the one whose identity is `18E1:0104`. A
cable bound to a driver that creates no COM port is invisible to it.

What SDD's MongoosePro package installs — read from the package's INF as it
is installed on the owner's own PC and recorded in
`docs/research/mongoose/MONGOOSE_REVERSE_ENGINEERING.md`: `monpjaguar.inf`
(Drew Technologies, DriverVer 09/26/2012 1.1.0.0) binds the whole
`USB\VID_18E1&PID_0104`, for x86 and for AMD64, to its own kernel driver
`dtmonpro.sys` (KMDF 1.9), device class `VehiclePassThru`
`{FB1CF0C4-B412-451F-9F04-DF7537A5003C}`, shown as "MongoosePro JLR J2534
Interface". No `usbser`, no COM port.

What SDD's own library does with it — `monpj432.dll`, the J2534 DLL, as the
research of 2026-08 recorded it (`MONGOOSE_REVERSE_ENGINEERING.md`,
`MONGOOSE_USB_PROTOCOL.md`, `MONGOOSE_IOCTL_MAP.md`, all static, nothing
invoked): it finds the device with `SetupDiGetClassDevsW` and
`SetupDiEnumDeviceInterfaces` by the **device interface GUID
`{F99F6FCE-03F4-4817-8A03-8CFDE4743B92}`** that `dtmonpro.sys` creates
(CONFIRMED: the GUID bytes sit in both binaries, the driver's `EvtDeviceAdd`
calls `WdfDeviceCreateDeviceInterface` with it and a NULL reference string);
opens it with `CreateFileW`; sends its command frames with overlapped
`WriteFile` and receives with `ReadFile`; and of the driver's five IOCTLs
calls exactly one, `0x5500601C`, which returns USB string descriptor 3, the
serial number. It never opens a COM port. The driver itself carries no
protocol: the positively identified code creates the device interface, picks
the USB pipes, moves `ReadFile`/`WriteFile` buffers to and from the bulk
pipes and serves the five IOCTLs (STRONG INFERENCE, `MONGOOSE_REVERSE_
ENGINEERING.md`); its read and write queues are sequential.

So the stream the DLL sends through `dtmonpro.sys` is the same framed stream
this application already sends through `usbser` — the framing was derived
from that DLL — and the firmware cannot tell which host driver carried the
bytes. Over `usbser` the exchange is hardware-confirmed since F1/F3: the
board-information command answered `0x8109`, 168 bytes, on the owner's PC.

### Why it never showed in development

The owner's Windows 11 PC has the same package installed (`oem90.inf` is
`monpjaguar.inf`), and Windows nevertheless bound the inbox `usbser` to the
cable (registry: service `usbser`, "USB Serial Device", `COM3`). Why is not
known: Microsoft's page on how Windows ranks driver packages (read
2026-11-11) gives WHQL and Authenticode signatures the same best signature
score and ranks a hardware-ID match above a compatible-ID match, which would
favour the package's INF. It was not investigated further. What matters is
the tester's machine: SDD works there, SDD's DLL never opens a COM port, so
on every PC where SDD works the cable is on `dtmonpro.sys` — and that is the
whole tester audience.

Until this record, the guide's answer was a Device Manager ritual before and
after every session. The product exists to take SDD's install misery away,
not to add a step to it.

## Decision

1. **A second transport crate, `transport-dtmonpro`**, Windows-only,
   implementing `transport_api::ByteTransport` over a handle opened on the
   device interface `dtmonpro.sys` exposes. Discovery:
   `SetupDiGetClassDevsW(&GUID, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE)`,
   `SetupDiEnumDeviceInterfaces`, `SetupDiGetDeviceInterfaceDetailW` for the
   path and `SetupDiGetDeviceInstanceIdW` for the identity. **The identity is
   the device instance id** — `USB\VID_18E1&PID_0104\<serial>` — parsed as
   the serial path parses the registry key: only `18E1:0104` is ever opened;
   the GUID is where to look, not proof of what was found. On other targets
   the crate is an enumerator that finds nothing and an `open` that refuses,
   so the workspace tests keep running on Linux.
2. **The byte stream and nothing else.** `CreateFileW` with read and write
   access, no sharing, `FILE_FLAG_OVERLAPPED`. A reader thread owns the one
   pending `ReadFile` (a 4 KiB buffer; the driver's read queue is
   sequential) and hands each completion to a bounded channel; `read` takes
   from that channel within the transport's read timeout, 250 ms by default
   and `set_read_timeout` as on the serial path, and keeps what the caller's
   buffer did not take. `write_all` is one overlapped `WriteFile` awaited
   for at most two seconds and cancelled if it does not complete. `close`
   cancels the pending read with `CancelIoEx`, joins the thread and closes
   the handle. **None of the driver's five IOCTLs is used**: not the string
   descriptor (the instance id already carries the serial), not the
   configuration descriptor, and never the port reset (`0x5500A004`) nor the
   vendor request `0xDA` (`0x5500A00C`), whose purpose is unknown. A sharing
   violation or access denied on open is reported as the adapter being in
   use by another program — SDD holding it — as the serial path reports a
   busy port.
3. **Nothing of the package is loaded, shipped or changed.** No
   `monpj432.dll`, no J2534 API, no `dtmonpro.sys` in the installer, and no
   driver rebinding by the application, ever: it opens an interface the
   user's own installed driver exposes, or it does not see the cable. The
   application still installs no driver.
4. **One list, two enumerators.** The shell asks the serial path first and
   the `dtmonpro` path second; a cable can be on one driver only, so one
   adapter appears once, and today's rules stand — one found: connect;
   several: choose; none: name a same-vendor device if there is one.
   `AdapterSummary.driver` carries which driver holds the cable (`usbser` or
   `dtmonpro`), `AdapterInfo.transport` names the path, and the adapter
   panel says it in words: on Windows's serial driver with its COM port, or
   on SDD's driver. For an adapter on `dtmonpro` the `port` field holds the
   device path the enumeration returned; the interface treats it as opaque.
5. **The protocol layer is untouched.** `MongooseJlrDevice<T: ByteTransport>`
   is generic over the transport already; the shell's `Link` gains a third
   variant and nothing in `mongoose-jlr` changes. The frames, timeouts and
   route handling are the ones confirmed over `usbser`. The invariant holds:
   this is transport, below protocol, below knowledge.
6. **32-bit and 64-bit alike.** The crate builds for `i686` and `x86_64`;
   every API it uses exists on every Windows 10.

## Validation

- `IMPLEMENTED`: the crate, the shell's merged discovery, the panel's words.
- `FIXTURE_TESTED`: the instance-id parsing; the reader-and-channel
  semantics (data, timeout, closed, device gone) against a fake pipe; the
  shell's merging of two enumerators and its wording against fakes — on
  Linux in CI, as the rest of the workspace.
- `HARDWARE_CONFIRMED` — the gate for the next build: on the owner's PC,
  where the package is installed, hand the cable to the package's driver in
  Device Manager ("Update driver", "Let me pick", "MongoosePro JLR J2534
  Interface"), plug the cable in; the application must list it on SDD's
  driver, connect, and get `0x8109` back as in F1/F3; unplugged, the panel
  must say so; plugged back, it must be found again; then hand the cable
  back to "USB Serial Device". The 32-bit build runs on his 64-bit Windows
  and takes the same test.
- Then the tester's 32-bit laptop with SDD: his report says whether the
  cable is found on SDD's driver with no Device Manager step at all.
  `VEHICLE_CONFIRMED` comes only from a car, and the first car is his.

## Consequences

- The guides: the Device Manager steps become a note for whoever prefers
  Windows's serial driver, not a requirement; README's sentence goes.
- Two programs cannot hold the cable at once: SDD open on it, the application
  says it is in use; the reverse holds too. That is normal.
- Unknown until hardware: whether `dtmonpro.sys` returns each USB transfer
  as one `ReadFile` completion the way `usbser` does — the device code reads
  the stream in chunks and parses frames as they complete, so how the
  stream is cut does not matter to it — and whether the pending read
  cancels cleanly on close; the official DLL imports `CancelIo`, so the
  driver is built for that. If hardware says otherwise, this record gets its
  "Built" note and the code follows.
- Left alone, on purpose: WinUSB or libusb (they rebind the device — the
  ritual this record removes); the application switching the binding itself
  (administrator rights, and it would touch SDD's installation); loading
  `monpj432.dll` (Drew's runtime, x86 only, the J2534 road this product does
  not take).

## Built, 2026-11-12

Decisions 1 to 6 were built the morning the record was accepted: the crate
`transport-dtmonpro` with nine tests against a fake pipe, the shell's
merged discovery with two tests, the panel's words; the Windows side
compiles for `x86_64` and `i686`, and 150 interface tests pass.

The hardware check could not run where this record put it. On the owner's
Windows 11 PC the cable was handed to the package's driver in Device
Manager and came up with a yellow triangle: Code 39, problem status
`0xC0E90002`, and the Code Integrity log of the same minute — "Code
Integrity determined that a process (System) attempted to load
dtmonpro.sys that did not meet the Authenticode signing level
requirements or violated code integrity policy", and "Windows is unable
to verify the image integrity of the file ... because file hash could not
be found on the system". Smart App Control was off (registry state 0);
memory integrity (HVCI) was running. So the 2012 driver, signed by Drew
Technologies and not by Microsoft, does not load on this Windows 11 at
all — which is also the answer to the question left open above: Windows
bound the inbox `usbser` because the package's own driver cannot start
here. SDD itself could not run on such a PC; on a machine like it the
serial path is the only path, and on the SDD-era laptops, where the
driver loads, the path of this record is the one that matters.

Handing the cable back was not a dialog's work: "Update driver" to "USB
Serial Device" did not stick, twice, nor after a restart. At the owner's
hand the package was removed from his PC —
`pnputil /delete-driver oem90.inf /uninstall /force` — and the replugged
cable came up as "USB Serial Device (COM4)". The new build then listed it
on Windows's serial driver, connected, and got `0x8109` back: the merged
discovery is `HARDWARE_CONFIRMED` on the serial path. The path through
`dtmonpro.sys` stays `IMPLEMENTED / FIXTURE_TESTED`, and its hardware check
moves to a PC where the driver loads. The owner moved it the same day, in
so many words («ворота на Geoff»): build 1.2.5 ships with this path
`FIXTURE_TESTED`, and the first tester's run on his SDD laptop — the cable
back on "MongoosePro JLR J2534 Interface", the application expected to
list it there unaided — is its hardware check; the serial path stays
beside it as the fallback either way.
