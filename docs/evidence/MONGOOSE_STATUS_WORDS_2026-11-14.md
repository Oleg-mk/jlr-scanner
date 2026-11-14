# The MongoosePro JLR firmware's status words, from the vendor library's own table

Date: 2026-11-14. Static, read-only: the installed vendor library
(`monpj432.dll`, 1.1.16, the file the research of 2026-08 analysed) was
read as bytes; nothing was executed, no adapter was touched. Written the
morning after the third real-car session left one word unexplained: the
`0x00000106` the adapter's indication carried after every transmit.

## Method

The library has a function that turns a status word into its name, for its
own log lines. It is compiled as a switch over four ranges, each a jump
table of `mov eax, <wide string>; ret` stubs (`.text`, file offsets
`0x37010`–`0x37250`, image base `0x10000000`). The dispatch compares the
word with `0x100`, `0x200`, `0x300`, subtracts the range's base and indexes
its table. Reading the tables back gives the mapping below; the script that
did it is a few dozen lines of Python over the PE sections and is not part
of the product.

## The table

Range 0, no family word: the J2534 error numbers, then the firmware's own.

| Code | Name | Code | Name |
| ---: | --- | ---: | --- |
| 0x00 | eSuccess | 0x1B | eMsgDiscardedQueueFull |
| 0x01 | eNotSupported | 0x1C | iMsgDiscardedMsgError |
| 0x02 | eInvalidChannelId | 0x1D | iMsgDiscardedTimeout |
| 0x03 | eInvalidProtocolID | 0x1E | iMsgDiscardedNoAck |
| 0x04 | eNullParameter | 0x1F | iMsgDiscardedBusFault |
| 0x05 | eInvalidIoctlValue | 0x20 | iMsgTxDone |
| 0x06 | eInvalidFlags | 0x21 | iMsgRxStart |
| 0x07 | eFailed | 0x22 | iMsgRxBreakStart |
| 0x08 | eDeviceNotConnected | 0x23 | iMsgRxBreakStop |
| 0x09 | eTimeout | 0x24 | iMsgTxDonePeriodic |
| 0x0A | eInvalidMsg | 0x25 | (none) |
| 0x0B | eInvalidTimeInterval | 0x26 | eMsgDiscardedNoFlowFilter |
| 0x0C | eExceededLimit | 0x27 | iMsgTxFCTimeout |
| 0x0D | eInvalidMsgId | 0x28 | iMsgTxDoneFC |
| 0x0E | eDeviceInUse | 0x29 | iMsgTxDoneSegment |
| 0x0F | eInvalidIoctlId | 0x2A | iMsgRxCFTimeout |
| 0x10 | eBufferEmpty | 0x2B | iMsgRxCFInvalid |
| 0x11 | eBufferFull | 0x2C | iMsgTxFFTimeout |
| 0x12 | eBufferOverflow | 0x2D | iMsgTxCFTimeout |
| 0x13 | ePinInvalid | 0x2E | iMsgTxSFTimeout |
| 0x14 | eChannelInUse | 0x2F | iMsgRxFCTimeout |
| 0x15 | eMsgProtocolId | 0x30 | iSingleWireChangeHS |
| 0x16 | eInvalidFilterId | 0x31 | iSingleWireChangeNS |
| 0x17 | eNoFlowControl | 0x32 | iMsgTimeout |
| 0x18 | eNotUnique | 0x33 | iMsgRxQueueOvflw |
| 0x19 | eInvalidBaudRate | 0x34–0x3B | eInvalidEntry, eInvalidValue, eInvalidMsgLength, eOutOfBuffers, ePinConflict, ePinNotSupported, eQueueFull, eInitTimeout |
| 0x1A | eInvalidDeviceId | 0x3C–0x42 | eFiveBaudInitFailed, eVbattLoss, eReflashWrongBoardType, eReflashInvalidImage, eReflashNoResponse, eReflashInvalidState, eReflashInvalidParameter |

Range 1, family `0x1xx` — the message family: `0x100` is compared for
exactly and names **eMsgQueued**; the table runs from `0x101`.

| Code | Name | Code | Name |
| ---: | --- | ---: | --- |
| **0x100** | **eMsgQueued** | 0x115 | iMsgRxFCTimeout |
| 0x101 | eMsgDiscardedQueueFull | 0x116 | iSingleWireChangeHS |
| 0x102 | iMsgDiscardedMsgError | 0x117 | iSingleWireChangeNS |
| 0x103 | iMsgDiscardedTimeout | 0x118 | iMsgTimeout |
| 0x104 | iMsgDiscardedNoAck | 0x119 | iMsgRxQueueOvflw |
| 0x105 | iMsgDiscardedBusFault | 0x11A | eInvalidEntry |
| **0x106** | **iMsgTxDone** | 0x11B | eInvalidValue |
| 0x107 | iMsgRxStart | 0x11C | eInvalidMsgLength |
| 0x108 | iMsgRxBreakStart | 0x11D | eOutOfBuffers |
| 0x109 | iMsgRxBreakStop | 0x11E | ePinConflict |
| 0x10A | iMsgTxDonePeriodic | 0x11F | ePinNotSupported |
| 0x10B | (none) | 0x120 | eQueueFull |
| 0x10C | eMsgDiscardedNoFlowFilter | 0x121 | eInitTimeout |
| 0x10D | iMsgTxFCTimeout | 0x122 | eFiveBaudInitFailed |
| 0x10E | iMsgTxDoneFC | 0x123 | eVbattLoss |
| 0x10F | iMsgTxDoneSegment | 0x124 | eReflashWrongBoardType |
| 0x110 | iMsgRxCFTimeout | 0x125 | eReflashInvalidImage |
| 0x111 | iMsgRxCFInvalid | 0x126 | eReflashNoResponse |
| 0x112 | iMsgTxFFTimeout | 0x127 | eReflashInvalidState |
| 0x113 | iMsgTxCFTimeout | 0x128 | eReflashInvalidParameter |
| 0x114 | iMsgTxSFTimeout | | |

