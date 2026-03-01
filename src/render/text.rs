use std::io;

use crate::render::{format_hex_dump, format_term_annotations, write_indent, INDENT_WIDTH};
use crate::types::{BitPatternField, FixedWidthField, TermAnnotation, Value, VariableLengthField};
use crate::write::SyntaxWrite;

/// Error type for the plain text renderer.
#[derive(Debug)]
pub struct TextRenderError(io::Error);

impl std::fmt::Display for TextRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text render error: {}", self.0)
    }
}

impl std::error::Error for TextRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<io::Error> for TextRenderError {
    fn from(e: io::Error) -> Self {
        TextRenderError(e)
    }
}

/// Column positions for field rendering.
/// These control where the width, descriptor, and value columns start.
struct FieldColumns {
    /// Column where the width number (or descriptor for variable-length) starts
    width_col: usize,
    /// Column where the descriptor starts (for fixed-width fields)
    descriptor_col: usize,
    /// Column where `= value` starts
    value_col: usize,
}

/// Default column positions matching the syntax-examples.md layout.
const COLUMNS: FieldColumns = FieldColumns {
    width_col: 49,
    descriptor_col: 58,
    value_col: 70,
};

/// Plain text renderer that writes MPEG syntax to any `io::Write`.
pub struct PlainTextRenderer<W> {
    writer: W,
    depth: usize,
    /// Stack tracking block types for debug assertions
    block_stack: Vec<BlockKind>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockKind {
    Element,
    If,
    For,
    While,
    DoWhile,
    Switch,
    Case,
}

impl<W: io::Write> PlainTextRenderer<W> {
    pub fn new(writer: W) -> Self {
        PlainTextRenderer {
            writer,
            depth: 0,
            block_stack: Vec::new(),
        }
    }

    /// Get the underlying writer back.
    pub fn into_inner(self) -> W {
        self.writer
    }

    fn indent(&mut self) -> io::Result<()> {
        write_indent(&mut self.writer, self.depth)
    }

    fn current_indent_width(&self) -> usize {
        self.depth * INDENT_WIDTH
    }

