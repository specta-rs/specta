use super::DataType;

/// Represents a Rust [tuple](https://doc.rust-lang.org/std/primitive.tuple.html) type.
///
/// The empty tuple `()` is represented as a tuple with no elements. Exporters may
/// render that specially, such as `null` in the TypeScript exporter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Tuple {
    /// Datatypes for each tuple element, in source order.
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
