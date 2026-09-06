# MongoosePro JLR USB protocol — статична реконструкція

## Scope і confidence

Це статичний опис monpj432.dll 1.1.16.0 та dtmonpro.sys 1.1.0.0. USB не відкривався, descriptors і трафік фізичного пристрою не читались.

CONFIRMED означає пряму інструкцію/таблицю/рядок; STRONG INFERENCE — узгоджену інтерпретацію; UNKNOWN — межу статичного аналізу.

## Transport stack

**CONFIRMED:**

    monpj432.dll framed byte stream
          |
          | overlapped WriteFile / ReadFile
          v
    dtmonpro.sys KMDF queues
          |
          | bulk OUT / bulk IN
          v
    MongoosePro firmware

Службові descriptor/serial/reset дії йдуть через DeviceIoControl; один vendor-specific request — через USB control endpoint 0.

WinUSB.dll, winusb.sys API та COM/serial configuration API у monpj432.dll не імпортуються.

## Device Interface

**CONFIRMED GUID:** {F99F6FCE-03F4-4817-8A03-8CFDE4743B92}.

| Evidence | Location |
|---|---|
| GUID bytes CE 6F 9F F9 F4 03 17 48 8A 03 8C FD E4 74 3B 92 | dtmonpro.sys RVA 0x30C8, file 0x24C8 |
| WdfDeviceCreateDeviceInterface(GUID, NULL) | dtmonpro.sys VA 0x17FA6 |
| same GUID | monpj432.dll RVA 0x7F900, file 0x7ED00 |
| SetupDiGetClassDevsW / SetupDiEnumDeviceInterfaces | monpj432.dll VA 0x1004B777 / 0x1004B7B4 |

The DLL obtains the generated path through SetupDiGetDeviceInterfaceDetailW and passes it to CreateFileW. There is no fixed symbolic name.

## USB configuration and pipes

**CONFIRMED driver flow:**

| Operation | Driver VA |
|---|---:|
| WdfUsbTargetDeviceCreate | 0x18165 |
| WdfUsbTargetDeviceRetrieveInformation | 0x181D5 |
| WdfUsbTargetDeviceRetrieveConfigDescriptor | 0x182xx and IOCTL handler |
| WdfUsbTargetDeviceSelectConfig | 0x186BF |
| enumerate interfaces/pipes | 0x1882F onward |
| WdfUsbInterfaceGetNumConfiguredPipes | 0x18846 |
| WdfUsbInterfaceGetConfiguredPipe | 0x18882 |
| WdfUsbTargetPipeSetNoMaximumPacketSizeCheck | 0x18895 |
| configure optional continuous reader | call at 0x123F3 |

Pipe selection:

- WDF_USB_PIPE_INFORMATION.PipeType == 3 (Bulk): WdfUsbTargetPipeIsInEndpoint at 0x188F2 stores bulk IN in device context +0x10; IsOutEndpoint at 0x18941 stores bulk OUT at +0x18.
- PipeType == 4 (Interrupt): stored at context +0x20 and used with continuous-reader support.
- Control endpoint 0 is used by WdfUsbTargetDeviceSendControlTransferSynchronously.
- No isochronous handling was found.

**UNKNOWN:** bConfigurationValue, interface number, bEndpointAddress values, endpoint numbers and wMaxPacketSize. The driver reads them from runtime descriptors; they are not hard-coded in the analysed files.

## Driver Read/Write behavior

**CONFIRMED:**

- user ReadFile requests reach EvtIoRead VA 0x11870 and are formatted for the selected bulk IN pipe (WdfUsbTargetPipeFormatRequestForRead call at 0x11A73);
- user WriteFile requests reach EvtIoWrite VA 0x11DF4 and are formatted for bulk OUT (FormatRequestForWrite call at 0x11FF3);
- separate sequential KMDF queues are assigned to WdfRequestTypeRead and WdfRequestTypeWrite at VA 0x17E06 and 0x17EEA;
- DeviceControl uses a parallel default queue; an additional manual queue exists.

No parsing of the Mongoose frame header or opcodes appears in these callbacks. The driver forwards buffers and completes WDF requests.

