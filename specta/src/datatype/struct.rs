use crate::datatype::{Attributes, DataType, Fields};

use super::StructBuilder;

use super::{NamedFields, UnnamedFields};

/// represents a Rust [struct](https://doc.rust-lang.org/std/keyword.struct.html).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Struct {
    pub fields: Fields,
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

    /// Construct a named struct.
    pub fn named() -> StructBuilder<NamedFields> {
        StructBuilder {
            fields: NamedFields {
                fields: Default::default(),
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
}

impl From<Struct> for DataType {
    fn from(t: Struct) -> Self {
        Self::Struct(t)
    }
}
