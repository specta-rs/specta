use std::collections::HashSet;

use specta::{
    datatype::{
        DataType, EnumRepr, EnumType, Fields, LiteralType, PrimitiveType,
    }, internal::{skip_fields, skip_fields_named}, SpectaID, TypeCollection
};

use crate::Error;

// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.

/// Validate the type and apply the Serde transformations.
pub fn validate(types: &TypeCollection) -> Result<(), Error> {
    for (_, ndt) in types.into_iter() {
        inner(&ndt.inner, &types, &mut Default::default())?;
    }

    Ok(())
}

fn inner(
    dt: &DataType,
    types: &TypeCollection,
    checked_references: &mut HashSet<SpectaID>,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => inner(ty, types, checked_references)?,
        DataType::Map(ty) => {
            is_valid_map_key(ty.key_ty(), types)?;
            inner(ty.value_ty(), types, checked_references)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            Fields::Unit => {}
            Fields::Unnamed(ty) => {
                for (_, ty) in skip_fields(ty.fields()) {
                    inner(ty, types, checked_references)?;
                }
            }
            Fields::Named(ty) => {
                for (_, (_, ty)) in skip_fields_named(ty.fields()) {
                    inner(ty, types, checked_references)?;
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
                            inner(ty, types, checked_references)?;
                        }
                    }
                    Fields::Unnamed(variant) => {
                        for (_, ty) in skip_fields(variant.fields()) {
                            inner(ty, types, checked_references)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for ty in ty.elements() {
                inner(ty, types, checked_references)?;
            }
        }
        DataType::Reference(ty) => {
            for generic in ty.generics() {
                inner(generic, types, checked_references)?;
            }

            #[allow(clippy::panic)]
            if !checked_references.contains(&ty.sid()) {
                checked_references.insert(ty.sid());
                let ty = types.get(ty.sid()).unwrap_or_else(|| {
                    panic!("Type '{}' was never populated.", ty.sid().type_name())
                }); // TODO: Error properly

                inner(&ty.inner, types, checked_references)?;
            }
        }
        _ => {}
    }

    Ok(())
}

// Typescript: Must be assignable to `string | number | symbol` says Typescript.
fn is_valid_map_key(key_ty: &DataType, types: &TypeCollection) -> Result<(), Error> {
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
            _ => Err(Error::InvalidMapKey),
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
            _ => Err(Error::InvalidMapKey),
        },
        // Enum of other valid types are also valid Eg. `"A" | "B"` or `"A" | 5` are valid
        DataType::Enum(ty) => {
            for (_variant_name, variant) in ty.variants() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Unnamed(item) => {
                        if item.fields().len() > 1 {
                            return Err(Error::InvalidMapKey);
                        }

                        if *ty.repr() != EnumRepr::Untagged {
                            return Err(Error::InvalidMapKey);
                        }
                    }
                    _ => return Err(Error::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Reference(r) => {
            let ty = types.get(r.sid()).expect("Type was never populated"); // TODO: Error properly

            // TODO: Bring back this
            // resolve_generics(ty.inner.clone(), r.generics())
            is_valid_map_key(&ty.inner, types)
        }
        _ => Err(Error::InvalidMapKey),
    }
}

// Serde does not allow serializing a variant of certain types of enum's.
fn validate_enum(e: &EnumType, types: &TypeCollection) -> Result<(), Error> {
    // You can't `#[serde(skip)]` your way to an empty enum.
    let valid_variants = e.variants().iter().filter(|(_, v)| !v.skip()).count();
    if valid_variants == 0 && !e.variants().is_empty() {
        return Err(Error::InvalidUsageOfSkip);
    }

    // Only internally tagged enums can be invalid.
    if let EnumRepr::Internal { .. } = e.repr() {
        validate_internally_tag_enum(e, types)?;
    }

    Ok(())
}

// Checks for specially internally tagged enums.
fn validate_internally_tag_enum(e: &EnumType, types: &TypeCollection) -> Result<(), Error> {
    for (_variant_name, variant) in e.variants() {
        match &variant.fields() {
            Fields::Unit => {}
            Fields::Named(_) => {}
            Fields::Unnamed(item) => {
                let mut fields = skip_fields(item.fields());

                let Some(first_field) = fields.next() else {
                    continue;
                };

                if fields.next().is_some() {
                    return Err(Error::InvalidInternallyTaggedEnum);
                }

                validate_internally_tag_enum_datatype(first_field.1, types)?;
            }
        }
    }

    Ok(())
}

// Internally tagged enums require map-type's (with a couple of exceptions like `null`)
// Which makes sense when you can't represent `{ "type": "A" } & string` in a single JSON value.
fn validate_internally_tag_enum_datatype(
    ty: &DataType,
    types: &TypeCollection,
) -> Result<(), Error> {
    match ty {
        // `serde_json::Any` can be *technically* be either valid or invalid based on the actual data but we are being strict and reject it.
        DataType::Any => return Err(Error::InvalidInternallyTaggedEnum),
        DataType::Map(_) => {}
        // Structs's are always map-types unless they are transparent then it depends on inner type. However, transparent passes through when calling `Type::inline` so we don't need to specially check that case.
        DataType::Struct(_) => {}
        DataType::Enum(ty) => match ty.repr() {
            // Is only valid if the enum itself is also valid.
            EnumRepr::Untagged => validate_internally_tag_enum(ty, types)?,
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
            let ty = types.get(ty.sid()).expect("Type was never populated"); // TODO: Error properly

            validate_internally_tag_enum_datatype(&ty.inner, types)?;
        }
        _ => return Err(Error::InvalidInternallyTaggedEnum),
    }

    Ok(())
}
