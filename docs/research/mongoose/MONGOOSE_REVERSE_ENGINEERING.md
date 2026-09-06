# MongoosePro JLR 1.1.16: статичний reverse engineering

## Межі дослідження

Цей звіт побудований лише зі статичного читання PE/INF/MSI, дизасемблювання та ресурсів. Жоден EXE не запускався, DLL не завантажувалась, J2534 exports не викликались, драйвер не завантажувався, USB/COM3 не відкривався, registry і Windows не змінювались.

Позначення впевненості:

- **CONFIRMED** — значення безпосередньо присутнє у файлі або однозначно випливає з інструкцій.
- **STRONG INFERENCE** — кілька незалежних статичних ознак узгоджуються, але немає фізичного trace.
- **WEAK INFERENCE** — правдоподібна інтерпретація одного фрагмента.
- **UNKNOWN** — статичних даних недостатньо.

## Короткий висновок

**CONFIRMED:** офіційний стек має таку будову:

    J2534 application
      -> monpj432.dll (x86: J2534 API, discovery, protocol framing,
                       command construction, receive dispatcher, updater)
      -> Win32 CreateFile/ReadFile/WriteFile/DeviceIoControl
      -> dtmonpro.sys (x64 KMDF 1.9: queues and USB transport)
      -> USB bulk OUT / bulk IN, optional interrupt pipe, EP0 control
      -> MongoosePro firmware

**STRONG INFERENCE:** у повному статичному control-flow аналізі dtmonpro.sys не знайдено J2534/CAN/ISO-TP command codec. Позитивно підтверджений код створює Device Interface, вибирає USB pipes, переносить ReadFile/WriteFile buffers між host і bulk pipes та обробляє п'ять службових IOCTL.

**CONFIRMED:** `monpj432.dll` містить істотну частину vehicle-protocol engine. Для ISO-15765 у DLL підтверджено validation flow-control filters, RX single/first/consecutive-frame dispatch, 12-bit length extraction, reassembly, sequence-number checking, timeout/abort state і host-side TX timeout/flow-control bookkeeping. Також у DLL є ISO-9141 checksum, ISO-14230 length та J1850 receive-status checks. Firmware усе ще виконує нижній bus/channel рівень, але попередня модель «усю ISO-TP логіку робить firmware» є неправильною.

## Інвентар і provenance

| Файл | Фактичний installed path | Analysis copy | SHA-256 | Version / arch | Authenticode |
|---|---|---|---|---|---|
| monpj432.dll | C:\Program Files (x86)\Drew Technologies, Inc\J2534\MongoosePro JLR\monpj432.dll | analysis/mongoosepro_jlr/binaries/monpj432.dll | 2710726FDD174F32B5E703E310CFCE047A663451EDBB3E35CA0C8EEFF6929413 | 1.1.16.0, x86 | Valid, Drew Technologies, Inc. |
| dtmonpro.sys | C:\Windows\System32\drivers\dtmonpro.sys | analysis/mongoosepro_jlr/binaries/dtmonpro.sys | C3A5B6DBDA2EB38CF4AA46C2C1660148F4FF0B7F51D96E1BCFE6875E29C12B4A | 1.1.0.0, x64 | Valid, Drew Technologies, Inc. |
| MPConfigApp.exe | C:\Program Files (x86)\Drew Technologies, Inc\J2534\MongoosePro JLR\MPConfigApp.exe | analysis/mongoosepro_jlr/binaries/MPConfigApp.exe | AB4F2E6632E30EE3AA433C0816CCC8AA5460DB9054C0F51612C2CBB4D7DD7C72 | 1.1.16.0, x86 | Valid, Drew Technologies, Inc. |
| j2534_loader.dll | C:\Program Files (x86)\Drew Technologies, Inc\J2534\MongoosePro JLR\j2534_loader.dll | analysis/mongoosepro_jlr/binaries/j2534_loader.dll | 2DB6388FBC85CC0214F0773FC0D0B1FBD0D259CC25F4C91A1CF300F779A5FC18 | 1.1.16.0, x86 | Valid, Drew Technologies, Inc. |
| monpjaguar.inf | C:\Windows\INF\oem29.inf | analysis/mongoosepro_jlr/binaries/monpjaguar.inf | 4775FF6619246898DDD51A221675EC83BC1901C73D821FE1A26AA1AA2D7B9A81 | text INF | covered by catalog |
| monpjaguar.cat | Driver Store/package source | analysis/mongoosepro_jlr/binaries/monpjaguar.cat | 88AFE6E4471A516DDBF8C06BAE3AFF621ACDB28D322ECFA596BDE17D39DA9A07 | catalog | Valid, Drew Technologies, Inc. |

