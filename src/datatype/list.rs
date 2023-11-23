use crate::DataType;

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    // The type of the elements in the list.
    pub(crate) ty: Box<DataType>,
    // Length is set for `[Type; N]` arrays.
    pub(crate) length: Option<usize>,
}

impl List {
    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    pub fn length(&self) -> Option<usize> {
        self.length
    }
}
