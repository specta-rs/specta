//! This module contains functions that are public for the sole reason of the macros.
//!
//! They will not be documented and may go through breaking changes without a major version bump!
//!
//! DO NOT USE THEM! You have been warned!

use std::{borrow::Cow, collections::HashMap};

#[cfg(feature = "interop")]
pub mod interop;

#[cfg(feature = "function")]
pub use paste::paste;

use crate::{
    datatype::{DataType, EnumVariants, Field, GenericType, List, Map, StructFields},
    Generics, ImplLocation, SpectaID, Type, TypeMap,
};

/// Functions used to construct `crate::datatype` types (they have private fields so can't be constructed directly).
/// We intentionally keep their fields private so we can modify them without a major version bump.
/// As this module is `#[doc(hidden)]` we allowed to make breaking changes within a minor version as it's only used by the macros.
pub mod construct {
    use std::borrow::Cow;

    use crate::{datatype::*, ImplLocation, SpectaID};

    pub const fn field(
        optional: bool,
        flatten: bool,
        deprecated: Option<DeprecatedType>,
        docs: Cow<'static, str>,
        ty: Option<DataType>,
    ) -> Field {
        Field {
            optional,
            flatten,
            deprecated,
            docs,
            ty,
        }
    }

    pub const fn r#struct(
        name: Cow<'static, str>,
        sid: Option<SpectaID>,
        generics: Vec<GenericType>,
        fields: StructFields,
    ) -> StructType {
        StructType {
            name,
            sid,
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
        sid: SpectaID,
        repr: EnumRepr,
        skip_bigint_checks: bool,
        generics: Vec<GenericType>,
        variants: Vec<(Cow<'static, str>, EnumVariant)>,
    ) -> EnumType {
        EnumType {
            name,
            sid: Some(sid),
            repr,
            skip_bigint_checks,
            generics,
            variants,
        }
    }

    pub const fn enum_variant(
        skip: bool,
        deprecated: Option<DeprecatedType>,
        docs: Cow<'static, str>,
        inner: EnumVariants,
    ) -> EnumVariant {
        EnumVariant {
            skip,
            docs,
            deprecated,
            inner,
        }
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
        deprecated: Option<DeprecatedType>,
        sid: SpectaID,
        impl_location: ImplLocation,
        inner: DataType,
    ) -> NamedDataType {
        NamedDataType {
            name,
            docs,
            deprecated,
            ext: Some(NamedDataTypeExt { sid, impl_location }),
            inner,
        }
    }

    pub const fn data_type_reference(
        name: Cow<'static, str>,
        sid: SpectaID,
        generics: Vec<(GenericType, DataType)>,
    ) -> DataTypeReference {
        DataTypeReference {
            name,
            sid,
            generics,
        }
    }

    pub const fn tuple(fields: Vec<DataType>) -> TupleType {
        TupleType { elements: fields }
    }

    pub const fn generic_data_type(name: &'static str) -> DataType {
        DataType::Generic(GenericType(Cow::Borrowed(name)))
    }

    pub const fn impl_location(loc: &'static str) -> ImplLocation {
        ImplLocation(loc)
    }

    /// Compute an SID hash for a given type.
    /// This will produce a type hash from the arguments.
    /// This hashing function was derived from <https://stackoverflow.com/a/71464396>
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

pub type NonSkipField<'a> = (&'a Field, &'a DataType);

pub fn skip_fields<'a>(
    fields: impl IntoIterator<Item = &'a Field>,
) -> impl Iterator<Item = NonSkipField<'a>> {
    fields
        .into_iter()
        .filter_map(|field| field.ty().map(|ty| (field, ty)))
}

pub fn skip_fields_named<'a>(
    fields: impl IntoIterator<Item = &'a (Cow<'static, str>, Field)>,
) -> impl Iterator<Item = (&'a Cow<'static, str>, NonSkipField<'a>)> {
    fields
        .into_iter()
        .filter_map(|(name, field)| field.ty().map(|ty| (name, (field, ty))))
}