Джерела: PE metadata у analysis/mongoosepro_jlr/metadata, Authenticode readback копій, INF DriverVer і MSI File table. Оригінали не модифікувались.

### Authenticode detail

**CONFIRMED on the analysis copies:** signer subject for DLL/SYS/EXE/CAT is `CN=Drew Technologies, Inc.`; issuer is `VeriSign Class 3 Code Signing 2010 CA`. The signing certificate validity interval is 2012-08-05 through 2015-09-18.

- `monpj432.dll`, `j2534_loader.dll`, `MPConfigApp.exe` and `monpjaguar.cat` carry a Symantec Time Stamping Services Signer G4 timestamp chain.
- `dtmonpro.sys` differs: its timestamp certificate is Symantec Time Stamping Services Signer G3, whose certificate expiry is 2013-01-01.
- `Get-AuthenticodeSignature` currently reports all five copies as `Valid`/`Signature verified`.

That user-mode Authenticode result does not imply that current Windows 11 kernel Code Integrity will admit this 2012 driver. It is consistent for a file signature to verify cryptographically while a newer kernel policy still rejects loading it; this report does not attempt to change or bypass that policy.

### PE imports/exports

- **monpj432.dll imports:** SETUPAPI, KERNEL32, ADVAPI32, USER32, WSOCK32, MFC/VC10 runtime. Критичні API: SetupDiGetClassDevsW, SetupDiEnumDeviceInterfaces, SetupDiGetDeviceInterfaceDetailW, CreateFileW, DeviceIoControl, ReadFile, WriteFile, CreateEventW, WaitForMultipleObjects, GetOverlappedResult, CancelIo.
- **dtmonpro.sys imports:** ntoskrnl.exe і WDFLDR.SYS; PE exports відсутні.
- **MPConfigApp.exe imports:** j2534_loader.dll, KERNEL32, ADVAPI32, USER32, COMCTL32, MFC/VC10 runtime.
- **j2534_loader.dll imports:** KERNEL32, ADVAPI32, USER32, MFC/VC10 runtime; exports — constructors/destructor/copy operator класу j2534device.

