use super::DataType;

/// Type of primitives like numbers and strings.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Primitive {
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f16,
    f32,
    f64,
    bool,
    char,
    String,
}

// impl Primitive {
//     /// Converts a [`Primitive`] into a Rust code string.
//     pub fn to_rust_str(&self) -> &'static str {
//         match self {
//             Self::i8 => "i8",
//             Self::i16 => "i16",
//             Self::i32 => "i32",
//             Self::i64 => "i64",
//             Self::i128 => "i128",
//             Self::isize => "isize",
//             Self::u8 => "u8",
//             Self::u16 => "u16",
//             Self::u32 => "u32",
//             Self::u64 => "u64",
//             Self::u128 => "u128",
//             Self::usize => "usize",
//             Self::f16 => "f16",
//             Self::f32 => "f32",
//             Self::f64 => "f64",
//             Self::bool => "bool",
//             Self::char => "char",
//             Self::String => "String",
//         }
//     }
// }

impl From<Primitive> for DataType {
    fn from(t: Primitive) -> Self {
        Self::Primitive(t)
    }
}
