use crate::{
    builder::StructBuilder,
    datatype::{DataType, Fields, RuntimeAttribute},
};

use super::{NamedFields, UnnamedFields};

/// represents a Rust [struct](https://doc.rust-lang.org/std/keyword.struct.html).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Struct {
    pub(crate) fields: Fields,
    pub(crate) attributes: Vec<RuntimeAttribute>,
}

impl Struct {
    /// Construct a new struct with no fields. Fields can be set later with `set_fields` or `fields_mut`.
    pub fn new() -> Self {
        Self {
            fields: Fields::Unit,
            attributes: Default::default(),
        }
    }

    /// Construct a new unit struct.
    pub fn unit() -> Self {
        Self::new()
    }

    /// Construct a named struct.
    pub fn named() -> StructBuilder<NamedFields> {
        StructBuilder {
            fields: NamedFields {
                fields: Default::default(),
                attributes: Default::default(),
            },
        }
    }

    /// Construct an unnamed struct.
    pub fn unnamed() -> StructBuilder<UnnamedFields> {
        StructBuilder {
            fields: UnnamedFields {
                fields: Default::default(),
                attributes: Default::default(),
            },
        }
    }

    /// Get a immutable reference to the fields of the struct.
    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    /// Get a mutable reference to the fields of the struct.
    pub fn fields_mut(&mut self) -> &mut Fields {
        &mut self.fields
    }

    /// Set the fields of the struct.
    pub fn set_fields(&mut self, fields: Fields) {
        self.fields = fields;
    }

    /// Get a immutable reference to the attributes of the struct.
    pub fn attributes(&self) -> &Vec<RuntimeAttribute> {
        &self.attributes
    }

    /// Get a mutable reference to the attributes of the struct.
    pub fn attributes_mut(&mut self) -> &mut Vec<RuntimeAttribute> {
        &mut self.attributes
    }

    /// Set the attributes of the struct.
    pub fn set_attributes(&mut self, attributes: Vec<RuntimeAttribute>) {
        self.attributes = attributes;
    }
}

impl From<Struct> for DataType {
    fn from(t: Struct) -> Self {
        Self::Struct(t)
    }
}
