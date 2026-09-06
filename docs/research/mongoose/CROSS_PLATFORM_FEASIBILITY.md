# Cross-platform replacement feasibility

## Question

Can the legacy stack:

    monpj432.dll + dtmonpro.sys

be replaced in the future by:

    our_mongoose_library
      -> user-space USB transport
      -> existing MongoosePro firmware

for Windows 11 and macOS?

This assessment uses static facts only and does not prescribe or perform Windows changes.

## Answer

**Can dtmonpro.sys be bypassed? PROBABLY YES.**

The driver is a thin KMDF USB transport:

- one bulk OUT pipe;
- one bulk IN pipe;
- optional interrupt continuous reader;
- USB descriptor/string queries;
- port reset;
- one no-data vendor control request;
- no identified CAN/J2534/ISO-TP codec.

All of these are representable by normal user-space USB APIs. No kernel-only timing primitive or bus protocol engine was found.

**Can monpj432.dll also be replaced? PROBABLY YES, but this is the larger task.**

The outer framing and command opcode families are statically recoverable. A replacement must still reproduce:

- J2534 API semantics and error mapping;
- device discovery/identity;
- command payload records and value IDs;
- async correlation, queues, timeouts and callbacks;
- channel/filter/message marshalling;
- ISO-15765 flow-control-filter validation, RX reassembly, sequence handling, padding/address modes, segmented-TX state and timeout behavior now confirmed inside the DLL;
- host-side ISO-9141 checksum, ISO-14230 length and J1850 receive-status semantics;
- exact firmware-version compatibility behavior;
- protocol-specific details that are not yet fully reconstructed.

The technical blocker is incomplete protocol knowledge and the size of the host protocol engine, not a demonstrated need for a kernel driver.

## Evidence supporting user-space transport

| Fact | Evidence | Consequence |
|---|---|---|
| ReadFile/WriteFile carry framed bytes | monpj432 VA 0x1006A420/0x1006A5B0 and frame writer/parser 0x1006E180/0x1006B1C0 | bulk transfer API can replace driver I/O |
| driver selects bulk IN/OUT at runtime | dtmonpro VA 0x1882F–0x18941 | no proprietary kernel transport primitive |
| optional interrupt pipe uses WDF continuous reader | call VA 0x123F3 | user-space async interrupt transfer is available |
| descriptor and string IOCTLs wrap standard USB operations | EvtIoDeviceControl VA 0x18B9C | libusb/IOKit/WinUSB equivalents exist |
| vendor request is an ordinary EP0 setup packet | bRequest 0xDA, bmRequestType 0x40 | user-space control transfer can represent it |
| command framing is in DLL, not driver | 0x51E6 frame guard only in monpj432 | driver need not parse protocol |
| ISO-TP state machine is in DLL, not driver | validation 0x1001FCC0; RX handlers 0x10021070/0x10021280/0x10021790; TX timeout 0x1001F6C0 | a replacement user library must reproduce it; no kernel dependency follows |

## Windows 11 options

### A. usbser.sys / COM

**Verdict: UNKNOWN; not the primary candidate.**

Pros if confirmed:

- built-in user-space serial API;
- no custom kernel code in the application architecture;
- easy asynchronous I/O.

Static problems:

- official monpj432.dll never opens COMx;
- official INF is VehiclePassThru/KMDF, not Ports;
- dtmonpro.sys binds the whole VID/PID and selects USB pipes itself;
- actual CDC descriptors and serial line requirements are unavailable;
- it is unknown whether COM3 carries the same 0x51E6-framed stream.

COM3 may be a usable fallback interface, but static evidence cannot distinguish that from a nonfunctional usbser association. It should not be the design baseline without a byte-for-byte capture.

### B. WinUSB

**Verdict: PROBABLY technically suitable.**

Required capabilities are available in the WinUSB user-space model:

- device/interface discovery;
- bulk read/write;
- interrupt transfers;
- control transfers;
- descriptor/string access;
- overlapped asynchronous operation.

Static caveat: the analysed device is currently claimed by a whole-device dtmonpro INF. Whether the physical firmware advertises a Microsoft OS compatible ID for WinUSB is UNKNOWN. This is deployment/binding work, not a reason to write a new kernel protocol driver.

### C. libusb on Windows

**Verdict: PROBABLY technically suitable and best for shared transport code.**

libusb exposes the same raw USB operations and can use an appropriate Windows user-space backend. It would permit one framing/command core to be shared with macOS.

The same binding caveat applies: a user-space backend must have access to the relevant device/interface. No binding change is proposed or performed in this report.

## macOS options

### A. native CDC serial

