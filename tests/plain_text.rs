use mpeg_syntax_dump::*;

fn render<F>(f: F) -> String
where
    F: FnOnce(&mut PlainTextRenderer<Vec<u8>>) -> Result<(), render::text::TextRenderError>,
{
    let mut r = PlainTextRenderer::new(Vec::new());
    f(&mut r).unwrap();
    String::from_utf8(r.into_inner()).unwrap()
}

#[test]
fn transport_packet_section1() {
    let output = render(|w| {
        w.begin_element("transport_packet", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "sync_byte",
            bits: 8,
            descriptor: "bslbf",
            value: Some(Value::Hex(0x47)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "transport_error_indicator",
            bits: 1,
            descriptor: "bslbf",
            value: Some(Value::Bool(false)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "payload_unit_start_indicator",
            bits: 1,
            descriptor: "bslbf",
            value: Some(Value::Bool(true)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "transport_priority",
            bits: 1,
            descriptor: "bslbf",
            value: Some(Value::Bool(false)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "PID",
            bits: 13,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(256)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "transport_scrambling_control",
            bits: 2,
            descriptor: "bslbf",
            value: Some(Value::BitString("00".to_string())),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "adaptation_field_control",
            bits: 2,
            descriptor: "bslbf",
            value: Some(Value::BitString("01".to_string())),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "continuity_counter",
            bits: 4,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(7)),
            comment: None,
        })?;
        w.ellipsis()?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
transport_packet() {
    sync_byte                                    8        bslbf       = 0x47
    transport_error_indicator                    1        bslbf       = 0
    payload_unit_start_indicator                 1        bslbf       = 1
    transport_priority                           1        bslbf       = 0
    PID                                          13       uimsbf      = 256
    transport_scrambling_control                 2        bslbf       = '00'
    adaptation_field_control                     2        bslbf       = '01'
    continuity_counter                           4        uimsbf      = 7
    ...
}
";
    assert_eq!(output, expected);
}

#[test]
fn if_else_not_taken_section8() {
    let output = render(|w| {
        w.begin_if(
            "program_number == '0'",
            &[TermAnnotation {
                name: "program_number",
                value: Value::Unsigned(1),
            }],
            false,
        )?;
        // not taken - empty body
        w.begin_else(true)?;
        w.fixed_width_field(&FixedWidthField {
            name: "program_map_PID",
            bits: 13,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(256)),
            comment: None,
        })?;
        w.end_if()?;
        Ok(())
    });

    let expected = "\
if (program_number == '0') {  /* program_number: 1 */
} else {
    program_map_PID                              13       uimsbf      = 256
}
";
    assert_eq!(output, expected);
}

#[test]
fn while_loop_section10() {
    let output = render(|w| {
        w.begin_while("next_bits( 8 ) == 0xFF")?;

        w.while_iteration(0)?;
        w.fixed_width_field(&FixedWidthField {
            name: "ff_byte",
            bits: 8,
            descriptor: "f(8)",
            value: Some(Value::Hex(0xFF)),
            comment: Some("equal to 0xFF"),
        })?;
        w.assignment("payloadType += 255", Some(&Value::Unsigned(255)))?;

        w.while_iteration(1)?;
        w.fixed_width_field(&FixedWidthField {
            name: "ff_byte",
            bits: 8,
            descriptor: "f(8)",
            value: Some(Value::Hex(0xFF)),
            comment: Some("equal to 0xFF"),
        })?;
        w.assignment("payloadType += 255", Some(&Value::Unsigned(510)))?;

        w.end_while()?;

        w.fixed_width_field(&FixedWidthField {
            name: "last_payload_type_byte",
            bits: 8,
            descriptor: "u(8)",
            value: Some(Value::Unsigned(5)),
            comment: None,
        })?;
        w.assignment(
            "payloadType += last_payload_type_byte",
            Some(&Value::Unsigned(515)),
        )?;

        Ok(())
    });

    // Check structural correctness
    assert!(output.starts_with("while (next_bits( 8 ) == 0xFF) {\n"));
    assert!(output.contains("    /* iteration: 0 */\n"));
    assert!(output.contains("    /* iteration: 1 */\n"));
    assert!(output.contains("ff_byte  /* equal to 0xFF */"));
    assert!(output.contains("/* = 255 */"));
    assert!(output.contains("/* = 510 */"));
    assert!(output.contains("}\n"));
    assert!(output.contains("/* = 515 */"));
}

#[test]
fn do_while_section11() {
    let output = render(|w| {
        w.begin_do_while()?;
        w.do_while_iteration(0)?;
        w.comment("... loop body ...")?;
        w.end_do_while("nextbits() == sync_byte")?;
        Ok(())
    });

    let expected = "\
do {
    /* iteration: 0 */
    /* ... loop body ... */
} while (nextbits() == sync_byte)
";
    assert_eq!(output, expected);
}

#[test]
fn raw_bytes_hex_dump() {
    let data: Vec<u8> = (0..32).collect();
    let output = render(|w| {
        w.begin_element("raw_data", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "data_byte",
            bits: 8,
            descriptor: "bslbf",
            value: None,
            comment: None,
        })?;
        w.raw_bytes(&data)?;
        w.end_element()?;
        Ok(())
    });

    assert!(output.contains("data_byte"));
    // First hex line: 16 bytes, 0x00..0x0f
    assert!(output.contains("00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f"));
    // Second hex line: 0x10..0x1f
    assert!(output.contains("10 11 12 13 14 15 16 17  18 19 1a 1b 1c 1d 1e 1f"));
}

