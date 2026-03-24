use mpeg_syntax_dump::*;

fn render<F>(f: F) -> String
where
    F: FnOnce(&mut CompactTextRenderer<Vec<u8>>) -> Result<(), render::text::TextRenderError>,
{
    let mut r = CompactTextRenderer::new(Vec::new());
    f(&mut r).unwrap();
    String::from_utf8(r.into_inner()).unwrap()
}

#[test]
fn basic_element_and_fields() {
    let output = render(|w| {
        w.begin_element("slice_header", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "slice_type",
            bits: 3,
            descriptor: "ue(v)",
            value: Some(Value::Unsigned(1)),
            comment: Some("P"),
        })?;
        w.fixed_width_field(&FixedWidthField {
            name: "frame_num",
            bits: 4,
            descriptor: "u(4)",
            value: Some(Value::Unsigned(3)),
            comment: None,
        })?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
slice_header:
    slice_type: 1  // P
    frame_num: 3
";
    assert_eq!(output, expected);
}

#[test]
fn suppresses_untaken_if() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "before",
            bits: 1,
            descriptor: "u(1)",
            value: Some(Value::Unsigned(0)),
            comment: None,
        })?;
        // Untaken branch — everything inside should be suppressed
        w.begin_if("some_flag", &[], false)?;
        w.fixed_width_field(&FixedWidthField {
            name: "hidden_field",
            bits: 8,
            descriptor: "u(8)",
            value: Some(Value::Unsigned(99)),
            comment: None,
        })?;
        w.end_if()?;
        w.fixed_width_field(&FixedWidthField {
            name: "after",
            bits: 1,
            descriptor: "u(1)",
            value: Some(Value::Unsigned(1)),
            comment: None,
        })?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    before: 0
    after: 1
";
    assert_eq!(output, expected);
}

#[test]
fn taken_if_is_transparent() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        w.begin_if("flag", &[], true)?;
        w.fixed_width_field(&FixedWidthField {
            name: "visible",
            bits: 1,
            descriptor: "u(1)",
            value: Some(Value::Bool(true)),
            comment: None,
        })?;
        w.end_if()?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    visible: 1
";
    assert_eq!(output, expected);
}

#[test]
fn if_else_shows_taken_branch() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        w.begin_if("condition_a", &[], false)?;
        w.fixed_width_field(&FixedWidthField {
            name: "branch_a",
            bits: 1, descriptor: "u(1)",
            value: Some(Value::Unsigned(1)), comment: None,
        })?;
        w.begin_else(true)?;
        w.fixed_width_field(&FixedWidthField {
            name: "branch_b",
            bits: 1, descriptor: "u(1)",
            value: Some(Value::Unsigned(2)), comment: None,
        })?;
        w.end_if()?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    branch_b: 2
";
    assert_eq!(output, expected);
}

#[test]
fn switch_shows_only_taken_case() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        w.begin_switch("type", &[])?;
        w.begin_case("A", false)?;
        w.fixed_width_field(&FixedWidthField {
            name: "case_a", bits: 1, descriptor: "u(1)",
            value: Some(Value::Unsigned(1)), comment: None,
        })?;
        w.end_case()?;
        w.begin_case("B", true)?;
        w.fixed_width_field(&FixedWidthField {
            name: "case_b", bits: 1, descriptor: "u(1)",
            value: Some(Value::Unsigned(2)), comment: None,
        })?;
        w.end_case()?;
        w.end_switch()?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    case_b: 2
";
    assert_eq!(output, expected);
}

#[test]
fn field_table_single_column() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        let rows: Vec<[Value; 1]> = vec![
            [Value::Unsigned(10)],
            [Value::Unsigned(20)],
            [Value::Unsigned(30)],
        ];
        let row_refs: Vec<&[Value]> = rows.iter().map(|r| r.as_slice()).collect();
        w.field_table(&FieldTable {
            columns: &[ColumnDef { name: "scale_factor", descriptor: "uimsbf", bits: Some(8) }],
            rows: &row_refs,
        })?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    scale_factor: [10, 20, 30]
";
    assert_eq!(output, expected);
}

#[test]
fn field_table_multi_column() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        let rows: Vec<[Value; 2]> = vec![
            [Value::Unsigned(5), Value::Unsigned(100)],
            [Value::Unsigned(3), Value::Unsigned(200)],
        ];
        let row_refs: Vec<&[Value]> = rows.iter().map(|r| r.as_slice()).collect();
        w.field_table(&FieldTable {
            columns: &[
                ColumnDef { name: "offset", descriptor: "uimsbf", bits: Some(5) },
                ColumnDef { name: "amp", descriptor: "uimsbf", bits: Some(4) },
            ],
            rows: &row_refs,
        })?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    offset  amp
         5  100
         3  200
";
    assert_eq!(output, expected);
}

#[test]
fn field_table_default_expansion_via_plain_text() {
    // Verify the default field_table implementation expands into a loop
    let mut r = PlainTextRenderer::new(Vec::new());
    let rows: Vec<[Value; 1]> = vec![
        [Value::Unsigned(10)],
        [Value::Unsigned(20)],
    ];
    let row_refs: Vec<&[Value]> = rows.iter().map(|r| r.as_slice()).collect();
    r.field_table(&FieldTable {
        columns: &[ColumnDef { name: "val", descriptor: "uimsbf", bits: Some(8) }],
        rows: &row_refs,
    }).unwrap();
    let output = String::from_utf8(r.into_inner()).unwrap();

    // Should produce a for loop with iterations
    assert!(output.contains("for (i = 0; i < 2; i++)"));
    assert!(output.contains("val[0]"));
    assert!(output.contains("val[1]"));
}

#[test]
fn compact_comment_and_ellipsis() {
    let output = render(|w| {
        w.comment("test comment")?;
        w.ellipsis()?;
        Ok(())
    });

    let expected = "\
// test comment
...
";
    assert_eq!(output, expected);
}

#[test]
fn compact_assignment() {
    let output = render(|w| {
        w.begin_element("test", None)?;
        w.assignment("x = y + 1", Some(&Value::Unsigned(42)))?;
        w.end_element()?;
        Ok(())
    });

    let expected = "\
test:
    x = y + 1 = 42
";
    assert_eq!(output, expected);
}
