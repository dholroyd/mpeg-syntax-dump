use std::io;

use crate::render::{format_hex_dump, write_indent, INDENT_WIDTH};
use crate::types::{BitPatternField, FixedWidthField, TermAnnotation, Value, VariableLengthField};
use crate::write::SyntaxWrite;

use super::text::TextRenderError;

const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Bold magenta for keywords (if, for, while, else, do)
const KW: &str = "\x1b[1;35m";
/// Bold cyan for element names
const ELEM: &str = "\x1b[1;36m";
/// Bold for field names (the actual bitstream data fields)
const FIELD: &str = "\x1b[1m";
/// Yellow for descriptors
const DESC: &str = "\x1b[33m";
/// Green for values
const VAL: &str = "\x1b[32m";
/// Dim for comments/annotations
const COMMENT: &str = "\x1b[2m";
/// Dim for structural punctuation ({, }, (, ))
const PUNCT: &str = "\x1b[2m";

/// Column positions (same as PlainTextRenderer).
struct FieldColumns {
    width_col: usize,
    descriptor_col: usize,
    value_col: usize,
}

const COLUMNS: FieldColumns = FieldColumns {
    width_col: 49,
    descriptor_col: 58,
    value_col: 70,
};

/// ANSI-colored renderer that writes MPEG syntax with color codes.
pub struct AnsiRenderer<W> {
    writer: W,
    depth: usize,
    block_stack: Vec<BlockKind>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockKind {
    Element,
    /// Tracks whether the most recent branch in this if/else chain was taken.
    If { taken: bool },
    For,
    While,
    DoWhile,
    Switch,
    Case { taken: bool },
}

impl<W: io::Write> AnsiRenderer<W> {
    pub fn new(writer: W) -> Self {
        AnsiRenderer {
            writer,
            depth: 0,
            block_stack: Vec::new(),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    fn indent(&mut self) -> io::Result<()> {
        write_indent(&mut self.writer, self.depth)
    }

    fn current_indent_width(&self) -> usize {
        self.depth * INDENT_WIDTH
    }

    fn write_fixed_field_line(
        &mut self,
        name: &str,
        comment: Option<&str>,
        bits: u32,
        descriptor: &str,
        value: Option<&Value>,
    ) -> io::Result<()> {
        self.indent()?;

        // Name portion (with optional inline comment)
        let name_plain = match comment {
            Some(c) => format!("{FIELD}{name}{RESET}  {COMMENT}/* {c} */{RESET}"),
            None => format!("{FIELD}{name}{RESET}"),
        };
        let name_visible_len = match comment {
            Some(c) => name.len() + 2 + 3 + c.len() + 3, // name + "  " + "/* " + comment + " */"
            None => name.len(),
        };

        let indent_w = self.current_indent_width();
        let name_end = indent_w + name_visible_len;

        let width_str = bits.to_string();
        let padding1 = if name_end < COLUMNS.width_col {
            COLUMNS.width_col - name_end
        } else {
            1
        };
        write!(self.writer, "{name_plain}{:padding1$}{width_str}", "")?;

        let width_end = COLUMNS.width_col + width_str.len();
        let padding2 = if width_end < COLUMNS.descriptor_col {
            COLUMNS.descriptor_col - width_end
        } else {
            1
        };
        write!(self.writer, "{:padding2$}{DESC}{descriptor}{RESET}", "")?;

        if let Some(val) = value {
            let desc_end = COLUMNS.descriptor_col + descriptor.len();
            let padding3 = if desc_end < COLUMNS.value_col {
                COLUMNS.value_col - desc_end
            } else {
                1
            };
            write!(self.writer, "{:padding3$}= {VAL}{val}{RESET}", "")?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_variable_field_line(
        &mut self,
        name: &str,
        comment: Option<&str>,
        descriptor: &str,
        value: Option<&Value>,
    ) -> io::Result<()> {
        self.indent()?;

        let name_plain = match comment {
            Some(c) => format!("{FIELD}{name}{RESET}  {COMMENT}/* {c} */{RESET}"),
            None => format!("{FIELD}{name}{RESET}"),
        };
        let name_visible_len = match comment {
            Some(c) => name.len() + 2 + 3 + c.len() + 3,
            None => name.len(),
        };

        let indent_w = self.current_indent_width();
        let name_end = indent_w + name_visible_len;

        let padding1 = if name_end < COLUMNS.descriptor_col {
            COLUMNS.descriptor_col - name_end
        } else {
            1
        };
        write!(
            self.writer,
            "{name_plain}{:padding1$}{DESC}{descriptor}{RESET}",
            ""
        )?;

        if let Some(val) = value {
            let desc_end = COLUMNS.descriptor_col + descriptor.len();
            let padding3 = if desc_end < COLUMNS.value_col {
                COLUMNS.value_col - desc_end
            } else {
                1
            };
            write!(self.writer, "{:padding3$}= {VAL}{val}{RESET}", "")?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_term_annotations(&mut self, terms: &[TermAnnotation<'_>]) -> io::Result<()> {
        if !terms.is_empty() {
            let inner: Vec<String> = terms
                .iter()
                .map(|t| format!("{}: {}", t.name, t.value))
                .collect();
            write!(self.writer, "  {COMMENT}/* {} */{RESET}", inner.join(", "))?;
        }
        Ok(())
    }
}

impl<W: io::Write> SyntaxWrite for AnsiRenderer<W> {
    type Error = TextRenderError;

    fn begin_element(&mut self, name: &str, params: Option<&str>) -> Result<(), Self::Error> {
        self.indent()?;
        match params {
            Some(p) => writeln!(self.writer, "{ELEM}{name}{RESET}{PUNCT}({RESET}{p}{PUNCT}) {{{RESET}")?,
            None => writeln!(self.writer, "{ELEM}{name}{RESET}{PUNCT}() {{{RESET}")?,
        }
        self.depth += 1;
        self.block_stack.push(BlockKind::Element);
        Ok(())
    }

    fn end_element(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Element));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "{PUNCT}}}{RESET}")?;
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
            writeln!(self.writer, "{DIM}{line}{RESET}")?;
        }
        Ok(())
    }

    fn begin_if(
        &mut self,
        condition: &str,
        terms: &[TermAnnotation<'_>],
        taken: bool,
    ) -> Result<(), Self::Error> {
        self.indent()?;
        if taken {
            write!(self.writer, "{KW}if{RESET} {PUNCT}({RESET}{condition}{PUNCT}) {{{RESET}")?;
        } else {
            write!(self.writer, "{DIM}if ({condition}) {{{RESET}")?;
        }
        self.write_term_annotations(terms)?;
        writeln!(self.writer)?;
        self.depth += 1;
        self.block_stack.push(BlockKind::If { taken });
        Ok(())
    }

    fn begin_else_if(
        &mut self,
        condition: &str,
        terms: &[TermAnnotation<'_>],
        taken: bool,
    ) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.last(), Some(BlockKind::If { .. })));
        self.depth -= 1;
        self.indent()?;
        if taken {
            write!(
                self.writer,
                "{PUNCT}}}{RESET} {KW}else if{RESET} {PUNCT}({RESET}{condition}{PUNCT}) {{{RESET}"
            )?;
        } else {
            write!(
                self.writer,
                "{DIM}}} else if ({condition}) {{{RESET}"
            )?;
        }
        self.write_term_annotations(terms)?;
        writeln!(self.writer)?;
        self.depth += 1;
        // Update the tracked taken state for this if-chain
        if let Some(BlockKind::If { taken: t }) = self.block_stack.last_mut() {
            *t = taken;
        }
        Ok(())
    }

    fn begin_else(&mut self, taken: bool) -> Result<(), Self::Error> {
        debug_assert!(matches!(self.block_stack.last(), Some(BlockKind::If { .. })));
        self.depth -= 1;
        self.indent()?;
        if taken {
            writeln!(self.writer, "{PUNCT}}}{RESET} {KW}else{RESET} {PUNCT}{{{RESET}")?;
        } else {
            writeln!(self.writer, "{DIM}}} else {{{RESET}")?;
        }
        self.depth += 1;
        if let Some(BlockKind::If { taken: t }) = self.block_stack.last_mut() {
            *t = taken;
        }
        Ok(())
    }

    fn end_if(&mut self) -> Result<(), Self::Error> {
        let block = self.block_stack.pop();
        debug_assert!(matches!(block, Some(BlockKind::If { .. })));
        let taken = matches!(block, Some(BlockKind::If { taken: true }));
        self.depth -= 1;
        self.indent()?;
        if taken {
            writeln!(self.writer, "{PUNCT}}}{RESET}")?;
        } else {
            writeln!(self.writer, "{DIM}}}{RESET}")?;
        }
        Ok(())
    }

    fn begin_for(
        &mut self,
        header: &str,
        terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.indent()?;
        write!(self.writer, "{KW}for{RESET} {PUNCT}({RESET}{header}{PUNCT}) {{{RESET}")?;
        self.write_term_annotations(terms)?;
        writeln!(self.writer)?;
        self.depth += 1;
        self.block_stack.push(BlockKind::For);
        Ok(())
    }

    fn for_iteration(&mut self, variable: &str, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{COMMENT}/* {variable}: {index} */{RESET}")?;
        Ok(())
    }

    fn end_for(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::For));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "{PUNCT}}}{RESET}")?;
        Ok(())
    }

