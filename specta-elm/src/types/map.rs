//! Rendering of maps as Elm `Dict`s, including validation of the key type.

use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Fields, Map, NamedReferenceType, Primitive, Reference},
};

use crate::{
    Error, Exporter,
    elm::ElmCoreLibImport,
    module::ModuleImports,
    types::{Ndt, as_type_argument},
};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    types: &Types,
    map: &Map,
    ndt: &Ndt,
    exporter: &E,
) -> Result<(), Error> {
    let path = format!("{}.<map_key>", ndt.rust_type_path());
    validate_map_key(map.key_ty(), types, path)?;

    let mut key = String::new();
    ndt.render(&mut key, imports, exporter, types, map.key_ty())?;

    let mut value = String::new();
    ndt.render(&mut value, imports, exporter, types, map.value_ty())?;

    s.push_str("Dict ");
    s.push_str(&as_type_argument(&key));
    s.push(' ');
    s.push_str(&as_type_argument(&value));
    imports.require_core(ElmCoreLibImport::Dict);

    Ok(())
}

/// Check that the key type can be serialised as a JSON object key.
fn validate_map_key(key_ty: &DataType, types: &Types, path: String) -> Result<(), Error> {
    validate_map_key_inner(key_ty, types, path, &mut HashSet::new())
}

fn validate_map_key_inner(
    key_ty: &DataType,
    types: &Types,
    path: String,
    visiting_named_refs: &mut HashSet<Reference>,
) -> Result<(), Error> {
    fn unwrap_synthetic_variant_fields<'a>(
        variant_name: &str,
        fields: &'a Fields,
    ) -> Option<&'a Fields> {
        let Fields::Named(named) = fields else {
            return None;
        };

        let mut live_fields = named
            .fields
            .iter()
            .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name.as_ref(), ty)));
        let (field_name, DataType::Enum(inner)) = live_fields.next()? else {
            return None;
        };

        if field_name != variant_name || live_fields.next().is_some() {
            return None;
        }

        match inner.variants.as_slice() {
            [(inner_name, inner_variant)] if inner_name == variant_name => {
                Some(&inner_variant.fields)
            }
            _ => None,
        }
    }

    match key_ty {
        DataType::Primitive(primitive) if primitive_is_valid_key(primitive) => Ok(()),
        DataType::Primitive(other) => Err(Error::invalid_map_key(
            path,
            invalid_primitive_reason(other),
        )),
        DataType::Enum(enm) => {
            for (variant_name, variant) in &enm.variants {
                let fields = unwrap_synthetic_variant_fields(variant_name, &variant.fields)
                    .unwrap_or(&variant.fields);

                match fields {
                    Fields::Unit => {}
                    Fields::Unnamed(unnamed) => {
                        let mut non_skipped =
                            unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
                        let Some(inner_ty) = non_skipped.next() else {
                            return Err(Error::invalid_map_key(
                                path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        };
                        if non_skipped.next().is_some() {
                            return Err(Error::invalid_map_key(
                                path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        }

                        validate_map_key_inner(
                            inner_ty,
                            types,
                            format!("{path}.{variant_name}"),
                            visiting_named_refs,
                        )?;
                    }
                    Fields::Named(_) => {
                        return Err(Error::invalid_map_key(
                            path,
                            format!(
                                "enum key variant '{variant_name}' serializes as a struct variant, which cannot be a JSON object key"
                            ),
                        ));
                    }
                }
            }

            Ok(())
        }
        DataType::Struct(strct) => {
            let Fields::Unnamed(unnamed) = &strct.fields else {
                return Err(Error::invalid_map_key(
                    path,
                    "struct keys must serialize as a newtype struct to be valid JSON object keys",
                ));
            };

            let mut non_skipped = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            let Some(inner_ty) = non_skipped.next() else {
                return Err(Error::invalid_map_key(
                    path,
                    "newtype struct map keys must have exactly one serializable field",
                ));
            };

            if non_skipped.next().is_some() {
                return Err(Error::invalid_map_key(
                    path,
                    "newtype struct map keys must have exactly one serializable field",
                ));
            }

            validate_map_key_inner(inner_ty, types, path, visiting_named_refs)
        }
        DataType::Reference(Reference::Named(reference)) => {
            let reference_key = Reference::Named(reference.clone());
            if !visiting_named_refs.insert(reference_key.clone()) {
                return Err(Error::invalid_map_key(
                    path,
                    "recursive map key reference cycle detected",
                ));
            }

            let result = match &reference.inner {
                NamedReferenceType::Reference { .. } => {
                    match types.get(reference).and_then(|ndt| ndt.ty.as_ref()) {
                        Some(ty) => validate_map_key_inner(ty, types, path, visiting_named_refs),
                        None => Err(Error::dangling_named_reference(
                            path,
                            format!("{reference:?}"),
                        )),
                    }
                }
                NamedReferenceType::Inline { dt, .. } => {
                    validate_map_key_inner(dt, types, path, visiting_named_refs)
                }
                NamedReferenceType::Recursive(_) => Err(Error::invalid_map_key(
                    path,
                    format!("recursive inline named map key reference {reference:?}"),
                )),
            };

            visiting_named_refs.remove(&reference_key);
            result
        }
        DataType::Generic(_) => Err(Error::invalid_map_key(
            path,
            "generic types are not supported by specta-elm",
        )),
        DataType::Reference(Reference::Opaque(_)) => Err(Error::invalid_map_key(
            path,
            "opaque references are not supported by specta-elm",
        )),
        DataType::Tuple(_) => Err(Error::invalid_map_key(
            path,
            "tuple keys cannot be serialised as JSON object keys",
        )),
        DataType::List(_)
        | DataType::Map(_)
        | DataType::Nullable(_)
        | DataType::Intersection(_) => Err(Error::invalid_map_key(
            path,
            "collection, map, and nullable keys cannot be serialised as JSON object keys",
        )),
    }
}

fn primitive_is_valid_key(primitive: &Primitive) -> bool {
    matches!(
        primitive,
        Primitive::bool
            | Primitive::i8
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
            | Primitive::str
            | Primitive::char
    )
}

fn invalid_primitive_reason(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::f16 | Primitive::f128 => {
            "f16 and f128 keys cannot be serialised as JSON object keys"
        }
        _ => "unsupported primitive key type",
    }
}
