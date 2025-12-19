//! [Serde](https://serde.rs) support for Specta
//!
//! This crate takes the types with their attribute metadata returned from the macros,
//! validates that the type is a valid Serde type, and applies any transformations needed.
//!
//! # Usage
//!
//! ```
//! let types = specta::TypeCollection::default();
//! let (ser_types, de_types) = specta_serde::process_for_both(&types).unwrap();
//! // Use your transformed `types` as normal with a language exporter
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod inflection;
mod repr;
pub mod serde_attrs;

pub use error::Error;
pub use inflection::RenameRule;
pub use repr::EnumRepr;
pub use serde_attrs::{
    SerdeAttributes, SerdeFieldAttributes, SerdeMode, apply_serde_transformations,
};

use specta::TypeCollection;
use specta::builder::NamedDataTypeBuilder;
use specta::datatype::{DataType, Enum, Fields, Generic, Primitive, Reference};
use specta::internal::{skip_fields, skip_fields_named};
use std::collections::HashSet;

/// Process a TypeCollection and return transformed types for serialization
///
/// This function takes a TypeCollection, validates each type, applies serde transformations
/// for serialization, and returns a new TypeCollection with the transformed types.
///
/// The validation ensures:
/// - Map keys are valid types (string/number types)
/// - Internally tagged enums are properly structured
/// - Skip attributes don't result in empty enums
pub fn process_for_serialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut new_types = TypeCollection::default();

    for ndt in types.into_unsorted_iter() {
        // First validate the original type
        validate_type(ndt.ty(), types, &[], &mut Default::default())?;

        // Then apply transformations
        let transformed_dt =
            serde_attrs::apply_serde_transformations(ndt.ty(), SerdeMode::Serialize)?;

        // Validate the transformed type as well
        validate_type(&transformed_dt, types, &[], &mut Default::default())?;

        // Create a new NamedDataType with the transformed DataType using the builder
        let builder =
            NamedDataTypeBuilder::new(ndt.name().clone(), ndt.generics().to_vec(), transformed_dt)
                .docs(ndt.docs().clone())
                .module_path(ndt.module_path().clone());

        // Set deprecated if present
        let builder = if let Some(deprecated) = ndt.deprecated() {
            builder.deprecated(deprecated.clone())
        } else {
            builder
        };

        builder.build(&mut new_types);
    }

    Ok(new_types)
}

/// Process a TypeCollection and return transformed types for deserialization
///
/// This function takes a TypeCollection, validates each type, applies serde transformations
/// for deserialization, and returns a new TypeCollection with the transformed types.
///
/// The validation ensures:
/// - Map keys are valid types (string/number types)
/// - Internally tagged enums are properly structured
/// - Skip attributes don't result in empty enums
pub fn process_for_deserialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut new_types = TypeCollection::default();

    for ndt in types.into_unsorted_iter() {
        // First validate the original type
        validate_type(ndt.ty(), types, &[], &mut Default::default())?;

        // Then apply transformations
        let transformed_dt =
            serde_attrs::apply_serde_transformations(ndt.ty(), SerdeMode::Deserialize)?;

        // Validate the transformed type as well
        validate_type(&transformed_dt, types, &[], &mut Default::default())?;

        // Create a new NamedDataType with the transformed DataType using the builder
        let builder =
            NamedDataTypeBuilder::new(ndt.name().clone(), ndt.generics().to_vec(), transformed_dt)
                .docs(ndt.docs().clone())
                .module_path(ndt.module_path().clone());

        // Set deprecated if present
        let builder = if let Some(deprecated) = ndt.deprecated() {
            builder.deprecated(deprecated.clone())
        } else {
            builder
        };

        builder.build(&mut new_types);
    }

    Ok(new_types)
}

/// Convenience function to process types for both serialization and deserialization
///
/// Returns a tuple of (serialization_types, deserialization_types)
pub fn process_for_both(types: &TypeCollection) -> Result<(TypeCollection, TypeCollection), Error> {
    let ser_types = process_for_serialization(types)?;
    let de_types = process_for_deserialization(types)?;
    Ok((ser_types, de_types))
}

