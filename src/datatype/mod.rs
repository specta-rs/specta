use std::{
    borrow::{Borrow, Cow},
    collections::BTreeMap,
};

mod r#enum;
mod literal;
mod object;
mod primitive;
mod tuple;

pub use literal::*;
pub use object::*;
pub use primitive::*;
pub use r#enum::*;
pub use tuple::*;

use crate::{ImplLocation, TypeSid};

/// A map used to store the types "discovered" while exporting a type.
/// You can iterate over this to export all types which the type/s you exported references on.
///
/// [`None`] indicates that the entry is a placeholder. It was reference but we haven't reached it's definition yet.
pub type TypeMap = BTreeMap<TypeSid, Option<NamedDataType>>;

/// Arguments for [`Type::inline`](crate::Type::inline), [`Type::reference`](crate::Type::reference) and [`Type::definition`](crate::Type::definition).
pub struct DefOpts<'a> {
    /// is the parent type inlined?
    pub parent_inline: bool,
    /// a map of types which have been visited. This prevents stack overflows when a type references itself and also allows the caller to get a list of all types in the "schema".
    pub type_map: &'a mut TypeMap,
}

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum DataType {
    // Always inlined
    Any,
    Primitive(PrimitiveType),
    Literal(LiteralType),
    /// Either a `Set` or a `Vec`
    List(Box<DataType>),
    Nullable(Box<DataType>),
    Map(Box<(DataType, DataType)>),
    // Named reference types
    Named(NamedDataType),
    // Anonymous Reference types
    Object(ObjectType),
    Enum(EnumType),
    Tuple(TupleType),
    // Result
    Result(Box<(DataType, DataType)>),
    // A reference type that has already been defined
    Reference(DataTypeReference),
    Generic(GenericType),
}

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    /// The name of the type
    pub name: Cow<'static, str>,
    /// The Specta ID for the type. The value for this should come from the `sid!();` macro.
    pub sid: Option<TypeSid>,
    /// The code location where this type is implemented. Used for error reporting.
    pub impl_location: Option<ImplLocation>,
    /// Rust documentation comments on the type
    pub comments: Vec<Cow<'static, str>>,
    /// DEPRECATED. This is not used and shouldn't be. Will be removed in Specta v2!
    pub export: Option<bool>,
    /// The Rust deprecated comment if the type is deprecated.
    pub deprecated: Option<Cow<'static, str>>,
    /// the actual type definition.
    pub item: NamedDataTypeItem,
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
/// // is: NamedDataTypeItem::Object
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
    Object(ObjectType),
    /// Represents an Rust enum
    Enum(EnumType),
    /// Represents an Rust struct with unnamed fields
    Tuple(TupleType),
}

impl NamedDataTypeItem {
    /// Converts a [`NamedDataTypeItem`] into a [`DataType`]
    pub fn datatype(self) -> DataType {
        match self {
            Self::Object(o) => o.into(),
            Self::Enum(e) => e.into(),
            Self::Tuple(t) => t.into(),
        }
    }

    /// Returns the generics arguments for the type
    pub fn generics(&self) -> Vec<GenericType> {
        match self {
            // Named struct
            Self::Object(ObjectType { generics, .. }) => generics.clone(),
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

/// A reference to a [`DataType`] that can be used before a type is resolved in order to
/// support recursive types without causing an infinite loop.
///
/// This works since a child type that references a parent type does not care about the
/// parent's fields, only really its name. Once all of the parent's fields have been
/// resolved will the parent's definition be placed in the type map.
///
// This doesn't account for flattening and inlining recursive types, however, which will
// require a more complex solution since it will require multiple processing stages.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct DataTypeReference {
    pub name: Cow<'static, str>,
    pub sid: TypeSid,
    pub generics: Vec<DataType>,
}

/// A generic ("placeholder") argument to a Specta-enabled type.
///
/// A generic does not hold a specific type instead it acts as a slot where a type can be provided when referencing this type.
///
/// A `GenericType` holds the identifier of the generic. Eg. Given a generic type `struct A<T>(T);` the generics will be represented as `vec![GenericType("A".into())]`
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct GenericType(pub Cow<'static, str>);

impl Borrow<str> for GenericType {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<'a> From<&'a str> for GenericType {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned().into())
    }
}

impl From<GenericType> for DataType {
    fn from(t: GenericType) -> Self {
        Self::Generic(t)
    }
}

impl<T: Into<DataType> + 'static> From<Vec<T>> for DataType {
    fn from(t: Vec<T>) -> Self {
        DataType::Enum(EnumType::Untagged {
            variants: t
                .into_iter()
                .map(|t| {
                    EnumVariant::Unnamed(TupleType::Named {
                        fields: vec![t.into()],
                        generics: vec![],
                    })
                })
                .collect(),
            generics: vec![],
        })
    }
}

impl<T: Into<DataType> + 'static> From<Option<T>> for DataType {
    fn from(t: Option<T>) -> Self {
        t.map(Into::into)
            .unwrap_or_else(|| LiteralType::None.into())
    }
}

impl<'a> From<&'a str> for DataType {
    fn from(t: &'a str) -> Self {
        LiteralType::String(t.to_string()).into()
    }
}

impl From<String> for DataType {
    fn from(t: String) -> Self {
        LiteralType::String(t).into()
    }
}

impl From<Cow<'static, str>> for DataType {
    fn from(t: Cow<'static, str>) -> Self {
        LiteralType::String(t.to_string()).into()
    }
}