#[track_caller]
pub fn flatten<T: Type>(sid: SpectaID, type_map: &mut TypeMap, generics: &[DataType]) -> DataType {
    type_map.flatten_stack.push(sid);

    #[allow(clippy::panic)]
    if type_map.flatten_stack.len() > 25 {
        // TODO: Handle this error without panicking
        panic!("Type recursion limit exceeded!");
    }

    let ty = T::inline(type_map, Generics::Provided(generics));

    type_map.flatten_stack.pop();

    ty
}

#[cfg(feature = "function")]
mod functions {
    use super::*;
    use crate::{datatype::DeprecatedType, datatype::Function, function::SpectaFn};

    #[doc(hidden)]
    /// A helper for exporting a command to a [`CommandDataType`].
    /// You shouldn't use this directly and instead should use [`fn_datatype!`](crate::fn_datatype).
    pub fn get_fn_datatype<TMarker, T: SpectaFn<TMarker>>(
        _: T,
        asyncness: bool,
        name: Cow<'static, str>,
        type_map: &mut TypeMap,
        fields: &[Cow<'static, str>],
        docs: Cow<'static, str>,
        deprecated: Option<DeprecatedType>,
        no_return_type: bool,
    ) -> Function {
        T::to_datatype(
            asyncness,
            name,
            type_map,
            fields,
            docs,
            deprecated,
            no_return_type,
        )
    }
}
#[cfg(feature = "function")]
pub use functions::*;

// TODO: Maybe make this a public utility?
// TODO: Should this be in the core or in `specta-serde`?
pub fn resolve_generics(mut dt: DataType, generics: &Vec<(GenericType, DataType)>) -> DataType {
    match dt {
        DataType::Primitive(_) | DataType::Literal(_) | DataType::Any | DataType::Unknown => dt,
        DataType::List(v) => DataType::List(List {
            ty: Box::new(resolve_generics(*v.ty, generics)),
            length: v.length,
            unique: v.unique,
        }),
        DataType::Nullable(v) => DataType::Nullable(Box::new(resolve_generics(*v, generics))),
        DataType::Map(v) => DataType::Map(Map {
            key_ty: Box::new(resolve_generics(*v.key_ty, generics)),
            value_ty: Box::new(resolve_generics(*v.value_ty, generics)),
        }),
        DataType::Struct(ref mut v) => match &mut v.fields {
            StructFields::Unit => dt,
            StructFields::Unnamed(f) => {
                for field in f.fields.iter_mut() {
                    field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
                }

                dt
            }
            StructFields::Named(f) => {
                for (_, field) in f.fields.iter_mut() {
                    field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
                }

                dt
            }
        },
        DataType::Enum(ref mut v) => {
            for (_, v) in v.variants.iter_mut() {
                match &mut v.inner {
                    EnumVariants::Unit => {}
                    EnumVariants::Named(f) => {
                        for (_, field) in f.fields.iter_mut() {
                            field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
                        }
                    }
                    EnumVariants::Unnamed(f) => {
                        for field in f.fields.iter_mut() {
                            field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
                        }
                    }
                }
            }

            dt
        }
        DataType::Tuple(ref mut v) => {
            for ty in v.elements.iter_mut() {
                *ty = resolve_generics(ty.clone(), generics);
            }

            dt
        }
        DataType::Reference(ref mut r) => {
            for (_, generic) in r.generics.iter_mut() {
                *generic = resolve_generics(generic.clone(), generics);
            }

            dt
        }
        DataType::Generic(g) => generics
            .iter()
            .find(|(name, _)| name == &g)
            .map(|(_, ty)| ty.clone())
            .unwrap_or_else(|| format!("Generic type `{g}` was referenced but not found").into()), // TODO: Error properly
    }
}

// TODO: This should go
/// post process the type map to detect duplicate type names
pub fn detect_duplicate_type_names(
    type_map: &TypeMap,
) -> Vec<(Cow<'static, str>, ImplLocation, ImplLocation)> {
    let mut errors = Vec::new();

    let mut map = HashMap::with_capacity(type_map.len());
    for (sid, dt) in type_map.iter() {
        if let Some(ext) = &dt.ext {
            if let Some((existing_sid, existing_impl_location)) =
                map.insert(dt.name.clone(), (sid, ext.impl_location))
            {
                if existing_sid != sid {
                    errors.push((dt.name.clone(), ext.impl_location, existing_impl_location));
                }
            }
        }
    }

    errors
}
