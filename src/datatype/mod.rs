use std::collections::BTreeMap;

mod r#enum;
mod object;

pub use object::*;
pub use r#enum::*;

use crate::{ImplLocation, TypeSid};

/// A map of type definitions
pub type TypeDefs = BTreeMap<TypeSid, DataType>;

/// arguments for [Type::inline](crate::Type::inline), [Type::reference](crate::Type::reference) and [Type::definition](crate::Type::definition).
pub struct DefOpts<'a> {
    /// is the parent type inlined?
    pub parent_inline: bool,
    /// a map of types which have been visited. This prevents stack overflows when a type references itself and also allows the caller to get a list of all types in the "schema".
    pub type_map: &'a mut TypeDefs,
}

/// A wrapper around [DataTypeItem] to store general information about the type.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum DataType {
    // Always inlined
    Any,
    Primitive(PrimitiveType),
    Literal(LiteralType),
    List(Box<DataType>),
    Nullable(Box<DataType>),
    Record(Box<(DataType, DataType)>),
    Tuple(TupleType),
    // Reference types
    Object(CustomDataType<ObjectType>),
    Enum(CustomDataType<EnumType>),
    // A reference type that has already been defined
    Reference(DataTypeReference),
    Generic(GenericType),
    /// Used when the type is not yet known. This allows us to avoid stack overflows.
    /// It should never be returned from the Specta functions. Doing so is classed as a bug!
    Placeholder,
}

/// Datatype to be put in the type map while field types are being resolved. Used in order to
/// support recursive types without causing an infinite loop.
///
/// This works since a child type that references a parent type does not care about the
/// parent's fields, only really its name. Once all of the parent's fields have been
/// resolved will the parent's definition be placed in the type map.
///
/// This doesn't account for flattening and inlining recursive types, however, which will
/// require a more complex solution since it will require multiple processing stages.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct DataTypeReference {
    pub name: &'static str,
    pub sid: TypeSid,
    pub generics: Vec<DataType>,
}

impl DataType {
    pub fn should_export(&self, default: bool) -> bool {
        match self {
            // TODO: Why did I comment these out? -> I think they can be removed?
            // DataTypeItem::Reference { .. } => true,
            // DataTypeItem::Generic(_) => true,
            DataType::Object(CustomDataType::Named { export, .. }) => export.unwrap_or(default),
            DataType::Enum(CustomDataType::Named { export, .. }) => export.unwrap_or(default),

            _ => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        // TODO: Can this emit the name even if wrapped in primitves? Eg. `Option<MyAwesomeCustomStruct>`?
        todo!();
    }

    pub fn sid(&self) -> Option<TypeSid> {
        match self {
            DataType::Object(CustomDataType::Named { sid, .. }) => Some(*sid),
            DataType::Enum(CustomDataType::Named { sid, .. }) => Some(*sid),
            DataType::Reference(DataTypeReference { sid, .. }) => Some(*sid), // TODO: Should I have this case?
            _ => None,
        }
    }
}

// TODO: Should a bunch of this stuff be moved into the `specta::datatype` module?

#[derive(Debug, Clone, PartialEq)]
pub enum CustomDataType<T> {
    Named {
        /// The name of the type
        name: &'static str,
        /// The Specta ID for the type. The value for this should come from the `sid!();` macro.
        sid: TypeSid,
        /// The code location where this type is implemented. Used for error reporting.
        impl_location: ImplLocation,
        /// Rust documentation comments on the type
        comments: &'static [&'static str],
        // Whether the type should export when the `export` feature is enabled.
        /// `None` will use the default which is why `false` is not just used.
        export: Option<bool>,
        /// The Rust deprecated comment if the type is deprecated.
        deprecated: Option<&'static str>,
        item: T,
    },
    Unnamed(T),
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct GenericType(pub &'static str); // TODO: Include SID and maybe lookup based on that?

/// this is used internally to represent the types.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum PrimitiveType {
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    char,
    String,
}

impl PrimitiveType {
    /// Converts a `PrimitiveType` into a Rust code string.
    pub fn to_rust_str(&self) -> &'static str {
        match self {
            Self::i8 => "i8",
            Self::i16 => "i16",
            Self::i32 => "i32",
            Self::i64 => "i64",
            Self::i128 => "i128",
            Self::isize => "isize",
            Self::u8 => "u8",
            Self::u16 => "u16",
            Self::u32 => "u32",
            Self::u64 => "u64",
            Self::u128 => "u128",
            Self::usize => "usize",
            Self::f32 => "f32",
            Self::f64 => "f64",
            Self::bool => "bool",
            Self::char => "char",
            Self::String => "String",
        }
    }
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct TupleType {
    pub fields: Vec<DataType>,
    pub generics: Vec<&'static str>,
}

/// this is used internally to represent the types.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum LiteralType {
    i8(i8),
    i16(i16),
    i32(i32),
    u8(u8),
    u16(u16),
    u32(u32),
    f32(f32),
    f64(f64),
    bool(bool),
    String(String),
    None,
}

impl From<PrimitiveType> for DataType {
    fn from(t: PrimitiveType) -> Self {
        Self::Primitive(t)
    }
}

impl From<LiteralType> for DataType {
    fn from(t: LiteralType) -> Self {
        Self::Literal(t)
    }
}

impl From<CustomDataType<ObjectType>> for DataType {
    fn from(t: CustomDataType<ObjectType>) -> Self {
        Self::Object(t)
    }
}

impl From<CustomDataType<EnumType>> for DataType {
    fn from(t: CustomDataType<EnumType>) -> Self {
        Self::Enum(t)
    }
}

impl From<ObjectType> for DataType {
    fn from(t: ObjectType) -> Self {
        Self::Object(CustomDataType::Unnamed(t))
    }
}

// TODO: Remove this
impl From<EnumType> for DataType {
    fn from(t: EnumType) -> Self {
        Self::Enum(CustomDataType::Unnamed(t))
    }
}

impl From<GenericType> for DataType {
    fn from(t: GenericType) -> Self {
        Self::Generic(t)
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        DataType::Tuple(t)
    }
}

// TODO: Remove this and do within `ToDataType` derive macro so it can be a `Named` enum?
impl<T: Into<DataType> + 'static> From<Vec<T>> for DataType {
    fn from(t: Vec<T>) -> Self {
        DataType::Enum(CustomDataType::Unnamed(EnumType {
            variants: t
                .into_iter()
                .map(|t| -> EnumVariant {
                    EnumVariant::Unnamed(TupleType {
                        fields: vec![t.into()],
                        generics: vec![],
                    })
                })
                .collect(),
            generics: vec![],
            repr: EnumRepr::Untagged,
        }))
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
