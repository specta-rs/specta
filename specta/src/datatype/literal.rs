use std::borrow::Cow;

use super::DataType;

/// Type of a literal value for things like const generics.
///
/// This also allows constructing discriminated unions in TypeScript,
/// and works well when combined with [`DataTypeFrom`](crate::DataTypeFrom).
/// You'll probably never use this type directly,
/// it's more for library authors.
#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
#[non_exhaustive] // TODO: Yes or no???
pub enum Literal {
    i8(i8),
    i16(i16),
    i32(i32),
    u8(u8),
    u16(u16),
    u32(u32),
    f32(f32),
    f64(f64),
    bool(bool),
    String(Cow<'static, str>),
    char(char),
    /// Standalone `null` without a known type
    None,
}

impl From<Literal> for DataType {
    fn from(t: Literal) -> Self {
        Self::Literal(t)
    }
}

macro_rules! impl_literal_conversion {
    ($($i:ident)+) => {$(
        impl From<$i> for Literal {
            fn from(t: $i) -> Self {
                Self::$i(t)
            }
        }
    )+};
}

impl_literal_conversion!(i8 i16 i32 u8 u16 u32 f32 f64 bool char);

impl From<String> for Literal {
    fn from(t: String) -> Self {
        Self::String(Cow::Owned(t))
    }
}
