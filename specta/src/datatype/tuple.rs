use super::DataType;

/// Represents a Rust [tuple](https://doc.rust-lang.org/std/primitive.tuple.html) type.
///
/// Be aware `()` is treated specially as `null` when using the Typescript exporter.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub(crate) elements: Vec<DataType>,
}

impl Tuple {
    /// Create a new tuple with the given elements.
    pub fn new(elements: Vec<DataType>) -> Self {
        Self { elements }
    }

    /// Get the elements of the tuple.
    pub fn elements(&self) -> &[DataType] {
        &self.elements
    }

    /// Get a mutable reference to the elements of the tuple.
    pub fn elements_mut(&mut self) -> &mut Vec<DataType> {
        &mut self.elements
    }
}

impl From<Tuple> for DataType {
    fn from(t: Tuple) -> Self {
        Self::Tuple(t)
    }
}
