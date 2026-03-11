use std::collections::HashSet;

use specta::{
    TypeCollection,
    datatype::{
        DataType, Enum, EnumVariant, Fields, GenericReference, Primitive, Reference, skip_fields,
        skip_fields_named,
    },
};

use crate::{Error, Result, SerdeContainerAttrs, repr::EnumRepr};

pub fn validate(types: &TypeCollection) -> Result<()> {
    for ndt in types.into_unsorted_iter() {
        let ndt_generics = ndt
            .generics()
            .iter()
            .map(|(generic, _)| {
                (
                    generic.clone(),
                    DataType::Reference(Reference::Generic(generic.clone())),
                )
            })
            .collect::<Vec<_>>();

        inner(
            ndt.ty(),
            types,
            &ndt_generics,
            &mut HashSet::new(),
            ndt.name().to_string(),
        )?;
    }

    Ok(())
}

fn inner(
    dt: &DataType,
    types: &TypeCollection,
    generics: &[(GenericReference, DataType)],
    checked_references: &mut HashSet<Reference>,
    path: String,
) -> Result<()> {
    match dt {
        DataType::Nullable(ty) => inner(ty, types, generics, checked_references, path)?,
        DataType::Map(map) => {
            is_valid_map_key(map.key_ty(), types, generics, format!("{path}.<map_key>"))?;
            inner(
                map.value_ty(),
                types,
                generics,
                checked_references,
                format!("{path}.<map_value>"),
            )?;
        }
        DataType::List(list) => {
            inner(
                list.ty(),
                types,
                generics,
                checked_references,
                format!("{path}.<list_item>"),
            )?;
        }
        DataType::Struct(strct) => {
            validate_container_attributes(
                strct.attributes(),
                types,
                generics,
                checked_references,
                &path,
            )?;

            match strct.fields() {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    for (idx, (_, ty)) in skip_fields(unnamed.fields()).enumerate() {
                        inner(
                            ty,
                            types,
                            generics,
                            checked_references,
                            format!("{path}[{idx}]"),
                        )?;
                    }
                }
                Fields::Named(named) => {
                    for (name, (_, ty)) in skip_fields_named(named.fields()) {
                        inner(
                            ty,
                            types,
                            generics,
                            checked_references,
                            format!("{path}.{name}"),
                        )?;
                    }
                }
            }
        }
        DataType::Enum(enm) => {
            validate_container_attributes(
                enm.attributes(),
                types,
                generics,
                checked_references,
                &path,
            )?;
            validate_enum(enm, types, path.clone())?;

            for (variant_name, variant) in enm.variants() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Named(named) => {
                        for (name, (_, ty)) in skip_fields_named(named.fields()) {
                            inner(
                                ty,
                                types,
                                generics,
                                checked_references,
                                format!("{path}::{variant_name}.{name}"),
                            )?;
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        for (idx, (_, ty)) in skip_fields(unnamed.fields()).enumerate() {
                            inner(
                                ty,
                                types,
                                generics,
                                checked_references,
                                format!("{path}::{variant_name}[{idx}]"),
                            )?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(tuple) => {
            for (idx, ty) in tuple.elements().iter().enumerate() {
                inner(
                    ty,
                    types,
                    generics,
                    checked_references,
                    format!("{path}[{idx}]"),
                )?;
            }
        }
        DataType::Reference(reference) => match reference {
            Reference::Named(reference) => {
                for (_, dt) in reference.generics() {
                    inner(
                        dt,
                        types,
                        generics,
                        checked_references,
                        format!("{path}.<generic>"),
                    )?;
                }

                if !checked_references.contains(&Reference::Named(reference.clone())) {
                    let reference_key = Reference::Named(reference.clone());
                    checked_references.insert(reference_key);
                    if let Some(ndt) = reference.get(types) {
                        inner(
                            ndt.ty(),
                            types,
                            reference.generics(),
                            checked_references,
                            ndt.name().to_string(),
                        )?;
                    }
                }
            }
            Reference::Generic(generic) => {
                let Some((_, ty)) = generics.iter().find(|(candidate, _)| candidate == generic)
                else {
                    return Err(Error::unresolved_generic_reference(
                        path,
                        format!("{generic:?}"),
                    ));
                };

                if matches!(ty, DataType::Reference(Reference::Generic(inner)) if inner == generic)
                {
                    return Ok(());
                }

                inner(
                    ty,
                    types,
                    generics,
                    checked_references,
                    format!("{path}.<generic_ref>"),
                )?;
            }
            Reference::Opaque(_) => {}
        },
        DataType::Primitive(_) => {}
    }

    Ok(())
}

fn validate_container_attributes(
    attrs: &specta::datatype::Attributes,
    types: &TypeCollection,
    generics: &[(GenericReference, DataType)],
    checked_references: &mut HashSet<Reference>,
    path: &str,
) -> Result<()> {
    if let Some(parsed) = attrs.get::<SerdeContainerAttrs>()
        && parsed.from.is_some()
        && parsed.try_from.is_some()
    {
        return Err(Error::invalid_conversion_usage(
            path,
            "`from` and `try_from` cannot be used together",
        ));
    }

    if let Some(conversions) = attrs.get::<SerdeContainerAttrs>() {
        for (suffix, target) in [
            ("<serde_into>", conversions.resolved_into.as_ref()),
            ("<serde_from>", conversions.resolved_from.as_ref()),
            ("<serde_try_from>", conversions.resolved_try_from.as_ref()),
        ] {
            if let Some(target) = target {
                inner(
                    target,
                    types,
                    generics,
                    checked_references,
                    format!("{path}.{suffix}"),
                )?;
            }
        }
    }

    Ok(())
}

fn is_valid_map_key(
    key_ty: &DataType,
    types: &TypeCollection,
    generics: &[(GenericReference, DataType)],
    path: String,
) -> Result<()> {
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
            | Primitive::str
            | Primitive::char,
        ) => Ok(()),
        DataType::Primitive(other) => Err(Error::invalid_map_key(
            path,
            format!("unsupported primitive key type {other:?}"),
        )),
        DataType::Enum(enm) => {
            let repr = enum_repr_from_attrs(enm.attributes())?;
            for (variant_name, variant) in enm.variants() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Unnamed(item) => {
                        if item.fields().len() > 1 {
                            return Err(Error::invalid_map_key(
                                &path,
                                format!(
                                    "enum key variant '{variant_name}' has more than one tuple field"
                                ),
                            ));
                        }

                        if repr != EnumRepr::Untagged {
                            return Err(Error::invalid_map_key(
                                &path,
                                "enum key with tuple variants must be #[serde(untagged)]",
                            ));
                        }
                    }
                    Fields::Named(_) => {
                        return Err(Error::invalid_map_key(
                            &path,
                            format!("enum key variant '{variant_name}' uses named fields"),
                        ));
                    }
                }
            }

            Ok(())
        }
        DataType::Tuple(tuple) => {
            if tuple.elements().is_empty() {
                return Err(Error::invalid_map_key(
                    path,
                    "empty tuple key is unsupported",
                ));
            }

            Ok(())
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let Some(ndt) = reference.get(types) {
                is_valid_map_key(ndt.ty(), types, reference.generics(), path)?;
            }

            Ok(())
        }
        DataType::Reference(Reference::Generic(generic)) => {
            let Some((_, ty)) = generics.iter().find(|(candidate, _)| candidate == generic) else {
                return Err(Error::unresolved_generic_reference(
                    path,
                    format!("{generic:?}"),
                ));
            };

            if matches!(ty, DataType::Reference(Reference::Generic(inner)) if inner == generic) {
                return Ok(());
            }

            is_valid_map_key(ty, types, generics, path)
        }
        DataType::Reference(Reference::Opaque(_)) => Err(Error::invalid_map_key(
            path,
            "opaque references cannot be map keys",
        )),
        DataType::List(_) | DataType::Map(_) | DataType::Struct(_) | DataType::Nullable(_) => {
            Err(Error::invalid_map_key(
                path,
                "key type is not supported by legacy map-key validation rules",
            ))
        }
    }
}

