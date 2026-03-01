use crate::write::SyntaxWrite;

/// Trait for types that can describe their MPEG syntax structure.
///
/// Producers implement this trait on wrapper structs that capture parsed
/// data and any needed context, then call methods on the provided
/// `SyntaxWrite` renderer to emit the syntax structure and field values.
pub trait SyntaxDescribe {
    fn describe<W: SyntaxWrite>(&self, w: &mut W) -> Result<(), W::Error>;
}