## Outer stream frame

**CONFIRMED layout, little-endian:**

| Offset | Size | Field |
|---:|---:|---|
| 0x00 | 2 | payload_length |
| 0x02 | 2 | payload_length XOR 0x51E6 |
| 0x04 | payload_length | payload |

Evidence:

- transport constructor VA 0x1006AE00 stores maximum payload 0x1800 at object +0x4020 and guard seed 0x51E6 at +0x4024 (instructions at 0x1006AE47 and 0x1006AE51);
- frame writer VA 0x1006E180 writes length at 0x1006E1D8, XOR guard at 0x1006E1E7–0x1006E1EB, copies payload at 0x1006E1EF, then sends length+4 bytes;
- parser VA 0x1006B1C0 requires at least four bytes, rejects zero/over-0x1800 length at 0x1006B271–0x1006B283, verifies XOR at 0x1006B289–0x1006B298, waits for length+4 bytes, then copies payload;
- on invalid header the parser advances one byte and retries at 0x1006B3CB–0x1006B3F8, providing stream resynchronization.

This field is a fixed XOR guard, not a CRC over the payload. No payload CRC was identified in the host framing code.

### Read/write buffering

- receive ReadFile size: 0x2000 bytes at VA 0x1006A604;
- host circular receive allocation: 0x4000 bytes at VA 0x1006AE05;
- maximum accepted frame payload: 0x1800 bytes;
- WriteFile/ReadFile are OVERLAPPED and use events plus GetOverlappedResult.

## Common command payload prefix

> **Representation clarification from the GM-II comparison:** the u16 at
> payload +0x04 is a little-endian raw command word whose low byte is the
> opcode and high byte is the request/response flag. Thus raw 0x0103 is wire
> bytes `03 01`, while raw 0x8003 is `03 80`. The old table values remain exact,
> but the 0x01xx group is not a separate numeric opcode namespace on wire. This
> distinction is safety-critical because GM-II uses `03 01` as GetVersion while
> JLR names it `cJumpToFirmware`.

Several board-command builders use a 12-byte base structure. The meaningful fields observed statically are:

| Payload offset | Size | Observed meaning |
|---:|---:|---|
| 0x00 | u16 | constant 0x0001 |
| 0x02 | u16 | constant 0x0000 |
| 0x04 | u8 + u8 | opcode + flag (also shown as a raw u16 command word) |
| 0x06 | u16 | sequence/correlation number; produced from a rolling nonzero byte counter |
| 0x08 | u16 | zero in simple requests; likely status/flags/reserved |
| 0x0A | u16 | alignment/reserved; not consistently initialized in the simple builders |

Evidence:

- constants at monpj432.dll VA 0x100806C8 (1) and 0x100806C4 (0), file offsets 0x7FAC8 and 0x7FAC4;
- command builders at VA 0x1003E820 (opcode 0x0103), 0x1003E910 (0x0102), 0x1003F030 (0x0100), 0x1003F160 (0x0109);
- sequence byte at object +0xC288 is forced nonzero, copied as u16, then incremented.

**UNKNOWN:** authoritative field names for offsets 0, 2, 8 and 0x0A. The table records observed values, not an invented ABI.

Variable commands append typed structures/data after the base prefix. For example 0x010C cUnprotectBootloader sends 0x18 bytes and contains constant 0x5A81C30E plus 0x5A000000 at builder VA 0x1003F351. This is dangerous firmware-path material and is documented only for recognition, not execution.

## `cOutboundData` batch record

**CONFIRMED:** serializer VA 0x1006B090 is called by batch writer VA 0x1006E2E0. The latter advances the source message pointer by 0x1038 bytes per item, exactly the 32-bit J2534 `PASSTHRU_MSG` stride used by this DLL. Each item is emitted as its own outer frame; the serializer returns `DataSize + 0x1C` bytes including the four-byte outer header.

The byte layout below is direct instruction-level evidence. Names marked unknown describe only their source/behavior:

