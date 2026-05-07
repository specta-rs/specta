use std::collections::HashSet;

use specta::{
    Types,
    datatype::{DataType, Enum, Field, Fields, NamedReferenceType, Reference, Variant},
};

use crate::{
    Error,
    parser::{SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs},
    phased::PhasedTy,
    repr::EnumRepr,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    Unified,
    Phases,
}

pub fn validate_for_mode(types: &Types, mode: ApplyMode) -> Result<(), Error> {
    for ndt in types.into_unsorted_iter() {
        let Some(ty) = &ndt.ty else {
            continue;
        };

        inner(
            ty,
            types,
            &mut HashSet::new(),
            ndt.name.to_string(),
            mode,
            true,
        )?;
    }

    Ok(())
}

pub(crate) fn validate_datatype_for_mode(
    dt: &DataType,
    types: &Types,
    mode: ApplyMode,
) -> Result<(), Error> {
    inner(
        dt,
        types,
        &mut HashSet::new(),
        "<top-level>".to_string(),
        mode,
        true,
    )
}

pub(crate) fn validate_datatype_for_mode_shallow(
    dt: &DataType,
    types: &Types,
    mode: ApplyMode,
) -> Result<(), Error> {
    inner(
        dt,
        types,
        &mut HashSet::new(),
        "<top-level>".to_string(),
        mode,
        false,
    )
}

