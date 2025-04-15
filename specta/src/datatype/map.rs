use super::DataType;

#[derive(Debug, Clone, PartialEq)]
pub struct Map(Box<(DataType, DataType)>);

impl Map {
    pub fn new(key_ty: DataType, value_ty: DataType) -> Self {
        Self(Box::new((key_ty, value_ty)))
    }

    /// The type of the map keys.
    pub fn key_ty(&self) -> &DataType {
        &self.0 .0
    }

    /// The type of the map values.
    pub fn value_ty(&self) -> &DataType {
        &self.0 .1
    }
}

impl From<Map> for DataType {
    fn from(t: Map) -> Self {
        Self::Map(t)
    }
}
