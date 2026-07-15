use std::collections::HashSet;

use specta::{
    Types,
    datatype::{
        DataType, Enum, Field, Fields, NamedFields, NamedReferenceType, Reference, Variant,
    },
};

use crate::{
    Error, PhaseRewrite, container_rename_all_rule, enum_variant_field_rename_rule,
    inflection::RenameRule,
    parser::{SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs},
    phase_field_key,
    phased::PhasedTy,
    repr::EnumRepr,
    serialized_variant_name,
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
                    validate_named_field_keys(
                        named,
                        struct_field_key_rules(&strct.attributes, &path)?,
                        &path,
                        mode,
                    )?;
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
            // Untagged enums never emit variant names, so attrs that only
            // rename variant labels are not part of the wire shape for them.
            let variant_names_emitted =
                !matches!(EnumRepr::from_attrs(&enm.attributes)?, EnumRepr::Untagged);
            validate_container_attributes(&enm.attributes, types, checked_references, &path, mode)?;
            let container_attrs = SerdeContainerAttrs::from_attributes(&enm.attributes)?;
            if container_attrs.as_ref().is_some_and(|attrs| attrs.default) {
                return Err(Error::invalid_phased_type_usage(
                    path,
                    "`#[serde(default)]` is only valid on structs",
                ));
            }
            validate_identifier_enum(enm, &path, mode)?;
            validate_enum(enm, types, path.clone(), mode)?;

            for (variant_name, variant) in &enm.variants {
                let variant_is_rendered = variant_is_rendered(variant)?;
                validate_variant_attributes(variant, format!("{path}::{variant_name}"), mode)?;
                if variant_names_emitted && variant_is_rendered {
                    validate_variant_key(variant_name, variant, &container_attrs, &path, mode)?;
                }
                match &variant.fields {
                    Fields::Unit => {}
                    Fields::Named(named) => {
                        if variant_is_rendered {
                            validate_named_field_keys(
                                named,
                                variant_field_key_rules(&container_attrs, variant, variant_name)?,
                                &format!("{path}::{variant_name}"),
                                mode,
                            )?;
                        }
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
    let Some(container_attrs) = SerdeContainerAttrs::from_attributes(attrs)? else {
        return Ok(());
    };

    if container_attrs.from.is_some() && container_attrs.try_from.is_some() {
        return Err(Error::invalid_conversion_usage(
            path,
            "`from` and `try_from` cannot be used together",
        ));
    }

    for (suffix, target) in [
        ("<serde_into>", container_attrs.resolved_into.as_ref()),
        ("<serde_from>", container_attrs.resolved_from.as_ref()),
        (
            "<serde_try_from>",
            container_attrs.resolved_try_from.as_ref(),
        ),
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

    if mode == ApplyMode::Unified {
        // The effective container name per direction defaults to the type's
        // own name (`path` is the type name here: the traversal resets it at
        // every named reference), so a one-sided rename equal to that name is
        // a no-op. `rename_all` / `rename_all_fields` are validated per field
        // and variant key via `validate_named_field_keys` /
        // `validate_variant_key` instead of comparing the raw rules.
        let rename_serialize = container_attrs.rename_serialize.as_deref().unwrap_or(path);
        let rename_deserialize = container_attrs
            .rename_deserialize
            .as_deref()
            .unwrap_or(path);
        if rename_serialize != rename_deserialize {
            return Err(Error::incompatible_rename(
                "container rename",
                path.to_string(),
                Some(rename_serialize.to_string()),
                Some(rename_deserialize.to_string()),
            ));
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

    // Variant renames and rename_all rules are validated by comparing the
    // *effective* per-direction names/keys in `validate_variant_key` and
    // `validate_named_field_keys` (see `inner`'s enum arm), not the raw attrs.

    if mode == ApplyMode::Unified
        && !variant.skip
        && serde_attrs.skip_serializing != serde_attrs.skip_deserializing
    {
        // Applies regardless of `untagged`: a mismatched skip means serde
        // constructs/emits this variant on one side but not the other
        // (`variant_is_skipped_for_mode` would otherwise drop it from the
        // unified type entirely, even for the side that still uses it).
        // `#[specta(skip)]` variants are exempt: they are dropped from both
        // phases before the serde skip flags matter.
        return Err(Error::incompatible_skip(
            "variant skip",
            path,
            serde_attrs.skip_serializing,
            serde_attrs.skip_deserializing,
        ));
    }

    Ok(())
}

/// The rename rules a struct's named field keys are subject to, per phase
/// (`Serialize`, then `Deserialize`), computed by the same helper the rewrite
/// path uses.
fn struct_field_key_rules(
    attrs: &specta::datatype::Attributes,
    path: &str,
) -> Result<(Option<RenameRule>, Option<RenameRule>), Error> {
    Ok((
        container_rename_all_rule(attrs, PhaseRewrite::Serialize, "struct rename_all", path)?,
        container_rename_all_rule(attrs, PhaseRewrite::Deserialize, "struct rename_all", path)?,
    ))
}

/// The rename rules an enum variant's named field keys are subject to
/// (variant `rename_all`, falling back to the container's
/// `rename_all_fields`), per phase, computed by the same helper the rewrite
/// path uses.
fn variant_field_key_rules(
    container_attrs: &Option<SerdeContainerAttrs>,
    variant: &Variant,
    variant_name: &str,
) -> Result<(Option<RenameRule>, Option<RenameRule>), Error> {
    Ok((
        enum_variant_field_rename_rule(
            container_attrs,
            variant,
            PhaseRewrite::Serialize,
            variant_name,
        )?,
        enum_variant_field_rename_rule(
            container_attrs,
            variant,
            PhaseRewrite::Deserialize,
            variant_name,
        )?,
    ))
}

/// Unified mode requires every live named field key to resolve to the same
/// effective wire key in both directions. Effective keys are computed with
/// [`phase_field_key`] — the exact function the rewrite path uses — so
/// explicit renames, rename rules applied to default names, and their no-op
/// coincidences are all judged by what actually reaches the wire, never by
/// comparing raw attribute options.
fn validate_named_field_keys(
    named: &NamedFields,
    (rule_serialize, rule_deserialize): (Option<RenameRule>, Option<RenameRule>),
    path: &str,
    mode: ApplyMode,
) -> Result<(), Error> {
    if mode != ApplyMode::Unified {
        return Ok(());
    }

    for (name, field) in &named.fields {
        if !field_has_live_key(field)? {
            continue;
        }

        let serde_attrs = SerdeFieldAttrs::from_attributes(&field.attributes)?;
        let serialize_key = phase_field_key(
            name.as_ref(),
            serde_attrs.as_ref(),
            rule_serialize,
            PhaseRewrite::Serialize,
        )?;
        let deserialize_key = phase_field_key(
            name.as_ref(),
            serde_attrs.as_ref(),
            rule_deserialize,
            PhaseRewrite::Deserialize,
        )?;

        if serialize_key != deserialize_key {
            return Err(Error::incompatible_rename(
                "field key",
                format!("{path}.{name}"),
                Some(serialize_key),
                Some(deserialize_key),
            ));
        }
    }

    Ok(())
}

/// Unified mode requires an emitted variant label to resolve to the same
/// effective name in both directions, computed with
/// [`serialized_variant_name`] — the exact function the rewrite path uses.
/// Variant-level `#[serde(untagged)]` variants and variants that are never
/// rendered have no emitted label to compare.
fn validate_variant_key(
    variant_name: &str,
    variant: &Variant,
    container_attrs: &Option<SerdeContainerAttrs>,
    path: &str,
    mode: ApplyMode,
) -> Result<(), Error> {
    // Dead variants (see `variant_is_rendered`) are gated by the caller.
    if mode != ApplyMode::Unified {
        return Ok(());
    }

    if SerdeVariantAttrs::from_attributes(&variant.attributes)?.is_some_and(|attrs| attrs.untagged)
    {
        return Ok(());
    }

    let serialize_name = serialized_variant_name(
        variant_name,
        variant,
        container_attrs,
        PhaseRewrite::Serialize,
    )?;
    let deserialize_name = serialized_variant_name(
        variant_name,
        variant,
        container_attrs,
        PhaseRewrite::Deserialize,
    )?;

    if serialize_name != deserialize_name {
        return Err(Error::incompatible_rename(
            "variant name",
            format!("{path}::{variant_name}"),
            Some(serialize_name),
            Some(deserialize_name),
        ));
    }

    Ok(())
}

/// Whether the variant is rendered in at least one phase. `#[specta(skip)]`
/// variants (also set by `#[serde(skip)]` at derive time) and variants
/// serde-skipped in both directions are dropped by
/// `filter_enum_variants_for_phase` before either phase's output is produced,
/// so nothing about them — names, field keys, skip asymmetry — can create a
/// phase difference.
fn variant_is_rendered(variant: &Variant) -> Result<bool, Error> {
    if variant.skip {
        return Ok(false);
    }

    Ok(!SerdeVariantAttrs::from_attributes(&variant.attributes)?
        .is_some_and(|attrs| attrs.skip_serializing && attrs.skip_deserializing))
}

/// Whether a named field's *key* is part of the local wire shape: excludes
/// fields with an erased type (never rendered), flattened fields (their keys
/// come from the flattened type), and fields skipped in both directions.
fn field_has_live_key(field: &Field) -> Result<bool, Error> {
    if field.ty.is_none() {
        return Ok(false);
    }

    let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(true);
    };

    Ok(!attrs.flatten && !(attrs.skip_serializing && attrs.skip_deserializing))
}

fn validate_field_attributes(field: &Field, path: String, mode: ApplyMode) -> Result<(), Error> {
    let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(());
    };

    // Field renames are validated by comparing the *effective* per-direction
    // keys in `validate_named_field_keys` (unnamed fields have no wire key,
    // so renames are meaningless for them).

    if mode == ApplyMode::Unified && serde_attrs.skip_serializing != serde_attrs.skip_deserializing
    {
        // Plain `#[serde(skip)]` sets both flags and never reaches here.
        // A one-sided skip means serde requires/emits the field on one side
        // but not the other, which a unified shape can't represent: it would
        // either wrongly drop a field one side needs, or wrongly keep a field
        // the other side never sees.
        return Err(Error::incompatible_skip(
            "field skip",
            path,
            serde_attrs.skip_serializing,
            serde_attrs.skip_deserializing,
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
    validate_adjacent_collapsed_newtype_variants(enm, &path, &repr, mode)?;

    if matches!(repr, EnumRepr::Internal { .. }) {
        validate_internally_tag_enum(enm, types, path, &mut HashSet::new())?;
    }

    Ok(())
}

/// An adjacently tagged newtype variant whose sole non-`Option` field is
/// `#[serde(skip)]`ped is direction-asymmetric even though the skip itself is
/// symmetric: serde's serializer omits the `content` key entirely, but its
/// deserializer still requires `content: null`. No unified shape can
/// represent both directions, so unified mode rejects it (matching the
/// policy for one-sided renames/skips). A skipped `Option` sole field is
/// exempt: serde's `missing_field` helper deserializes a missing `content`
/// key as `None`, so the unified `content?: null` shape is exact for both
/// directions.
fn validate_adjacent_collapsed_newtype_variants(
    enm: &Enum,
    path: &str,
    repr: &EnumRepr,
    mode: ApplyMode,
) -> Result<(), Error> {
    if mode != ApplyMode::Unified || !matches!(repr, EnumRepr::Adjacent { .. }) {
        return Ok(());
    }

    for (name, variant) in &enm.variants {
        if variant.skip {
            continue;
        }
        // `#[serde(untagged)]` variants bypass the tag/content representation
        // entirely (they serialize as their bare payload), so there is no
        // `content` key to be asymmetric about.
        if SerdeVariantAttrs::from_attributes(&variant.attributes)?.is_some_and(|attrs| {
            attrs.untagged || (attrs.skip_serializing && attrs.skip_deserializing)
        }) {
            continue;
        }

        let Fields::Unnamed(unnamed) = &variant.fields else {
            continue;
        };
        let [field] = unnamed.fields.as_slice() else {
            continue;
        };

        // Only serde-level symmetric skips exhibit the asymmetry (one-sided
        // skips are already rejected by the field-level check; specta-only
        // skips are invisible to serde).
        if !SerdeFieldAttrs::from_attributes(&field.attributes)?
            .is_some_and(|attrs| attrs.skip_serializing && attrs.skip_deserializing)
        {
            continue;
        }

        if field.attributes.contains_key(crate::NULLABLE_FIELD) {
            continue;
        }

        return Err(Error::invalid_phased_type_usage(
            format!("{path}.{name}"),
            "`#[serde(skip)]` on an adjacently tagged newtype variant's non-`Option` field \
             requires `PhasesFormat` because serde's serializer omits the `content` key while \
             its deserializer requires `content: null`",
        ));
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

    if !matches!(
        repr,
        EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. }
    ) {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` requires a tagged enum representation",
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

fn validate_internally_tag_enum(
    enm: &Enum,
    types: &Types,
    path: String,
    seen: &mut HashSet<Reference>,
) -> Result<(), Error> {
    for (variant_name, variant) in &enm.variants {
        validate_internally_tag_variant(enm, variant_name, variant, types, &path, seen)?;
    }

    Ok(())
}

fn validate_internally_tag_variant(
    enm: &Enum,
    variant_name: &str,
    variant: &Variant,
    types: &Types,
    path: &str,
    seen: &mut HashSet<Reference>,
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

            validate_internally_tag_enum_datatype(first_field, types, path, variant_name, seen)
        }
    }
}

// `seen` tracks named references on the current recursion path so cyclic
// (self- or mutually-recursive) untagged payloads terminate; a reference
// already on the path is being validated further up the stack. Mirrors
// `internal_tag_payload_compatibility` in `lib.rs`.
fn validate_internally_tag_enum_datatype(
    ty: &DataType,
    types: &Types,
    path: &str,
    variant_name: &str,
    seen: &mut HashSet<Reference>,
) -> Result<(), Error> {
    match ty {
        DataType::Map(_) => Ok(()),
        DataType::Struct(_) => Ok(()),
        DataType::Reference(Reference::Named(reference)) => {
            let key = Reference::Named(reference.clone());
            if !seen.insert(key.clone()) {
                return Ok(());
            }

            let result = named_reference_ty(reference, types).map_or(Ok(()), |ty| {
                validate_internally_tag_enum_datatype(ty, types, path, variant_name, seen)
            });
            seen.remove(&key);

            result
        }
        DataType::Enum(enm) => match EnumRepr::from_attrs(&enm.attributes)? {
            EnumRepr::Untagged => validate_internally_tag_enum(enm, types, path.to_string(), seen),
            EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } => Ok(()),
        },
        DataType::Tuple(tuple) if tuple.elements.is_empty() => Ok(()),
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
