//! Golden bytes for the three KWP2000 requests this product makes and for
//! the messages a module answers with (ADR-0029). Synthetic: they prove the
//! data link, never a vehicle.

use kwp2000::{
    dtc_by_status, ecu_identification, interpret, parse_leading_message, parse_message,
    response_code_text, split_messages, start_communication_reply, strip_echo, sum_checksum,
    DtcByStatus, KwpError, KwpMessage, KwpReply, KwpRequest, ALL_DTC_GROUPS, PROTOCOL_FAMILY,
    READ_DTC_BY_STATUS_CAPABILITY, READ_ECU_IDENTIFICATION_CAPABILITY, RESPONSE_PENDING,
    SID_READ_DTC_BY_STATUS, SID_READ_ECU_IDENTIFICATION, SID_START_COMMUNICATION, TESTER_ADDRESS,
};

const ABS: u8 = 0x29;

/// A module's message to the tester, built the way a module builds it, in
/// the short length form.
fn module_message(source: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x80 | payload.len() as u8, TESTER_ADDRESS, source];
    bytes.extend_from_slice(payload);
    bytes.push(sum_checksum(&bytes));
    bytes
}

/// The same in the long length form: a zero length in the format byte and
/// the length as its own byte after the addresses.
fn long_module_message(source: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x80, TESTER_ADDRESS, source, payload.len() as u8];
    bytes.extend_from_slice(payload);
    bytes.push(sum_checksum(&bytes));
    bytes
}

#[test]
fn start_communication_is_the_physical_header_the_service_and_the_sum() {
    let request = KwpRequest::start_communication(ABS);
    assert_eq!(request.as_bytes(), &[0x81, 0x29, 0xF1, 0x81, 0x1C]);
    assert_eq!(request.target(), ABS);
    assert_eq!(request.source(), TESTER_ADDRESS);
    assert_eq!(request.service_id(), SID_START_COMMUNICATION);
}

#[test]
fn read_ecu_identification_carries_the_option() {
    let request = KwpRequest::read_ecu_identification(ABS, 0x8A);
    assert_eq!(request.as_bytes(), &[0x82, 0x29, 0xF1, 0x1A, 0x8A, 0x40]);
    assert_eq!(request.service_id(), SID_READ_ECU_IDENTIFICATION);
}

#[test]
fn read_dtc_by_status_carries_the_mask_and_the_group_big_endian() {
    let request = KwpRequest::read_dtc_by_status(ABS, 0x00, ALL_DTC_GROUPS);
    assert_eq!(
        request.as_bytes(),
        &[0x84, 0x29, 0xF1, 0x18, 0x00, 0xFF, 0x00, 0xB5]
    );
    assert_eq!(request.service_id(), SID_READ_DTC_BY_STATUS);
}

#[test]
fn the_handshake_reply_yields_the_key_bytes() {
    let message = parse_message(&[0x83, 0xF1, 0x29, 0xC1, 0xEF, 0x8F, 0xDC]).unwrap();
    assert_eq!(
        message,
        KwpMessage {
            target: TESTER_ADDRESS,
            source: ABS,
            payload: vec![0xC1, 0xEF, 0x8F],
        }
    );
    let reply = interpret(&message, SID_START_COMMUNICATION).unwrap();
    assert_eq!(
        reply,
        KwpReply::Positive {
            service: SID_START_COMMUNICATION,
            data: vec![0xEF, 0x8F],
        }
    );
    assert_eq!(
        start_communication_reply(&reply).unwrap().key_bytes,
        [0xEF, 0x8F]
    );
}

#[test]
fn an_identification_is_the_option_the_bytes_and_the_text() {
    let mut payload = vec![0x5A, 0x8A];
    payload.extend_from_slice(b"SRB500010\x00");
    let message = parse_message(&module_message(ABS, &payload)).unwrap();
    let reply = interpret(&message, SID_READ_ECU_IDENTIFICATION).unwrap();
    let identification = ecu_identification(&reply).unwrap();
    assert_eq!(identification.option, 0x8A);
    assert_eq!(identification.bytes, b"SRB500010\x00".to_vec());
    assert_eq!(identification.text, "SRB500010.");
}

#[test]
fn a_fault_list_is_count_then_code_and_status_per_entry() {
    let payload = [0x58, 0x02, 0xC1, 0x21, 0x60, 0xC1, 0x22, 0xE0];
    let message = parse_message(&module_message(ABS, &payload)).unwrap();
    let reply = interpret(&message, SID_READ_DTC_BY_STATUS).unwrap();
    assert_eq!(
        dtc_by_status(&reply).unwrap(),
        vec![
            DtcByStatus {
                code: 0xC121,
                status: 0x60,
            },
            DtcByStatus {
                code: 0xC122,
                status: 0xE0,
            },
        ]
    );
    // No codes: an empty list, not an error.
    let none = interpret(
        &parse_message(&module_message(ABS, &[0x58, 0x00])).unwrap(),
        SID_READ_DTC_BY_STATUS,
    )
    .unwrap();
    assert_eq!(dtc_by_status(&none).unwrap(), Vec::<DtcByStatus>::new());
    // A count the bytes do not bear out is refused, not guessed at.
    let short = interpret(
        &parse_message(&module_message(ABS, &[0x58, 0x02, 0xC1, 0x21, 0x60])).unwrap(),
        SID_READ_DTC_BY_STATUS,
    )
    .unwrap();
    assert!(matches!(dtc_by_status(&short), Err(KwpError::Malformed(_))));
}