fn inner(
    dt: &DataType,
    types: &Types,
    checked_references: &mut HashSet<Reference>,
    path: String,
    mode: ApplyMode,
    follow_named_references: bool,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => inner(
            ty,
            types,
            checked_references,
            path,
            mode,
            follow_named_references,
        )?,
        DataType::Map(map) => {
            inner(
                map.key_ty(),
                types,
                checked_references,
                format!("{path}.<map_key>"),
                mode,
                follow_named_references,
            )?;
            inner(
                map.value_ty(),
                types,
                checked_references,
                format!("{path}.<map_value>"),
                mode,
                follow_named_references,
            )?;
        }
        DataType::List(list) => {
            inner(
                &list.ty,
                types,
                checked_references,
                format!("{path}.<list_item>"),
                mode,
                follow_named_references,
            )?;
        }
        DataType::Struct(strct) => {
            validate_container_attributes(
                &strct.attributes,
                types,
                checked_references,
                &path,
                mode,
            )?;
            if let Some(attrs) = SerdeContainerAttrs::from_attributes(&strct.attributes)? {
                if attrs.variant_identifier || attrs.field_identifier {
                    return Err(Error::invalid_phased_type_usage(
                        path,
                        "`#[serde(variant_identifier)]` and `#[serde(field_identifier)]` are only valid on enums",
                    ));
                }

                if attrs.untagged {
                    return Err(Error::invalid_phased_type_usage(
                        path,
                        "`#[serde(untagged)]` is only valid on enums",
                    ));
                }

                if attrs.content.is_some() {
                    return Err(Error::invalid_phased_type_usage(
                        path,
                        "`#[serde(content = ...)]` is only valid on enums",
                    ));
                }

                if attrs.tag.is_some() && !matches!(&strct.fields, Fields::Named(_)) {
                    return Err(Error::invalid_phased_type_usage(
                        path,
                        "`#[serde(tag = ...)]` on structs requires named fields",
                    ));
                }
            }

            match &strct.fields {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    for (idx, (_, ty)) in unnamed
                        .fields
                        .iter()
                        .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                        .enumerate()
                    {
                        inner(
                            ty,
                            types,
                            checked_references,
                            format!("{path}[{idx}]"),
                            mode,
                            follow_named_references,
                        )?;
                    }
                }
                Fields::Named(named) => {
                    for (name, (field, _)) in named
                        .fields
                        .iter()
                        .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                    {
                        validate_field_attributes(field, format!("{path}.{name}"), mode)?;
                    }
                    for (name, (_, ty)) in named
                        .fields
                        .iter()
                        .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                    {
                        inner(
                            ty,
                            types,
                            checked_references,
                            format!("{path}.{name}"),
                            mode,
                            follow_named_references,
                        )?;
                    }
                }
            }
        }
        DataType::Enum(enm) => {
            validate_container_attributes(&enm.attributes, types, checked_references, &path, mode)?;
            if SerdeContainerAttrs::from_attributes(&enm.attributes)?
                .is_some_and(|attrs| attrs.default)
            {
                return Err(Error::invalid_phased_type_usage(
                    path,
                    "`#[serde(default)]` is only valid on structs",
                ));
            }
            validate_identifier_enum(enm, &path, mode)?;
            validate_enum(enm, types, path.clone(), mode)?;

            for (variant_name, variant) in &enm.variants {
                validate_variant_attributes(variant, format!("{path}::{variant_name}"), mode)?;
                match &variant.fields {
                    Fields::Unit => {}
                    Fields::Named(named) => {
                        for (name, (field, _)) in named.fields.iter().filter_map(|(name, field)| {
                            field.ty.as_ref().map(|ty| (name, (field, ty)))
                        }) {
                            validate_field_attributes(
                                field,
                                format!("{path}::{variant_name}.{name}"),
                                mode,
                            )?;
                        }
                        for (name, (_, ty)) in named.fields.iter().filter_map(|(name, field)| {
                            field.ty.as_ref().map(|ty| (name, (field, ty)))
                        }) {
                            inner(
                                ty,
                                types,
                                checked_references,
                                format!("{path}::{variant_name}.{name}"),
                                mode,
                                follow_named_references,
                            )?;
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        for (idx, (field, _)) in unnamed
                            .fields
                            .iter()
                            .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                            .enumerate()
                        {
                            validate_field_attributes(
                                field,
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
                            )?;
                        }
                        for (idx, (_, ty)) in unnamed
                            .fields
                            .iter()
                            .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                            .enumerate()
                        {
                            inner(
                                ty,
                                types,
                                checked_references,
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
                                follow_named_references,
                            )?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(tuple) => {
            for (idx, ty) in tuple.elements.iter().enumerate() {
                inner(
                    ty,
                    types,
                    checked_references,
                    format!("{path}[{idx}]"),
                    mode,
                    follow_named_references,
                )?;
            }
        }
        DataType::Intersection(types_) => {
            for (idx, ty) in types_.iter().enumerate() {
                inner(
                    ty,
                    types,
                    checked_references,
                    format!("{path}.<intersection_{idx}>"),
                    mode,
                    follow_named_references,
                )?;
            }
        }
        DataType::Reference(reference) => match reference {
            Reference::Named(reference) => {
                if let NamedReferenceType::Reference {
                    generics: reference_generics,
                    ..
                } = &reference.inner
                {
                    for (_, dt) in reference_generics {
                        inner(
                            dt,
                            types,
                            checked_references,
                            format!("{path}.<generic>"),
                            mode,
                            follow_named_references,
                        )?;
                    }
                }

                if follow_named_references
                    && !checked_references.contains(&Reference::Named(reference.clone()))
                {
                    let reference_key = Reference::Named(reference.clone());
                    checked_references.insert(reference_key);
                    if let Some(ty) = named_reference_ty(reference, types) {
                        let name = types
                            .get(reference)
                            .map(|ndt| ndt.name.to_string())
                            .unwrap_or_else(|| path.clone());
                        inner(
                            ty,
                            types,
                            checked_references,
                            name,
                            mode,
                            follow_named_references,
                        )?;
                    }
                }
            }
            Reference::Opaque(reference) => {
                if let Some(phased) = reference.downcast_ref::<PhasedTy>() {
                    if mode == ApplyMode::Unified {
                        return Err(Error::invalid_phased_type_usage(
                            path,
                            "`specta_serde::Phased<Serialize, Deserialize>` requires `PhasesFormat`",
                        ));
                    }

                    inner(
                        &phased.serialize,
                        types,
                        checked_references,
                        format!("{path}.<phased_serialize>"),
                        mode,
                        follow_named_references,
                    )?;
                    inner(
                        &phased.deserialize,
                        types,
                        checked_references,
                        format!("{path}.<phased_deserialize>"),
                        mode,
                        follow_named_references,
                    )?;
                }
            }
        },
        DataType::Generic(_) | DataType::Primitive(_) => {}
    }

    Ok(())
}

fn validate_identifier_enum(enm: &Enum, path: &str, mode: ApplyMode) -> Result<(), Error> {
    let Some(attrs) = SerdeContainerAttrs::from_attributes(&enm.attributes)? else {
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
            "identifier enums require `PhasesFormat` because they widen deserialize-only input shape",
        ));
    }

    if attrs.variant_identifier
        && let Some((name, _)) = enm
            .variants
            .iter()
            .find(|(_, variant)| !matches!(&variant.fields, Fields::Unit))
    {
        return Err(Error::invalid_phased_type_usage(
            path,
            format!(
                "`variant_identifier` requires all unit variants, but variant `{name}` is not unit"
            ),
        ));
    }

    if attrs.field_identifier {
        let variants = &enm.variants;
        for (idx, (name, variant)) in variants.iter().enumerate() {
            let is_last = idx + 1 == variants.len();
            match &variant.fields {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) if is_last && unnamed.fields.len() == 1 => {}
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
    checked_references: &mut HashSet<Reference>,
    path: &str,
    mode: ApplyMode,
) -> Result<(), Error> {
    if let Some(parsed) = SerdeContainerAttrs::from_attributes(attrs)?
        && parsed.from.is_some()
        && parsed.try_from.is_some()
    {
        return Err(Error::invalid_conversion_usage(
            path,
            "`from` and `try_from` cannot be used together",
        ));
    }

    if let Some(conversions) = SerdeContainerAttrs::from_attributes(attrs)? {
        for (suffix, target) in [
            ("<serde_into>", conversions.resolved_into.as_ref()),
            ("<serde_from>", conversions.resolved_from.as_ref()),
            ("<serde_try_from>", conversions.resolved_try_from.as_ref()),
        ] {
            if let Some(target) = target {
                inner(
                    target,
                    types,
                    checked_references,
                    format!("{path}.{suffix}"),
                    mode,
                    true,
                )?;
            }
        }
    }

    Ok(())
}

fn validate_variant_attributes(
    variant: &Variant,
    path: String,
    mode: ApplyMode,
) -> Result<(), Error> {
    let Some(serde_attrs) = SerdeVariantAttrs::from_attributes(&variant.attributes)? else {
        return Ok(());
    };

    let has_type_override = has_type_override(&variant.attributes);

    if mode == ApplyMode::Unified && !serde_attrs.aliases.is_empty() {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(alias = ...)]` requires `PhasesFormat` because aliases widen deserialize-only input shape",
        ));
    }
    if serde_attrs.has_serialize_with {
        ensure_codec_override(has_type_override, &path, "serialize_with")?;
    }
    if serde_attrs.has_deserialize_with {
        ensure_codec_override(has_type_override, &path, "deserialize_with")?;
    }
    if serde_attrs.has_with {
        ensure_codec_override(has_type_override, &path, "with")?;
    }

    if mode == ApplyMode::Unified
        && serde_attrs.untagged
        && serde_attrs.skip_serializing != serde_attrs.skip_deserializing
    {
        return Err(Error::invalid_phased_type_usage(
            path,
            "phase-specific `#[serde(untagged)]` variants require `PhasesFormat` because unified mode would drop one branch",
        ));
    }

    Ok(())
}

fn validate_field_attributes(field: &Field, path: String, mode: ApplyMode) -> Result<(), Error> {
    let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(());
    };

    if mode == ApplyMode::Unified
        && let (Some(serialize), Some(deserialize)) = (
            serde_attrs.rename_serialize.as_deref(),
            serde_attrs.rename_deserialize.as_deref(),
        )
        && serialize != deserialize
    {
        return Err(Error::incompatible_rename(
            "field rename",
            path,
            Some(serialize.to_string()),
            Some(deserialize.to_string()),
        ));
    }

    let has_type_override = has_type_override(&field.attributes);

    if mode == ApplyMode::Unified && !serde_attrs.aliases.is_empty() {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(alias = ...)]` requires `PhasesFormat` because aliases widen deserialize-only input shape",
        ));
    }

    if serde_attrs.has_serialize_with {
        ensure_codec_override(has_type_override, &path, "serialize_with")?;
    }
    if serde_attrs.has_deserialize_with {
        ensure_codec_override(has_type_override, &path, "deserialize_with")?;
    }
    if serde_attrs.has_with {
        ensure_codec_override(has_type_override, &path, "with")?;
    }

    if mode == ApplyMode::Unified && serde_attrs.skip_serializing_if.is_some() {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`skip_serializing_if` requires `PhasesFormat` because unified mode cannot represent conditional omission",
        ));
    }

    Ok(())
}

fn ensure_codec_override(
    has_type_override: bool,
    path: &str,
    attr: &'static str,
) -> Result<(), Error> {
    if has_type_override {
        return Ok(());
    }

    Err(Error::unsupported_serde_custom_codec(
        path.to_string(),
        attr,
    ))
}

fn has_type_override(attributes: &specta::datatype::Attributes) -> bool {
    attributes
        .get_named_as::<bool>("specta:type_override")
        .copied()
        .unwrap_or(false)
}

fn validate_enum(enm: &Enum, types: &Types, path: String, mode: ApplyMode) -> Result<(), Error> {
    let valid_variants = enm
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .count();
    if valid_variants == 0 && !&enm.variants.is_empty() {
        return Err(Error::invalid_usage_of_skip(
            path,
            "all variants are skipped, resulting in an invalid non-empty enum",
        ));
    }

    let repr = EnumRepr::from_attrs(&enm.attributes)?;

    validate_untagged_variants(enm, &path)?;
    validate_other_variant(enm, &path, &repr, mode)?;

    if matches!(repr, EnumRepr::Internal { .. }) {
        validate_internally_tag_enum(enm, types, path)?;
    }

    Ok(())
}

fn validate_untagged_variants(enm: &Enum, path: &str) -> Result<(), Error> {
    let mut seen_untagged = false;

    for (name, variant) in &enm.variants {
        let is_untagged = SerdeVariantAttrs::from_attributes(&variant.attributes)?
            .is_some_and(|attrs| attrs.untagged);

        if is_untagged {
            seen_untagged = true;
            continue;
        }

        if seen_untagged && !variant.skip {
            return Err(Error::invalid_phased_type_usage(
                path,
                format!(
                    "`#[serde(untagged)]` variants must be ordered last, but variant `{name}` appears after an untagged variant"
                ),
            ));
        }
    }

    Ok(())
}

