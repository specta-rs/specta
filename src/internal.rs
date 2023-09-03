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

    pub const fn field(optional: bool, flatten: bool, ty: DataType) -> Field {
        Field {
            optional,
            flatten,
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

    pub const fn enum_variant_unit() -> EnumVariant {
        EnumVariant::Unit
    }

    pub const fn enum_variant_unnamed(fields: Vec<Field>) -> EnumVariant {
        EnumVariant::Unnamed(UnnamedFields { fields })
    }

    pub const fn enum_variant_named(
        fields: Vec<(Cow<'static, str>, Field)>,
        tag: Option<Cow<'static, str>>,
    ) -> EnumVariant {
        EnumVariant::Named(NamedFields { fields, tag })
    }

    pub const fn named_data_type(
        name: Cow<'static, str>,
        comments: Vec<Cow<'static, str>>,
        deprecated: Option<Cow<'static, str>>,
        sid: SpectaID,
        impl_location: ImplLocation,
        export: Option<bool>,
        item: DataType,
    ) -> NamedDataType {
        NamedDataType {
            name,
            comments,
            deprecated,
            ext: Some(NamedDataTypeExt {
                sid,
                impl_location,
                export,
            }),
            item,
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
}