#[test]
fn a_negative_response_is_the_modules_own_word() {
    let message = parse_message(&[0x83, 0xF1, 0x29, 0x7F, 0x1A, 0x12, 0x48]).unwrap();
    let reply = interpret(&message, SID_READ_ECU_IDENTIFICATION).unwrap();
    assert_eq!(
        reply,
        KwpReply::Negative {
            service: SID_READ_ECU_IDENTIFICATION,
            code: 0x12,
        }
    );
    assert_eq!(
        ecu_identification(&reply),
        Err(KwpError::Refused {
            service: SID_READ_ECU_IDENTIFICATION,
            code: 0x12,
        })
    );
    assert_eq!(
        response_code_text(0x12),
        "sub-function not supported or invalid format"
    );
    assert_eq!(
        response_code_text(RESPONSE_PENDING),
        "request correctly received, response pending"
    );
    assert_eq!(
        response_code_text(0x99),
        "a response code this product does not name"
    );
    // A negative response about another service is not this request's answer.
    assert_eq!(
        interpret(&message, SID_READ_DTC_BY_STATUS),
        Err(KwpError::UnexpectedService {
            expected: SID_READ_DTC_BY_STATUS,
            actual: SID_READ_ECU_IDENTIFICATION,
        })
    );
}

#[test]
fn a_positive_response_to_another_service_is_not_this_requests_answer() {
    let message = parse_message(&module_message(ABS, &[0x5A, 0x8A, 0x41])).unwrap();
    assert_eq!(
        interpret(&message, SID_READ_DTC_BY_STATUS),
        Err(KwpError::UnexpectedService {
            expected: 0x58,
            actual: 0x5A,
        })
    );
    let empty = KwpMessage {
        target: TESTER_ADDRESS,
        source: ABS,
        payload: Vec::new(),
    };
    assert_eq!(
        interpret(&empty, SID_READ_DTC_BY_STATUS),
        Err(KwpError::EmptyPayload)
    );
}

#[test]
fn the_long_length_form_is_read_like_the_short_one() {
    let mut payload = vec![0x5A, 0x8B];
    payload.extend((0..70u8).map(|index| b'A' + index % 26));
    let bytes = long_module_message(ABS, &payload);
    assert_eq!(bytes[0], 0x80);
    assert_eq!(bytes[3], 72);
    let message = parse_message(&bytes).unwrap();
    assert_eq!(message.payload, payload);
    let identification =
        ecu_identification(&interpret(&message, SID_READ_ECU_IDENTIFICATION).unwrap()).unwrap();
    assert_eq!(identification.bytes.len(), 70);
    assert!(identification
        .text
        .starts_with("ABCDEFGHIJKLMNOPQRSTUVWXYZABCD"));
}

#[test]
fn a_bad_checksum_a_short_message_and_a_headerless_one_are_refused() {
    assert_eq!(
        parse_message(&[0x83, 0xF1, 0x29, 0xC1, 0xEF, 0x8F, 0xDD]),
        Err(KwpError::Checksum {
            expected: 0xDC,
            actual: 0xDD,
        })
    );
    assert_eq!(
        parse_message(&[0x83, 0xF1, 0x29]),
        Err(KwpError::TooShort(3))
    );
    assert_eq!(
        parse_message(&[0x83, 0xF1, 0x29, 0xC1, 0xEF]),
        Err(KwpError::Incomplete {
            declared: 7,
            available: 5,
        })
    );
    // No address information: the CARB and header-less forms are not
    // spoken here.
    assert_eq!(
        parse_message(&[0x03, 0xC1, 0xEF, 0x8F, 0x00]),
        Err(KwpError::NoAddressInformation(0x03))
    );
    // Two messages where one was asked for.
    let mut two = module_message(ABS, &[0xC1, 0xEF, 0x8F]);
    two.extend(module_message(ABS, &[0xC1, 0xEF, 0x8F]));
    assert_eq!(parse_message(&two), Err(KwpError::TrailingBytes(7)));
    assert_eq!(parse_leading_message(&two).unwrap().1, 7);
}

#[test]
fn a_k_line_stream_carries_the_echo_first_and_the_answer_after() {
    let request = KwpRequest::start_communication(ABS);
    let mut stream = request.as_bytes().to_vec();
    stream.extend(module_message(ABS, &[0xC1, 0xEF, 0x8F]));
    stream.extend(module_message(ABS, &[0x7F, 0x1A, RESPONSE_PENDING]));

    let answered = strip_echo(&stream, &request);
    assert_eq!(answered.len(), stream.len() - 5);
    let messages = split_messages(answered).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].payload, vec![0xC1, 0xEF, 0x8F]);
    assert_eq!(
        interpret(&messages[1], SID_READ_ECU_IDENTIFICATION),
        Ok(KwpReply::Negative {
            service: SID_READ_ECU_IDENTIFICATION,
            code: RESPONSE_PENDING,
        })
    );

    let bare = module_message(ABS, &[0xC1, 0xEF, 0x8F]);
    assert_eq!(strip_echo(&bare, &request), &bare[..]);
}

#[test]
fn the_names_the_knowledge_base_records_are_the_crates_own() {
    assert_eq!(PROTOCOL_FAMILY, "KW2000");
    assert_eq!(
        READ_ECU_IDENTIFICATION_CAPABILITY,
        "kwp2000.service1a.read_ecu_identification.read_only"
    );
    assert_eq!(
        READ_DTC_BY_STATUS_CAPABILITY,
        "kwp2000.service18.read_dtc_by_status.read_only"
    );
}