Повні import lists містяться у metadata/*.json; PE dumps — у analysis/mongoosepro_jlr/dumps/*.pe.txt.

The complete imported-module set for `monpj432.dll` is `ADVAPI32.dll`, `KERNEL32.dll`, `mfc100u.dll`, `MSVCP100.dll`, `MSVCR100.dll`, `SETUPAPI.dll`, `USER32.dll`, and `WSOCK32.dll`. All three non-system x86 VC10/MFC dependencies are presently available in `C:\Windows\SysWOW64` at version 10.00.40219.325; this was a read-only existence/version check, not a DLL load. `dtmonpro.sys` imports only `ntoskrnl.exe` and `WDFLDR.SYS`; current `WdfLdr.sys` is present under `C:\Windows\System32\drivers`.

## INF і binding

**CONFIRMED:** monpjaguar.inf:

- Hardware ID: USB\VID_18E1&PID_0104.
- Description: MongoosePro JLR J2534 Interface.
- Class: VehiclePassThru; class GUID {FB1CF0C4-B412-451F-9F04-DF7537A5003C}.
- Service: dtmonpro, SERVICE_KERNEL_DRIVER, demand start.
- Binary: %SystemRoot%\System32\drivers\dtmonpro.sys.
- DriverVer: 09/26/2012,1.1.0.0.
- KMDF library: 1.9.
- Manufacturer model is NTAMD64.

Evidence: monpjaguar.inf sections Version, DeviceList.NTAMD64, DriverInstall.NT.Services and DriverInstall.NT.Wdf.

The INF matches the whole USB device ID, not an interface-specific MI_xx ID. Hardware selection is done by PnP/INF; no VID/PID comparison was found inside driver code.

## monpj432.dll

PE image base is 0x10000000. PDB reference at RVA 0x92450, file offset 0x91850:

    U:\MongoosePro\trunk\windows_code\Release\monpj432.pdb

### Export table

| Ordinal | Export | RVA |
|---:|---|---:|
| 1 | PassThruOpen | 0x500C0 |
| 2 | PassThruClose | 0x51F70 |
| 3 | PassThruConnect | 0x525D0 |
| 4 | PassThruDisconnect | 0x53F60 |
| 5 | PassThruReadMsgs | 0x54580 |
| 6 | PassThruWriteMsgs | 0x55120 |
| 7 | PassThruStartPeriodicMsg | 0x55C00 |
| 8 | PassThruStopPeriodicMsg | 0x56570 |
| 9 | PassThruStartMsgFilter | 0x56B40 |
| 10 | PassThruStopMsgFilter | 0x579C0 |
| 11 | PassThruSetProgrammingVoltage | 0x57FE0 |
| 12 | PassThruReadVersion | 0x58C80 |
| 13 | PassThruGetLastError | 0x59990 |
| 14 | PassThruIoctl | 0x59D90 |
| 15 | PassThruFirmwareUpdate | 0x615E0 |
| 16 | PassThruReadDetails | 0x61F40 |
| 17 | PassThruSetIncomingMsgCallback | 0x60F30 |
| 21 | PassThruGetNextCarDAQ | 0x60200 |

Ordinals 18–20 are zero-RVA holes in the Export Address Table, not callable functions.

### Device discovery and open

**CONFIRMED:** the runtime Device Interface GUID is:

    {F99F6FCE-03F4-4817-8A03-8CFDE4743B92}

Evidence:

- GUID bytes in dtmonpro.sys at RVA 0x30C8, file offset 0x24C8.
- EvtDeviceAdd calls WdfDeviceCreateDeviceInterface at VA 0x17FA6 with that GUID and a NULL reference string.
- The same GUID is embedded in monpj432.dll at RVA 0x7F900, file offset 0x7ED00.
- monpj432.dll calls SetupDiGetClassDevsW at VA 0x1004B777 and SetupDiEnumDeviceInterfaces at 0x1004B7B4 with this GUID; more discovery call sites occur in the 0x10069FA0 and 0x10070ACE clusters.
- SetupDiGetDeviceInterfaceDetailW supplies the actual PnP device path, which is then passed to CreateFileW.

**CONFIRMED:** there is no fixed hard-coded symbolic device name. Windows generates the interface link; the DLL opens the path returned by SetupAPI.

### Win32/driver boundary

**CONFIRMED:**

- CreateFileW device open in the asynchronous transport cluster at VA 0x1006A0CB.
- WriteFile wrapper begins VA 0x1006A420; WriteFile call at 0x1006A473; OVERLAPPED state is stored in the transport object.
- ReadFile wrapper begins VA 0x1006A5B0; ReadFile call at 0x1006A604; maximum read request is 0x2000 bytes.
- Both wrappers use events, wait functions and GetOverlappedResult.
- Frame writer VA 0x1006E180 calls the WriteFile wrapper at 0x1006E221.
- Receive loop VA 0x1006E480 calls the ReadFile wrapper at 0x1006E4F1 and feeds the parser at 0x1006E590.
- Frame parser begins VA 0x1006B1C0.

For these .text addresses, file offset = RVA - 0xC00. Examples: frame writer file offset 0x6D580; parser file offset 0x6A5C0.

DeviceIoControl is not the main J2534 transport. Direct DLL calls at VA 0x1004BAAB and 0x1006F9B4 use only 0x5500601C to query USB string index 3. Commands such as open/channel/filter/write are sent through framed WriteFile data.

### Host-side ISO-15765 та legacy protocol logic

**CONFIRMED:** ISO-TP receive reassembly виконується в `monpj432.dll`:

- filter/message validation at VA 0x1001FCC0 checks equal pattern/flow-control sizes, equal TxFlags except padding, DataSize 5 with `ISO15765_ADDR_TYPE` and 4 without it;
- VA 0x10021070 parses a First Frame, extracts the 12-bit ISO-TP total length and rejects values beyond 0x0FFF;
- VA 0x10021280 handles Consecutive Frames, compares the PCI sequence nibble with the expected value, advances it modulo 16, copies only the remaining bytes and aborts/reset state on mismatch;
- VA 0x10021790 dispatches PCI type 0/1/2 and explicitly resets an interrupted segmented receive when a new Single Frame or First Frame arrives;
- VA 0x10021650 builds/delivers the completed J2534 receive message from the reassembly buffer.

**CONFIRMED:** the DLL also owns material ISO-TP transmit/control state:

- timeout handler VA 0x1001F6C0 distinguishes RX timeout from `Aborting ISO15675 Tx due to timeout` (the embedded string contains the vendor typo `ISO15675`);
- validation path logs `Cannot send segmented message, no matching Flow Control Filter` at VA 0x1001FC6A;
- ISO-15765-specific channel paths at VA 0x10022A90 and 0x10022FC0 search for a matching underlying CAN channel through matcher VA 0x1003E780;
- configuration names embedded in the DLL include `ISO15765_BS`, `ISO15765_STMIN`, `STMIN_TX`, `ISO15765_WFT_MAX`, `ISO15765_PAD_VALUE` and `ISO15765_FRAME_PAD`;
- RTTI includes `cHSISO15765` and `cSWISO15765` classes.

This proves host-side segmentation/reassembly orchestration; it does not prove that every microsecond-scale timer or CAN-frame transmit primitive is host-only. The lower CAN operation remains behind the firmware command protocol.

Other confirmed host checks include an ISO-9141 checksum-mismatch branch at VA 0x1002B6CF, ISO-14230 short-message checks at VA 0x1001DCCE/0x1002C93C, and J1850PWM receive-status handling at VA 0x10030392.

### J2534-to-command mapping

The table below distinguishes confirmed command names from higher-level mapping:

| J2534 export | Internal stream operation | Confidence/evidence |
|---|---|---|
| PassThruOpen | discovery/open; cOpenDevice (0x0003); board/version checks | STRONG INFERENCE from export RVA, cOpenDevice command table, SetupAPI/CreateFile cluster and automatic-update branch |
| PassThruClose | cCloseDevice (0x0005), handle/thread cleanup | STRONG INFERENCE |
| PassThruConnect | cOpenChannel (0x0006), followed by value/config commands | STRONG INFERENCE |
| PassThruDisconnect | cCloseChannel (0x0007) | STRONG INFERENCE |
| PassThruReadMsgs | drains host queue populated by cInboundData (0x0009)/cIndication (0x000A) receive dispatch | STRONG INFERENCE |
| PassThruWriteMsgs | message serializers at VA 0x1006B090/0x1006B120, cOutboundData (0x0008) | STRONG INFERENCE |
| Start/StopPeriodicMsg | table/value commands | WEAK INFERENCE for exact subcommand/record layout |
| Start/StopMsgFilter | cTableAddEntry / RemoveEntry / ModifyEntry / Clear (0x000D–0x0010) | STRONG INFERENCE; exact filter record fields remain partly unknown |
| SetProgrammingVoltage | cSetPin (0x0012) and/or value command | WEAK INFERENCE for exact opcode |
| ReadVersion/ReadDetails | cGetBoardStatus (0x0101), cGetBoardInfo (0x0109), cGetString (0x0013) | STRONG INFERENCE |
| PassThruIoctl | cIoctl (0x0011), cSetValue/cGetValue, cSetData/cGetData | STRONG INFERENCE; IOCTL-specific payload layouts are not all named |
| PassThruFirmwareUpdate | firmware command family 0x0102/0x0103/0x010A–0x010C/0x0111 | CONFIRMED path, dangerous; see MONGOOSE_FIRMWARE_PATH.md |

The command-name switch is function VA 0x1003B080. Its jump tables start at RVAs 0x3B274, 0x3B2CC, 0x3B314/0x3B358 and 0x3B458. A reproducible extractor is analysis/mongoosepro_jlr/scripts/protocol_tables.py.

## dtmonpro.sys

PE image base is 0x10000. PDB reference at RVA 0x3200, file offset 0x2600:

    u:\mongoosepro\trunk\usb_driver\sys\objfre_win7_amd64\amd64\dtmonpro.pdb

**CONFIRMED driver model:** KMDF 1.9.

- PE entry VA 0x12710, RVA 0x2710, file offset 0x1B10.
- WDF loader binding leads to WdfDriverCreate at VA 0x1A31E.
- WDF_DRIVER_CONFIG references EvtDeviceAdd at VA 0x179B4.
- WDF function table base VA 0x141C0 is corroborated by WdfDriverCreate index 116 and call signature.

EvtDeviceAdd creates the WDFDEVICE, USB target, queues and interface. Important callbacks:

| Function | VA |
|---|---:|
| EvtDeviceAdd | 0x179B4 |
| EvtIoRead | 0x11870 |
| EvtIoWrite | 0x11DF4 |
| EvtIoStop | 0x1236C |
| EvtIoDeviceControl | 0x18B9C |

Queue construction:

- default parallel DeviceControl queue at VA 0x17CCB–0x17D11;
- sequential Read queue at 0x17D59–0x17DA6 and dispatch assignment at 0x17E06;
- sequential Write queue at 0x17E4C–0x17E99 and dispatch assignment at 0x17EEA;
- manual queue at 0x17F24–0x17F62.

USB initialization and endpoint selection are detailed in MONGOOSE_USB_PROTOCOL.md. IOCTL dispatch is detailed in MONGOOSE_IOCTL_MAP.md.

## j2534_loader.dll

**CONFIRMED role:** generic x86 dynamic J2534 discovery/binding wrapper, not USB transport.

- Default constructor VA 0x10001120 enumerates registry and loads FunctionLibrary.
- Explicit-path constructor VA 0x10001310 calls LoadLibraryW at 0x10001353, then resolves J2534 exports with GetProcAddress.
- Registry enumeration VA 0x10001B20 opens HKLM\SOFTWARE\PassThruSupport.04.04 and reads FunctionLibrary and ConfigApplication.
- Config launcher VA 0x10002610 reads ConfigApplication and calls CreateProcessW at 0x100026A5. This only occurs when a caller requests that helper; it is not an installer action.

Resolved object slots:

| Offset | Function |
|---:|---|
| +0x00..+0x34 | standard PassThruOpen through PassThruIoctl |
| +0x38 | PassThruGetNextCarDAQ |
| +0x3C | PassThruFirmwareUpdate |
| +0x40 | PassThruReadDetails |
| +0x44 | HMODULE |

### Current J2534 registration (read-only snapshot)

The native 64-bit branch `HKLM\SOFTWARE\PassThruSupport.04.04` is absent. The x86 branch `HKLM\SOFTWARE\WOW6432Node\PassThruSupport.04.04` contains two distinct products:

- `Drew Technologies Inc. - Mongoose JLR` points to legacy `C:\Windows\SysWOW64\MONGJ432.DLL` and `MongConf.exe`;
- `Drew Technologies Inc. - MongoosePro JLR` points to `C:\Program Files (x86)\Drew Technologies, Inc\J2534\MongoosePro JLR\monpj432.dll` and `MPConfigApp.exe`.

For the Pro entry, confirmed DWORD capability values set to 1 are `CAN`, `CAN_PS`, `ISO14230`, `ISO14230_PS`, `ISO15765`, `ISO15765_PS`, `ISO9141`, `ISO9141_PS`, `J1850PWM`, and `UART_ECHO_BYTE_PS`. `Vendor` is `Drew Technologies Inc.` and `Name` is `MongoosePro JLR`.

This was a read-only registry query. It also confirms that `MONGJ432.dll` and `monpj432.dll` belong to different registered generations; they are not filename variants of the same Pro DLL.

## MPConfigApp and MSI

MPConfigApp uses j2534_loader rather than implementing USB transport. Firmware path evidence is in MONGOOSE_FIRMWARE_PATH.md.

**CONFIRMED:** static MSI database inspection shows:

- File table installs monpj432.dll, dtmonpro.sys, MPConfigApp.exe, j2534_loader.dll, INF/CAT, KMDF coinstaller and VC10/MFC runtime.
- Registry table writes FunctionLibrary, ConfigApplication, Vendor, Name and protocol capability values under PassThruSupport.04.04.
- CustomAction table contains only WiX UI helpers, downgrade prevention, DIFxApp driver processing/install/uninstall/rollback/cleanup and VC runtime folder setters.
- InstallExecuteSequence contains file/registry/shortcut/driver installation actions; it contains no MPConfig launch, PassThruOpen, firmware action or generic arbitrary EXE action.

Therefore the 1.1.16 MSI itself does not automatically invoke firmware update. This conclusion is scoped to the analysed MSI SHA/content; it does not make running MPConfig or PassThruOpen safe.

## Де розміщена protocol logic

| Layer | Confirmed responsibility | Not proven |
|---|---|---|
| monpj432.dll | J2534 exports, PnP discovery, device handle, stream framing/resync, command structures, async thread/queues, timeouts, response dispatch, ISO-15765 filter validation/RX reassembly/TX control state, legacy-protocol validation, version checks, firmware update client | exact ownership of every low-level timer and frame-send primitive |
| dtmonpro.sys | KMDF PnP/power, USB select/config, bulk Read/Write forwarding, optional interrupt reader, serial/config descriptor IOCTL, reset/vendor control | CAN/J2534 translation — no evidence found |
| firmware | accepts named open/channel/value/table/data/board/reflash commands, performs lower bus/channel operations and returns command responses | internal CAN-controller code and the remainder of the host/device ISO-TP split cannot be inspected from high-entropy resource payloads |

**Result:** dtmonpro.sys is a thin transport with small device-management additions. It is not wholly transparent because it owns USB configuration, queues, reset and one vendor control request, but it does not implement the J2534 protocol engine.

## Reproducibility artifacts

- analysis/mongoosepro_jlr/binaries — immutable analysis copies.
- analysis/mongoosepro_jlr/dumps — PE headers and full disassembly.
- analysis/mongoosepro_jlr/metadata — PE/import/export/string/xref JSON.
- analysis/mongoosepro_jlr/scripts/pe_static.py — PE parser.
- analysis/mongoosepro_jlr/scripts/xref_disasm.py — xref/disassembly helper.
- analysis/mongoosepro_jlr/scripts/protocol_tables.py — opcode table extractor.
- analysis/mongoosepro_jlr/reports/extracted_resources — extracted firmware/bootloader RCDATA.

## Статично невідоме

- actual USB configuration/interface/endpoint addresses and maximum packet sizes;
- whether usbser COM3 carries the same stream;
- exact semantic names of IOCTL 0x55006018 and vendor request 0xDA;
- exact structure of every command payload and J2534 protocol-specific value ID;
- exact ownership of low-level ISO-TP transmit timing/CAN-frame emission after the confirmed host-side segmentation, reassembly and flow-control orchestration;
- response timing, retry behavior and device quirks;
- firmware payload cipher/compression/integrity algorithm.

These cannot be promoted to confirmed facts without USB descriptors/captures or analysis of a decoded firmware image. No such physical testing was performed.
