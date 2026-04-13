use super::DataType;

/// List of items. This will be a [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) or similar types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct List {
    pub ty: Box<DataType>,
    pub length: Option<usize>,
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
