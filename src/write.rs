use crate::types::{BitPatternField, FixedWidthField, TermAnnotation, Value, VariableLengthField};

/// Trait for rendering MPEG specification syntax structures.
///
/// Renderers implement this trait to produce output in a specific format
/// (plain text, ANSI-colored text, HTML, etc.). Producers call these
/// methods to describe the syntax structure and field values.
///
/// Methods use `begin_`/`end_` pairs rather than closures to avoid
/// borrow-checker issues when the producer needs `&mut W` inside a
/// closure while also borrowing `&self` for data access.
pub trait SyntaxWrite {
    type Error: std::error::Error;

    // ── Element blocks ──────────────────────────────────────

    /// Begin a named syntax element (a "syntax table" in MPEG specs).
    ///
    /// `params` is `None` for unparameterized elements like `transport_packet()`,
    /// or `Some("payloadType, payloadSize")` for parameterized ones like
    /// `sei_payload(payloadType, payloadSize)`.
    fn begin_element(&mut self, name: &str, params: Option<&str>)
        -> Result<(), Self::Error>;

    /// End the current syntax element.
    fn end_element(&mut self) -> Result<(), Self::Error>;

    // ── Fields ──────────────────────────────────────────────

    /// Render a fixed-width field.
    fn fixed_width_field(&mut self, field: &FixedWidthField<'_>)
        -> Result<(), Self::Error>;

    /// Render a variable-length coded field.
    fn variable_length_field(&mut self, field: &VariableLengthField<'_>)
        -> Result<(), Self::Error>;

    /// Render a fixed bit pattern or marker bit.
    fn bit_pattern(&mut self, field: &BitPatternField<'_>)
        -> Result<(), Self::Error>;

    // ── Raw byte hex dump ───────────────────────────────────

    /// Render raw bytes as a hex dump. Called after a field template line;
    /// the renderer formats as hex lines (16 bytes per line).
    fn raw_bytes(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    // ── Conditionals ────────────────────────────────────────

    /// Begin an `if` block. The renderer adds `if (condition) {` and any
    /// term annotations. `taken` is a hint for dimming not-taken branches.
    fn begin_if(&mut self, condition: &str,
        terms: &[TermAnnotation<'_>], taken: bool)
        -> Result<(), Self::Error>;

    /// Close the previous branch and open an `else if` branch.
    fn begin_else_if(&mut self, condition: &str,
        terms: &[TermAnnotation<'_>], taken: bool)
        -> Result<(), Self::Error>;

    /// Close the previous branch and open an `else` branch.
    fn begin_else(&mut self, taken: bool)
        -> Result<(), Self::Error>;

    /// Close the final branch of an if/else-if/else chain.
    fn end_if(&mut self) -> Result<(), Self::Error>;

    // ── For loops ───────────────────────────────────────────

    /// Begin a `for` loop. `header` is the loop clause, e.g.
    /// `"i = 0; i < N; i++"`. The renderer adds `for (header) {`.
    fn begin_for(&mut self, header: &str,
        terms: &[TermAnnotation<'_>])
        -> Result<(), Self::Error>;

    /// Mark a for-loop iteration with the variable name and index.
    fn for_iteration(&mut self, variable: &str, index: u64)
        -> Result<(), Self::Error>;

    /// End a `for` loop.
    fn end_for(&mut self) -> Result<(), Self::Error>;

    // ── While loops ─────────────────────────────────────────

    /// Begin a `while` loop.
    fn begin_while(&mut self, condition: &str)
        -> Result<(), Self::Error>;

    /// Mark a while-loop iteration.
    fn while_iteration(&mut self, index: u64)
        -> Result<(), Self::Error>;

    /// End a `while` loop.
    fn end_while(&mut self) -> Result<(), Self::Error>;

    // ── Do-while loops ──────────────────────────────────────

    /// Begin a `do-while` loop.
    fn begin_do_while(&mut self) -> Result<(), Self::Error>;

    /// Mark a do-while iteration.
    fn do_while_iteration(&mut self, index: u64)
        -> Result<(), Self::Error>;

    /// End a `do-while` loop with the given condition.
    fn end_do_while(&mut self, condition: &str)
        -> Result<(), Self::Error>;

    // ── Switch/case ────────────────────────────────────────

    /// Begin a `switch` statement. `expression` is the switch discriminator,
    /// e.g. `"id"`. `terms` provides term annotations for the discriminator.
    fn begin_switch(&mut self, expression: &str,
        terms: &[TermAnnotation<'_>]) -> Result<(), Self::Error>;

    /// Begin a `case` within a switch. `label` is the case label,
    /// e.g. `"ID_CPE"`. `taken` indicates whether this is the active case.
    fn begin_case(&mut self, label: &str, taken: bool)
        -> Result<(), Self::Error>;

    /// End the current case.
    fn end_case(&mut self) -> Result<(), Self::Error>;

    /// End the switch statement.
    fn end_switch(&mut self) -> Result<(), Self::Error>;

    // ── Assignments ─────────────────────────────────────────

    /// Render an inline variable assignment. `computed_value` of `Some(value)`
    /// renders a trailing `/* = value */` annotation.
    fn assignment(&mut self, expression: &str,
        computed_value: Option<&Value>) -> Result<(), Self::Error>;

    // ── Comments and ellipsis ───────────────────────────────

    /// Render a standalone comment line.
    fn comment(&mut self, text: &str) -> Result<(), Self::Error>;

    /// Render an ellipsis (`...`) indicating omitted content.
    fn ellipsis(&mut self) -> Result<(), Self::Error>;
}