| Absolute frame offset | Payload offset | Size | Value/source | Confidence |
|---:|---:|---:|---|---|
| 0x00 | — | u16 | inner payload length = `PASSTHRU_MSG.DataSize + 0x18` | CONFIRMED |
| 0x02 | — | u16 | length XOR 0x51E6 | CONFIRMED |
| 0x04 | 0x00 | u16 | channel/device object identifier supplied by caller | STRONG; exact class field name UNKNOWN |
| 0x06 | 0x02 | u16 | constant 0 | CONFIRMED |
| 0x08 | 0x04 | u16 | 0x0008 (`cOutboundData`) | CONFIRMED |
| 0x0A | 0x06 | u16 | caller-supplied transaction/correlation-like field | bytes CONFIRMED; semantic STRONG INFERENCE |
| 0x0C | 0x08 | u16 | total message count passed to batch writer | CONFIRMED by loop counter reuse |
| 0x0E | 0x0A | u16 | not written by this serializer; reserved/stale scratch bytes | CONFIRMED non-write; semantic UNKNOWN |
| 0x10 | 0x0C | u32 | `PASSTHRU_MSG.TxFlags` (`msg + 0x08`) | CONFIRMED |
| 0x14 | 0x10 | u32 | same caller-supplied value for every item in batch | bytes CONFIRMED; likely per-call timeout, semantic not promoted |
| 0x18 | 0x14 | u16 | low 16 bits of `PASSTHRU_MSG.DataSize` (`msg + 0x10`) | CONFIRMED |
| 0x1A | 0x16 | u16 | low 16 bits of `PASSTHRU_MSG.ExtraDataIndex` (`msg + 0x14`) | CONFIRMED |
| 0x1C | 0x18 | DataSize | `PASSTHRU_MSG.Data[]` (`msg + 0x18`) | CONFIRMED |

Instruction evidence is VA 0x1006B09B–0x1006B0FE. `ProtocolID` at `PASSTHRU_MSG + 0x00`, `RxStatus` at +0x04 and `Timestamp` at +0x0C are not copied by this serializer. The selected channel already carries protocol identity; no stronger semantic claim is made for the omitted fields.

Variant serializer VA 0x1006B120 emits the same record but selects one of two supplied identifier pointers by comparing `PASSTHRU_MSG.ProtocolID`. It is used by VA 0x1006E3B0 and the dual-underlying-channel ISO-15765 path at VA 0x10025B40. This corroborates that ISO-15765 traffic is mapped onto underlying CAN channels in host code.

**Implementation warning:** because DataSize and ExtraDataIndex are truncated to u16 on this wire path and payload length is limited to 0x1800, a future codec must reproduce the old DLL's validation and truncation boundaries rather than assume the full 32-bit J2534 fields appear on USB.

## Opcode table

The names below are **CONFIRMED strings selected by function VA 0x1003B080**. Direction/meaning beyond the literal name is marked where inferred. String RVAs/file offsets can be reproduced with analysis/mongoosepro_jlr/scripts/protocol_tables.py.

### Core commands

| Opcode | Literal name | Direction | Response / note |
|---:|---|---|---|
| 0x0000 | cPrintDebugText | device→host, STRONG | asynchronous debug text |
| 0x0001 | cRespGeneral | device→host, STRONG | generic response |
| 0x0002 | cRespString | device→host, STRONG | string response |
| 0x0003 | cOpenDevice | host→device, STRONG | 0x8003 cOpenDeviceResp |
| 0x0004 | cGetDeviceConfiguration | host→device, STRONG | exact response form not named |
| 0x0005 | cCloseDevice | host→device, STRONG | 0x8005 cCloseDeviceResp |
| 0x0006 | cOpenChannel | host→device, STRONG | 0x8006 cOpenChannelResp |
| 0x0007 | cCloseChannel | host→device, STRONG | 0x8007 cCloseChannelResp |
| 0x0008 | cOutboundData | host→device, STRONG | 0x8008 cOutboundDataResp |
| 0x0009 | cInboundData | device→host, STRONG | async receive |
| 0x000A | cIndication | device→host, STRONG | async indication |
| 0x000B | cSetValue | host→device, STRONG | 0x800B cSetValueResp |
| 0x000C | cGetValue | host→device, STRONG | 0x800C cGetValueResp |
| 0x000D | cTableAddEntry | host→device, STRONG | 0x800D cTableAddEntryResp |
| 0x000E | cTableRemoveEntry | host→device, STRONG | 0x800E cTableRemoveEntryResp |
| 0x000F | cTableModifyEntry | host→device, STRONG | no dedicated response name found |
| 0x0010 | cTableClear | host→device, STRONG | 0x8010 cTableClearResp |
| 0x0011 | cIoctl | host→device, STRONG | 0x8011 cIoctlResp |
| 0x0012 | cSetPin | host→device, STRONG | 0x8012 cSetPinResp |
| 0x0013 | cGetString | host→device, STRONG | 0x8013 cGetStringResp |
| 0x0014 | cSetData | host→device, STRONG | 0x8014 cSetDataResp |
| 0x0015 | cGetData | host→device, STRONG | 0x8015 cGetDataResp |