fn validate_enum(enm: &Enum, types: &TypeCollection, path: String) -> Result<()> {
    let valid_variants = enm
        .variants()
        .iter()
        .filter(|(_, variant)| !variant.skip())
        .count();
    if valid_variants == 0 && !enm.variants().is_empty() {
        return Err(Error::invalid_usage_of_skip(
            path,
            "all variants are skipped, resulting in an invalid non-empty enum",
        ));
    }

    if matches!(
        enum_repr_from_attrs(enm.attributes())?,
        EnumRepr::Internal { .. }
    ) {
        validate_internally_tag_enum(enm, types, path)?;
    }

    Ok(())
}

fn validate_internally_tag_enum(enm: &Enum, types: &TypeCollection, path: String) -> Result<()> {
    for (variant_name, variant) in enm.variants() {
        validate_internally_tag_variant(enm, variant_name, variant, types, &path)?;
    }

    Ok(())
}

fn validate_internally_tag_variant(
    enm: &Enum,
    variant_name: &str,
    variant: &EnumVariant,
    types: &TypeCollection,
    path: &str,
) -> Result<()> {
    let _ = enm;
    match &variant.fields() {
        Fields::Unit | Fields::Named(_) => Ok(()),
        Fields::Unnamed(unnamed) => {
            let mut fields = skip_fields(unnamed.fields());
            let Some((_, first_field)) = fields.next() else {
                return Ok(());
            };

            if fields.next().is_some() {
                return Err(Error::invalid_internally_tagged_enum(
                    path,
                    variant_name,
                    "tuple variant must have at most one non-skipped field",
                ));
            }

            validate_internally_tag_enum_datatype(first_field, types, path, variant_name)
        }
    }
}

