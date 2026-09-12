//! Golden bytes for the two DS2 requests this product makes and for the
//! frames a module answers with (ADR-0029). Synthetic: they prove the
//! framing, never a vehicle.

use ds2::{
    fault_memory, identification, parse_frame, parse_leading_frame, printable_text, split_frames,
    strip_echo, words_from, xor_checksum, Ds2Error, Ds2Frame, Ds2Request, COMMAND_FAULT_MEMORY,
    COMMAND_IDENTIFICATION, ECU_IDENTIFICATION_CAPABILITY, FAULT_MEMORY_CAPABILITY,
    PROTOCOL_FAMILY, REPLY_ACCEPTED,
};

/// A module's frame, built the way a module builds it.
fn module_frame(node_address: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![node_address, (data.len() + 3) as u8];
    bytes.extend_from_slice(data);
    bytes.push(xor_checksum(&bytes));
    bytes
}

#[test]
fn the_identification_request_is_address_length_command_xor() {
    let request = Ds2Request::ecu_identification(0x72);
    assert_eq!(request.as_bytes(), &[0x72, 0x04, 0x00, 0x76]);
    assert_eq!(request.node_address(), 0x72);
    assert_eq!(request.command(), COMMAND_IDENTIFICATION);
}

#[test]
fn the_fault_memory_request_is_address_length_command_xor() {
    let request = Ds2Request::fault_memory(0x72);
    assert_eq!(request.as_bytes(), &[0x72, 0x04, 0x04, 0x72]);
    assert_eq!(request.command(), COMMAND_FAULT_MEMORY);
    // Another address, another checksum: the XOR covers the address.
    assert_eq!(
        Ds2Request::fault_memory(0x00).as_bytes(),
        &[0x00, 0x04, 0x04, 0x00]
    );
}

#[test]
fn an_accepted_reply_yields_its_payload_and_text() {
    let frame = parse_frame(&[0x72, 0x06, 0xA0, 0x31, 0x32, 0xD7]).unwrap();
    assert_eq!(
        frame,
        Ds2Frame {
            node_address: 0x72,
            data: vec![REPLY_ACCEPTED, 0x31, 0x32],
        }
    );
    assert!(frame.accepted());
    assert_eq!(frame.payload(), &[0x31, 0x32]);
    let identification = identification(&frame).unwrap();
    assert_eq!(identification.node_address, 0x72);
    assert_eq!(identification.bytes, vec![0x31, 0x32]);
    assert_eq!(identification.text, "12");
}

#[test]
fn a_reply_the_module_did_not_accept_keeps_its_own_word() {
    let bytes = module_frame(0x72, &[0xFF, 0x01]);
    let frame = parse_frame(&bytes).unwrap();
    assert!(!frame.accepted());
    assert_eq!(frame.status(), Some(0xFF));
    // Nothing is dropped from what it said.
    assert_eq!(frame.payload(), &[0xFF, 0x01]);
    assert_eq!(
        identification(&frame),
        Err(Ds2Error::NotAccepted { status: 0xFF })
    );
    assert_eq!(
        fault_memory(&frame),
        Err(Ds2Error::NotAccepted { status: 0xFF })
    );
    let empty = parse_frame(&module_frame(0x72, &[])).unwrap();
    assert_eq!(fault_memory(&empty), Err(Ds2Error::EmptyReply));
}

#[test]
fn the_fault_memory_is_handed_over_as_bytes_and_its_words_are_only_candidates() {
    let bytes = module_frame(0x72, &[0xA0, 0x02, 0x00, 0x0B, 0x21, 0x00, 0x1C, 0x22]);
    let frame = parse_frame(&bytes).unwrap();
    let memory = fault_memory(&frame).unwrap();
    assert_eq!(memory.bytes, vec![0x02, 0x00, 0x0B, 0x21, 0x00, 0x1C, 0x22]);
    // Read from the second byte, the words are the shape SDD's index uses;
    // read from the first, they are not — the layout is the module's, and
    // the crate decides nothing about it.
    assert_eq!(words_from(&memory.bytes, 1), vec![0x000B, 0x2100, 0x1C22]);
    assert_eq!(words_from(&memory.bytes, 0), vec![0x0200, 0x0B21, 0x001C]);
    assert_eq!(words_from(&memory.bytes, 7), Vec::<u16>::new());
    assert_eq!(words_from(&memory.bytes, 99), Vec::<u16>::new());
}

#[test]
fn a_bad_checksum_a_short_frame_and_a_wrong_length_are_refused() {
    assert_eq!(
        parse_frame(&[0x72, 0x06, 0xA0, 0x31, 0x32, 0xD6]),
        Err(Ds2Error::Checksum {
            expected: 0xD7,
            actual: 0xD6,
        })
    );
    assert_eq!(parse_frame(&[0x72, 0x04]), Err(Ds2Error::TooShort(2)));
    assert_eq!(
        parse_frame(&[0x72, 0x02, 0x00]),
        Err(Ds2Error::LengthTooSmall(2))
    );
    assert_eq!(
        parse_frame(&[0x72, 0x08, 0xA0, 0x31]),
        Err(Ds2Error::Incomplete {
            declared: 8,
            available: 4,
        })
    );
    // One frame was asked for; a second one behind it is not silently lost.
    let mut two = module_frame(0x72, &[0xA0]);
    two.extend(module_frame(0x72, &[0xA0]));
    assert_eq!(parse_frame(&two), Err(Ds2Error::TrailingBytes(4)));
    assert_eq!(parse_leading_frame(&two).unwrap().1, 4);
}

#[test]
fn a_k_line_stream_carries_the_echo_first_and_the_answer_after() {
    let request = Ds2Request::ecu_identification(0x72);
    let mut stream = request.as_bytes().to_vec();
    stream.extend(module_frame(0x72, &[0xA0, 0x44, 0x53, 0x4D]));
    stream.extend(module_frame(0x72, &[0xA0, 0x01]));

    let answered = strip_echo(&stream, &request);
    assert_eq!(answered.len(), stream.len() - 4);
    let frames = split_frames(answered).unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(printable_text(frames[0].payload()), "DSM");
    assert_eq!(frames[1].payload(), &[0x01]);

    // A stream without the echo is left as it is.
    let bare = module_frame(0x72, &[0xA0]);
    assert_eq!(strip_echo(&bare, &request), &bare[..]);
}

#[test]
fn printable_text_shows_ascii_and_dots_and_nothing_else() {
    assert_eq!(printable_text(b"LCM 6 1234"), "LCM 6 1234");
    assert_eq!(
        printable_text(&[0x00, 0x41, 0xFF, 0x7F, 0x20, 0x7E]),
        ".A.. ~"
    );
    assert_eq!(printable_text(&[]), "");
}

#[test]
fn the_names_the_knowledge_base_records_are_the_crates_own() {
    assert_eq!(PROTOCOL_FAMILY, "DS2");
    assert_eq!(
        ECU_IDENTIFICATION_CAPABILITY,
        "ds2.ecu_identification.read_only"
    );
    assert_eq!(FAULT_MEMORY_CAPABILITY, "ds2.fault_memory.read_only");
    assert!(ECU_IDENTIFICATION_CAPABILITY.ends_with(".read_only"));
    assert!(FAULT_MEMORY_CAPABILITY.ends_with(".read_only"));
}