#[test]
fn variable_length_fields() {
    let output = render(|w| {
        w.variable_length_field(&VariableLengthField {
            name: "seq_parameter_set_id",
            descriptor: "ue(v)",
            value: Some(Value::Unsigned(0)),
            comment: None,
        })?;
        w.variable_length_field(&VariableLengthField {
            name: "offset_for_non_ref_pic",
            descriptor: "se(v)",
            value: Some(Value::Signed(-2)),
            comment: None,
        })?;
        Ok(())
    });

    assert!(output.contains("seq_parameter_set_id"));
    assert!(output.contains("ue(v)"));
    assert!(output.contains("= 0"));
    assert!(output.contains("offset_for_non_ref_pic"));
    assert!(output.contains("se(v)"));
    assert!(output.contains("= -2"));
}

#[test]
fn for_loop_with_iterations() {
    let output = render(|w| {
        w.begin_for(
            "i = 0; i < N; i++",
            &[TermAnnotation {
                name: "N",
                value: Value::Unsigned(2),
            }],
        )?;

        w.for_iteration("i", 0)?;
        w.fixed_width_field(&FixedWidthField {
            name: "data_byte",
            bits: 8,
            descriptor: "bslbf",
            value: Some(Value::Hex(0xAA)),
            comment: None,
        })?;

        w.for_iteration("i", 1)?;
        w.fixed_width_field(&FixedWidthField {
            name: "data_byte",
            bits: 8,
            descriptor: "bslbf",
            value: Some(Value::Hex(0xBB)),
            comment: None,
        })?;

        w.end_for()?;
        Ok(())
    });

    assert!(output.starts_with("for (i = 0; i < N; i++) {  /* N: 2 */\n"));
    assert!(output.contains("    /* i: 0 */\n"));
    assert!(output.contains("    /* i: 1 */\n"));
    assert!(output.contains("= 0xAA"));
    assert!(output.contains("= 0xBB"));
    assert!(output.ends_with("}\n"));
}