Range 2, family `0x2xx` — channel and configuration: `0x201` eInvalidEntry,
`0x202` eInvalidValue, `0x203` eInvalidMsgLength, `0x204` eOutOfBuffers,
`0x205` ePinConflict, **`0x206` ePinNotSupported**, `0x207` eQueueFull,
**`0x208` eInitTimeout**, `0x209` eFiveBaudInitFailed, `0x20A` eVbattLoss,
`0x20B`–`0x20F` the five reflash errors.

Range 3, family `0x3xx` — reflash: `0x301` eReflashWrongBoardType, `0x302`
eReflashInvalidImage, `0x303` eReflashNoResponse, `0x304`
eReflashInvalidState, `0x305` eReflashInvalidParameter.

The names with an `e` are the firmware's answers to a command; the names
with an `i` are what it reports on its own, in a `cIndication`. The
library's own log lines say as much: "Waiting for start filter response",
"Firmware reported a bus fault (bus off?) during transmit", "Firmware
refused to transmit message because of no flow control filter", "Firmware
sent an iMsgRxQueueOvflw indicating that it lost a message", "respFifo
unexpected type of cIndication".

## Every status this project has seen, named

| Observed | Where | Name | Fits |
| --- | --- | --- | --- |
| `0x003` with "cOpenChannel: Unsupported or Invalid Resource ID" | bench 2026-09-06 | eInvalidProtocolID | yes: a resource is a protocol to the library |
| `0x206` with "SetPins: … only supports CAN on pins 6 and 14" | bench 2026-09-06 | ePinNotSupported | yes, word for word |
| `0x208` with "Fast Init: Timeout on response" | bench 2026-09-12 | eInitTimeout | yes, word for word |
| `0x100` on `0x8008`, no text, after every transmit | bench and both cars | eMsgQueued | yes: the transmit was queued — the acknowledgement, as read since `ADR-0042` |
| `0x106` in the `cIndication` after every transmit | the tester's car, 2026-11-13, both buses | **iMsgTxDone** | the transmit completed |
| `0x105`, `0x10C` | the sister product's open client, as "bus off" and "no flow-control filter" | iMsgDiscardedBusFault, eMsgDiscardedNoFlowFilter | yes, both; its `0x121` "no source address" is eInitTimeout here — the two libraries' tables part at the tail |

Five independent matches, three of them word for word against the
firmware's own texts. The table is `CONFIRMED` for this library's build.

## What it settles

1. **`0x100` means queued.** The acknowledgement says the firmware took the
   frame, and nothing about the bus — which is what the bare adapter showed
   (§10 of the audit) and what `ADR-0042` assumed.
2. **`0x106` means the transmit completed.** A CAN frame completes only
   when another node on the bus acknowledges it: with no node the
   controller never completes it (the bare adapter sent no indication at
   all), and a bus at the wrong rate ends in `iMsgDiscardedBusFault` or
   `iMsgDiscardedNoAck`, not in TxDone. So on 2026-11-13 the tester's
   Discovery 4 acknowledged all 45 frames, 16 on HS-CAN at 500 kbit/s and
   29 on MS-CAN at 125 kbit/s: the adapter was on both buses, at the right
   rate, through the cable, the driver and this application's own frames.
   The second reading of the audit's §11 — a transmit that fails — is out.
3. **What is left is reception.** The firmware forwarded no received frame
   to the host on either bus, and none to a listen-only channel on HS-CAN
   with the ignition on. The sister product's client states the rule for
   its firmware — a pass filter per identifier, or you receive nothing —
   and nothing observed on this adapter contradicts it. The filter table
   entry (`cTableAddEntry`) is the next work, by its own record.
4. The next build should print these names beside the words in every
   trace, so a session file reads "0x100 eMsgQueued … 0x106 iMsgTxDone"
   without this document in hand.

## Validation

| Item | State |
| --- | --- |
| The table, code to name, for library 1.1.16 | `CONFIRMED` (static, five matches) |
| `0x100` = queued, `0x106` = transmit completed, as the firmware's meaning | `CONFIRMED` by the table; the firmware's behaviour on the bus behind the words is the library's reading, not a captured bus |
| The tester's car on both buses at the right rate | `VEHICLE_CONFIRMED` through this reading (45 completed transmits) |
| Reception without a filter table entry | `NOT_YET_VALIDATED`; nothing received so far |
| The filter record's bytes | unknown; the capture of the library's own session on the owner's bench, or the static reading of its builder, settles it |
