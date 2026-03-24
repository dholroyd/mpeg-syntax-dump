use std::io;

use crate::render::{format_hex_dump, write_indent};
use crate::types::{BitPatternField, FieldTable, FixedWidthField, TermAnnotation, Value, VariableLengthField};
use crate::write::SyntaxWrite;

use super::text::TextRenderError;

/// Compact text renderer producing LLM-optimised output.
///
/// Compared to [`PlainTextRenderer`](super::text::PlainTextRenderer):
/// - Fields render as `name: value` (no bit widths, no descriptors)
/// - Untaken conditional branches and switch cases are suppressed entirely
/// - [`FieldTable`] values collapse to inline lists or aligned tables
/// - No spec-grammar keywords (`if`, `for`, `switch`, braces)
pub struct CompactTextRenderer<W> {
    writer: W,
    depth: usize,
    block_stack: Vec<BlockKind>,
    /// When set, all output is suppressed until depth returns to this level.
    /// Used to hide untaken conditional branches.
    suppress_depth: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockKind {
    Element,
    /// `indented` tracks whether this if block added a depth level.
    /// Taken branches are transparent (no indent), untaken are suppressed.
    If { indented: bool },
    For,
    While,
    DoWhile,
    Switch,
    /// `indented` tracks whether this case added a depth level.
    Case { indented: bool },
}

impl<W: io::Write> CompactTextRenderer<W> {
    pub fn new(writer: W) -> Self {
        CompactTextRenderer {
            writer,
            depth: 0,
            block_stack: Vec::new(),
            suppress_depth: None,
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    fn suppressed(&self) -> bool {
        self.suppress_depth.is_some()
    }

    fn indent(&mut self) -> io::Result<()> {
        write_indent(&mut self.writer, self.depth)
    }

    fn write_field(&mut self, name: &str, value: Option<&Value>, comment: Option<&str>) -> io::Result<()> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        match (value, comment) {
            (Some(val), Some(c)) => writeln!(self.writer, "{name}: {val}  // {c}"),
            (Some(val), None) => writeln!(self.writer, "{name}: {val}"),
            (None, Some(c)) => writeln!(self.writer, "{name}  // {c}"),
            (None, None) => writeln!(self.writer, "{name}"),
        }
    }
}

impl<W: io::Write> SyntaxWrite for CompactTextRenderer<W> {
    type Error = TextRenderError;

    fn begin_element(&mut self, name: &str, params: Option<&str>) -> Result<(), Self::Error> {
        if !self.suppressed() {
            self.indent()?;
            match params {
                Some(p) => writeln!(self.writer, "{name}({p}):")?,
                None => writeln!(self.writer, "{name}:")?,
            }
        }
        self.depth += 1;
        self.block_stack.push(BlockKind::Element);
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Element));
        self.depth -= 1;
        Ok(())
    }

    fn fixed_width_field(&mut self, field: &FixedWidthField<'_>) -> Result<(), Self::Error> {
        self.write_field(field.name, field.value.as_ref(), field.comment)?;
        Ok(())
    }

    fn variable_length_field(&mut self, field: &VariableLengthField<'_>) -> Result<(), Self::Error> {
        self.write_field(field.name, field.value.as_ref(), field.comment)?;
        Ok(())
    }

    fn bit_pattern(&mut self, field: &BitPatternField<'_>) -> Result<(), Self::Error> {
        self.write_field(field.name, Some(&field.value), None)?;
        Ok(())
    }

    fn raw_bytes(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        if data.len() <= 16 {
            // Single-line compact hex
            self.indent()?;
            let hex: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
            writeln!(self.writer, "{}", hex.join(" "))?;
        } else {
            let lines = format_hex_dump(data);
            for line in &lines {
                self.indent()?;
                writeln!(self.writer, "{line}")?;
            }
        }
        Ok(())
    }

    // ── Conditionals: suppress untaken branches ──────────────