#[test]
fn parameterized_element() {
    let output = render(|w| {
        w.begin_element("sei_payload", Some("payloadType, payloadSize"))?;
        w.ellipsis()?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
sei_payload(payloadType, payloadSize) {
    ...
}
";
    assert_eq!(output, expected);
}

#[test]
fn bit_pattern_field() {
    let output = render(|w| {
        w.bit_pattern(&BitPatternField {
            name: "'0010'",
            bits: 4,
            descriptor: "bslbf",
            value: Value::BitString("0010".to_string()),
        })?;
        w.bit_pattern(&BitPatternField {
            name: "marker_bit",
            bits: 1,
            descriptor: "bslbf",
            value: Value::Bool(true),
        })?;
        Ok(())
    });

    assert!(output.contains("'0010'"));
    assert!(output.contains("4"));
    assert!(output.contains("= '0010'"));
    assert!(output.contains("marker_bit"));
    assert!(output.contains("= 1"));
}

#[test]
fn assignment_with_and_without_value() {
    let output = render(|w| {
        w.assignment("moreDataFlag = 1", None)?;
        w.assignment(
            "CurrMbAddr = first_mb_in_slice * ( 1 + MbaffFrameFlag )",
            Some(&Value::Unsigned(0)),
        )?;
        Ok(())
    });

    // Simple assignment: no annotation
    assert!(output.contains("moreDataFlag = 1\n"));
    // Complex assignment: has annotation
    assert!(output.contains("/* = 0 */"));
}

#[test]
fn else_if_chain() {
    let output = render(|w| {
        w.begin_if(
            "pic_order_cnt_type == 0",
            &[TermAnnotation {
                name: "pic_order_cnt_type",
                value: Value::Unsigned(1),
            }],
            false,
        )?;
        // not taken
        w.begin_else_if(
            "pic_order_cnt_type == 1",
            &[TermAnnotation {
                name: "pic_order_cnt_type",
                value: Value::Unsigned(1),
            }],
            true,
        )?;
        w.variable_length_field(&VariableLengthField {
            name: "delta_pic_order_always_zero_flag",
            descriptor: "u(1)",
            value: Some(Value::Bool(false)),
            comment: None,
        })?;
        w.end_if()?;
        Ok(())
    });

    assert!(output.contains("if (pic_order_cnt_type == 0) {  /* pic_order_cnt_type: 1 */\n"));
    assert!(output.contains(
        "} else if (pic_order_cnt_type == 1) {  /* pic_order_cnt_type: 1 */\n"
    ));
    assert!(output.contains("delta_pic_order_always_zero_flag"));
    assert!(output.ends_with("}\n"));
}

#[test]
fn adaptation_field_nested_conditionals() {
    let output = render(|w| {
        w.begin_element("adaptation_field", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "adaptation_field_length",
            bits: 8,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(7)),
            comment: None,
        })?;
        w.begin_if(
            "adaptation_field_length > 0",
            &[TermAnnotation {
                name: "adaptation_field_length",
                value: Value::Unsigned(7),
            }],
            true,
        )?;
        w.fixed_width_field(&FixedWidthField {
            name: "PCR_flag",
            bits: 1,
            descriptor: "bslbf",
            value: Some(Value::Bool(true)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "OPCR_flag",
            bits: 1,
            descriptor: "bslbf",
            value: Some(Value::Bool(false)),
            comment: None,
        })?;
        // Taken if
        w.begin_if(
            "PCR_flag == '1'",
            &[TermAnnotation {
                name: "PCR_flag",
                value: Value::Bool(true),
            }],
            true,
        )?;
        w.fixed_width_field(&FixedWidthField {
            name: "program_clock_reference_base",
            bits: 33,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(95443)),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "reserved",
            bits: 6,
            descriptor: "bslbf",
            value: Some(Value::BitString("111111".to_string())),
            comment: None,
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "program_clock_reference_extension",
            bits: 9,
            descriptor: "uimsbf",
            value: Some(Value::Unsigned(0)),
            comment: None,
        })?;
        w.end_if()?;
        // Not-taken if
        w.begin_if(
            "OPCR_flag == '1'",
            &[TermAnnotation {
                name: "OPCR_flag",
                value: Value::Bool(false),
            }],
            false,
        )?;
        w.end_if()?;
        w.end_if()?;
        w.end_element()?;
        Ok(())
    });

    // Structural checks
    assert!(output.starts_with("adaptation_field() {\n"));
    assert!(output.contains("if (adaptation_field_length > 0) {  /* adaptation_field_length: 7 */\n"));
    assert!(output.contains("if (PCR_flag == '1') {  /* PCR_flag: 1 */\n"));
    assert!(output.contains("program_clock_reference_base"));
    assert!(output.contains("= 95443"));
    assert!(output.contains("= '111111'"));
    // Not-taken OPCR block: condition line then immediate close
    assert!(output.contains("if (OPCR_flag == '1') {  /* OPCR_flag: 0 */\n"));
    // The not-taken OPCR block: condition line then immediate close (no body content)
    assert!(output.contains("if (OPCR_flag == '1') {  /* OPCR_flag: 0 */\n        }\n"));
    assert!(output.ends_with("}\n"));
}
