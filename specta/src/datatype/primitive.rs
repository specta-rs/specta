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

impl From<Primitive> for DataType {
    fn from(t: Primitive) -> Self {
        Self::Primitive(t)
    }
}
