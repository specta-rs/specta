use super::DataType;

/// A regular tuple
///
/// Represented in Rust as `(...)` and in TypeScript as `[...]`.
/// Be aware `()` is treated specially as `null` in Typescript.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub(crate) elements: Vec<DataType>,
}

impl Tuple {
    pub fn elements(&self) -> &Vec<DataType> {
        &self.elements
    }
}

impl From<Tuple> for DataType {
    fn from(t: Tuple) -> Self {
        Self::Tuple(t)
    }
}
