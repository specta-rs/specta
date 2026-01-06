//! This module contains functions that are public for the sole reason of the macros.
//!
//! They will not be documented and may go through breaking changes without a major version bump!
//!
//! DO NOT USE THEM! You have been warned!

use std::borrow::Cow;

#[cfg(feature = "function")]
pub use paste::paste;

use crate::datatype::{DataType, Field};

/// Functions used to construct `crate::datatype` types (they have private fields so can't be constructed directly).
/// We intentionally keep their fields private so we can modify them without a major version bump.
/// As this module is `#[doc(hidden)]` we allowed to make breaking changes within a minor version as it's only used by the macros.
pub mod construct {
    use std::borrow::Cow;

    use crate::{Type, TypeCollection, datatype::*};

    pub const fn r#struct(fields: Fields, attributes: Vec<RuntimeAttribute>) -> Struct {
        Struct { fields, attributes }
    }

    pub const fn fields_unnamed(fields: Vec<Field>, attributes: Vec<RuntimeAttribute>) -> Fields {
        Fields::Unnamed(UnnamedFields { fields, attributes })
    }

    pub const fn fields_named(
        fields: Vec<(Cow<'static, str>, Field)>,
        attributes: Vec<RuntimeAttribute>,
    ) -> Fields {
        Fields::Named(NamedFields { fields, attributes })
    }

    pub const fn enum_variant(
        skip: bool,
        deprecated: Option<DeprecatedType>,
        docs: Cow<'static, str>,
        fields: Fields,
        attributes: Vec<RuntimeAttribute>,
    ) -> EnumVariant {
        EnumVariant {
            skip,
            docs,
            deprecated,
            fields,
            attributes,
        }
    }

    pub const fn r#enum(
        variants: Vec<(Cow<'static, str>, EnumVariant)>,
        attributes: Vec<RuntimeAttribute>,
    ) -> Enum {
        Enum {
            variants,
            attributes,
        }
    }

    pub const fn tuple(fields: Vec<DataType>) -> Tuple {
        Tuple { elements: fields }
    }

    pub const fn generic_data_type(name: &'static str) -> Generic {
        Generic(Cow::Borrowed(name))
    }

    pub fn transparent_field<T: Type>(types: &mut TypeCollection) -> Fields {
        Fields::Unnamed(UnnamedFields {
            fields: vec![Field {
                optional: false,
                deprecated: None,
                docs: Cow::Borrowed(""),
                inline: true,
                ty: Some(T::definition(types)),
                attributes: vec![],
            }],
            attributes: vec![],
        })
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

#[cfg(feature = "function")]
mod functions {
    use super::*;
    use crate::{TypeCollection, datatype::DeprecatedType, datatype::Function, function::SpectaFn};

    #[doc(hidden)]
    /// A helper for exporting a command to a [`CommandDataType`].
    /// You shouldn't use this directly and instead should use [`fn_datatype!`](crate::fn_datatype).
    pub fn get_fn_datatype<TMarker, T: SpectaFn<TMarker>>(
        _: T,
        asyncness: bool,
        name: Cow<'static, str>,
        types: &mut TypeCollection,
        fields: &[Cow<'static, str>],
        docs: Cow<'static, str>,
        deprecated: Option<DeprecatedType>,
        no_return_type: bool,
    ) -> Function {
        T::to_datatype(
            asyncness,
            name,
            types,
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
// pub fn resolve_generics(mut dt: DataType, generics: &[DataType)]) -> DataType {
//     match dt {
//         DataType::Primitive(_) | DataType::Literal(_) | DataType::Any | DataType::Unknown => dt,
//         DataType::List(v) => DataType::List(List {
//             ty: Box::new(resolve_generics(*v.ty, generics)),
//             length: v.length,
//             unique: v.unique,
//         }),
//         DataType::Nullable(v) => DataType::Nullable(Box::new(resolve_generics(*v, generics))),
//         DataType::Map(v) => DataType::Map(Map {
//             key_ty: Box::new(resolve_generics(*v.key_ty, generics)),
//             value_ty: Box::new(resolve_generics(*v.value_ty, generics)),
//         }),
//         DataType::Struct(ref mut v) => match &mut v.fields {
//             Fields::Unit => dt,
//             Fields::Unnamed(f) => {
//                 for field in f.fields.iter_mut() {
//                     field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
//                 }

//                 dt
//             }
//             Fields::Named(f) => {
//                 for (_, field) in f.fields.iter_mut() {
//                     field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
//                 }

//                 dt
//             }
//         },
//         DataType::Enum(ref mut v) => {
//             for (_, v) in v.variants.iter_mut() {
//                 match &mut v.fields {
//                     Fields::Unit => {}
//                     Fields::Named(f) => {
//                         for (_, field) in f.fields.iter_mut() {
//                             field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
//                         }
//                     }
//                     Fields::Unnamed(f) => {
//                         for field in f.fields.iter_mut() {
//                             field.ty = field.ty.take().map(|v| resolve_generics(v, generics));
//                         }
//                     }
//                 }
//             }

//             dt
//         }
//         DataType::Tuple(ref mut v) => {
//             for ty in v.elements.iter_mut() {
//                 *ty = resolve_generics(ty.clone(), generics);
//             }

//             dt
//         }
//         DataType::Reference(ref mut r) => {
//             for generic in r.generics.iter_mut() {
//                 *generic = resolve_generics(generic.clone(), generics);
//             }

//             dt
//         }
//         DataType::Generic(g) => generics
//             .iter()
//             .find(|(name, _)| name == &g)
//             .map(|(_, ty)| ty.clone())
//             .unwrap_or_else(|| format!("Generic type `{g}` was referenced but not found").into()), // TODO: Error properly
//     }
// }
