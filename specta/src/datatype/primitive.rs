use super::DataType;

/// Rust built-in primitive type.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// [`i8`] primitive.
    i8,
    /// [`i16`] primitive.
    i16,
    /// [`i32`] primitive.
    i32,
    /// [`i64`] primitive.
    i64,
    /// [`i128`] primitive.
    i128,
    /// [`isize`] primitive.
    isize,
    /// [`u8`] primitive.
    u8,
    /// [`u16`] primitive.
    u16,
    /// [`u32`] primitive.
    u32,
    /// [`u64`] primitive.
    u64,
    /// [`u128`] primitive.
    u128,
    /// [`usize`] primitive.
    usize,
    /// [`f16`] primitive.
    f16,
    /// [`f32`] primitive.
    f32,
    /// [`f64`] primitive.
    f64,
    /// [`f128`] primitive.
    f128,
    /// [`bool`] primitive.
    bool,
    /// [`char`] primitive.
    char,
    /// [`str`] primitive.
    str,
}

impl From<Primitive> for DataType {
    fn from(t: Primitive) -> Self {
        Self::Primitive(t)
    }
}
