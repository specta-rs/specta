//! Serde support for Specta
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use std::collections::HashSet;

use thiserror::Error;

use specta::{
    datatype::{
        DataType, EnumRepr, EnumType, EnumVariants, LiteralType, PrimitiveType, StructFields,
    },
    internal::{resolve_generics, skip_fields, skip_fields_named},
    SpectaID, TypeCollection,
};

// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.

#[derive(Error, Debug, PartialEq)]
pub enum SerdeError {
    #[error("A map key must be a 'string' or 'number' type")]
    InvalidMapKey,
    #[error("#[specta(tag = \"...\")] cannot be used with tuple variants")]
    InvalidInternallyTaggedEnum,
    #[error("the usage of #[specta(skip)] means the type can't be serialized")]
    InvalidUsageOfSkip,
}

/// Check that a [DataType] is a valid for Serde.
///
/// This can be used by exporters which wanna do export-time checks that all types are compatible with Serde formats.
pub fn is_valid_ty(dt: &DataType, type_map: &TypeCollection) -> Result<(), SerdeError> {
    is_valid_ty_internal(dt, type_map, &mut Default::default())
}

fn is_valid_ty_internal(
    dt: &DataType,
    type_map: &TypeCollection,
    checked_references: &mut HashSet<SpectaID>,
) -> Result<(), SerdeError> {
    match dt {
        DataType::Nullable(ty) => is_valid_ty(ty, type_map)?,
        DataType::Map(ty) => {
            is_valid_map_key(ty.key_ty(), type_map)?;
            is_valid_ty_internal(ty.value_ty(), type_map, checked_references)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            StructFields::Unit => {}
            StructFields::Unnamed(ty) => {
                for (_, ty) in skip_fields(ty.fields()) {
                    is_valid_ty_internal(ty, type_map, checked_references)?;
                }
            }
            StructFields::Named(ty) => {
                for (_, (_, ty)) in skip_fields_named(ty.fields()) {
                    is_valid_ty_internal(ty, type_map, checked_references)?;
                }
            }
        },
        DataType::Enum(ty) => {
            validate_enum(ty, type_map)?;

            for (_variant_name, variant) in ty.variants().iter() {
                match &variant.inner() {
                    EnumVariants::Unit => {}
                    EnumVariants::Named(variant) => {
                        for (_, (_, ty)) in skip_fields_named(variant.fields()) {
                            is_valid_ty_internal(ty, type_map, checked_references)?;
                        }
                    }
                    EnumVariants::Unnamed(variant) => {
                        for (_, ty) in skip_fields(variant.fields()) {
                            is_valid_ty_internal(ty, type_map, checked_references)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for ty in ty.elements() {
                is_valid_ty_internal(ty, type_map, checked_references)?;
            }
        }
        DataType::Reference(ty) => {
            for (_, generic) in ty.generics() {
                is_valid_ty_internal(generic, type_map, checked_references)?;
            }

            #[allow(clippy::panic)]
            if !checked_references.contains(&ty.sid()) {
                checked_references.insert(ty.sid());
                let ty = type_map.get(ty.sid()).unwrap_or_else(|| {
                    panic!("Type '{}' was never populated.", ty.sid().type_name())
                }); // TODO: Error properly

                is_valid_ty_internal(&ty.inner, type_map, checked_references)?;
            }
        }
        _ => {}
    }

    Ok(())
}

// Typescript: Must be assignable to `string | number | symbol` says Typescript.
fn is_valid_map_key(key_ty: &DataType, type_map: &TypeCollection) -> Result<(), SerdeError> {
    match key_ty {
        DataType::Any => Ok(()),
        DataType::Primitive(ty) => match ty {
            PrimitiveType::i8
            | PrimitiveType::i16
            | PrimitiveType::i32
            | PrimitiveType::i64
            | PrimitiveType::i128
            | PrimitiveType::isize
            | PrimitiveType::u8
            | PrimitiveType::u16
            | PrimitiveType::u32
            | PrimitiveType::u64
            | PrimitiveType::u128
            | PrimitiveType::usize
            | PrimitiveType::f32
            | PrimitiveType::f64
            | PrimitiveType::String
            | PrimitiveType::char => Ok(()),
            _ => Err(SerdeError::InvalidMapKey),
        },
        DataType::Literal(ty) => match ty {
            LiteralType::i8(_)
            | LiteralType::i16(_)
            | LiteralType::i32(_)
            | LiteralType::u8(_)
            | LiteralType::u16(_)
            | LiteralType::u32(_)
            | LiteralType::f32(_)
            | LiteralType::f64(_)
            | LiteralType::String(_)
            | LiteralType::char(_) => Ok(()),
            _ => Err(SerdeError::InvalidMapKey),
        },
        // Enum of other valid types are also valid Eg. `"A" | "B"` or `"A" | 5` are valid
        DataType::Enum(ty) => {
            for (_variant_name, variant) in ty.variants() {
                match &variant.inner() {
                    EnumVariants::Unit => {}
                    EnumVariants::Unnamed(item) => {
                        if item.fields().len() > 1 {
                            return Err(SerdeError::InvalidMapKey);
                        }

                        if *ty.repr() != EnumRepr::Untagged {
                            return Err(SerdeError::InvalidMapKey);
                        }
                    }
                    _ => return Err(SerdeError::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Reference(r) => {
            let ty = type_map.get(r.sid()).expect("Type was never populated"); // TODO: Error properly

            is_valid_map_key(&resolve_generics(ty.inner.clone(), r.generics()), type_map)
        }
        _ => Err(SerdeError::InvalidMapKey),
    }
}

// Serde does not allow serializing a variant of certain types of enum's.
fn validate_enum(e: &EnumType, type_map: &TypeCollection) -> Result<(), SerdeError> {
    // You can't `#[serde(skip)]` your way to an empty enum.
    let valid_variants = e.variants().iter().filter(|(_, v)| !v.skip()).count();
    if valid_variants == 0 && !e.variants().is_empty() {
        return Err(SerdeError::InvalidUsageOfSkip);
    }

    // Only internally tagged enums can be invalid.
    if let EnumRepr::Internal { .. } = e.repr() {
        validate_internally_tag_enum(e, type_map)?;
    }

    Ok(())
}

// Checks for specially internally tagged enums.
fn validate_internally_tag_enum(e: &EnumType, type_map: &TypeCollection) -> Result<(), SerdeError> {
    for (_variant_name, variant) in e.variants() {
        match &variant.inner() {
            EnumVariants::Unit => {}
            EnumVariants::Named(_) => {}
            EnumVariants::Unnamed(item) => {
                let mut fields = skip_fields(item.fields());

                let Some(first_field) = fields.next() else {
                    continue;
                };

                if fields.next().is_some() {
                    return Err(SerdeError::InvalidInternallyTaggedEnum);
                }

                validate_internally_tag_enum_datatype(first_field.1, type_map)?;
            }
        }
    }

    Ok(())
}

// Internally tagged enums require map-type's (with a couple of exceptions like `null`)
// Which makes sense when you can't represent `{ "type": "A" } & string` in a single JSON value.
fn validate_internally_tag_enum_datatype(
    ty: &DataType,
    type_map: &TypeCollection,
) -> Result<(), SerdeError> {
    match ty {
        // `serde_json::Any` can be *technically* be either valid or invalid based on the actual data but we are being strict and reject it.
        DataType::Any => return Err(SerdeError::InvalidInternallyTaggedEnum),
        DataType::Map(_) => {}
        // Structs's are always map-types unless they are transparent then it depends on inner type. However, transparent passes through when calling `Type::inline` so we don't need to specially check that case.
        DataType::Struct(_) => {}
        DataType::Enum(ty) => match ty.repr() {
            // Is only valid if the enum itself is also valid.
            EnumRepr::Untagged => validate_internally_tag_enum(ty, type_map)?,
            // Eg. `{ "Variant": "value" }` is a map-type so valid.
            EnumRepr::External => {}
            // Eg. `{ "type": "variant", "field": "value" }` is a map-type so valid.
            EnumRepr::Internal { .. } => {}
            // Eg. `{ "type": "variant", "c": {} }` is a map-type so valid.
            EnumRepr::Adjacent { .. } => {}
        },
        // `()` is `null` and is valid
        DataType::Tuple(ty) if ty.elements().is_empty() => {}
        // References need to be checked against the same rules.
        DataType::Reference(ty) => {
            let ty = type_map.get(ty.sid()).expect("Type was never populated"); // TODO: Error properly

            validate_internally_tag_enum_datatype(&ty.inner, type_map)?;
        }
        _ => return Err(SerdeError::InvalidInternallyTaggedEnum),
    }

    Ok(())
}
