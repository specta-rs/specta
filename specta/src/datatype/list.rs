use super::DataType;

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    ty: Box<DataType>,
    length: Option<usize>,
    unique: bool,
}

impl List {
    pub fn new(ty: DataType, length: Option<usize>, unique: bool) -> Self {
        Self {
            ty: Box::new(ty),
            length,
            unique,
        }
    }

    /// The type of the elements in the list.
    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    /// Length is set for `[Type; N]` arrays.
    pub fn length(&self) -> Option<usize> {
        self.length
    }

    /// Are each elements unique? Eg. `HashSet` or `BTreeSet`
    pub fn unique(&self) -> bool {
        self.unique
    }
}

impl From<List> for DataType {
    fn from(t: List) -> Self {
        Self::List(t)
    }
}
