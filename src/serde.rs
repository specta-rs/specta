use thiserror::Error;

use crate::{DataType, EnumRepr, EnumVariant, LiteralType, PrimitiveType, StructFields, TypeMap};

// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.

#[derive(Error, Debug, PartialEq)]
pub enum SerdeError {
    #[error("A map key must be a 'string' or 'number' type")]
    InvalidMapKey,
}

/// Check that a [DataType] is a valid for Serde.
///
/// This can be used by exporters which wanna do export-time checks that all types are compatible with Serde formats.
pub(crate) fn is_valid_ty(dt: &DataType, type_map: &TypeMap) -> Result<(), SerdeError> {
    match dt {
        DataType::Nullable(ty) => is_valid_ty(&ty, type_map)?,
        DataType::Map(ty) => {
            is_valid_map_key(&ty.0, type_map)?;
            is_valid_ty(&ty.1, type_map)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            StructFields::Unit => {}
            StructFields::Unnamed(ty) => {
                for field in ty.fields().iter() {
                    is_valid_ty(&field.ty, type_map)?;
                }
            }
            StructFields::Named(ty) => {
                for (_field_name, field) in ty.fields().iter() {
                    is_valid_ty(&field.ty, type_map)?;
                }
            }
        },
        DataType::Enum(ty) => {
            for (_variant_name, variant) in ty.variants().iter() {
                match variant {
                    EnumVariant::Unit => {}
                    EnumVariant::Named(variant) => {
                        for (_field_name, field) in variant.fields.iter() {
                            is_valid_ty(&field.ty, type_map)?;
                        }
                    }
                    EnumVariant::Unnamed(variant) => {
                        for field in variant.fields.iter() {
                            is_valid_ty(&field.ty, type_map)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for field in ty.fields.iter() {
                is_valid_ty(&field, type_map)?;
            }
        }
        DataType::Result(ty) => {
            is_valid_ty(&ty.0, type_map)?;
            is_valid_ty(&ty.1, type_map)?;
        }
        DataType::Reference(ty) => {
            for generic in &ty.generics {
                is_valid_ty(&generic, type_map)?;
            }
        }
        _ => {}
    }

    Ok(())
}

// Typescript: Must be assignable to `string | number | symbol` says Typescript.
fn is_valid_map_key(key_ty: &DataType, type_map: &TypeMap) -> Result<(), SerdeError> {
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
            for (_variant_name, variant) in &ty.variants {
                match variant {
                    EnumVariant::Unit => {}
                    EnumVariant::Unnamed(item) => {
                        if item.fields.len() > 1 {
                            return Err(SerdeError::InvalidMapKey);
                        }

                        if ty.repr != EnumRepr::Untagged {
                            return Err(SerdeError::InvalidMapKey);
                        }
                    }
                    _ => return Err(SerdeError::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Reference(ty) => {
            let ty = type_map
                .get(&ty.sid)
                .as_ref()
                .expect("Reference type not found")
                .as_ref()
                .expect("Type was never populated"); // TODO: Error properly

            is_valid_map_key(&ty.inner, type_map)
        }
        _ => Err(SerdeError::InvalidMapKey),
    }
}
