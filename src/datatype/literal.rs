use crate::DataType;

/// Type of a literal value for things like const generics.
///
/// This also allows constructing discriminated unions in TypeScript,
/// and works well when combined with [`DataTypeFrom`](crate::DataTypeFrom).
/// You'll probably never use this type directly,
/// it's more for library authors.
#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
pub enum LiteralType {
    i8(i8),
    i16(i16),
    i32(i32),
    u8(u8),
    u16(u16),
    u32(u32),
    f32(f32),
    f64(f64),
    bool(bool),
    String(String),
    /// Standalone `null` without a known type
    None,
}

impl From<LiteralType> for DataType {
    fn from(t: LiteralType) -> Self {
        Self::Literal(t)
    }
}
