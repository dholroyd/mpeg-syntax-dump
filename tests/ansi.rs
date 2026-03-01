use mpeg_syntax_dump::*;

/// Strip ANSI escape codes from a string for text comparison.
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm'
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn render_ansi<F>(f: F) -> String
where
    F: FnOnce(&mut AnsiRenderer<Vec<u8>>) -> Result<(), render::text::TextRenderError>,
{
    let mut r = AnsiRenderer::new(Vec::new());
    f(&mut r).unwrap();
    String::from_utf8(r.into_inner()).unwrap()
}

#[test]
fn ansi_contains_escape_codes() {
    let output = render_ansi(|w| {
        w.begin_element("transport_packet", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "sync_byte",
            bits: 8,
            descriptor: "bslbf",
            value: Some(Value::Hex(0x47)),
            comment: None,
        })?;
        w.end_element()?;
        Ok(())
    });

    // Should contain ANSI escape codes
    assert!(output.contains("\x1b["));
    // Element name should be bold cyan
    assert!(output.contains("\x1b[1;36mtransport_packet\x1b[0m"));
    // Descriptor should be yellow
    assert!(output.contains("\x1b[33mbslbf\x1b[0m"));
    // Value should be green
    assert!(output.contains("\x1b[32m0x47\x1b[0m"));
}

#[test]
fn ansi_stripped_matches_plain_text_structure() {
    let ansi_output = render_ansi(|w| {
        w.begin_element("test_element", None)?;
        w.fixed_width_field(&FixedWidthField {
            name: "field_a",
            bits: 8,
            descriptor: "bslbf",
            value: Some(Value::Unsigned(42)),
            comment: None,
        })?;
        w.begin_if(
            "x == 1",
            &[TermAnnotation {
                name: "x",
                value: Value::Unsigned(1),
            }],
            true,
        )?;
        w.ellipsis()?;
        w.end_if()?;
        w.end_element()?;
        Ok(())
    });

    let stripped = strip_ansi(&ansi_output);

    // After stripping ANSI codes, structure should be intact
    assert!(stripped.contains("test_element() {"));
    assert!(stripped.contains("field_a"));
    assert!(stripped.contains("= 42"));
    assert!(stripped.contains("if (x == 1) {"));
    assert!(stripped.contains("/* x: 1 */"));
    assert!(stripped.contains("..."));
    assert!(stripped.contains("}"));
}

#[test]
fn ansi_keywords_colored() {
    let output = render_ansi(|w| {
        w.begin_if("cond", &[], true)?;
        w.end_if()?;
        w.begin_for("i = 0; i < n; i++", &[])?;
        w.end_for()?;
        w.begin_while("true")?;
        w.end_while()?;
        w.begin_do_while()?;
        w.end_do_while("false")?;
        Ok(())
    });

    // Keywords should be bold magenta (\x1b[1;35m)
    assert!(output.contains("\x1b[1;35mif\x1b[0m"));
    assert!(output.contains("\x1b[1;35mfor\x1b[0m"));
    assert!(output.contains("\x1b[1;35mwhile\x1b[0m"));
    assert!(output.contains("\x1b[1;35mdo\x1b[0m"));
}
