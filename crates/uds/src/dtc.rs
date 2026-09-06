//! ISO 14229-1 ReadDTCInformation decoding for the one report the application
//! reads. Decoding only: the outbound surface stays the narrow constructor in
//! the crate root.

use crate::UdsError;

/// ReadDTCInformation sub-function `reportDTCByStatusMask`.
pub const SUB_FUNCTION_REPORT_DTC_BY_STATUS_MASK: u8 = 0x02;

/// One DTC as the module reports it: the three-byte code and its status byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DtcRecord {
    pub code: [u8; 3],
    pub status: u8,
}

impl DtcRecord {
    /// The SAE J2012 five-character code carried by the high and middle
    /// bytes, such as `P0301`.
    pub fn j2012_code(&self) -> String {
        let system = match self.code[0] >> 6 {
            0 => 'P',
            1 => 'C',
            2 => 'B',
            _ => 'U',
        };
        format!(
            "{system}{}{:X}{:02X}",
            (self.code[0] >> 4) & 0x3,
            self.code[0] & 0xF,
            self.code[1]
        )
    }

    /// The low byte: the failure type byte of ISO 14229-1 / SAE J2012-DA.
    pub fn failure_type_byte(&self) -> u8 {
        self.code[2]
    }

    /// Code and failure type together, as `P0301-00`.
    pub fn code_with_failure_type(&self) -> String {
        format!("{}-{:02X}", self.j2012_code(), self.code[2])
    }
}

/// Positive response to `reportDTCByStatusMask`, after the sub-function byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DtcStatusReport {
    pub status_availability_mask: u8,
    pub records: Vec<DtcRecord>,
}

/// Decode the data that follows the sub-function byte of a positive
/// `reportDTCByStatusMask` response: one availability mask, then four bytes
/// per DTC. A trailing partial record is malformed, not truncated silently.
pub fn decode_dtc_by_status_mask(data: &[u8]) -> Result<DtcStatusReport, UdsError> {
    let (&status_availability_mask, rest) = data
        .split_first()
        .ok_or(UdsError::MalformedPositiveResponse)?;
    if rest.len() % 4 != 0 {
        return Err(UdsError::MalformedPositiveResponse);
    }
    let records = rest
        .chunks_exact(4)
        .map(|chunk| DtcRecord {
            code: [chunk[0], chunk[1], chunk[2]],
            status: chunk[3],
        })
        .collect();
    Ok(DtcStatusReport {
        status_availability_mask,
        records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_codes_across_all_four_systems() {
        let report = decode_dtc_by_status_mask(&[
            0xFF, 0x03, 0x01, 0x00, 0x08, 0x41, 0x23, 0x45, 0x2F, 0x9A, 0xBC, 0xDE, 0x01, 0xC1,
            0x00, 0x00, 0x04,
        ])
        .unwrap();
        assert_eq!(report.status_availability_mask, 0xFF);
        let codes: Vec<_> = report
            .records
            .iter()
            .map(DtcRecord::code_with_failure_type)
            .collect();
        assert_eq!(codes, ["P0301-00", "C0123-45", "B1ABC-DE", "U0100-00"]);
        assert_eq!(report.records[0].status, 0x08);
        assert_eq!(report.records[3].failure_type_byte(), 0x00);
    }

    #[test]
    fn an_empty_report_has_a_mask_and_no_records() {
        let report = decode_dtc_by_status_mask(&[0x09]).unwrap();
        assert_eq!(report.status_availability_mask, 0x09);
        assert!(report.records.is_empty());
    }

    #[test]
    fn partial_records_and_missing_masks_are_malformed() {
        assert_eq!(
            decode_dtc_by_status_mask(&[]),
            Err(UdsError::MalformedPositiveResponse)
        );
        assert_eq!(
            decode_dtc_by_status_mask(&[0xFF, 0x03, 0x01, 0x00]),
            Err(UdsError::MalformedPositiveResponse)
        );
    }
}
