use crate::datatype::{Attributes, DataType, Fields};

use super::StructBuilder;

use super::{NamedFields, UnnamedFields};

/// Runtime representation of a Rust [`struct`](https://doc.rust-lang.org/std/keyword.struct.html).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Struct {
    /// Field layout for the struct.
    pub fields: Fields,
    /// Runtime attributes attached to the struct container.
    pub attributes: Attributes,
}

// Do not implement `Default` for `Struct` as it's unclear what that would be. `Unit`, yes but still.

impl Struct {
    /// Construct a new unit struct.
    pub fn unit() -> Self {
        Self {
            fields: Fields::Unit,
            attributes: Default::default(),
        }
    }

    /// Starts building a struct with named fields.
    pub fn named() -> StructBuilder<NamedFields> {
        StructBuilder {
            fields: NamedFields {
                fields: Default::default(),
            },
        }
    }

    /// Starts building a tuple struct with unnamed fields.
    pub fn unnamed() -> StructBuilder<UnnamedFields> {
        StructBuilder {
            fields: UnnamedFields {
                fields: Default::default(),
            },
        }
    }
}

impl From<Struct> for DataType {
    fn from(t: Struct) -> Self {
        Self::Struct(t)
    }
}