/// Internal validation function that recursively validates types
fn validate_type(
    dt: &DataType,
    types: &TypeCollection,
    generics: &[(Generic, DataType)],
    checked_references: &mut HashSet<Reference>,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => validate_type(ty, types, generics, checked_references)?,
        DataType::Map(ty) => {
            is_valid_map_key(ty.key_ty(), types, generics)?;
            validate_type(ty.value_ty(), types, generics, checked_references)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            Fields::Unit => {}
            Fields::Unnamed(ty) => {
                for (_, ty) in skip_fields(ty.fields()) {
                    validate_type(ty, types, generics, checked_references)?;
                }
            }
            Fields::Named(ty) => {
                for (_, (_, ty)) in skip_fields_named(ty.fields()) {
                    validate_type(ty, types, generics, checked_references)?;
                }
            }
        },
        DataType::Enum(ty) => {
            validate_enum(ty, types)?;

            for (_variant_name, variant) in ty.variants().iter() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Named(variant) => {
                        for (_, (_, ty)) in skip_fields_named(variant.fields()) {
                            validate_type(ty, types, generics, checked_references)?;
                        }
                    }
                    Fields::Unnamed(variant) => {
                        for (_, ty) in skip_fields(variant.fields()) {
                            validate_type(ty, types, generics, checked_references)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for ty in ty.elements() {
                validate_type(ty, types, generics, checked_references)?;
            }
        }
        DataType::List(ty) => {
            validate_type(ty.ty(), types, generics, checked_references)?;
        }
        DataType::Reference(r) => {
            for (_, dt) in r.generics() {
                validate_type(dt, types, &[], checked_references)?;
            }

            if !checked_references.contains(r) {
                checked_references.insert(r.clone());
                if let Some(ndt) = r.get(types) {
                    validate_type(ndt.ty(), types, r.generics(), checked_references)?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

// Typescript: Must be assignable to `string | number | symbol` says Typescript.
fn is_valid_map_key(
    key_ty: &DataType,
    types: &TypeCollection,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    match key_ty {
        DataType::Primitive(
            Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::i64
            | Primitive::i128
            | Primitive::isize
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32
            | Primitive::u64
            | Primitive::u128
            | Primitive::usize
            | Primitive::f32
            | Primitive::f64
            | Primitive::String
            | Primitive::char,
        ) => Ok(()),
        DataType::Primitive(_) => Err(Error::InvalidMapKey),
        // Enum of other valid types are also valid Eg. `"A" | "B"` or `"A" | 5` are valid
        DataType::Enum(ty) => {
            for (_variant_name, variant) in ty.variants() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Unnamed(item) => {
                        if item.fields().len() > 1 {
                            return Err(Error::InvalidMapKey);
                        }

                        // TODO: Check enum representation for untagged requirement
                        // if *ty.repr().unwrap_or(&EnumRepr::External) != EnumRepr::Untagged {
                        //     return Err(Error::InvalidMapKey);
                        // }
                    }
                    _ => return Err(Error::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Tuple(t) => {
            if t.elements().is_empty() {
                return Err(Error::InvalidMapKey);
            }

            Ok(())
        }
        DataType::Reference(r) => {
            if let Some(ndt) = r.get(types) {
                is_valid_map_key(ndt.ty(), types, r.generics())?;
            }
            Ok(())
        }
        DataType::Generic(g) => {
            let ty = generics
                .iter()
                .find(|(ge, _)| ge == g)
                .map(|(_, dt)| dt)
                .expect("unable to find expected generic type"); // TODO: Proper error instead of panicking

            is_valid_map_key(ty, types, &[])
        }
        _ => Err(Error::InvalidMapKey),
    }
}

// Serde does not allow serializing a variant of certain types of enum's.
fn validate_enum(e: &Enum, _types: &TypeCollection) -> Result<(), Error> {
    // You can't `#[serde(skip)]` your way to an empty enum.
    let valid_variants = e.variants().iter().filter(|(_, v)| !v.skip()).count();
    if valid_variants == 0 && !e.variants().is_empty() {
        return Err(Error::InvalidUsageOfSkip);
    }

    // TODO: Implement internally tagged enum validation
    // Only internally tagged enums can be invalid.
    // if let Some(EnumRepr::Internal { .. }) = get_enum_repr_from_attributes(e.attributes()) {
    //     validate_internally_tag_enum(e, types)?;
    // }

    Ok(())
}

// TODO: Implement these validation functions once enum representation parsing is complete
// fn validate_internally_tag_enum(e: &Enum, types: &TypeCollection) -> Result<(), Error> {
//     for (_variant_name, variant) in e.variants() {
//         match &variant.fields() {
//             Fields::Unit => {}
//             Fields::Named(_) => {}
//             Fields::Unnamed(item) => {
//                 let mut fields = skip_fields(item.fields());
//
//                 let Some(first_field) = fields.next() else {
//                     continue;
//                 };
//
//                 if fields.next().is_some() {
//                     return Err(Error::InvalidInternallyTaggedEnum);
//                 }
//
//                 validate_internally_tag_enum_datatype(first_field.1, types)?;
//             }
//         }
//     }
//
//     Ok(())
// }

// fn validate_internally_tag_enum_datatype(
//     ty: &DataType,
//     types: &TypeCollection,
// ) -> Result<(), Error> {
//     match ty {
//         DataType::Map(_) => {}
//         DataType::Struct(_) => {}
//         DataType::Enum(ty) => {
//             // TODO: Check enum representation
//         }
//         DataType::Tuple(ty) if ty.elements().is_empty() => {}
//         DataType::Reference(r) => {
//             if let Some(ndt) = r.get(types) {
//                 validate_internally_tag_enum_datatype(ndt.ty(), types)?;
//             }
//         }
//         _ => return Err(Error::InvalidInternallyTaggedEnum),
//     }
//
//     Ok(())
// }
