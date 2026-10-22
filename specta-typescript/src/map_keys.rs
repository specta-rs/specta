use std::collections::HashSet;

use specta::{
    Types,
    datatype::{
        DataType, Fields, Generic, GenericReference, NamedReferenceType, Primitive, Reference,
    },
};

use crate::Error;

/// The key-safe wire shape a validated map key normalises to.
///
/// JSON object property names are always strings, and TypeScript mapped-type
/// keys must satisfy the built-in `PropertyKey` (`string | number | symbol`).
/// A key that is not already a valid `PropertyKey` therefore cannot reuse the
/// value rendering (`number | null`, `bigint`, a bare type parameter) and must
/// render through one of these shapes instead. See [`map_key_shape`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapKeyShape {
    /// The key expression already satisfies `PropertyKey`; render it as-is.
    Property,
    /// Wide integers and this crate's lossless `BigInt` remap. serde_json
    /// stringifies integer map keys, so these render as lexical string keys.
    String,
    /// Floats and this crate's lossless `Number` remap. `NaN`/`Infinity` are
    /// not representable as JSON keys, so the key-safe form drops the `null`
    /// branch and renders plain `number`.
    Number,
    /// A type parameter used (possibly through wrappers) as the key. It is not
    /// constrained to `PropertyKey` at its declaration, so the mapped type
    /// must intersect it locally.
    TypeParameter,
}

/// Classifies a validated map key into the shape it should render as.
///
/// This resolves through inline references, named newtype wrappers and generic
/// arguments so that a wide integer or lossless remap hidden behind a wrapper
/// still renders key-safely, while a concrete wrapper over an already-valid
/// key (`MaybeValidKey<string>`) keeps rendering as its own name.
///
/// Only call this after [`validate_map_key`], which rejects genuinely invalid
/// key shapes (structs, tuples, collections, ...). The returned
/// [`MapKeyShape::Property`] means "render the key expression unchanged".
pub(crate) fn map_key_shape(
    key_ty: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
) -> MapKeyShape {
    map_key_shape_inner(key_ty, types, generics, &mut HashSet::new())
}

