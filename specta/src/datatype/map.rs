use super::DataType;

/// A map of items. This will be a [`HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html) or similar types.
#[derive(Debug, Clone, PartialEq)]
pub struct Map(Box<(DataType, DataType)>);

impl Map {
    /// Create a new map with the given key and value types.
    pub fn new(key_ty: DataType, value_ty: DataType) -> Self {
        Self(Box::new((key_ty, value_ty)))
    }

    /// The type of the map keys.
    pub fn key_ty(&self) -> &DataType {
        &self.0 .0
    }

    /// Get a mutable reference to the type of the map keys.
    pub fn key_ty_mut(&mut self) -> &mut DataType {
        &mut self.0 .0
    }

    /// Set the type of the map keys.
    pub fn set_key_ty(&mut self, key_ty: DataType) {
        self.0 .0 = key_ty;
    }

    /// The type of the map values.
    pub fn value_ty(&self) -> &DataType {
        &self.0 .1
    }

    /// Get a mutable reference to the type of the map values.
    pub fn value_ty_mut(&mut self) -> &mut DataType {
        &mut self.0 .1
    }

    /// Set the type of the map values.
    pub fn set_value_ty(&mut self, value_ty: DataType) {
        self.0 .1 = value_ty;
    }
}

impl From<Map> for DataType {
    fn from(t: Map) -> Self {
        Self::Map(t)
    }
}
