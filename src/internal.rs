//! This module contains functions that are public for the sole reason of the macros.
//!
//! They will not be documented and may go through breaking changes without a major version bump!
//!
//! DO NOT USE THEM! You have been warned!

#[cfg(feature = "export")]
pub use ctor;

#[cfg(feature = "functions")]
pub use specta_macros::fn_datatype;

/// Functions used to construct `crate::datatype` types (they have private fields so can't be constructed directly).
/// We intentionally keep their fields private so we can modify them without a major version bump.
/// As this module is `#[doc(hidden)]` we allowed to make breaking changes within a minor version as it's only used by the macros.
pub mod construct {
    use std::borrow::Cow;

    use crate::{datatype::*, ImplLocation, SpectaID};

    pub const fn field(
        skip: bool,
        optional: bool,
        flatten: bool,
        docs: Cow<'static, str>,
        ty: DataType,
    ) -> Field {
        Field {
            skip,
            optional,
            flatten,
            docs,
            ty,
        }
    }

    pub const fn r#struct(
        name: Cow<'static, str>,
        generics: Vec<GenericType>,
        fields: StructFields,
    ) -> StructType {
        StructType {
            name,
            generics,
            fields,
        }
    }

    pub const fn struct_unit() -> StructFields {
        StructFields::Unit
    }

    pub const fn struct_unnamed(fields: Vec<Field>) -> StructFields {
        StructFields::Unnamed(UnnamedFields { fields })
    }

    pub const fn struct_named(
        fields: Vec<(Cow<'static, str>, Field)>,
        tag: Option<Cow<'static, str>>,
    ) -> StructFields {
        StructFields::Named(NamedFields { fields, tag })
    }

    pub const fn r#enum(
        name: Cow<'static, str>,
        repr: EnumRepr,
        generics: Vec<GenericType>,
        variants: Vec<(Cow<'static, str>, EnumVariant)>,
    ) -> EnumType {
        EnumType {
            name,
            repr,
            generics,
            variants,
        }
    }

    pub const fn enum_variant(
        skip: bool,
        docs: Cow<'static, str>,
        inner: EnumVariants,
    ) -> EnumVariant {
        EnumVariant { skip, docs, inner }
    }

    pub const fn enum_variant_unit() -> EnumVariants {
        EnumVariants::Unit
    }

    pub const fn enum_variant_unnamed(fields: Vec<Field>) -> EnumVariants {
        EnumVariants::Unnamed(UnnamedFields { fields })
    }

    pub const fn enum_variant_named(
        fields: Vec<(Cow<'static, str>, Field)>,
        tag: Option<Cow<'static, str>>,
    ) -> EnumVariants {
        EnumVariants::Named(NamedFields { fields, tag })
    }

    pub const fn named_data_type(
        name: Cow<'static, str>,
        docs: Cow<'static, str>,
        deprecated: Option<Cow<'static, str>>,
        sid: SpectaID,
        impl_location: ImplLocation,
        export: Option<bool>,
        inner: DataType,
    ) -> NamedDataType {
        NamedDataType {
            name,
            docs,
            deprecated,
            ext: Some(NamedDataTypeExt {
                sid,
                impl_location,
                export,
            }),
            inner,
        }
    }

    pub const fn data_type_reference(
        name: Cow<'static, str>,
        sid: SpectaID,
        generics: Vec<DataType>,
    ) -> DataTypeReference {
        DataTypeReference {
            name,
            sid,
            generics,
        }
    }

    pub const fn tuple(fields: Vec<DataType>) -> TupleType {
        TupleType { fields }
    }

    pub const fn impl_location(loc: &'static str) -> ImplLocation {
        ImplLocation(loc)
    }

    /// Compute an SID hash for a given type.
    /// This will produce a type hash from the arguments.
    /// This hashing function was derived from https://stackoverflow.com/a/71464396
    pub const fn sid(type_name: &'static str, type_identifier: &'static str) -> SpectaID {
        let mut hash = 0xcbf29ce484222325;
        let prime = 0x00000100000001B3;

        let mut bytes = type_name.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(prime);
            i += 1;
        }

        bytes = type_identifier.as_bytes();
        i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(prime);
            i += 1;
        }

        SpectaID { type_name, hash }
    }
}