fn map_key_shape_inner(
    key_ty: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    visiting: &mut HashSet<Reference>,
) -> MapKeyShape {
    match key_ty {
        DataType::Primitive(primitive) => match primitive {
            // `f16`/`f128` are rejected by `primitive_is_valid_key` before a
            // key is ever rendered, but classify them as `Number` defensively
            // so this match stays total over the primitive set.
            Primitive::f16 | Primitive::f32 | Primitive::f64 | Primitive::f128 => {
                MapKeyShape::Number
            }
            Primitive::usize
            | Primitive::isize
            | Primitive::i64
            | Primitive::u64
            | Primitive::i128
            | Primitive::u128 => MapKeyShape::String,
            _ => MapKeyShape::Property,
        },
        DataType::Reference(Reference::Opaque(r)) => {
            if r.downcast_ref::<crate::opaque::BigInt>().is_some() {
                MapKeyShape::String
            } else if r.downcast_ref::<crate::opaque::Number>().is_some() {
                MapKeyShape::Number
            } else {
                MapKeyShape::Property
            }
        }
        DataType::Reference(Reference::Named(r)) => {
            let reference_key = Reference::Named(r.clone());
            if !visiting.insert(reference_key.clone()) {
                return MapKeyShape::Property;
            }

            let shape = match &r.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    map_key_shape_inner(dt, types, generics, visiting)
                }
                NamedReferenceType::Reference {
                    generics: arguments,
                    ..
                } => types
                    .get(r)
                    .and_then(|ndt| ndt.ty.clone())
                    .map(|mut body| {
                        substitute_generics(&mut body, arguments);
                        map_key_shape_inner(&body, types, generics, visiting)
                    })
                    .unwrap_or(MapKeyShape::Property),
                NamedReferenceType::Recursive(_) => MapKeyShape::Property,
            };

            visiting.remove(&reference_key);
            shape
        }
        DataType::Struct(strct) => {
            // A named struct is only a valid key when it serialises as a
            // newtype, in which case the key is its single live field.
            let Fields::Unnamed(unnamed) = &strct.fields else {
                return MapKeyShape::Property;
            };

            let mut live = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            match (live.next(), live.next()) {
                (Some(inner), None) => map_key_shape_inner(inner, types, generics, visiting),
                _ => MapKeyShape::Property,
            }
        }
        DataType::Generic(generic) => {
            // A scoped substitution (e.g. a named reference's generic
            // argument) can bind the parameter to a concrete key shape.
            match generics.iter().find(|(reference, _)| reference == generic) {
                Some((_, substituted)) if !matches!(substituted, DataType::Generic(inner) if inner == generic) => {
                    map_key_shape_inner(substituted, types, generics, visiting)
                }
                _ => MapKeyShape::TypeParameter,
            }
        }
        _ => MapKeyShape::Property,
    }
}

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
        DataType::Primitive(primitive) if primitive_is_valid_key(primitive.clone()) => Ok(()),
        DataType::Primitive(other) => Err(Error::invalid_map_key(
            path,
            invalid_primitive_reason(other.clone()),
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
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' must serialize as a newtype value"
                                ),
                            ));
                        };
                        if non_skipped.next().is_some() {
                            return Err(Error::invalid_map_key(
                                &path,
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
            let Fields::Unnamed(unnamed) = &strct.fields else {
                return Err(Error::invalid_map_key(
                    path,
                    "struct keys must serialize as a newtype struct to be valid serde_json map keys",
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
                NamedReferenceType::Reference { generics, .. } => {
                    if let Some(ndt) = types.get(reference) {
                        if let Some(ty) = ndt.ty.as_ref() {
                            let mut ty = ty.clone();
                            substitute_generics(&mut ty, generics);
                            validate_map_key_inner(&ty, types, path, visiting_named_refs)
                        } else {
                            Err(Error::dangling_named_reference(
                                path,
                                format!("{reference:?}"),
                            ))
                        }
                    } else {
                        Err(Error::dangling_named_reference(
                            path,
                            format!("{reference:?}"),
                        ))
                    }
                }
                NamedReferenceType::Inline { dt, .. } => {
                    let inline_path = path.clone();
                    validate_map_key_inner(dt, types, path, visiting_named_refs)
                        .map_err(|err| err.with_inline_trace(types.get(reference), inline_path))
                }
                NamedReferenceType::Recursive(_) => Err(Error::invalid_map_key(
                    path,
                    format!("recursive inline named map key reference {reference:?}"),
                )),
            };

            visiting_named_refs.remove(&reference_key);
            result
        }
        DataType::Generic(_) => Ok(()),
        DataType::Reference(Reference::Opaque(r)) => {
            // `define(literal)` is the user-facing escape hatch for inserting a
            // verbatim TypeScript expression into the type graph. Treat it as
            // valid for map keys -- the caller is asserting that `literal` is
            // a serializable JSON map key, and rejecting it forces them to
            // duplicate the bigint-style rewrite already done by libraries
            // like taurpc (which converts `i64` -> `define("number")` to
            // satisfy the BigInt-forbidden filter).
            //
            // `BigInt`/`Number` are this crate's own lossless remaps. At a key
            // position they render as a lexical `string`/`number` key (see
            // `primitives::map_key_datatype`), so a key whose *named*
            // definition was remapped (`struct Id(u64)` -> `bigint`) stays
            // valid. Other opaque flavors -- Tauri channels, branded types --
            // still fail loudly.
            if r.downcast_ref::<crate::opaque::Define>().is_some()
                || r.downcast_ref::<crate::opaque::BigInt>().is_some()
                || r.downcast_ref::<crate::opaque::Number>().is_some()
            {
                Ok(())
            } else {
                Err(Error::invalid_map_key(
                    path,
                    "opaque references cannot be validated as serde_json map keys",
                ))
            }
        }
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
            if let Some((_, ty)) = generics.iter().find(|(reference, _)| reference == generic) {
                *dt = ty.clone();
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, generics),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_field_generics(&mut variant.fields, generics);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in &mut tuple.elements {
                substitute_generics(ty, generics);
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } = &mut reference.inner
            {
                for (_, ty) in reference_generics {
                    substitute_generics(ty, generics);
                }
            }
        }
        DataType::Intersection(types) => {
            for ty in types {
                substitute_generics(ty, generics);
            }
        }
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(fields: &mut Fields, generics: &[(Generic, DataType)]) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &mut named.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
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
