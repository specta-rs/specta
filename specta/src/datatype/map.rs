use super::DataType;

#[derive(Debug, Clone, PartialEq)]

pub struct Map {
    // TODO: Box these fields together as an internal optimization.
    // The type of the map keys.
    pub(crate) key_ty: Box<DataType>,
    // The type of the map values.
    pub(crate) value_ty: Box<DataType>,
}

impl Map {
    pub fn key_ty(&self) -> &DataType {
        &self.key_ty
    }

    pub fn value_ty(&self) -> &DataType {
        &self.value_ty
    }
}