fn validate_other_variant(
    enm: &Enum,
    path: &str,
    repr: &EnumRepr,
    mode: ApplyMode,
) -> Result<(), Error> {
    let mut other_variants = Vec::new();
    for (name, variant) in &enm.variants {
        if SerdeVariantAttrs::from_attributes(&variant.attributes)?.is_some_and(|attrs| attrs.other)
        {
            other_variants.push((name, variant));
        }
    }

    if other_variants.is_empty() {
        return Ok(());
    }

    if mode == ApplyMode::Unified {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` requires `PhasesFormat` because it widens deserialize-only input shape",
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
    if !matches!(&variant.fields, Fields::Unit) {
        return Err(Error::invalid_phased_type_usage(
            path,
            format!("`#[serde(other)]` variant `{name}` must be a unit variant"),
        ));
    }

    Ok(())
}

fn validate_internally_tag_enum(enm: &Enum, types: &Types, path: String) -> Result<(), Error> {
    for (variant_name, variant) in &enm.variants {
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
) -> Result<(), Error> {
    let _ = enm;
    if SerdeVariantAttrs::from_attributes(&variant.attributes)?.is_some_and(|attrs| attrs.untagged)
    {
        return Ok(());
    }

    match &variant.fields {
        Fields::Unit | Fields::Named(_) => Ok(()),
        Fields::Unnamed(unnamed) => {
            let mut fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)));
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
) -> Result<(), Error> {
    match ty {
        DataType::Map(_) => Ok(()),
        DataType::Struct(_) => Ok(()),
        DataType::Reference(Reference::Named(reference))
            if matches!(reference.inner, NamedReferenceType::Inline { .. }) =>
        {
            if let Some(ty) = named_reference_ty(reference, types) {
                validate_internally_tag_enum_datatype(ty, types, path, variant_name)?;
            }

            Ok(())
        }
        DataType::Enum(enm) => match EnumRepr::from_attrs(&enm.attributes)? {
            EnumRepr::Untagged => validate_internally_tag_enum(enm, types, path.to_string()),
            EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } => Ok(()),
        },
        DataType::Tuple(tuple) if tuple.elements.is_empty() => Ok(()),
        DataType::Reference(Reference::Named(reference)) => {
            if let Some(ty) = named_reference_ty(reference, types) {
                validate_internally_tag_enum_datatype(ty, types, path, variant_name)?;
            }

            Ok(())
        }
        DataType::Reference(Reference::Opaque(_))
        | DataType::Generic(_)
        | DataType::Intersection(_)
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

fn named_reference_ty<'a>(
    reference: &'a specta::datatype::NamedReference,
    types: &'a Types,
) -> Option<&'a DataType> {
    match &reference.inner {
        NamedReferenceType::Inline { dt, .. } => Some(dt),
        NamedReferenceType::Reference { .. } => types.get(reference)?.ty.as_ref(),
        NamedReferenceType::Recursive(_) => None,
    }
}
