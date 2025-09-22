use std::collections::{BTreeMap, HashSet};

use specta::{
    datatype::{DataType, Enum, EnumRepr, Fields, Generic, Literal, Primitive},
    internal::{skip_fields, skip_fields_named},
    SpectaID, TypeCollection,
};

use crate::Error;

// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.

/// Validate the type and apply the Serde transformations.
pub fn validate(types: &TypeCollection) -> Result<(), Error> {
    for ndt in types.into_unsorted_iter() {
        inner(
            ndt.ty(),
            &types,
            &Default::default(),
            &mut Default::default(),
        )?;
    }

    Ok(())
}

// TODO: Remove this once we redo the Typescript exporter.
pub fn validate_dt(ty: &DataType, types: &TypeCollection) -> Result<(), Error> {
    inner(ty, &types, &Default::default(), &mut Default::default())?;

    for ndt in types.into_unsorted_iter() {
        inner(
            ndt.ty(),
            &types,
            &Default::default(),
            &mut Default::default(),
        )?;
    }

    Ok(())
}

fn inner(
    dt: &DataType,
    types: &TypeCollection,
    generics: &BTreeMap<Generic, DataType>,
    checked_references: &mut HashSet<SpectaID>,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => inner(ty, types, generics, checked_references)?,
        DataType::Map(ty) => {
            is_valid_map_key(ty.key_ty(), types, generics)?;
            inner(ty.value_ty(), types, generics, checked_references)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            Fields::Unit => {}
            Fields::Unnamed(ty) => {
                for (_, ty) in skip_fields(ty.fields()) {
                    inner(ty, types, generics, checked_references)?;
                }
            }
            Fields::Named(ty) => {
                for (_, (_, ty)) in skip_fields_named(ty.fields()) {
                    inner(ty, types, generics, checked_references)?;
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
                            inner(ty, types, generics, checked_references)?;
                        }
                    }
                    Fields::Unnamed(variant) => {
                        for (_, ty) in skip_fields(variant.fields()) {
                            inner(ty, types, generics, checked_references)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for ty in ty.elements() {
                inner(ty, types, generics, checked_references)?;
            }
        }
        DataType::Reference(r) => {
            for (_, dt) in r.generics() {
                inner(dt, types, &Default::default(), checked_references)?;
            }

            #[allow(clippy::panic)]
            if !checked_references.contains(&r.sid()) {
                checked_references.insert(r.sid());
                // TODO: We don't error here for `Any`/`Unknown` in the TS exporter
                if let Some(ty) = types.get(r.sid()) {
                    inner(ty.ty(), types, r.generics(), checked_references)?;
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
    generics: &BTreeMap<Generic, DataType>,
) -> Result<(), Error> {
    match key_ty {
        DataType::Primitive(ty) => match ty {
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
            | Primitive::char => Ok(()),
            _ => Err(Error::InvalidMapKey),
        },
        DataType::Literal(ty) => match ty {
            Literal::i8(_)
            | Literal::i16(_)
            | Literal::i32(_)
            | Literal::u8(_)
            | Literal::u16(_)
            | Literal::u32(_)
            | Literal::f32(_)
            | Literal::f64(_)
            | Literal::String(_)
            | Literal::char(_) => Ok(()),
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

                        if *ty.repr().unwrap_or(&EnumRepr::External) != EnumRepr::Untagged {
                            return Err(Error::InvalidMapKey);
                        }
                    }
                    _ => return Err(Error::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Tuple(t) => {
            if t.elements().len() == 0 {
                return Err(Error::InvalidMapKey);
            }

            Ok(())
        }
        DataType::Reference(r) => {
            let ty = types.get(r.sid()).expect("Type was never populated"); // TODO: Error properly

            is_valid_map_key(ty.ty(), types, r.generics())
        }
        DataType::Generic(g) => {
            let ty = generics.get(g).expect("bruh");

            is_valid_map_key(&ty, types, &Default::default())
        }
        _ => Err(Error::InvalidMapKey),
    }
}

// Serde does not allow serializing a variant of certain types of enum's.
fn validate_enum(e: &Enum, types: &TypeCollection) -> Result<(), Error> {
    // You can't `#[serde(skip)]` your way to an empty enum.
    let valid_variants = e.variants().iter().filter(|(_, v)| !v.skip()).count();
    if valid_variants == 0 && !e.variants().is_empty() {
        return Err(Error::InvalidUsageOfSkip);
    }

    // Only internally tagged enums can be invalid.
    if let EnumRepr::Internal { .. } = e.repr().unwrap_or(&EnumRepr::External) {
        validate_internally_tag_enum(e, types)?;
    }

    Ok(())
}

// Checks for specially internally tagged enums.
fn validate_internally_tag_enum(e: &Enum, types: &TypeCollection) -> Result<(), Error> {
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
        // DataType::Any => return Err(Error::InvalidInternallyTaggedEnum), // TODO: Do we need to fix this?
        DataType::Map(_) => {}
        // Structs's are always map-types unless they are transparent then it depends on inner type. However, transparent passes through when calling `Type::inline` so we don't need to specially check that case.
        DataType::Struct(_) => {}
        DataType::Enum(ty) => match ty.repr().unwrap_or(&EnumRepr::External) {
            // Is only valid if the enum itself is also valid.
            EnumRepr::Untagged => validate_internally_tag_enum(ty, types)?,
            // Eg. `{ "Variant": "value" }` is a map-type so valid.
            EnumRepr::External => {}
            // Eg. `{ "type": "variant", "field": "value" }` is a map-type so valid.
            EnumRepr::Internal { .. } => {}
            // Eg. `{ "type": "variant", "c": {} }` is a map-type so valid.
            EnumRepr::Adjacent { .. } => {}
            // String enums serialize as strings, which are valid
            EnumRepr::String { .. } => {}
        },
        // `()` is `null` and is valid
        DataType::Tuple(ty) if ty.elements().is_empty() => {}
        // References need to be checked against the same rules.
        DataType::Reference(ty) => {
            let ty = types.get(ty.sid()).expect("Type was never populated"); // TODO: Error properly

            validate_internally_tag_enum_datatype(ty.ty(), types)?;
        }
        _ => return Err(Error::InvalidInternallyTaggedEnum),
    }

    Ok(())
}
