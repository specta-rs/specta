use std::borrow::Cow;

use crate::{
    builder::StructBuilder,
    datatype::{DataType, Fields},
};

use super::{NamedFields, UnnamedFields};

/// represents a Rust [struct](https://doc.rust-lang.org/std/keyword.struct.html).
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub(crate) fields: Fields,
}

impl Struct {
    /// Construct a new unit struct.
    pub fn unit() -> Self {
        Self {
            fields: Fields::Unit,
        }
    }

    /// Construct a named struct.
    pub fn named() -> StructBuilder<NamedFields> {
        StructBuilder {
            fields: NamedFields {
                fields: Default::default(),
                tag: Default::default(),
            },
        }
    }

    /// Construct an unnamed struct.
    pub fn unnamed() -> StructBuilder<UnnamedFields> {
        StructBuilder {
            fields: UnnamedFields {
                fields: Default::default(),
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

    /// Get a immutable reference to the tag of the struct.
    pub fn tag(&self) -> Option<&Cow<'static, str>> {
        match &self.fields {
            Fields::Unit => None,
            Fields::Unnamed(_) => None,
            Fields::Named(named) => named.tag.as_ref(),
        }
    }

    /// Set the tag of the struct.
    pub fn set_tag(&mut self, tag: Option<Cow<'static, str>>) {
        match &mut self.fields {
            Fields::Unit => {}
            Fields::Unnamed(_) => {}
            Fields::Named(named) => named.tag = tag,
        }
    }
}

impl From<Struct> for DataType {
    fn from(t: Struct) -> Self {
        Self::Struct(t)
    }
}
