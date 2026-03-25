use std::collections::HashSet;

use specta::{
    datatype::{DataType, Fields, GenericReference, Primitive, Reference},
    Types,
};

use crate::Error;

pub(crate) fn validate_map_key(
    key_ty: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    path: String,
) -> Result<(), Error> {
    validate_map_key_inner(
        key_ty,
        types,
        generics,
        path,
        &mut HashSet::new(),
        &mut HashSet::new(),
    )
}

fn validate_map_key_inner(
    key_ty: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    path: String,
    visiting_named_refs: &mut HashSet<Reference>,
    visiting_generic_refs: &mut HashSet<(GenericReference, DataType)>,
) -> Result<(), Error> {
    match key_ty {
        DataType::Primitive(primitive) if primitive_is_valid_key(primitive.clone()) => Ok(()),
        DataType::Primitive(other) => Err(Error::invalid_map_key(
            path,
            invalid_primitive_reason(other.clone()),
        )),
        DataType::Enum(enm) => {
            for (variant_name, variant) in enm.variants() {
                match variant.fields() {
                    Fields::Unit => {}
                    Fields::Unnamed(unnamed) => {
                        let non_skipped = unnamed.fields().iter().filter_map(|field| field.ty()).count();
                        if non_skipped != 1 {
                            return Err(Error::invalid_map_key(
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        }
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
            let Fields::Unnamed(unnamed) = strct.fields() else {
                return Err(Error::invalid_map_key(
                    path,
                    "struct keys must serialize as a newtype struct to be valid serde_json map keys",
                ));
            };

            let mut non_skipped = unnamed.fields().iter().filter_map(|field| field.ty());
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

            validate_map_key_inner(
                inner_ty,
                types,
                generics,
                path,
                visiting_named_refs,
                visiting_generic_refs,
            )
        }
        DataType::Reference(Reference::Named(reference)) => {
            let reference_key = Reference::Named(reference.clone());
            if !visiting_named_refs.insert(reference_key.clone()) {
                return Err(Error::invalid_map_key(
                    path,
                    "recursive map key reference cycle detected",
                ));
            }

            let result = if let Some(ndt) = reference.get(types) {
                let merged_generics = merged_generics(generics, reference.generics());
                validate_map_key_inner(
                    ndt.ty(),
                    types,
                    &merged_generics,
                    path,
                    visiting_named_refs,
                    visiting_generic_refs,
                )
            } else {
                Err(Error::invalid_map_key(
                    path,
                    format!("unresolved named map key reference {reference:?}"),
                ))
            };

            visiting_named_refs.remove(&reference_key);
            result
        }
        DataType::Reference(Reference::Generic(generic)) => {
            let Some((_, ty)) = generics.iter().find(|(candidate, _)| candidate == generic) else {
                return Ok(());
            };

            if matches!(ty, DataType::Reference(Reference::Generic(inner)) if inner == generic) {
                return Ok(());
            }

            let resolved = resolve_generics_in_datatype(ty, generics);
            let generic_state = (generic.clone(), resolved.clone());
            if !visiting_generic_refs.insert(generic_state.clone()) {
                return Ok(());
            }

            let result = validate_map_key_inner(
                &resolved,
                types,
                generics,
                path,
                visiting_named_refs,
                visiting_generic_refs,
            );
            visiting_generic_refs.remove(&generic_state);

            result
        }
        DataType::Reference(Reference::Opaque(_)) => Err(Error::invalid_map_key(
            path,
            "opaque references cannot be validated as serde_json map keys",
        )),
        DataType::Tuple(_) => Err(Error::invalid_map_key(
            path,
            "tuple keys are not supported by serde_json map key serialization",
        )),
        DataType::List(_) | DataType::Map(_) | DataType::Nullable(_) => Err(Error::invalid_map_key(
            path,
            "collection, map, and nullable keys are not supported by serde_json map key serialization",
        )),
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

fn merged_generics(
    parent: &[(GenericReference, DataType)],
    child: &[(GenericReference, DataType)],
) -> Vec<(GenericReference, DataType)> {
    let unshadowed_parent = parent
        .iter()
        .filter(|(parent_generic, _)| {
            !child
                .iter()
                .any(|(child_generic, _)| child_generic == parent_generic)
        })
        .cloned();

    child
        .iter()
        .map(|(generic, dt)| (generic.clone(), resolve_generics_in_datatype(dt, parent)))
        .chain(unshadowed_parent)
        .collect()
}

fn resolve_generics_in_datatype(
    dt: &DataType,
    generics: &[(GenericReference, DataType)],
) -> DataType {
    fn resolve(
        dt: &DataType,
        generics: &[(GenericReference, DataType)],
        visiting: &mut Vec<GenericReference>,
    ) -> DataType {
        match dt {
            DataType::Primitive(_) => dt.clone(),
            DataType::List(list) => {
                let mut out = list.clone();
                out.set_ty(resolve(list.ty(), generics, visiting));
                DataType::List(out)
            }
            DataType::Map(map) => {
                let mut out = map.clone();
                out.set_key_ty(resolve(map.key_ty(), generics, visiting));
                out.set_value_ty(resolve(map.value_ty(), generics, visiting));
                DataType::Map(out)
            }
            DataType::Nullable(inner) => {
                DataType::Nullable(Box::new(resolve(inner, generics, visiting)))
            }
            DataType::Struct(strct) => {
                let mut out = strct.clone();
                match out.fields_mut() {
                    Fields::Unit => {}
                    Fields::Unnamed(unnamed) => {
                        for field in unnamed.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve(ty, generics, visiting);
                            }
                        }
                    }
                    Fields::Named(named) => {
                        for (_, field) in named.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve(ty, generics, visiting);
                            }
                        }
                    }
                }
                DataType::Struct(out)
            }
            DataType::Enum(enm) => {
                let mut out = enm.clone();
                for (_, variant) in out.variants_mut() {
                    match variant.fields_mut() {
                        Fields::Unit => {}
                        Fields::Unnamed(unnamed) => {
                            for field in unnamed.fields_mut() {
                                if let Some(ty) = field.ty_mut() {
                                    *ty = resolve(ty, generics, visiting);
                                }
                            }
                        }
                        Fields::Named(named) => {
                            for (_, field) in named.fields_mut() {
                                if let Some(ty) = field.ty_mut() {
                                    *ty = resolve(ty, generics, visiting);
                                }
                            }
                        }
                    }
                }
                DataType::Enum(out)
            }
            DataType::Tuple(tuple) => {
                let mut out = tuple.clone();
                for element in out.elements_mut() {
                    *element = resolve(element, generics, visiting);
                }
                DataType::Tuple(out)
            }
            DataType::Reference(Reference::Generic(generic)) => {
                if visiting.iter().any(|seen| seen == generic) {
                    return dt.clone();
                }

                if let Some((_, resolved)) =
                    generics.iter().find(|(candidate, _)| candidate == generic)
                {
                    if matches!(resolved, DataType::Reference(Reference::Generic(inner)) if inner == generic)
                    {
                        dt.clone()
                    } else {
                        visiting.push(generic.clone());
                        let out = resolve(resolved, generics, visiting);
                        visiting.pop();
                        out
                    }
                } else {
                    dt.clone()
                }
            }
            DataType::Reference(_) => dt.clone(),
        }
    }

    resolve(dt, generics, &mut Vec::new())
}
