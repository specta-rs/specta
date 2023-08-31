use std::borrow::Cow;

use crate::{DataType, EnumType, GenericType, ImplLocation, SpectaID, StructType, TupleType};

/// A NamedDataTypeImpl includes extra information which is only available for [NamedDataType]'s that come from a real Rust type.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataTypeExt {
    /// The Specta ID for the type. The value for this should come from the `sid!();` macro.
    pub(crate) sid: SpectaID,
    /// The code location where this type is implemented. Used for error reporting.
    pub(crate) impl_location: ImplLocation,
    // TODO: Undeprecate this and handle it properly!
    // TODO: Support different export contexts
    /// DEPRECATED. This is not used and shouldn't be. Will be removed in Specta v2!
    pub(crate) export: Option<bool>,
}

impl NamedDataTypeExt {
    pub fn sid(&self) -> &SpectaID {
        &self.sid
    }

    pub fn impl_location(&self) -> &ImplLocation {
        &self.impl_location
    }

    pub fn export(&self) -> &Option<bool> {
        &self.export
    }
}

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    /// The name of the type
    pub(crate) name: Cow<'static, str>,
    /// Rust documentation comments on the type
    pub(crate) comments: Vec<Cow<'static, str>>,
    /// The Rust deprecated comment if the type is deprecated.
    pub(crate) deprecated: Option<Cow<'static, str>>,
    /// Extra information that comes from a real Rust type (using the `Type` macro).
    /// This will be `None` when constructing [NamedDataType] using `StructType::to_named` or `TupleType::to_named` since those types do not correspond to actual Rust types.
    pub(crate) ext: Option<NamedDataTypeExt>,
    /// the actual type definition.
    // This field is public because we match on it in flattening code. // TODO: Review if this can be fixed when reviewing the flattening logic/error handling
    pub item: NamedDataTypeItem,
}

impl NamedDataType {
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn comments(&self) -> &Vec<Cow<'static, str>> {
        &self.comments
    }

    pub fn deprecated(&self) -> &Option<Cow<'static, str>> {
        &self.deprecated
    }

    pub fn ext(&self) -> &Option<NamedDataTypeExt> {
        &self.ext
    }
}

impl From<NamedDataType> for DataType {
    fn from(t: NamedDataType) -> Self {
        Self::Named(t)
    }
}

/// The possible types for a [`NamedDataType`].
///
/// This type will model the type of the Rust type that is being exported but be aware of the following:
/// ```rust
/// #[derive(serde::Serialize)]
/// struct Demo {}
/// // is: NamedDataTypeItem::Struct
/// // typescript: `{}`
///
/// #[derive(serde::Serialize)]
/// struct Demo2();
/// // is: NamedDataTypeItem::Tuple(TupleType::Unnamed)
/// // typescript: `[]`
///
/// #[derive(specta::Type)]
/// struct Demo3;
///// is: NamedDataTypeItem::Tuple(TupleType::Named(_))
/// // typescript: `null`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum NamedDataTypeItem {
    /// Represents an Rust struct with named fields
    Struct(StructType),
    /// Represents an Rust enum
    Enum(EnumType),
    /// Represents an Rust struct with unnamed fields
    Tuple(TupleType),
}

impl NamedDataTypeItem {
    /// Converts a [`NamedDataTypeItem`] into a [`DataType`]
    pub fn datatype(self) -> DataType {
        match self {
            Self::Struct(o) => o.into(),
            Self::Enum(e) => e.into(),
            Self::Tuple(t) => t.into(),
        }
    }

    /// Returns the generics arguments for the type
    pub fn generics(&self) -> Vec<GenericType> {
        match self {
            // Named struct
            Self::Struct(StructType { generics, .. }) => generics.clone(),
            // Enum
            Self::Enum(e) => e.generics().clone(),
            // Struct with unnamed fields
            Self::Tuple(tuple) => match tuple {
                TupleType::Unnamed => vec![],
                TupleType::Named { generics, .. } => generics.clone(),
            },
        }
    }
}
