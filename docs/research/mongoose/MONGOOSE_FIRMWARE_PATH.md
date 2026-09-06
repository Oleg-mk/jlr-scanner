# MongoosePro JLR firmware/update path — static analysis

## Safety statement

No updater, MPConfigApp, DLL export, USB request or firmware command was executed. This document identifies dangerous paths so they can be avoided.

## Executive result

- **CONFIRMED:** firmware and bootloader images are embedded inside monpj432.dll, not MPConfigApp.exe.
- **CONFIRMED:** MPConfigApp is a UI/client that calls exported PassThruFirmwareUpdate through j2534_loader.dll.
- **CONFIRMED:** PassThruOpen contains an automatic firmware-update branch when its version/state checks say an update is required.
- **CONFIRMED:** the analysed 1.1.16 MSI does not launch MPConfigApp, PassThruOpen or any firmware custom action during installation.

Therefore:

- installer alone: no automatic firmware update path found;
- running MPConfig and selecting update: explicit update path exists;
- calling PassThruFirmwareUpdate: explicit update path exists;
- calling PassThruOpen: can enter an automatic update path and is not a read-only probe.

## MPConfigApp UI to DLL

MPConfigApp.exe is x86, image base 0x400000.

**CONFIRMED call path:**

    firmware-update UI handler (VA 0x00403690)
      -> CreateThread at VA 0x00403733
      -> worker VA 0x004035C0
      -> j2534device object [esi+0x24]
      -> function pointer [object+0x3C]
      -> PassThruFirmwareUpdate call at VA 0x00403621
      -> callback VA 0x00403530

j2534_loader.dll stores PassThruFirmwareUpdate at object offset +0x3C. This independently identifies the indirect call.

The string Starting Firmware Update at MPConfig VA 0x0040BC88 is referenced by the progress callback path near VA 0x0040377A.

For MPConfig .text, file offset = RVA - 0xC00. The worker VA 0x004035C0 therefore maps to file offset 0x29C0.

**Result:** MPConfig does not implement a separate USB updater. It delegates to monpj432.dll.

## Explicit DLL updater

monpj432.dll exports:

    PassThruFirmwareUpdate, ordinal 15, RVA 0x615E0, file offset 0x609E0

The update implementation uses the normal Mongoose framed WriteFile/ReadFile transport and the firmware command family described below.

## Automatic updater inside PassThruOpen

PassThruOpen begins at RVA 0x500C0.

**CONFIRMED branch evidence:**

- around VA 0x10051080, state bytes at device object +0x109 and +0x10A are checked;
- a Bluetooth-specific block at 0x1005109E–0x1005112D refuses update and directs the user to USB;
- string Attempting automatic firmware update is used at VA 0x1005113B;
- updater/UI context is constructed around 0x10051158;
- CFirmwareUpdate-like object construction call at 0x10051178 enters the update workflow.

Related strings in monpj432.dll include:

- Requires firmware update...
- Attempting automatic firmware update
- Bluetooth ... cannot update ... use USB

**CONFIRMED conclusion:** PassThruOpen is not guaranteed to be a harmless handle-open. If the DLL's version/status logic sets the update-required state, it can initiate firmware update automatically.

This is why PassThruOpen was not called.

## Embedded image resources

monpj432.dll resource loader is VA 0x1000A1D0:

- selector 1 chooses RCDATA ID 5005 (0x138D) at VA 0x1000A226;
- selector 0 chooses RCDATA ID 5006 (0x138E) at VA 0x1000A22D;
- FindResourceW(type 10), SizeofResource, LoadResource and LockResource calls occur at 0x1000A239, 0x1000A292, 0x1000A2F4 and 0x1000A35A.

### RCDATA 5005 — firmware

| Property | Value |
|---|---|
| DLL resource RVA / file offset | 0x236CE0 / 0x2338E0 |
| total bytes | 112,780 |
| extracted file | analysis/mongoosepro_jlr/reports/extracted_resources/monpj432_rcdata_5005.bin |
| whole-resource SHA-256 | 30C679B73BBADB32B79DF4554EF226E9B33545C141A15CFF7FDB62C897142B4E |
| payload bytes after 0x6C header | 112,672 |
| payload SHA-256 | 81775DC5B910C9B3944594732FA0E762D3EBF73387961A3C019A4C103A13E106 |
| payload entropy | 7.9982 bits/byte |
| embedded name | MongoosePro Jaguar Firmware |
| version bytes at header +0x08 | 00 10 01 01 |

Version bytes read naturally as components 1.1.16.0 in reverse display order; this is a **STRONG INFERENCE** corroborated by DLL version 1.1.16.0.

### RCDATA 5006 — bootloader

| Property | Value |
|---|---|
| DLL resource RVA / file offset | 0x25256C / 0x24F16C |
| total bytes | 19,876 |
| extracted file | analysis/mongoosepro_jlr/reports/extracted_resources/monpj432_rcdata_5006.bin |
| whole-resource SHA-256 | FB0AC39612655D5449647D8E00CDBA318C6682BC75D25CA3FC383180C739DD3B |
| payload bytes after 0x6C header | 19,768 |
| payload SHA-256 | 4013B9D4C52AFAA9CEC46CF04593FAF92B2BC37E6A467616DDF7CAF2D4891A8E |
| payload entropy | 7.9899 bits/byte |
| embedded name | MongoosePro Jaguar Bootloader |
| version bytes at header +0x08 | 00 08 01 01 |

The version interpretation 1.1.8.0 is a **STRONG INFERENCE**.

## Image container format

Both resources start with a 0x6C-byte custom header:

| Offset | Size | Firmware 5005 | Bootloader 5006 | Interpretation |
|---:|---:|---:|---:|---|
| 0x00 | u32 LE | 0x6C | 0x6C | header size, CONFIRMED by payload boundary/use |
| 0x04 | u16 LE | 2 | 1 | image type discriminator, STRONG INFERENCE |
| 0x06 | u16 LE | 5 | 5 | product/family ID, WEAK/STRONG INFERENCE |
| 0x08 | 4 bytes | 00 10 01 01 | 00 08 01 01 | version tuple, STRONG INFERENCE |
| 0x0C | 64 bytes | ASCII firmware name | ASCII bootloader name | CONFIRMED |
| 0x4C | u32 LE | 0x000055D8 | 0x0000B0B4 | checksum/metadata field, semantic UNKNOWN |
| 0x50 | 28 bytes | opaque | opaque | likely integrity/crypto material, algorithm UNKNOWN |
| 0x6C | remaining | high-entropy payload | high-entropy payload | encrypted/compressed/obfuscated; exact form UNKNOWN |

The payloads have near-maximum entropy and no conventional executable/image magic at the payload start. Static evidence cannot distinguish encryption from strong compression or recover the MCU memory map.

### Negative format-identification tests

To avoid over-identifying the opaque fields, the analysis copies were checked without modifying them:

- the 28 bytes at +0x50 do **not** equal SHA-224 of the payload, the first 0x50 header bytes, the complete resource, or the resource with the 28-byte field zeroed/omitted;
- the u32 at +0x4C does **not** equal CRC32 or Adler32 of the payload;
- its low u16 does **not** equal CRC-CCITT with initial 0/0xFFFF or Modbus CRC16 of the payload.

These negative results do not rule out a signature fragment, encrypted key/IV material, a checksum over decoded plaintext, a vendor CRC variant or non-integrity metadata. The correct classification remains **UNKNOWN**.

## Firmware/bootloader command family

Command-name switch: monpj432.dll VA 0x1003B080.

| Request | Name | Response | Risk |
|---:|---|---:|---|
| 0x0102 | cResetBoard | 0x8102 | resets device state |
| 0x0103 | cJumpToFirmware | 0x8103 | changes execution state |
| 0x0109 | cGetBoardInfo | 0x8109 | nominally read-like |
| 0x010A | cReflashBoard | 0x810A | writes firmware |
| 0x010B | cWriteSerialNumber | 0x810B | persistent identity write |
| 0x010C | cUnprotectBootloader | 0x810C | removes bootloader protection |
| 0x0111 | cCheckCRN | 0x8111 | integrity/authorization check, exact meaning UNKNOWN |
| 0x0112 | cUpdateBTModule | 0x8112 | writes Bluetooth module firmware |

Literal string RVAs and file offsets:

- cJumpToFirmware: RVA 0x8B6B0, file 0x8AAB0;
- cReflashBoard: RVA 0x8B764, file 0x8AB64;
- cWriteSerialNumber: RVA 0x8B780, file 0x8AB80;
- cUnprotectBootloader: RVA 0x8B7A8, file 0x8ABA8.

Simple command builders:

- cJumpToFirmware builder VA 0x1003E820;
- cResetBoard builder VA 0x1003E910;
- cGetBoardInfo builder VA 0x1003F160;
- cUnprotectBootloader builder VA 0x1003F270.

The update path also contains progress/wait strings for reflash start and continuation. Exact reflash block format, addressing and acknowledgement rules are not fully proven and are intentionally not presented as an executable procedure.

## USB control request versus protocol command

dtmonpro.sys exposes IOCTL 0x5500A00C, which sends EP0 vendor request:

    bmRequestType 0x40, bRequest 0xDA,
    wValue 0, wIndex 0, wLength 0

**UNKNOWN:** whether this is a bootloader transition. No direct DeviceIoControl(0x5500A00C) caller was found in this monpj432.dll, while named firmware transitions are definitely encoded as stream opcodes. It is unsafe to equate bRequest 0xDA with cJumpToFirmware without additional evidence.

## Installer behavior

The analysed MSI was opened only as a Windows Installer database:

    .tmp-mongoosepro-jlr-1.1.16\J2534_MongoosePro_JLR_x64.msi
    SHA-256 4DC6D9F2C7F4379B8AF4DCECF8A79EF3BA9D5FD190F7CD689C6CD0CAC555F96F

**CONFIRMED absence of automatic updater action:**

- CustomAction contains WiX UI validation/EULA, downgrade prevention, DIFxApp driver install/uninstall/rollback/cleanup, and VC runtime folder setters.
- InstallExecuteSequence has InstallFiles, driver processing, shortcuts, registry writes and product publication only.
- InstallUISequence ends by ExecuteAction/ExitDialog; no MPConfig launch action exists.
- Shortcut table merely creates MongoosePro JLR Configuration and manual shortcuts.

The MSI writes:

- FunctionLibrary = installed monpj432.dll;
- ConfigApplication = installed MPConfigApp.exe;
- capability values for CAN, ISO15765, ISO14230, ISO9141, J1850PWM and related PS variants.

Registration makes the applications discoverable; it does not call them.

## Final risk classification

| Action | Static firmware-update risk |
|---|---|
| install analysed MSI | no automatic firmware action found |
| load DLL only | loader initialization not proven to update, but not executed here |
| PassThruOpen | **YES: automatic update branch confirmed** |
| launch MPConfigApp | exposes explicit update UI/path |
| PassThruFirmwareUpdate | **YES: explicit updater** |
| normal driver ReadFile/WriteFile | depends entirely on bytes sent |
| IOCTL 0x5500A00C | unknown state-changing vendor request; avoid |

No firmware operation was performed.