### Board, bootloader and maintenance commands

| Opcode | Literal name | Direction | Response / safety |
|---:|---|---|---|
| 0x0100 | cEchoPacket | host→device | response form UNKNOWN |
| 0x0101 | cGetBoardStatus | host→device | response likely generic/string; exact code UNKNOWN |
| 0x0102 | cResetBoard | host→device | 0x8102 cResetBoardResp; dangerous state change |
| 0x0103 | cJumpToFirmware | host→device | 0x8103 cJumpToFirmwareResp; firmware-state command |
| 0x0104 | cSetBoardID | host→device | state-changing |
| 0x0105 | cSetBoardLed | host→device | state-changing |
| 0x0106 | cSyncClock | host→device | state-changing |
| 0x0107 | cGetStats | host→device | response form UNKNOWN |
| 0x0108 | cBoardSleep | host→device | state-changing |
| 0x0109 | cGetBoardInfo | host→device | 0x8109 cGetBoardInfoResp |
| 0x010A | cReflashBoard | host→device | 0x810A cReflashBoardResp; firmware write |
| 0x010B | cWriteSerialNumber | host→device | 0x810B response; persistent write |
| 0x010C | cUnprotectBootloader | host→device | 0x810C response; dangerous |
| 0x010D | cBrownoutIndication | likely device→host | direction is STRONG INFERENCE from name |
| 0x010E–0x0110 | invalid/?cmd? | UNKNOWN | all map to default string |
| 0x0111 | cCheckCRN | host→device | 0x8111 cCheckCRNResp |
| 0x0112 | cUpdateBTModule | host→device | 0x8112 response; firmware write |

Response opcodes commonly equal request opcode OR 0x8000, but not every request has a named dedicated response in the switch. This is an observed convention, not a guarantee for unknown commands.

## Async receive

**STRONG INFERENCE:** bulk IN is a continuous byte stream containing the same outer frames. The DLL's background receive loop:

1. issues overlapped ReadFile;
2. appends bytes to a circular buffer;
3. validates/resynchronizes outer frames;
4. dispatches the payload by opcode and correlation sequence;
5. places cInboundData/cIndication results into host-side queues/callbacks.

Evidence: VA 0x1006E480 receive loop, parser 0x1006B1C0, dispatcher/logging call to command-name switch at 0x10043B2A, and J2534 receive/callback exports.

The exact cInboundData record layout is only partially recoverable from serializers/parsers and is not claimed complete here.

### Parsed payload dispatcher header

**CONFIRMED:** after the four-byte outer frame is removed, dispatcher VA 0x10043610 treats the payload as a common command header:

| Payload offset | Size | Dispatcher use | Confidence |
|---:|---:|---|---|
| 0x00 | u16 | first endpoint/object identifier; direction-dependent role | bytes present, semantic UNKNOWN |
| 0x02 | u16 | lookup key for the target host channel/device object | CONFIRMED use at VA 0x10043650/0x1004369E/0x1004379B |
| 0x04 | u16 | command/response opcode | CONFIRMED switch at VA 0x10043619 |
| 0x06 | u16 | outstanding-operation correlation value for selected responses | CONFIRMED comparisons at VA 0x10043787 and 0x1004381D |
| 0x08 | u16 | response-specific flag/count | bytes CONFIRMED; exact generic semantic UNKNOWN |
| 0x0A | u16 | reserved/response-specific | UNKNOWN |
| 0x0C | u32 | status/subcommand field used by multiple handlers | CONFIRMED use; meaning depends on opcode |
| 0x10 onward | variable | command-specific fields/data | CONFIRMED |

For opcode 0x0009 (`cInboundData`), the object is selected with the u16 at +0x02 and a pointer to payload +0x0C is passed to a channel virtual handler at VA 0x10043685–0x10043693. For opcode 0x000A (`cIndication`), the same lookup is used and the whole record is handed to a different virtual handler at VA 0x10043708–0x10043713. Named synchronous responses are routed by opcode and the +0x06 correlation field.

Request and response roles of the two leading u16 identifiers appear direction-dependent (source/destination-like), but their vendor names are not available. This report intentionally does not rename them `DeviceID`/`ChannelID` universally.

## ISO-15765 processing above the USB command layer

The outer frame and `cInboundData` record are not themselves ISO-TP frames. `monpj432.dll` performs a second, protocol-specific processing stage after command dispatch.

**CONFIRMED host-side receive behavior:**

- PCI type dispatch (Single/First/Consecutive) at VA 0x10021790;
- First Frame 12-bit total-length extraction and 0x0FFF bound at VA 0x10021070;
- Consecutive Frame sequence comparison, modulo-16 increment and bounded copy at VA 0x10021280;
- interrupted segmented-transfer reset and completed J2534-message construction at VA 0x10021440/0x10021650;
- four-byte CAN identifier reconstruction in big-endian byte order, with 11-bit versus 29-bit range validation in the filter path around VA 0x1001FB63 and message path around VA 0x100201FF.

**CONFIRMED host-side transmit/control evidence:** RX and TX timeout states at VA 0x1001F6C0, mandatory matching flow-control filter for segmented messages at VA 0x1001FC6A, CAN-channel matching at VA 0x10022B93/0x10023266, and ISO-TP configuration keys for BS, STmin, WFT_MAX, padding and address type.

Therefore a raw-USB replacement cannot merely forward J2534 payloads. It must reproduce this host protocol state machine or expose a lower-level API whose caller does so. Exact on-wire `cOutboundData` fields and the residual firmware timing role remain incomplete.

## COM3 question

**CONFIRMED for the official DrewTech path:**

- monpj432.dll discovers {F99F...} with SetupAPI and opens its device-interface path.
- It does not search for COMx and imports no SetCommState, GetCommState, SetupComm, PurgeComm or WaitCommEvent.
- monpjaguar.inf binds USB\VID_18E1&PID_0104 as a whole-device KMDF driver, not as a Ports-class driver.
- dtmonpro.sys selects USB bulk/interrupt pipes directly.

Therefore COM3 is not part of the official J2534 1.1.16 data path.

**UNKNOWN:** why Windows exposed the same VID/PID as USB Serial Device (COM3) without the DrewTech driver. Static package data cannot distinguish among:

- a genuinely CDC-compatible descriptor/data interface;
- Windows compatible-ID fallback using usbser;
- a firmware/fallback enumeration whose serial endpoint does not carry the Mongoose framed protocol;
- a different configuration/interface selected under another driver.

No evidence was found that normal DrewTech open performs a CDC-to-vendor mode switch. The one vendor control request bRequest 0xDA is exposed through a dedicated IOCTL and is not part of the ordinary ReadFile/WriteFile path; its exact purpose is UNKNOWN.

## What a future capture must establish

Without performing it here, only physical descriptor/capture evidence can provide:

- configuration and interface descriptors;
- exact endpoint addresses and max packet sizes;
- whether COM3 bytes match the 0x51E6 outer framing;
- control transfers during initial open;
- precise command payload fields, response timing and retries.

No USB or COM operation was performed in this analysis.
