//! Types related to working with [`DataType`]. Exposed for advanced users.

mod r#enum;
mod fields;
mod function;
mod generic;
mod inline;
mod list;
mod literal;
mod map;
mod named;
mod primitive;
pub mod reference;
mod r#struct;
mod tuple;

pub use fields::*;
pub use function::*;
pub use generic::*;
pub use inline::*;
pub use list::*;
pub use literal::*;
pub use map::*;
pub use named::*;
pub use primitive::*;
pub use r#enum::*;
pub use r#struct::*;
pub use tuple::*;

use crate::SpectaID;
use std::borrow::Cow;

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // Always inlined
    Any,
    Unknown,
    Primitive(PrimitiveType),
    Literal(LiteralType),
    /// Either a `Set` or a `Vec`
    List(List),
    Map(Map),
    Nullable(Box<DataType>),
    // Anonymous Reference types
    Struct(StructType),
    Enum(EnumType),
    Tuple(TupleType),
    // A reference type that has already been defined
    Reference(reference::Reference),
    Generic(GenericType),
}

impl DataType {
    pub fn generics(&self) -> Option<&Vec<GenericType>> {
        match self {
            Self::Struct(s) => Some(s.generics()),
            Self::Enum(e) => Some(e.generics()),
            _ => None,
        }
    }

    /// convert a [`DataType`] to a named [`NamedDataType`].
    ///
    /// This is perfect if you want to export a type as a named type.
    pub fn to_named(self, name: impl Into<Cow<'static, str>>) -> NamedDataType {
        NamedDataType {
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            ext: None,
            inner: self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeprecatedType {
    /// A type that has been deprecated without a message.
    ///
    /// Eg. `#[deprecated]`
    Deprecated,
    /// A type that has been deprecated with a message and an optional `since` version.
    ///
    /// Eg. `#[deprecated = "Use something else"]` or `#[deprecated(since = "1.0.0", message = "Use something else")]`
    DeprecatedWithSince {
        since: Option<Cow<'static, str>>,
        note: Cow<'static, str>,
    },
}

impl<T: Into<DataType> + 'static> From<Vec<T>> for DataType {
    fn from(t: Vec<T>) -> Self {
        DataType::Enum(EnumType {
            name: "Vec".into(),
            sid: None,
            repr: EnumRepr::Untagged,
            skip_bigint_checks: false,
            variants: t
                .into_iter()
                .map(|t| {
                    let ty: DataType = t.into();
                    (
                        match &ty {
                            DataType::Struct(s) => s.name.clone(),
                            DataType::Enum(e) => e.name().clone(),
                            // TODO: This is probs gonna cause problems so we should try and remove the need for this entire impl block if we can.
                            _ => "".into(),
                        },
                        EnumVariant {
                            skip: false,
                            docs: Cow::Borrowed(""),
                            deprecated: None,
                            fields: Fields::Unnamed(UnnamedFields {
                                fields: vec![Field {
                                    optional: false,
                                    flatten: false,
                                    deprecated: None,
                                    docs: Cow::Borrowed(""),
                                    inline: false,
                                    ty: Some(ty),
                                }],
                            }),
                        },
                    )
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

impl<'a> From<Cow<'a, str>> for DataType {
    fn from(t: Cow<'a, str>) -> Self {
        LiteralType::String(t.to_string()).into()
    }
}