fn validate_internally_tag_enum_datatype(
    ty: &DataType,
    types: &TypeCollection,
    path: &str,
    variant_name: &str,
) -> Result<()> {
    match ty {
        DataType::Map(_) => Ok(()),
        DataType::Struct(_) => Ok(()),
        DataType::Enum(enm) => match enum_repr_from_attrs(enm.attributes())? {
            EnumRepr::Untagged => validate_internally_tag_enum(enm, types, path.to_string()),
            EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } => Ok(()),
        },
        DataType::Tuple(tuple) if tuple.elements().is_empty() => Ok(()),
        DataType::Reference(Reference::Named(reference)) => {
            if let Some(ndt) = reference.get(types) {
                validate_internally_tag_enum_datatype(ndt.ty(), types, path, variant_name)?;
            }

            Ok(())
        }
        DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_))
        | DataType::Tuple(_)
        | DataType::Primitive(_)
        | DataType::List(_)
        | DataType::Nullable(_) => Err(Error::invalid_internally_tagged_enum(
            path,
            variant_name,
            "payload cannot be merged with an internal tag",
        )),
    }
}

fn enum_repr_from_attrs(attrs: &specta::datatype::Attributes) -> Result<EnumRepr> {
    let Some(container_attrs) = attrs.get::<SerdeContainerAttrs>() else {
        return Ok(EnumRepr::External);
    };

    if container_attrs.untagged {
        return Ok(EnumRepr::Untagged);
    }

    Ok(
        match (
            container_attrs.tag.as_deref(),
            container_attrs.content.as_deref(),
        ) {
            (Some(tag), Some(content)) => EnumRepr::Adjacent {
                tag: tag.to_string().into(),
                content: content.to_string().into(),
            },
            (Some(tag), None) => EnumRepr::Internal {
                tag: tag.to_string().into(),
            },
            (None, Some(_)) => {
                return Err(Error::invalid_enum_representation(
                    "`content` is set without `tag`",
                ));
            }
            (None, None) => EnumRepr::External,
        },
    )
}
