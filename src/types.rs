use std::fmt;

/// How a field value should be displayed.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Decimal unsigned: `= 256`
    Unsigned(u64),
    /// Decimal signed: `= -2`
    Signed(i64),
    /// Hexadecimal: `= 0x47`
    Hex(u64),
    /// Bit string: `= '111111'`
    BitString(String),
    /// Boolean displayed as 0/1: `= 1`
    Bool(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unsigned(v) => write!(f, "{v}"),
            Value::Signed(v) => write!(f, "{v}"),
            Value::Hex(v) => write!(f, "0x{v:02X}"),
            Value::BitString(s) => write!(f, "'{s}'"),
            Value::Bool(b) => write!(f, "{}", if *b { 1 } else { 0 }),
        }
    }
}

/// Term annotation: `/* name: value */` on condition/loop lines.
pub struct TermAnnotation<'a> {
    pub name: &'a str,
    pub value: Value,
}

/// A fixed-width field read from the bitstream.
pub struct FixedWidthField<'a> {
    /// Field name, e.g. `"sync_byte"`, `"offset_for_ref_frame[ 0 ]"`
    pub name: &'a str,
    /// Number of bits
    pub bits: u32,
    /// Type descriptor, e.g. `"bslbf"`, `"u(8)"`, `"f(1)"`
    pub descriptor: &'a str,
    /// Decoded value, or `None` for raw-byte field templates
    pub value: Option<Value>,
    /// Inline comment, e.g. `"equal to 0x03"`
    pub comment: Option<&'a str>,
}

/// A variable-length coded field.
pub struct VariableLengthField<'a> {
    /// Field name
    pub name: &'a str,
    /// Descriptor, e.g. `"ue(v)"`, `"se(v)"`, `"me(v)"`
    pub descriptor: &'a str,
    /// Decoded value
    pub value: Option<Value>,
    /// Inline comment
    pub comment: Option<&'a str>,
}

/// A fixed bit pattern or marker bit field.
pub struct BitPatternField<'a> {
    /// Pattern name, e.g. `"'0010'"` or `"marker_bit"`
    pub name: &'a str,
    /// Number of bits
    pub bits: u32,
    /// Descriptor, e.g. `"bslbf"`
    pub descriptor: &'a str,
    /// The actual value
    pub value: Value,
}
