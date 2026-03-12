use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Enum, Fields, GenericReference, Primitive, Reference, Variant},
};

use crate::{
    Error,
    internal::{Result, SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs, SpectaTypeAttr},
    phased::PhasedTy,
    repr::EnumRepr,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    Unified,
    Phases,
}

pub fn validate_for_mode(types: &Types, mode: ApplyMode) -> Result<()> {
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
            mode,
        )?;
    }

    Ok(())
}

fn inner(
    dt: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    checked_references: &mut HashSet<Reference>,
    path: String,
    mode: ApplyMode,
) -> Result<()> {
    match dt {
        DataType::Nullable(ty) => inner(ty, types, generics, checked_references, path, mode)?,
        DataType::Map(map) => {
            is_valid_map_key(map.key_ty(), types, generics, format!("{path}.<map_key>"))?;
            inner(
                map.value_ty(),
                types,
                generics,
                checked_references,
                format!("{path}.<map_value>"),
                mode,
            )?;
        }
        DataType::List(list) => {
            inner(
                list.ty(),
                types,
                generics,
                checked_references,
                format!("{path}.<list_item>"),
                mode,
            )?;
        }
        DataType::Struct(strct) => {
            validate_container_attributes(
                strct.attributes(),
                types,
                generics,
                checked_references,
                &path,
                mode,
            )?;
            if strct
                .attributes()
                .get::<SerdeContainerAttrs>()
                .is_some_and(|attrs| attrs.variant_identifier || attrs.field_identifier)
            {
                return Err(Error::invalid_phased_type_usage(
                    path,
                    "`#[serde(variant_identifier)]` and `#[serde(field_identifier)]` are only valid on enums",
                ));
            }

            match strct.fields() {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    for (idx, (_, ty)) in unnamed
                        .fields()
                        .iter()
                        .filter_map(|field| field.ty().map(|ty| (field, ty)))
                        .enumerate()
                    {
                        inner(
                            ty,
                            types,
                            generics,
                            checked_references,
                            format!("{path}[{idx}]"),
                            mode,
                        )?;
                    }
                }
                Fields::Named(named) => {
                    for (name, (field, _)) in named
                        .fields()
                        .iter()
                        .filter_map(|(name, field)| field.ty().map(|ty| (name, (field, ty))))
                    {
                        validate_field_attributes(
                            field.attributes(),
                            format!("{path}.{name}"),
                            mode,
                        )?;
                    }
                    for (name, (_, ty)) in named
                        .fields()
                        .iter()
                        .filter_map(|(name, field)| field.ty().map(|ty| (name, (field, ty))))
                    {
                        inner(
                            ty,
                            types,
                            generics,
                            checked_references,
                            format!("{path}.{name}"),
                            mode,
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
                mode,
            )?;
            validate_identifier_enum(enm, &path, mode)?;
            validate_enum(enm, types, path.clone(), mode)?;

            for (variant_name, variant) in enm.variants() {
                validate_variant_attributes(
                    variant.attributes(),
                    format!("{path}::{variant_name}"),
                    mode,
                )?;
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Named(named) => {
                        for (name, (field, _)) in named
                            .fields()
                            .iter()
                            .filter_map(|(name, field)| field.ty().map(|ty| (name, (field, ty))))
                        {
                            validate_field_attributes(
                                field.attributes(),
                                format!("{path}::{variant_name}.{name}"),
                                mode,
                            )?;
                        }
                        for (name, (_, ty)) in named
                            .fields()
                            .iter()
                            .filter_map(|(name, field)| field.ty().map(|ty| (name, (field, ty))))
                        {
                            inner(
                                ty,
                                types,
                                generics,
                                checked_references,
                                format!("{path}::{variant_name}.{name}"),
                                mode,
                            )?;
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        for (idx, (field, _)) in unnamed
                            .fields()
                            .iter()
                            .filter_map(|field| field.ty().map(|ty| (field, ty)))
                            .enumerate()
                        {
                            validate_field_attributes(
                                field.attributes(),
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
                            )?;
                        }
                        for (idx, (_, ty)) in unnamed
                            .fields()
                            .iter()
                            .filter_map(|field| field.ty().map(|ty| (field, ty)))
                            .enumerate()
                        {
                            inner(
                                ty,
                                types,
                                generics,
                                checked_references,
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
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
                    mode,
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
                        mode,
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
                            mode,
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
                    mode,
                )?;
            }
            Reference::Opaque(reference) => {
                if let Some(phased) = reference.downcast_ref::<PhasedTy>() {
                    inner(
                        &phased.serialize,
                        types,
                        generics,
                        checked_references,
                        format!("{path}.<phased_serialize>"),
                        mode,
                    )?;
                    inner(
                        &phased.deserialize,
                        types,
                        generics,
                        checked_references,
                        format!("{path}.<phased_deserialize>"),
                        mode,
                    )?;
                }
            }
        },
        DataType::Primitive(_) => {}
    }

    Ok(())
}

fn validate_identifier_enum(enm: &Enum, path: &str, mode: ApplyMode) -> Result<()> {
    let Some(attrs) = enm.attributes().get::<SerdeContainerAttrs>() else {
        return Ok(());
    };

    if !attrs.variant_identifier && !attrs.field_identifier {
        return Ok(());
    }

    if attrs.variant_identifier && attrs.field_identifier {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`variant_identifier` and `field_identifier` cannot be used together",
        ));
    }

    if mode == ApplyMode::Unified {
        return Err(Error::invalid_phased_type_usage(
            path,
            "identifier enums require `apply_phases` because they widen deserialize-only input shape",
        ));
    }

    if attrs.variant_identifier {
        if let Some((name, _)) = enm
            .variants()
            .iter()
            .find(|(_, variant)| !matches!(variant.fields(), Fields::Unit))
        {
            return Err(Error::invalid_phased_type_usage(
                path,
                format!(
                    "`variant_identifier` requires all unit variants, but variant `{name}` is not unit"
                ),
            ));
        }
    }

    if attrs.field_identifier {
        let variants = enm.variants();
        for (idx, (name, variant)) in variants.iter().enumerate() {
            let is_last = idx + 1 == variants.len();
            match variant.fields() {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) if is_last && unnamed.fields().len() == 1 => {}
                _ => {
                    return Err(Error::invalid_phased_type_usage(
                        path,
                        format!(
                            "`field_identifier` requires unit variants and an optional final newtype fallback; invalid variant `{name}`"
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_container_attributes(
    attrs: &specta::datatype::Attributes,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    checked_references: &mut HashSet<Reference>,
    path: &str,
    mode: ApplyMode,
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
                    mode,
                )?;
            }
        }
    }

    Ok(())
}

fn validate_variant_attributes(
    attrs: &specta::datatype::Attributes,
    path: String,
    _mode: ApplyMode,
) -> Result<()> {
    let Some(serde_attrs) = attrs.get::<SerdeVariantAttrs>() else {
        return Ok(());
    };

    if serde_attrs.has_serialize_with {
        ensure_codec_override(attrs, &path, "serialize_with")?;
    }
    if serde_attrs.has_deserialize_with {
        ensure_codec_override(attrs, &path, "deserialize_with")?;
    }
    if serde_attrs.has_with {
        ensure_codec_override(attrs, &path, "with")?;
    }

    Ok(())
}

fn validate_field_attributes(
    attrs: &specta::datatype::Attributes,
    path: String,
    mode: ApplyMode,
) -> Result<()> {
    let Some(serde_attrs) = attrs.get::<SerdeFieldAttrs>() else {
        return Ok(());
    };

    if serde_attrs.has_serialize_with {
        ensure_codec_override(attrs, &path, "serialize_with")?;
    }
    if serde_attrs.has_deserialize_with {
        ensure_codec_override(attrs, &path, "deserialize_with")?;
    }
    if serde_attrs.has_with {
        ensure_codec_override(attrs, &path, "with")?;
    }

    if mode == ApplyMode::Unified && serde_attrs.skip_serializing_if.is_some() {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`skip_serializing_if` requires `apply_phases` because unified mode cannot represent conditional omission",
        ));
    }

    Ok(())
}

fn ensure_codec_override(
    attrs: &specta::datatype::Attributes,
    path: &str,
    attr: &'static str,
) -> Result<()> {
    if attrs.get::<SpectaTypeAttr>().is_some() {
        return Ok(());
    }

    Err(Error::unsupported_serde_custom_codec(
        path.to_string(),
        attr,
    ))
}

fn is_valid_map_key(
    key_ty: &DataType,
    types: &Types,
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

fn validate_enum(enm: &Enum, types: &Types, path: String, mode: ApplyMode) -> Result<()> {
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

    let repr = enum_repr_from_attrs(enm.attributes())?;

    validate_other_variant(enm, &path, &repr, mode)?;

    if matches!(repr, EnumRepr::Internal { .. }) {
        validate_internally_tag_enum(enm, types, path)?;
    }

    Ok(())
}

fn validate_other_variant(enm: &Enum, path: &str, repr: &EnumRepr, mode: ApplyMode) -> Result<()> {
    let other_variants = enm
        .variants()
        .iter()
        .filter_map(|(name, variant)| {
            variant
                .attributes()
                .get::<SerdeVariantAttrs>()
                .is_some_and(|attrs| attrs.other)
                .then_some((name, variant))
        })
        .collect::<Vec<_>>();

    if other_variants.is_empty() {
        return Ok(());
    }

    if mode == ApplyMode::Unified {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` requires `apply_phases` because it widens deserialize-only input shape",
        ));
    }

    if !matches!(repr, EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. }) {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` requires a tagged enum representation (`tag` with optional `content`)",
        ));
    }

    if other_variants.len() > 1 {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` can only be used on a single enum variant",
        ));
    }

    let (name, variant) = other_variants[0];
    if !matches!(variant.fields(), Fields::Unit) {
        return Err(Error::invalid_phased_type_usage(
            path,
            format!("`#[serde(other)]` variant `{name}` must be a unit variant"),
        ));
    }

    Ok(())
}

fn validate_internally_tag_enum(enm: &Enum, types: &Types, path: String) -> Result<()> {
    for (variant_name, variant) in enm.variants() {
        validate_internally_tag_variant(enm, variant_name, variant, types, &path)?;
    }

    Ok(())
}

fn validate_internally_tag_variant(
    enm: &Enum,
    variant_name: &str,
    variant: &Variant,
    types: &Types,
    path: &str,
) -> Result<()> {
    let _ = enm;
    match &variant.fields() {
        Fields::Unit | Fields::Named(_) => Ok(()),
        Fields::Unnamed(unnamed) => {
            let mut fields = unnamed
                .fields()
                .iter()
                .filter_map(|field| field.ty().map(|ty| (field, ty)));
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
    types: &Types,
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