**Verdict: UNKNOWN.**

This depends entirely on the unresolved COM3/CDC question. No static Apple/CDC path exists in the DrewTech package, and no physical descriptors were read.

### B. IOKit / user-space USB

**Verdict: PROBABLY suitable.**

IOKit can represent bulk, interrupt and control transfers and descriptor access. It would require a macOS-specific discovery/I/O backend while sharing the Mongoose framing and J2534/protocol logic.

### C. libusb on macOS

**Verdict: PROBABLY suitable and the strongest common-denominator candidate.**

The driver behavior is simple enough to map to libusb:

| dtmonpro behavior | libusb-style primitive |
|---|---|
| select configuration/interface | get/set configuration, claim interface |
| bulk ReadFile/WriteFile | async bulk transfers |
| optional interrupt reader | async interrupt transfer |
| serial string index 3 | get string descriptor |
| config descriptor IOCTL | get active config descriptor |
| vendor bRequest 0xDA | control transfer |
| reset IOCTL | reset device, if ever explicitly needed |

The last two are state-changing and should not be part of basic discovery/open unless later trace evidence proves they are required.

## Suggested future architecture

    Cross-platform protocol core
      - outer frame encode/decode (length, length XOR 0x51E6)
      - opcode/request correlation
      - command payload codecs
      - async receive dispatcher
      - timeout/error model
      - host ISO-TP engine: filters, segmentation/reassembly, FC/BS/STmin/WFT, padding/address modes
      - legacy-protocol validation/error mapping
      - firmware-update commands excluded by policy
             |
             +-- Windows WinUSB/libusb backend
             |
             +-- macOS libusb or IOKit backend
             |
             +-- optional serial backend only if COM/CDC is proven

For a Windows J2534 deliverable, a thin PassThruSupport-compatible x86 DLL can sit above the common core. For macOS, the same core can expose a native library/API rather than Windows J2534 registry conventions.

## What is already sufficient to implement statically

- PE/architecture inventory;
- Device Interface GUID for compatibility studies;
- USB transport types: bulk IN/OUT plus optional interrupt and EP0;
- outer frame layout and resynchronization;
- 0x1800 maximum payload used by the old DLL;
- common command prefix fields;
- complete literal opcode-name switch;
- response high-bit convention for named responses;
- serial descriptor index 3 behavior;
- all driver IOCTL wire-equivalent operations.

## What remains insufficient

**Requires additional evidence before a compatible implementation can be called complete:**

- exact USB interface/endpoint addresses and packet sizes;
- exact open-time control/transfer sequence;
- complete payload layouts for open channel, value IDs, filters and messages;
- bitrate/protocol enumeration encoding;
- all J2534 ioctl payload mappings;
- cInboundData and indication corner cases;
- timeouts/retries/error/status codes;
- exact remaining ISO-15765 split for low-level transmit timing/frame emission; substantial host responsibility is already confirmed;
- COM3 equivalence or non-equivalence;
- firmware-version compatibility matrix.

These are protocol-validation gaps. They do not establish a kernel-driver requirement.

## Feasibility matrix

| Platform / transport | Feasibility | Confidence | Main unresolved issue |
|---|---|---|---|
| Windows usbser/COM | UNKNOWN | low | whether COM carries real Mongoose stream |
| Windows WinUSB | PROBABLY | medium-high | interface binding/deployment and descriptors |
| Windows libusb | PROBABLY | medium-high | backend access/binding and descriptors |
| macOS native CDC | UNKNOWN | low | actual CDC functionality |
| macOS IOKit USB | PROBABLY | medium-high | descriptors and protocol payload completion |
| macOS libusb | PROBABLY | medium-high | descriptors and protocol payload completion |

## Best candidate

**Best shared transport candidate: libusb raw USB, with an optional native WinUSB backend where Windows packaging benefits.**

Reason:

- directly matches confirmed bulk/interrupt/control operations;
- avoids recreating a proprietary kernel protocol driver;
- supports one protocol core on Windows and macOS;
- does not depend on the unproven COM3/CDC path.

This is a feasibility conclusion, not authorization to rebind the device or alter Windows.

## Bottom line

- dtmonpro.sys thin transport: **YES, with small device-management additions**.
- kernel driver fundamentally required: **NO evidence**.
- bypass dtmonpro.sys: **PROBABLY**.
- one user-space implementation for Windows 11 and macOS: **PROBABLY**, but it must replace the DLL protocol engine as well as the USB framing.
- COM as common transport: **UNKNOWN**.
- raw USB through libusb/WinUSB/IOKit: **best-supported static design**.

No physical testing or system change was performed.