    /// Write a fixed-width field line with proper column alignment.
    fn write_fixed_field_line(
        &mut self,
        name: &str,
        comment: Option<&str>,
        bits: u32,
        descriptor: &str,
        value: Option<&Value>,
    ) -> io::Result<()> {
        self.indent()?;

        // Build the name portion (possibly with inline comment)
        let name_part = match comment {
            Some(c) => format!("{name}  /* {c} */"),
            None => name.to_string(),
        };

        let indent_w = self.current_indent_width();
        let name_end = indent_w + name_part.len();

        // Pad to width column
        let width_str = bits.to_string();
        let padding1 = if name_end < COLUMNS.width_col {
            COLUMNS.width_col - name_end
        } else {
            1
        };
        write!(self.writer, "{name_part}{:padding1$}{width_str}", "")?;

        // Pad to descriptor column
        let width_end = COLUMNS.width_col + width_str.len();
        let padding2 = if width_end < COLUMNS.descriptor_col {
            COLUMNS.descriptor_col - width_end
        } else {
            1
        };
        write!(self.writer, "{:padding2$}{descriptor}", "")?;

        // Value
        if let Some(val) = value {
            let desc_end = COLUMNS.descriptor_col + descriptor.len();
            let padding3 = if desc_end < COLUMNS.value_col {
                COLUMNS.value_col - desc_end
            } else {
                1
            };
            write!(self.writer, "{:padding3$}= {val}", "")?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    /// Write a variable-length field line with proper column alignment.
    fn write_variable_field_line(
        &mut self,
        name: &str,
        comment: Option<&str>,
        descriptor: &str,
        value: Option<&Value>,
    ) -> io::Result<()> {
        self.indent()?;

        let name_part = match comment {
            Some(c) => format!("{name}  /* {c} */"),
            None => name.to_string(),
        };

        let indent_w = self.current_indent_width();
        let name_end = indent_w + name_part.len();

        // Variable-length fields put the descriptor at the descriptor column
        let padding1 = if name_end < COLUMNS.descriptor_col {
            COLUMNS.descriptor_col - name_end
        } else {
            1
        };
        write!(self.writer, "{name_part}{:padding1$}{descriptor}", "")?;

        if let Some(val) = value {
            let desc_end = COLUMNS.descriptor_col + descriptor.len();
            let padding3 = if desc_end < COLUMNS.value_col {
                COLUMNS.value_col - desc_end
            } else {
                1
            };
            write!(self.writer, "{:padding3$}= {val}", "")?;
        }

        writeln!(self.writer)?;
        Ok(())
    }
}

impl<W: io::Write> SyntaxWrite for PlainTextRenderer<W> {
    type Error = TextRenderError;

    fn begin_element(&mut self, name: &str, params: Option<&str>) -> Result<(), Self::Error> {
        self.indent()?;
        match params {
            Some(p) => writeln!(self.writer, "{name}({p}) {{")?,
            None => writeln!(self.writer, "{name}() {{")?,
        }
        self.depth += 1;
        self.block_stack.push(BlockKind::Element);
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Element));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}}")?;
        Ok(())
    }

    fn fixed_width_field(&mut self, field: &FixedWidthField<'_>) -> Result<(), Self::Error> {
        self.write_fixed_field_line(
            field.name,
            field.comment,
            field.bits,
            field.descriptor,
            field.value.as_ref(),
        )?;
        Ok(())
    }

    fn variable_length_field(
        &mut self,
        field: &VariableLengthField<'_>,
    ) -> Result<(), Self::Error> {
        self.write_variable_field_line(
            field.name,
            field.comment,
            field.descriptor,
            field.value.as_ref(),
        )?;
        Ok(())
    }

    fn bit_pattern(&mut self, field: &BitPatternField<'_>) -> Result<(), Self::Error> {
        self.write_fixed_field_line(
            field.name,
            None,
            field.bits,
            field.descriptor,
            Some(&field.value),
        )?;
        Ok(())
    }

    fn raw_bytes(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        let lines = format_hex_dump(data);
        for line in &lines {
            self.indent()?;
            writeln!(self.writer, "{line}")?;
        }
        Ok(())
    }

    fn begin_if(
        &mut self,
        condition: &str,
        terms: &[TermAnnotation<'_>],
        _taken: bool,
    ) -> Result<(), Self::Error> {
        self.indent()?;
        let annotations = format_term_annotations(terms);
        writeln!(self.writer, "if ({condition}) {{{annotations}")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::If);
        Ok(())
    }

    fn begin_else_if(
        &mut self,
        condition: &str,
        terms: &[TermAnnotation<'_>],
        _taken: bool,
    ) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.last(), Some(&BlockKind::If));
        self.depth -= 1;
        self.indent()?;
        let annotations = format_term_annotations(terms);
        writeln!(self.writer, "}} else if ({condition}) {{{annotations}")?;
        self.depth += 1;
        Ok(())
    }

    fn begin_else(&mut self, _taken: bool) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.last(), Some(&BlockKind::If));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}} else {{")?;
        self.depth += 1;
        Ok(())
    }

    fn end_if(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::If));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}}")?;
        Ok(())
    }

    fn begin_for(
        &mut self,
        header: &str,
        terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.indent()?;
        let annotations = format_term_annotations(terms);
        writeln!(self.writer, "for ({header}) {{{annotations}")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::For);
        Ok(())
    }

    fn for_iteration(&mut self, variable: &str, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "/* {variable}: {index} */")?;
        Ok(())
    }

    fn end_for(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::For));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}}")?;
        Ok(())
    }

    fn begin_while(&mut self, condition: &str) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "while ({condition}) {{")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::While);
        Ok(())
    }

    fn while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "/* iteration: {index} */")?;
        Ok(())
    }

    fn end_while(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::While));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}}")?;
        Ok(())
    }

    fn begin_do_while(&mut self) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "do {{")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::DoWhile);
        Ok(())
    }

    fn do_while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "/* iteration: {index} */")?;
        Ok(())
    }

    fn end_do_while(&mut self, condition: &str) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::DoWhile));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}} while ({condition})")?;
        Ok(())
    }

    fn begin_switch(
        &mut self,
        expression: &str,
        terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.indent()?;
        let annotations = format_term_annotations(terms);
        writeln!(self.writer, "switch ({expression}) {{{annotations}")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::Switch);
        Ok(())
    }

    fn begin_case(&mut self, label: &str, _taken: bool) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "case {label}:")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::Case);
        Ok(())
    }

    fn end_case(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Case));
        self.indent()?;
        writeln!(self.writer, "break")?;
        self.depth -= 1;
        Ok(())
    }

    fn end_switch(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Switch));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "}}")?;
        Ok(())
    }

    fn assignment(
        &mut self,
        expression: &str,
        computed_value: Option<&Value>,
    ) -> Result<(), Self::Error> {
        self.indent()?;
        match computed_value {
            Some(val) => writeln!(self.writer, "{expression}{:width$}/* = {val} */", "",
                width = COLUMNS.value_col.saturating_sub(self.current_indent_width() + expression.len()))?,
            None => writeln!(self.writer, "{expression}")?,
        }
        Ok(())
    }

    fn comment(&mut self, text: &str) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "/* {text} */")?;
        Ok(())
    }

    fn ellipsis(&mut self) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "...")?;
        Ok(())
    }
}