    fn begin_if(
        &mut self,
        _condition: &str,
        _terms: &[TermAnnotation<'_>],
        taken: bool,
    ) -> Result<(), Self::Error> {
        if !taken && !self.suppressed() {
            self.suppress_depth = Some(self.depth);
        }
        // Taken branches are transparent — no extra indentation
        self.block_stack.push(BlockKind::If { indented: false });
        Ok(())
    }

    fn begin_else_if(
        &mut self,
        _condition: &str,
        _terms: &[TermAnnotation<'_>],
        taken: bool,
    ) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.last(), Some(BlockKind::If { .. })));
        if taken {
            if self.suppress_depth == Some(self.depth) {
                self.suppress_depth = None;
            }
        } else if !self.suppressed() {
            self.suppress_depth = Some(self.depth);
        }
        Ok(())
    }

    fn begin_else(&mut self, taken: bool) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.last(), Some(BlockKind::If { .. })));
        if taken {
            if self.suppress_depth == Some(self.depth) {
                self.suppress_depth = None;
            }
        } else if !self.suppressed() {
            self.suppress_depth = Some(self.depth);
        }
        Ok(())
    }

    fn end_if(&mut self) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.pop(), Some(BlockKind::If { .. })));
        if self.suppress_depth == Some(self.depth) {
            self.suppress_depth = None;
        }
        Ok(())
    }

    // ── Loops: compact iteration labels ──────────────────────

    fn begin_for(
        &mut self,
        _header: &str,
        _terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.depth += 1;
        self.block_stack.push(BlockKind::For);
        Ok(())
    }

    fn for_iteration(&mut self, _variable: &str, index: u64) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        writeln!(self.writer, "[{index}]:")?;
        Ok(())
    }

    fn end_for(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::For));
        self.depth -= 1;
        Ok(())
    }

    fn begin_while(&mut self, _condition: &str) -> Result<(), Self::Error> {
        self.depth += 1;
        self.block_stack.push(BlockKind::While);
        Ok(())
    }

    fn while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        writeln!(self.writer, "[{index}]:")?;
        Ok(())
    }

    fn end_while(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::While));
        self.depth -= 1;
        Ok(())
    }

    fn begin_do_while(&mut self) -> Result<(), Self::Error> {
        self.depth += 1;
        self.block_stack.push(BlockKind::DoWhile);
        Ok(())
    }

    fn do_while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        writeln!(self.writer, "[{index}]:")?;
        Ok(())
    }

    fn end_do_while(&mut self, _condition: &str) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::DoWhile));
        self.depth -= 1;
        Ok(())
    }

    // ── Switch/case: suppress untaken cases ──────────────────

    fn begin_switch(
        &mut self,
        _expression: &str,
        _terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.block_stack.push(BlockKind::Switch);
        Ok(())
    }

    fn begin_case(&mut self, _label: &str, taken: bool) -> Result<(), Self::Error> {
        if !taken && !self.suppressed() {
            self.suppress_depth = Some(self.depth);
        }
        // Taken cases are transparent — no extra indentation
        self.block_stack.push(BlockKind::Case { indented: false });
        Ok(())
    }

    fn end_case(&mut self) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.pop(), Some(BlockKind::Case { .. })));
        if self.suppress_depth == Some(self.depth) {
            self.suppress_depth = None;
        }
        Ok(())
    }

    fn end_switch(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Switch));
        Ok(())
    }

    // ── Field tables: collapsed output ───────────────────────

    fn field_table(&mut self, table: &FieldTable<'_>) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        if table.rows.is_empty() {
            return Ok(());
        }

        if table.columns.len() == 1 {
            // Single column: name: [v0, v1, v2]
            let col = &table.columns[0];
            self.indent()?;
            write!(self.writer, "{}: [", col.name)?;
            for (i, row) in table.rows.iter().enumerate() {
                if i > 0 {
                    write!(self.writer, ", ")?;
                }
                if let Some(val) = row.first() {
                    write!(self.writer, "{val}")?;
                }
            }
            writeln!(self.writer, "]")?;
        } else {
            // Multi column: aligned table with header row
            // Compute column widths from headers and values
            let mut widths: Vec<usize> = table.columns.iter().map(|c| c.name.len()).collect();
            for row in table.rows {
                for (i, val) in row.iter().enumerate() {
                    if i < widths.len() {
                        let val_len = format!("{val}").len();
                        if val_len > widths[i] {
                            widths[i] = val_len;
                        }
                    }
                }
            }

            // Header
            self.indent()?;
            for (i, col) in table.columns.iter().enumerate() {
                if i > 0 {
                    write!(self.writer, "  ")?;
                }
                write!(self.writer, "{:>width$}", col.name, width = widths[i])?;
            }
            writeln!(self.writer)?;

            // Rows
            for row in table.rows {
                self.indent()?;
                for (i, val) in row.iter().enumerate() {
                    if i > 0 {
                        write!(self.writer, "  ")?;
                    }
                    let formatted = format!("{val}");
                    if i < widths.len() {
                        write!(self.writer, "{:>width$}", formatted, width = widths[i])?;
                    } else {
                        write!(self.writer, "{formatted}")?;
                    }
                }
                writeln!(self.writer)?;
            }
        }

        Ok(())
    }

    // ── Assignments, comments, ellipsis ──────────────────────

    fn assignment(
        &mut self,
        expression: &str,
        computed_value: Option<&Value>,
    ) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        match computed_value {
            Some(val) => writeln!(self.writer, "{expression} = {val}")?,
            None => writeln!(self.writer, "{expression}")?,
        }
        Ok(())
    }

    fn comment(&mut self, text: &str) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        writeln!(self.writer, "// {text}")?;
        Ok(())
    }

    fn ellipsis(&mut self) -> Result<(), Self::Error> {
        if self.suppressed() {
            return Ok(());
        }
        self.indent()?;
        writeln!(self.writer, "...")?;
        Ok(())
    }
}
