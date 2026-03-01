pub mod text;
pub mod ansi;

use std::io;

use crate::types::Value;

/// Default number of spaces per indentation level.
pub const INDENT_WIDTH: usize = 4;

/// Maximum number of bytes shown in a raw hex dump before truncation.
pub const MAX_HEX_BYTES: usize = 128;

/// Number of bytes per hex dump line.
pub const HEX_BYTES_PER_LINE: usize = 16;

const INDENT_BUF: &[u8; 32] = b"                                ";

/// Write indentation spaces to the output.
pub fn write_indent(w: &mut impl io::Write, depth: usize) -> io::Result<()> {
    let total = depth * INDENT_WIDTH;
    if total == 0 {
        return Ok(());
    }
    if total <= INDENT_BUF.len() {
        w.write_all(&INDENT_BUF[..total])
    } else {
        let buf = vec![b' '; total];
        w.write_all(&buf)
    }
}

/// Format a value for display, returning the formatted string.
pub fn format_value(value: &Value) -> String {
    value.to_string()
}

/// Format term annotations as a trailing comment: `  /* name: value */`
/// If multiple terms, they are comma-separated within a single comment.
pub fn format_term_annotations(terms: &[crate::types::TermAnnotation<'_>]) -> String {
    if terms.is_empty() {
        return String::new();
    }
    let inner: Vec<String> = terms
        .iter()
        .map(|t| format!("{}: {}", t.name, t.value))
        .collect();
    format!("  /* {} */", inner.join(", "))
}

/// Format raw bytes as hex dump lines. Returns a Vec of formatted lines.
/// Each line contains up to 16 bytes, with a space separator after the 8th byte.
/// If the data exceeds `MAX_HEX_BYTES`, the dump is truncated and ends with `...`.
pub fn format_hex_dump(data: &[u8]) -> Vec<String> {
    let truncated = data.len() > MAX_HEX_BYTES;
    let show_bytes = if truncated { MAX_HEX_BYTES } else { data.len() };
    let mut lines = Vec::new();

    for chunk_start in (0..show_bytes).step_by(HEX_BYTES_PER_LINE) {
        let chunk_end = (chunk_start + HEX_BYTES_PER_LINE).min(show_bytes);
        let chunk = &data[chunk_start..chunk_end];
        let mut line = String::new();
        for (i, byte) in chunk.iter().enumerate() {
            if i > 0 {
                line.push(' ');
            }
            if i == 8 {
                line.push(' ');
            }
            line.push_str(&format!("{byte:02x}"));
        }
        lines.push(line);
    }

    if truncated {
        lines.push("...".to_string());
    }

    lines
}
