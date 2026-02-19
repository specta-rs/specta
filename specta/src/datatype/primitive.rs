use super::DataType;

/// Type of primitives like numbers and strings.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// An `i8` primitive.
    i8,
    /// An `i16` primitive.
    i16,
    /// An `i32` primitive.
    i32,
    /// An `i64` primitive.
    i64,
    /// An `i128` primitive.
    i128,
    /// An `isize` primitive.
    isize,
    /// A `u8` primitive.
    u8,
    /// A `u16` primitive.
    u16,
    /// A `u32` primitive.
    u32,
    /// A `u64` primitive.
    u64,
    /// A `u128` primitive.
    u128,
    /// A `usize` primitive.
    usize,
    /// An `f16` primitive.
    f16,
    /// An `f32` primitive.
    f32,
    /// An `f64` primitive.
    f64,
    /// A `bool` primitive.
    bool,
    /// A `char` primitive.
    char,
    /// A `String` primitive.
    String,
}

impl From<Primitive> for DataType {
    fn from(t: Primitive) -> Self {
        Self::Primitive(t)
    }
}
