use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Fields, Generic, NamedReferenceType, Primitive, Reference},
};

use crate::{Error, opaque};

const SERDE_CONTAINER_UNTAGGED: &str = "serde:container:untagged";
const SERDE_VARIANT_UNTAGGED: &str = "serde:variant:untagged";
const SERDE_ENUM_REPR_REWRITTEN: &str = "specta_serde:enum_repr_rewritten";

pub(crate) fn validate_map_key(
    key_ty: &DataType,
    types: &Types,
    path: String,
) -> Result<(), Error> {
    validate_map_key_inner(key_ty, types, path, &mut HashSet::new())
}

fn validate_map_key_inner(
    key_ty: &DataType,
    types: &Types,
    path: String,
    visiting: &mut HashSet<Reference>,
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
        DataType::Primitive(primitive) if primitive_is_valid_key(primitive.clone()) => Ok(()),
        DataType::Primitive(primitive) => Err(Error::invalid_map_key(
            path,
            invalid_primitive_reason(primitive.clone()),
        )),
        DataType::Enum(enm) => {
            let untagged = enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED);
            let rewritten = enm.attributes.contains_key(SERDE_ENUM_REPR_REWRITTEN);
            for (variant_name, variant) in &enm.variants {
                let fields = unwrap_synthetic_variant_fields(variant_name, &variant.fields)
                    .unwrap_or(&variant.fields);
                match fields {
                    Fields::Unit
                        if !untagged
                            && !variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED) => {}
                    Fields::Unit => {
                        return Err(Error::invalid_map_key(
                            &path,
                            format!(
                                "untagged enum key variant '{variant_name}' does not serialize as a string"
                            ),
                        ));
                    }
                    Fields::Unnamed(fields) => {
                        if !untagged
                            && !rewritten
                            && !variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED)
                        {
                            return Err(Error::invalid_map_key(
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' uses tagged newtype or tuple serialization, which serde_json rejects"
                                ),
                            ));
                        }
                        let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                        let Some(inner) = fields.next() else {
                            return Err(Error::invalid_map_key(
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        };
                        if fields.next().is_some() {
                            return Err(Error::invalid_map_key(
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        }
                        validate_map_key_inner(
                            inner,
                            types,
                            format!("{path}.{variant_name}"),
                            visiting,
                        )?;
                    }
                    Fields::Named(_) => {
                        return Err(Error::invalid_map_key(
                            &path,
                            format!(
                                "enum key variant '{variant_name}' serializes as a struct variant, which serde_json rejects"
                            ),
                        ));
                    }
                }
            }
            Ok(())
        }
        DataType::Struct(strct) => {
            let Fields::Unnamed(fields) = &strct.fields else {
                return Err(Error::invalid_map_key(
                    path,
                    "struct keys must serialize as a newtype struct to be valid serde_json map keys",
                ));
            };
            let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
            let Some(inner) = fields.next() else {
                return Err(Error::invalid_map_key(
                    path,
                    "newtype struct map keys must have exactly one serializable field",
                ));
            };
            if fields.next().is_some() {
                return Err(Error::invalid_map_key(
                    path,
                    "newtype struct map keys must have exactly one serializable field",
                ));
            }
            validate_map_key_inner(inner, types, path, visiting)
        }
        DataType::Reference(Reference::Named(reference)) => {
            let key = Reference::Named(reference.clone());
            if !visiting.insert(key.clone()) {
                return Err(Error::invalid_map_key(
                    path,
                    "recursive map key reference cycle detected",
                ));
            }

            let result = match &reference.inner {
                NamedReferenceType::Reference { generics, .. } => types
                    .get(reference)
                    .and_then(|ndt| ndt.ty.as_ref())
                    .ok_or_else(|| Error::dangling_named_reference(format!("{reference:?}")))
                    .and_then(|ty| {
                        let mut ty = ty.clone();
                        substitute_generics(&mut ty, generics);
                        validate_map_key_inner(&ty, types, path, visiting)
                    }),
                NamedReferenceType::Inline { dt, .. } => {
                    validate_map_key_inner(dt, types, path, visiting)
                }
                NamedReferenceType::Recursive(_) => Err(Error::invalid_map_key(
                    path,
                    format!("recursive inline named map key reference {reference:?}"),
                )),
            };

            visiting.remove(&key);
            result
        }
        DataType::Generic(_) => Ok(()),
        DataType::Reference(Reference::Opaque(reference))
            if reference.downcast_ref::<opaque::Define>().is_some() =>
        {
            Ok(())
        }
        DataType::Reference(Reference::Opaque(_)) => Err(Error::invalid_map_key(
            path,
            "opaque references cannot be validated as serde_json map keys",
        )),
        DataType::Tuple(_) => Err(Error::invalid_map_key(
            path,
            "tuple keys are not supported by serde_json map key serialization",
        )),
        DataType::List(_)
        | DataType::Map(_)
        | DataType::Nullable(_)
        | DataType::Intersection(_) => Err(Error::invalid_map_key(
            path,
            "collection, map, and nullable keys are not supported by serde_json map key serialization",
        )),
    }
}

fn substitute_generics(dt: &mut DataType, generics: &[(Generic, DataType)]) {
    match dt {
        DataType::Generic(generic) => {
            if let Some((_, replacement)) = generics.iter().find(|(name, _)| name == generic) {
                *dt = replacement.clone();
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, generics),
        DataType::Enum(enm) => enm
            .variants
            .iter_mut()
            .for_each(|(_, variant)| substitute_field_generics(&mut variant.fields, generics)),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter_mut()
            .for_each(|ty| substitute_generics(ty, generics)),
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, generics),
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => reference_generics
                .iter_mut()
                .for_each(|(_, ty)| substitute_generics(ty, generics)),
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Intersection(types) => types
            .iter_mut()
            .for_each(|ty| substitute_generics(ty, generics)),
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(fields: &mut Fields, generics: &[(Generic, DataType)]) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|field| field.ty.as_mut())
            .for_each(|ty| substitute_generics(ty, generics)),
        Fields::Named(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|(_, field)| field.ty.as_mut())
            .for_each(|ty| substitute_generics(ty, generics)),
    }
}

fn primitive_is_valid_key(primitive: Primitive) -> bool {
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

fn invalid_primitive_reason(primitive: Primitive) -> &'static str {
    match primitive {
        Primitive::f16 | Primitive::f128 => {
            "f16 and f128 keys are not supported by serde_json map key serialization"
        }
        _ => "unsupported primitive key type for serde_json map key serialization",
    }
}
