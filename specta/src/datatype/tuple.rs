use super::DataType;

/// Represents a Rust [tuple](https://doc.rust-lang.org/std/primitive.tuple.html) type.
///
/// Be aware `()` is treated specially as `null` when using the Typescript exporter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Tuple {
    pub elements: Vec<DataType>,
}

impl Tuple {
    /// Create a new tuple with the given elements.
    pub fn new(elements: Vec<DataType>) -> Self {
        Self { elements }
    }
}

impl From<Tuple> for DataType {
    fn from(t: Tuple) -> Self {
        Self::Tuple(t)
    }
}
