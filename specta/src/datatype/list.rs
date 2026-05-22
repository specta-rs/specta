use super::DataType;

/// Sequential collection type, such as [`Vec`](std::vec::Vec), arrays, slices,
/// or set-like collections.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct List {
    /// Type of each element in the list.
    pub ty: Box<DataType>,
    /// Fixed number of elements when known.
    ///
    /// `None` represents a variable-length collection.
    pub length: Option<usize>,
    /// Whether elements are expected to be unique, as with set-like types.
    pub unique: bool,
}

impl List {
    /// Create a new list of a given type.
    pub fn new(ty: DataType) -> Self {
        Self {
            ty: Box::new(ty),
            length: None,
            unique: false,
        }
    }
}

impl From<List> for DataType {
    fn from(t: List) -> Self {
        Self::List(t)
    }
}
