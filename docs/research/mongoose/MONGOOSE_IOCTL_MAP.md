# dtmonpro.sys IOCTL map

## Scope

This table is reconstructed from EvtIoDeviceControl in dtmonpro.sys 1.1.0.0. It does not invoke the driver.

Driver image base is 0x10000. EvtIoDeviceControl is VA 0x18B9C, RVA 0x8B9C, file offset 0x4B9C. Unknown codes complete with STATUS_INVALID_DEVICE_REQUEST (0xC0000010) at VA 0x18C53.

All five codes use custom DeviceType 0x5500 and METHOD_BUFFERED.

## CTL_CODE decode

    DeviceType = code >> 16
    Access     = (code >> 14) & 3
    Function   = (code >> 2) & 0xFFF
    Method     = code & 3

Access 1 is FILE_READ_ACCESS; access 2 is FILE_WRITE_ACCESS.

## Dispatch table

| IOCTL | DeviceType | Access | Function | Method | Input | Output | Handler/effect | Confidence |
|---:|---:|---:|---:|---|---|---|---|---|
| 0x55006000 | 0x5500 | READ | 0x800 | BUFFERED | none observed | variable, configuration-descriptor size | returns active USB configuration descriptor | CONFIRMED |
| 0x5500A004 | 0x5500 | WRITE | 0x801 | BUFFERED | none observed | none | synchronous USB port reset | CONFIRMED |
| 0x5500A00C | 0x5500 | WRITE | 0x803 | BUFFERED | none observed | none | EP0 vendor request 0xDA, host-to-device, no data | CONFIRMED transfer; purpose UNKNOWN |
| 0x55006018 | 0x5500 | READ | 0x806 | BUFFERED | none observed | exactly 8 bytes: DWORD 1, DWORD 1 | fixed driver/API tuple | bytes CONFIRMED; semantic name UNKNOWN |
| 0x5500601C | 0x5500 | READ | 0x807 | BUFFERED | none observed | UTF-16 USB string, variable | returns USB string descriptor index 3, LANGID 0x0409 | CONFIRMED |

## Per-code evidence

### 0x55006000 — configuration descriptor

Branch begins VA 0x18E0A.

1. WdfUsbTargetDeviceRetrieveConfigDescriptor is called with NULL buffer to obtain required length.
2. WdfRequestRetrieveOutputBuffer requests that size.
3. WdfUsbTargetDeviceRetrieveConfigDescriptor fills the caller buffer.
4. WdfRequestCompleteWithInformation returns the number of bytes.

No input structure is consumed. Exact output bytes depend on the connected device's active descriptor and therefore are not present statically.

### 0x5500A004 — reset port

Dispatch branch VA 0x18C6C calls helper VA 0x18F6C. The helper calls WdfUsbTargetDeviceResetPortSynchronously at VA 0x18FDE. There is no data buffer.

This changes USB device/port state and was not invoked.

### 0x5500A00C — vendor control transfer

Dispatch branch VA 0x18C5D calls helper VA 0x19068. It builds this setup packet at VA 0x190E7–0x19135:

| Field | Value |
|---|---:|
| bmRequestType | 0x40: host-to-device, vendor, device recipient |
| bRequest | 0xDA |
| wValue | 0 |
| wIndex | 0 |
| wLength | 0 |

WdfUsbTargetDeviceSendControlTransferSynchronously is called at VA 0x19135.

**UNKNOWN:** semantic purpose. A reset/re-enumeration or bootloader transition is plausible because it is a no-data vendor command near USB reset support, but no direct monpj432.dll call site to this IOCTL was found. It must not be labeled as firmware transition without a trace or additional caller evidence.

### 0x55006018 — fixed 8-byte tuple

Branch begins VA 0x18D96. It requires an output buffer of at least 8 bytes, writes two little-endian DWORD values {1, 1}, and completes with Information=8.

**UNKNOWN:** whether these are ABI version fields, capabilities, or another compatibility tuple. The bytes are confirmed; the name is not.

### 0x5500601C — USB serial string

Branch begins VA 0x18C76:

- output buffer is obtained from the WDF request;
- WdfUsbTargetDeviceQueryString is called with StringIndex=3 and LANGID=0x0409;
- returned UTF-16 bytes are completed to user mode.

Direct monpj432.dll call sites:

- VA 0x1004BAAB, preceding CreateFileW at 0x1004BA52;
- VA 0x1006F9B4, preceding CreateFileW at 0x1006F95C.

Both calls pass output length 0x40 and expect BytesReturned == 0x20. The DLL then takes every second byte to construct a 16-character narrow serial string. This is consistent with the previously observed AOLHE00000xxxxxx but no live query was made.

## What monpj432.dll actually uses

**CONFIRMED:** among direct imported DeviceIoControl call sites in monpj432.dll, only 0x5500601C is present.

The principal J2534 protocol does not use Windows IOCTL for each J2534 operation:

- Open/Connect/Write/Filter/PassThruIoctl are encoded as Mongoose command opcodes;
- the bytes are sent with overlapped WriteFile;
- responses/async data arrive through ReadFile.

In particular, J2534 PassThruIoctl maps to stream command cIoctl (opcode 0x0011) and associated value/data commands. It must not be confused with Windows DeviceIoControl or with the five kernel IOCTL values above.

## Driver request queues

| Queue | Mode | Callback | Setup evidence |
|---|---|---|---|
| DeviceControl | default parallel | EvtIoDeviceControl VA 0x18B9C | VA 0x17CCB–0x17D11 |
| Read | sequential | EvtIoRead VA 0x11870 | VA 0x17D59–0x17DA6; dispatch type 3 at 0x17E06 |
| Write | sequential | EvtIoWrite VA 0x11DF4 | VA 0x17E4C–0x17E99; dispatch type 4 at 0x17EEA |
| manual | manual | internal pending/cancel flow | VA 0x17F24–0x17F62 |

## Unknowns

- No static name was found for the private DeviceType 0x5500 family.
- Other package versions or applications could call the four IOCTLs not directly referenced by this monpj432.dll.
- Descriptor output and endpoint values require a physical descriptor read.
- The purpose of bRequest 0xDA cannot be safely inferred from this driver alone.

No IOCTL was executed during this analysis.
