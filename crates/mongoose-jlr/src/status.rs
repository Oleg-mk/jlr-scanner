//! The firmware's status words by name (ADR-0043, decision 6).
//!
//! The vendor library (`monpj432.dll` 1.1.16) names every status word for its
//! own log lines in one function, a switch over four ranges; read back from
//! its bytes on 2026-11-14 (`docs/evidence/MONGOOSE_STATUS_WORDS_2026-11-14.md`)
//! it named every status this project had seen, three of them word for word
//! against the firmware's own texts. The table is that library build's. It is
//! data about the adapter, not knowledge of a vehicle.

/// Range 0 of the library's switch: the J2534 error numbers, then the
/// firmware's own, `0x00` to `0x42`. `0x25` has no name in the library.
const RANGE_0: [Option<&str>; 0x43] = [
    Some("eSuccess"),
    Some("eNotSupported"),
    Some("eInvalidChannelId"),
    Some("eInvalidProtocolID"),
    Some("eNullParameter"),
    Some("eInvalidIoctlValue"),
    Some("eInvalidFlags"),
    Some("eFailed"),
    Some("eDeviceNotConnected"),
    Some("eTimeout"),
    Some("eInvalidMsg"),
    Some("eInvalidTimeInterval"),
    Some("eExceededLimit"),
    Some("eInvalidMsgId"),
    Some("eDeviceInUse"),
    Some("eInvalidIoctlId"),
    Some("eBufferEmpty"),
    Some("eBufferFull"),
    Some("eBufferOverflow"),
    Some("ePinInvalid"),
    Some("eChannelInUse"),
    Some("eMsgProtocolId"),
    Some("eInvalidFilterId"),
    Some("eNoFlowControl"),
    Some("eNotUnique"),
    Some("eInvalidBaudRate"),
    Some("eInvalidDeviceId"),
    // 0x1B: from here on the firmware's own words, which ranges 1 to 3
    // repeat under their family.
    Some("eMsgDiscardedQueueFull"),
    Some("iMsgDiscardedMsgError"),
    Some("iMsgDiscardedTimeout"),
    Some("iMsgDiscardedNoAck"),
    Some("iMsgDiscardedBusFault"),
    Some("iMsgTxDone"),
    Some("iMsgRxStart"),
    Some("iMsgRxBreakStart"),
    Some("iMsgRxBreakStop"),
    Some("iMsgTxDonePeriodic"),
    None,
    Some("eMsgDiscardedNoFlowFilter"),
    Some("iMsgTxFCTimeout"),
    Some("iMsgTxDoneFC"),
    Some("iMsgTxDoneSegment"),
    Some("iMsgRxCFTimeout"),
    Some("iMsgRxCFInvalid"),
    Some("iMsgTxFFTimeout"),
    Some("iMsgTxCFTimeout"),
    Some("iMsgTxSFTimeout"),
    Some("iMsgRxFCTimeout"),
    Some("iSingleWireChangeHS"),
    Some("iSingleWireChangeNS"),
    Some("iMsgTimeout"),
    Some("iMsgRxQueueOvflw"),
    // 0x34
    Some("eInvalidEntry"),
    Some("eInvalidValue"),
    Some("eInvalidMsgLength"),
    Some("eOutOfBuffers"),
    Some("ePinConflict"),
    Some("ePinNotSupported"),
    Some("eQueueFull"),
    Some("eInitTimeout"),
    Some("eFiveBaudInitFailed"),
    Some("eVbattLoss"),
    // 0x3E
    Some("eReflashWrongBoardType"),
    Some("eReflashInvalidImage"),
    Some("eReflashNoResponse"),
    Some("eReflashInvalidState"),
    Some("eReflashInvalidParameter"),
];

/// The name the vendor library gives a status word, or `None` for a word its
/// table does not have.
///
/// Family `0x1xx` is the message family: `0x100` is compared for exactly
/// (eMsgQueued) and `0x101`–`0x128` run as range 0 from `0x1B`. Family
/// `0x2xx`, channel and configuration, runs `0x201`–`0x20F` as range 0 from
/// `0x34`; family `0x3xx`, reflash, `0x301`–`0x305` as range 0 from `0x3E`.
pub fn status_name(status: u32) -> Option<&'static str> {
    let from_range_0 = |index: u32| RANGE_0.get(usize::try_from(index).ok()?).copied()?;
    match status {
        0x000..=0x042 => from_range_0(status),
        0x100 => Some("eMsgQueued"),
        0x101..=0x128 => from_range_0(status - 0x100 + 0x1A),
        0x201..=0x20F => from_range_0(status - 0x200 + 0x33),
        0x301..=0x305 => from_range_0(status - 0x300 + 0x3D),
        _ => None,
    }
}

/// A status word as every trace and error says it: `0x00000106 iMsgTxDone`,
/// or `0x00000200 (unnamed)` for a word the library's table does not have.
pub fn status_words(status: u32) -> String {
    match status_name(status) {
        Some(name) => format!("{status:#010X} {name}"),
        None => format!("{status:#010X} (unnamed)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every status this project has seen, by the evidence document's own
    /// table, and the sister product's two.
    #[test]
    fn every_status_seen_has_the_librarys_name() {
        for (status, name) in [
            (0x000, "eSuccess"),
            (0x003, "eInvalidProtocolID"),
            (0x100, "eMsgQueued"),
            (0x105, "iMsgDiscardedBusFault"),
            (0x106, "iMsgTxDone"),
            (0x10C, "eMsgDiscardedNoFlowFilter"),
            (0x202, "eInvalidValue"),
            (0x203, "eInvalidMsgLength"),
            (0x206, "ePinNotSupported"),
            (0x208, "eInitTimeout"),
        ] {
            assert_eq!(status_name(status), Some(name), "{status:#X}");
        }
    }

    /// The edges of each range, as the evidence document lists them.
    #[test]
    fn the_ranges_run_as_the_library_lays_them_out() {
        for (status, name) in [
            (0x01A, "eInvalidDeviceId"),
            (0x01B, "eMsgDiscardedQueueFull"),
            (0x033, "iMsgRxQueueOvflw"),
            (0x042, "eReflashInvalidParameter"),
            (0x101, "eMsgDiscardedQueueFull"),
            (0x119, "iMsgRxQueueOvflw"),
            (0x11A, "eInvalidEntry"),
            (0x121, "eInitTimeout"),
            (0x128, "eReflashInvalidParameter"),
            (0x201, "eInvalidEntry"),
            (0x20A, "eVbattLoss"),
            (0x20B, "eReflashWrongBoardType"),
            (0x20F, "eReflashInvalidParameter"),
            (0x301, "eReflashWrongBoardType"),
            (0x305, "eReflashInvalidParameter"),
        ] {
            assert_eq!(status_name(status), Some(name), "{status:#X}");
        }
    }

    /// Words the table does not have stay unnamed, never guessed: the
    /// unnamed slots of ranges 0 and 1, and the table-type refusal's `0x200`.
    #[test]
    fn a_word_outside_the_table_is_unnamed() {
        for status in [
            0x025, 0x043, 0x0FF, 0x10B, 0x129, 0x200, 0x210, 0x300, 0x306,
        ] {
            assert_eq!(status_name(status), None, "{status:#X}");
        }
        assert_eq!(status_words(0x106), "0x00000106 iMsgTxDone");
        assert_eq!(status_words(0x200), "0x00000200 (unnamed)");
    }
}
