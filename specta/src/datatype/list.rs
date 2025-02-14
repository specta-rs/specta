use super::DataType;

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    // The type of the elements in the list.
    pub(crate) ty: Box<DataType>,
    // Length is set for `[Type; N]` arrays.
    pub(crate) length: Option<usize>,
    // Are each elements unique? Eg. `HashSet` or `BTreeSet`
    pub(crate) unique: bool,
}

impl List {
    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    pub fn length(&self) -> Option<usize> {
        self.length
    }

    pub fn unique(&self) -> bool {
        self.unique
    }
}

impl From<List> for DataType {
    fn from(t: List) -> Self {
        Self::List(t)
    }
}