    fn begin_while(&mut self, condition: &str) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{KW}while{RESET} {PUNCT}({RESET}{condition}{PUNCT}) {{{RESET}")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::While);
        Ok(())
    }

    fn while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{COMMENT}/* iteration: {index} */{RESET}")?;
        Ok(())
    }

    fn end_while(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::While));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "{PUNCT}}}{RESET}")?;
        Ok(())
    }

    fn begin_do_while(&mut self) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{KW}do{RESET} {PUNCT}{{{RESET}")?;
        self.depth += 1;
        self.block_stack.push(BlockKind::DoWhile);
        Ok(())
    }

    fn do_while_iteration(&mut self, index: u64) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{COMMENT}/* iteration: {index} */{RESET}")?;
        Ok(())
    }

    fn end_do_while(&mut self, condition: &str) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::DoWhile));
        self.depth -= 1;
        self.indent()?;
        writeln!(
            self.writer,
            "{PUNCT}}}{RESET} {KW}while{RESET} {PUNCT}({RESET}{condition}{PUNCT}){RESET}"
        )?;
        Ok(())
    }

    fn begin_switch(
        &mut self,
        expression: &str,
        terms: &[TermAnnotation<'_>],
    ) -> Result<(), Self::Error> {
        self.indent()?;
        write!(self.writer, "{KW}switch{RESET} {PUNCT}({RESET}{expression}{PUNCT}) {{{RESET}")?;
        self.write_term_annotations(terms)?;
        writeln!(self.writer)?;
        self.depth += 1;
        self.block_stack.push(BlockKind::Switch);
        Ok(())
    }

    fn begin_case(&mut self, label: &str, taken: bool) -> Result<(), Self::Error> {
        self.indent()?;
        if taken {
            writeln!(self.writer, "{KW}case{RESET} {label}:")?;
        } else {
            writeln!(self.writer, "{DIM}case {label}:{RESET}")?;
        }
        self.depth += 1;
        self.block_stack.push(BlockKind::Case { taken });
        Ok(())
    }

    fn end_case(&mut self) -> Result<(), Self::Error> {
        let block = self.block_stack.pop();
        debug_assert!(matches!(block, Some(BlockKind::Case { .. })));
        let taken = matches!(block, Some(BlockKind::Case { taken: true }));
        self.indent()?;
        if taken {
            writeln!(self.writer, "{KW}break{RESET}")?;
        } else {
            writeln!(self.writer, "{DIM}break{RESET}")?;
        }
        self.depth -= 1;
        Ok(())
    }

    fn end_switch(&mut self) -> Result<(), Self::Error> {
        debug_assert_eq!(self.block_stack.pop(), Some(BlockKind::Switch));
        self.depth -= 1;
        self.indent()?;
        writeln!(self.writer, "{PUNCT}}}{RESET}")?;
        Ok(())
    }

    fn assignment(
        &mut self,
        expression: &str,
        computed_value: Option<&Value>,
    ) -> Result<(), Self::Error> {
        self.indent()?;
        match computed_value {
            Some(val) => {
                let width = COLUMNS
                    .value_col
                    .saturating_sub(self.current_indent_width() + expression.len());
                writeln!(
                    self.writer,
                    "{expression}{:width$}{COMMENT}/* = {VAL}{val}{COMMENT} */{RESET}",
                    ""
                )?;
            }
            None => writeln!(self.writer, "{expression}")?,
        }
        Ok(())
    }

    fn comment(&mut self, text: &str) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "{COMMENT}/* {text} */{RESET}")?;
        Ok(())
    }

    fn ellipsis(&mut self) -> Result<(), Self::Error> {
        self.indent()?;
        writeln!(self.writer, "...")?;
        Ok(())
    }
}
