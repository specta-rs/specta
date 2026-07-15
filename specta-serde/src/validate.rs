use std::collections::{HashMap, HashSet};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Field, Fields, Generic, NamedFields, NamedReference, NamedReferenceType,
        Reference, Variant,
    },
};

use crate::{
    Error, Phase, PhaseRewrite, container_rename_all_rule, enum_variant_field_rename_rule,
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
            None,
            FlattenDirections::LIVE,
        )?;
    }

    // Generic definitions are necessarily conditional: an internally tagged
    // `E<T> { V(T) }` is valid for map-like `T` and invalid for scalar `T`.
    // Walk registered roots as use sites so their concrete generic arguments
    // are available through `GenericEnv` when following the named reference.
    for root in types.roots() {
        inner(
            root,
            types,
            &mut HashSet::new(),
            "<root>".to_string(),
            mode,
            true,
            None,
            FlattenDirections::LIVE,
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
        None,
        FlattenDirections::LIVE,
    )
}

pub(crate) fn validate_datatype_for_mode_shallow(
    dt: &DataType,
    types: &Types,
    mode: ApplyMode,
    phase: Phase,
) -> Result<(), Error> {
    inner(
        dt,
        types,
        &mut HashSet::new(),
        "<top-level>".to_string(),
        mode,
        false,
        None,
        FlattenDirections::for_phase(phase),
    )
}

fn inner(
    dt: &DataType,
    types: &Types,
    checked_references: &mut HashSet<(Reference, FlattenDirections)>,
    path: String,
    mode: ApplyMode,
    follow_named_references: bool,
    env: Option<&GenericEnv<'_>>,
    root_directions: FlattenDirections,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => inner(
            ty,
            types,
            checked_references,
            path,
            mode,
            follow_named_references,
            env,
            root_directions,
        )?,
        DataType::Map(map) => {
            inner(
                map.key_ty(),
                types,
                checked_references,
                format!("{path}.<map_key>"),
                mode,
                follow_named_references,
                env,
                root_directions,
            )?;
            inner(
                map.value_ty(),
                types,
                checked_references,
                format!("{path}.<map_value>"),
                mode,
                follow_named_references,
                env,
                root_directions,
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
                env,
                root_directions,
            )?;
        }
        DataType::Struct(strct) => {
            let container_attrs = SerdeContainerAttrs::from_attributes(&strct.attributes)?;
            let declared_deserialize_live = root_directions.deserialize
                && !container_attrs.as_ref().is_some_and(|attrs| {
                    attrs.resolved_from.is_some() || attrs.resolved_try_from.is_some()
                });
            validate_container_attributes(
                &strct.attributes,
                types,
                checked_references,
                &path,
                mode,
                env,
                root_directions,
            )?;
            if let Some(attrs) = &container_attrs {
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

            // serde_derive silently ignores every `skip` spelling on a
            // newtype struct's only field. The Type derive preserves that
            // field and marks it so exporters can retain the bare wire shape.
            if let Fields::Unnamed(unnamed) = &strct.fields
                && let [field] = unnamed.fields.as_slice()
                && !field
                    .attributes
                    .contains_key(crate::SERDE_NEWTYPE_SKIP_IGNORED)
                && SerdeFieldAttrs::from_attributes(&field.attributes)?
                    .is_some_and(|attrs| attrs.skip_serializing || attrs.skip_deserializing)
            {
                return Err(Error::invalid_usage_of_skip(
                    path,
                    "serde ignores `skip` on a newtype struct's only field (the wire format \
                     stays the bare inner value), so the export cannot represent it; remove \
                     the attribute -- it has no effect on the serde wire format",
                ));
            }

            match &strct.fields {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    if root_directions.serialize
                        && !container_attrs
                            .as_ref()
                            .is_some_and(|attrs| attrs.resolved_into.is_some())
                    {
                        validate_unnamed_conditional_omission(unnamed, &path)?;
                    }
                    for (idx, (field, _)) in unnamed
                        .fields
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, field)| field.ty.as_ref().map(|ty| (idx, (field, ty))))
                    {
                        validate_field_attributes(field, format!("{path}[{idx}]"), mode)?;
                    }
                    for (idx, (field, ty)) in unnamed
                        .fields
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, field)| field.ty.as_ref().map(|ty| (idx, (field, ty))))
                    {
                        inner(
                            ty,
                            types,
                            checked_references,
                            format!("{path}[{idx}]"),
                            mode,
                            follow_named_references,
                            env,
                            root_directions.and(FlattenDirections::for_field(field)?),
                        )?;
                    }
                }
                Fields::Named(named) => {
                    validate_named_field_keys(
                        named,
                        struct_field_key_rules(&strct.attributes, &path)?,
                        &path,
                        declared_deserialize_live,
                        mode,
                    )?;
                    for (name, (field, ty)) in named
                        .fields
                        .iter()
                        .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                    {
                        validate_field_attributes(field, format!("{path}.{name}"), mode)?;
                        validate_flatten_field(
                            field,
                            ty,
                            types,
                            &format!("{path}.{name}"),
                            mode,
                            root_directions,
                            env,
                        )?;
                    }
                    for (name, (field, ty)) in named
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
                            env,
                            root_directions.and(FlattenDirections::for_field(field)?),
                        )?;
                    }
                }
            }
        }
        DataType::Enum(enm) => {
            let container_attrs = SerdeContainerAttrs::from_attributes(&enm.attributes)?;
            let declared_directions = FlattenDirections {
                serialize: root_directions.serialize
                    && !container_attrs
                        .as_ref()
                        .is_some_and(|attrs| attrs.resolved_into.is_some()),
                deserialize: root_directions.deserialize
                    && !container_attrs.as_ref().is_some_and(|attrs| {
                        attrs.resolved_from.is_some() || attrs.resolved_try_from.is_some()
                    }),
            };
            // Untagged enums never emit variant names, so attrs that only
            // rename variant labels are not part of the wire shape for them.
            let variant_names_emitted =
                !matches!(EnumRepr::from_attrs(&enm.attributes)?, EnumRepr::Untagged);
            validate_container_attributes(
                &enm.attributes,
                types,
                checked_references,
                &path,
                mode,
                env,
                root_directions,
            )?;
            if container_attrs.as_ref().is_some_and(|attrs| attrs.default) {
                return Err(Error::invalid_phased_type_usage(
                    path,
                    "`#[serde(default)]` is only valid on structs",
                ));
            }
            validate_identifier_enum(enm, &path, mode)?;
            validate_enum(
                enm,
                types,
                path.clone(),
                mode,
                declared_directions.deserialize,
                env,
            )?;
            if variant_names_emitted {
                validate_variant_aliases(
                    enm,
                    &container_attrs,
                    &path,
                    declared_directions.deserialize,
                    mode,
                )?;
            }

            for (variant_name, variant) in &enm.variants {
                let variant_is_rendered = variant_is_rendered(variant)?;
                validate_variant_attributes(variant, format!("{path}::{variant_name}"), mode)?;
                let variant_directions =
                    FlattenDirections::for_variant(variant)?.and(root_directions);
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
                                declared_directions.deserialize && variant_directions.deserialize,
                                mode,
                            )?;
                        }
                        for (name, (field, ty)) in
                            named.fields.iter().filter_map(|(name, field)| {
                                field.ty.as_ref().map(|ty| (name, (field, ty)))
                            })
                        {
                            let path = format!("{path}::{variant_name}.{name}");
                            validate_field_attributes(field, path.clone(), mode)?;
                            validate_flatten_field(
                                field,
                                ty,
                                types,
                                &path,
                                mode,
                                variant_directions,
                                env,
                            )?;
                        }
                        for (name, (field, ty)) in
                            named.fields.iter().filter_map(|(name, field)| {
                                field.ty.as_ref().map(|ty| (name, (field, ty)))
                            })
                        {
                            inner(
                                ty,
                                types,
                                checked_references,
                                format!("{path}::{variant_name}.{name}"),
                                mode,
                                follow_named_references,
                                env,
                                variant_directions.and(FlattenDirections::for_field(field)?),
                            )?;
                        }
                    }
                    Fields::Unnamed(unnamed) => {
                        if declared_directions.serialize && variant_is_serialized(variant)? {
                            validate_unnamed_conditional_omission(
                                unnamed,
                                &format!("{path}::{variant_name}"),
                            )?;
                        }
                        for (idx, (field, _)) in
                            unnamed
                                .fields
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, field)| {
                                    field.ty.as_ref().map(|ty| (idx, (field, ty)))
                                })
                        {
                            validate_field_attributes(
                                field,
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
                            )?;
                        }
                        for (idx, (field, ty)) in
                            unnamed
                                .fields
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, field)| {
                                    field.ty.as_ref().map(|ty| (idx, (field, ty)))
                                })
                        {
                            inner(
                                ty,
                                types,
                                checked_references,
                                format!("{path}::{variant_name}[{idx}]"),
                                mode,
                                follow_named_references,
                                env,
                                variant_directions.and(FlattenDirections::for_field(field)?),
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
                    env,
                    root_directions,
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
                    env,
                    root_directions,
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
                            env,
                            root_directions,
                        )?;
                    }
                }

                // The memo is keyed on the reference with its generic
                // arguments *resolved through the current substitution*, so
                // the same syntactic reference reached under two different
                // outer instantiations (say `Outer<Inner>` and
                // `Outer<Vec<u8>>` both naming `FlattenGeneric<T>` in their
                // body) is re-validated for each. Identical
                // (reference, resolved arguments) pairs still dedupe, which
                // keeps cyclic graphs terminating; if resolving a key would
                // grow past `RESOLVED_KEY_NODE_BUDGET` (only possible for
                // hand-built non-regular generics - derived ones don't
                // compile), it falls back to the syntactic key, trading that
                // pathological corner's strictness for termination. The key
                // also carries the walk's flatten liveness, so a visit through
                // a phase-dead field can't mask a later live visit; liveness
                // only shrinks along a path and has four values, so the key
                // space stays finite.
                let reference_key = (resolved_reference_key(reference, env), root_directions);
                if follow_named_references && !checked_references.contains(&reference_key) {
                    checked_references.insert(reference_key);
                    if let Some(ty) = named_reference_ty(reference, types) {
                        let name = types
                            .get(reference)
                            .map(|ndt| ndt.name.to_string())
                            .unwrap_or_else(|| path.clone());
                        // A resolved definition's `Generic` placeholders are
                        // bound by *this* reference's concrete arguments,
                        // which were written in the current scope - hence a
                        // new frame with `parent: env`. Inline datatypes were
                        // written at the use site, so they keep the current
                        // scope.
                        let reference_frame = match &reference.inner {
                            NamedReferenceType::Reference { generics, .. } => Some(GenericEnv {
                                map: generics,
                                parent: env,
                            }),
                            NamedReferenceType::Inline { .. }
                            | NamedReferenceType::Recursive(_) => None,
                        };
                        inner(
                            ty,
                            types,
                            checked_references,
                            name,
                            mode,
                            follow_named_references,
                            reference_frame.as_ref().or(env),
                            root_directions,
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

                    // Each phased side is only ever rendered for its own
                    // phase, so flatten liveness narrows accordingly.
                    inner(
                        &phased.serialize,
                        types,
                        checked_references,
                        format!("{path}.<phased_serialize>"),
                        mode,
                        follow_named_references,
                        env,
                        root_directions.and(FlattenDirections::SERIALIZE_ONLY),
                    )?;
                    inner(
                        &phased.deserialize,
                        types,
                        checked_references,
                        format!("{path}.<phased_deserialize>"),
                        mode,
                        follow_named_references,
                        env,
                        root_directions.and(FlattenDirections::DESERIALIZE_ONLY),
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
    checked_references: &mut HashSet<(Reference, FlattenDirections)>,
    path: &str,
    mode: ApplyMode,
    env: Option<&GenericEnv<'_>>,
    root_directions: FlattenDirections,
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

    // A directional conversion target is only used in its own phase (`into`
    // only ever serializes, `from`/`try_from` only ever deserialize - see
    // `select_conversion_target`), so flatten liveness narrows per target,
    // composed with the caller's root liveness.
    for (suffix, target, target_directions) in [
        (
            "<serde_into>",
            container_attrs.resolved_into.as_ref(),
            FlattenDirections::SERIALIZE_ONLY,
        ),
        (
            "<serde_from>",
            container_attrs.resolved_from.as_ref(),
            FlattenDirections::DESERIALIZE_ONLY,
        ),
        (
            "<serde_try_from>",
            container_attrs.resolved_try_from.as_ref(),
            FlattenDirections::DESERIALIZE_ONLY,
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
                env,
                root_directions.and(target_directions),
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
    declared_deserialize_live: bool,
    mode: ApplyMode,
) -> Result<(), Error> {
    if mode != ApplyMode::Unified || !declared_deserialize_live {
        return Ok(());
    }

    let mut occupied_deserialize_keys = HashMap::new();
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

        occupied_deserialize_keys.insert(deserialize_key, name.as_ref());
    }

    for (name, field) in &named.fields {
        if !field_has_live_key(field)? {
            continue;
        }

        let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
            continue;
        };
        for alias in attrs.aliases {
            if let Some(owner) = occupied_deserialize_keys.get(alias.as_str())
                && *owner != name.as_ref()
            {
                return Err(Error::invalid_phased_type_usage(
                    format!("{path}.{name}"),
                    format!(
                        "field alias `{alias}` collides with a key already accepted by `{owner}`; unified alias lowering cannot represent serde's key precedence safely"
                    ),
                ));
            }
            occupied_deserialize_keys.insert(alias, name.as_ref());
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

fn validate_variant_aliases(
    enm: &Enum,
    container_attrs: &Option<SerdeContainerAttrs>,
    path: &str,
    declared_deserialize_live: bool,
    mode: ApplyMode,
) -> Result<(), Error> {
    if mode != ApplyMode::Unified || !declared_deserialize_live {
        return Ok(());
    }

    let mut occupied_deserialize_names = HashMap::new();
    for (name, variant) in &enm.variants {
        let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if !FlattenDirections::for_variant(variant)?.deserialize
            || attrs.as_ref().is_some_and(|attrs| attrs.untagged)
        {
            continue;
        }

        occupied_deserialize_names.insert(
            serialized_variant_name(name, variant, container_attrs, PhaseRewrite::Deserialize)?,
            name.as_ref(),
        );
    }

    for (name, variant) in &enm.variants {
        let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if !FlattenDirections::for_variant(variant)?.deserialize
            || attrs.as_ref().is_some_and(|attrs| attrs.untagged)
        {
            continue;
        }

        for alias in attrs.into_iter().flat_map(|attrs| attrs.aliases) {
            if let Some(owner) = occupied_deserialize_names.get(alias.as_str())
                && *owner != name.as_ref()
            {
                return Err(Error::invalid_phased_type_usage(
                    format!("{path}::{name}"),
                    format!(
                        "variant alias `{alias}` collides with a name already accepted by `{owner}`; unified alias lowering cannot represent serde's name precedence safely"
                    ),
                ));
            }
            occupied_deserialize_names.insert(alias, name.as_ref());
        }
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

fn variant_is_serialized(variant: &Variant) -> Result<bool, Error> {
    if variant.skip {
        return Ok(false);
    }

    Ok(!SerdeVariantAttrs::from_attributes(&variant.attributes)?
        .is_some_and(|attrs| attrs.skip_serializing))
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

    Ok(!(attrs.flatten || attrs.skip_serializing && attrs.skip_deserializing))
}

fn validate_field_attributes(field: &Field, path: String, mode: ApplyMode) -> Result<(), Error> {
    let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(());
    };

    // Field renames are validated by comparing the *effective* per-direction
    // keys in `validate_named_field_keys` (unnamed fields have no wire key,
    // so renames are meaningless for them).

    if mode == ApplyMode::Unified
        && !field
            .attributes
            .contains_key(crate::SERDE_NEWTYPE_SKIP_IGNORED)
        && serde_attrs.skip_serializing != serde_attrs.skip_deserializing
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

    if serde_attrs.has_serialize_with {
        ensure_codec_override(has_type_override, &path, "serialize_with")?;
    }
    if serde_attrs.has_deserialize_with {
        ensure_codec_override(has_type_override, &path, "deserialize_with")?;
    }
    if serde_attrs.has_with {
        ensure_codec_override(has_type_override, &path, "with")?;
    }

    Ok(())
}

fn validate_unnamed_conditional_omission(
    unnamed: &specta::datatype::UnnamedFields,
    path: &str,
) -> Result<(), Error> {
    let mut conditional_omission = None;
    for (idx, field) in unnamed.fields.iter().enumerate() {
        if unnamed_field_is_absent_from_serialization(field)? {
            continue;
        }

        if let Some(omitted_idx) = conditional_omission {
            return Err(Error::invalid_phased_type_usage(
                format!("{path}[{omitted_idx}]"),
                "`skip_serializing_if` on an unnamed field is only representable on the final live field because omitting an earlier element shifts every following value",
            ));
        }

        let has_skip_serializing_if = SerdeFieldAttrs::from_attributes(&field.attributes)?
            .is_some_and(|attrs| attrs.skip_serializing_if.is_some());
        if has_skip_serializing_if {
            conditional_omission = Some(idx);
        }
    }

    Ok(())
}

fn unnamed_field_is_absent_from_serialization(field: &Field) -> Result<bool, Error> {
    if field.ty.is_none() {
        return Ok(true);
    }

    Ok(SerdeFieldAttrs::from_attributes(&field.attributes)?
        .is_some_and(|attrs| attrs.skip_serializing))
}

/// Validates a `#[serde(flatten)]`-ed field's own type, independent of the
/// caller's usual traversal. Serde only lets a flattened field serialize as a
/// struct or map (or an `Option`/reference resolving to one); anything else
/// is a runtime `"can only flatten structs and maps"` error from serde_json,
/// so we surface it at export time instead.
fn validate_flatten_field(
    field: &Field,
    ty: &DataType,
    types: &Types,
    path: &str,
    mode: ApplyMode,
    container: FlattenDirections,
    env: Option<&GenericEnv<'_>>,
) -> Result<(), Error> {
    let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(());
    };

    if !serde_attrs.flatten {
        return Ok(());
    }

    // A skipped direction never flattens, so its shape can't cause a flatten
    // error however non-flattenable it is (verified against serde_json 1.x).
    // Full `#[serde(skip)]` erases the field's type entirely and never
    // reaches this check; one-sided skips keep the type but kill their
    // direction. Under `PhasesFormat` the skipped phase drops the field
    // before flatten lowering; unified mode drops the field for a skip in
    // *either* direction (see `should_skip_field_for_mode` in `lib.rs`), so
    // any skip means nothing flattens there. The enclosing container's
    // liveness (an enum variant's own one-sided skips - see
    // `filter_enum_variants_for_phase`) ANDs in: a phase the variant doesn't
    // exist in can't flatten any of its fields.
    let directions = FlattenDirections {
        serialize: container.serialize && !serde_attrs.skip_serializing,
        deserialize: container.deserialize && !serde_attrs.skip_deserializing,
    };

    let live = match mode {
        ApplyMode::Unified => directions.serialize && directions.deserialize,
        ApplyMode::Phases => directions.serialize || directions.deserialize,
    };
    if !live {
        return Ok(());
    }

    validate_flatten_target(ty, types, path, &mut HashSet::new(), env, directions)
}

/// Which directions of a flattened field actually hit the wire. A direction
/// killed by a one-sided serde skip doesn't need a flattenable shape, which
/// matters wherever the two directions' shapes can diverge: explicit
/// [`PhasedTy`] overrides and container conversion targets.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FlattenDirections {
    serialize: bool,
    deserialize: bool,
}

impl FlattenDirections {
    /// Both directions hit the wire (a plain struct field's container).
    const LIVE: Self = Self {
        serialize: true,
        deserialize: true,
    };

    /// The single direction a per-phase rendering serves:
    /// `PhasesFormat::map_type` selects and rewrites one phase of an inline
    /// datatype, so only that phase's flatten shapes are reachable there.
    fn for_phase(phase: Phase) -> Self {
        match phase {
            Phase::Serialize => Self::SERIALIZE_ONLY,
            Phase::Deserialize => Self::DESERIALIZE_ONLY,
        }
    }

    /// Both liveness constraints apply (e.g. a variant's own skips *and* the
    /// walk's root phase).
    fn and(self, other: Self) -> Self {
        Self {
            serialize: self.serialize && other.serialize,
            deserialize: self.deserialize && other.deserialize,
        }
    }

    /// Only the serialize direction is reachable (an `into` conversion wire,
    /// or a `Phased` override's serialize side).
    const SERIALIZE_ONLY: Self = Self {
        serialize: true,
        deserialize: false,
    };

    /// Only the deserialize direction is reachable (a `from`/`try_from`
    /// conversion wire, or a `Phased` override's deserialize side).
    const DESERIALIZE_ONLY: Self = Self {
        serialize: false,
        deserialize: true,
    };

    /// The directions in which a field exists on the wire: a one-sided serde
    /// skip removes it from that phase before the rewrite ever visits its
    /// type (`should_skip_field_for_mode` in `lib.rs`), so nothing reached
    /// *through* the field can flatten there. (Fully skipped fields carry no
    /// type and are never walked at all.)
    fn for_field(field: &Field) -> Result<Self, Error> {
        if field
            .attributes
            .contains_key(crate::SERDE_NEWTYPE_SKIP_IGNORED)
        {
            return Ok(Self::LIVE);
        }
        let attrs = SerdeFieldAttrs::from_attributes(&field.attributes)?;
        Ok(Self {
            serialize: !attrs.as_ref().is_some_and(|attrs| attrs.skip_serializing),
            deserialize: !attrs.as_ref().is_some_and(|attrs| attrs.skip_deserializing),
        })
    }

    /// The directions in which an enum variant exists on the wire: a fully
    /// skipped variant exists in neither, and a one-sided variant skip drops
    /// it from that phase (`filter_enum_variants_for_phase` in `lib.rs`).
    fn for_variant(variant: &Variant) -> Result<Self, Error> {
        if variant.skip {
            return Ok(Self {
                serialize: false,
                deserialize: false,
            });
        }

        let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        Ok(Self {
            serialize: !attrs.as_ref().is_some_and(|attrs| attrs.skip_serializing),
            deserialize: !attrs.as_ref().is_some_and(|attrs| attrs.skip_deserializing),
        })
    }
}

/// Lexically scoped substitution environment for resolving
/// [`DataType::Generic`] placeholders while checking a flatten target.
///
/// Each time the main [`inner`] walk or [`validate_flatten_target`] follows a
/// named reference that carries concrete generic arguments, it pushes a new
/// frame mapping the definition's parameters to those arguments (so a flatten
/// field declared *inside* a generic definition is checked against each
/// instantiation it's reached through). The arguments themselves were
/// written one scope up, so when a placeholder resolves to an argument, the
/// argument is validated against this frame's `parent`. Substitution is lazy
/// (no datatype is ever rewritten), which keeps the `seen` keys syntactic and
/// guarantees termination even for non-regular recursive generics like
/// `struct N<T>(Box<N<Option<T>>>)`.
struct GenericEnv<'a> {
    map: &'a [(Generic, DataType)],
    parent: Option<&'a GenericEnv<'a>>,
}

/// Upper bound on the number of datatype nodes materialized while resolving a
/// reference's generic arguments for a memo key. Regular type graphs resolve
/// in a handful of nodes; only a hand-built non-regular generic (e.g.
/// `N<T>` whose body names `N<Option<T>>` - the derive can't produce one,
/// rustc fails to monomorphize `Type` for it) grows without bound, and the
/// budget converts that into a fallback to the syntactic key so the walk
/// still terminates.
const RESOLVED_KEY_NODE_BUDGET: usize = 256;

/// Builds the `checked_references` memo key for a named reference: the
/// reference with its generic arguments resolved through `env`, so the same
/// syntactic reference reached under different substitutions gets distinct
/// keys (and identical substitutions still dedupe). Falls back to the
/// syntactic reference when there is nothing to resolve or the node budget is
/// exhausted.
///
/// Termination: every key is either a syntactic reference occurring in the
/// finite type graph, or a reference whose resolved arguments are trees of at
/// most [`RESOLVED_KEY_NODE_BUDGET`] nodes built from clones of graph nodes -
/// a finite set either way, so a walk deduped on these keys terminates.
fn resolved_reference_key(reference: &NamedReference, env: Option<&GenericEnv<'_>>) -> Reference {
    let mut resolved = reference.clone();

    if env.is_some() {
        let mut budget = RESOLVED_KEY_NODE_BUDGET;
        let ok = match &mut resolved.inner {
            NamedReferenceType::Reference { generics, .. } => generics
                .iter_mut()
                .all(|(_, dt)| resolve_generics_for_key(dt, env, &mut budget)),
            // An `#[specta(inline)]` reference carries its instantiation
            // inside the inline datatype itself (as unresolved `Generic`
            // placeholders when it sits in another generic definition), so
            // the key must resolve that content too or two instantiations of
            // the same inline reference would collide.
            NamedReferenceType::Inline { dt, .. } => resolve_generics_for_key(dt, env, &mut budget),
            NamedReferenceType::Recursive(_) => true,
        };

        if !ok {
            return Reference::Named(reference.clone());
        }
    }

    Reference::Named(resolved)
}

/// Resolves [`DataType::Generic`] placeholders in `dt` (in place) through the
/// substitution environment, for memo-key purposes. Returns `false` when the
/// node budget runs out, in which case the caller discards the partial result
/// and falls back to the syntactic key. Datatypes stored inside attributes
/// (resolved conversion targets) are not rewritten; a key collision there
/// would only re-skip a validation, never mis-validate.
fn resolve_generics_for_key(
    dt: &mut DataType,
    env: Option<&GenericEnv<'_>>,
    budget: &mut usize,
) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;

    match dt {
        DataType::Generic(generic) => {
            let Some((arg, parent)) = env.and_then(|env| {
                env.map
                    .iter()
                    .find(|(param, _)| param == generic)
                    .map(|(_, arg)| (arg, env.parent))
            }) else {
                return true;
            };

            // The argument was written one scope up, so it resolves against
            // the parent environment.
            let mut resolved = arg.clone();
            if !resolve_generics_for_key(&mut resolved, parent, budget) {
                return false;
            }
            *dt = resolved;
            true
        }
        DataType::Nullable(inner) => resolve_generics_for_key(inner, env, budget),
        DataType::List(list) => resolve_generics_for_key(&mut list.ty, env, budget),
        DataType::Map(map) => {
            resolve_generics_for_key(map.key_ty_mut(), env, budget)
                && resolve_generics_for_key(map.value_ty_mut(), env, budget)
        }
        DataType::Tuple(tuple) => tuple
            .elements
            .iter_mut()
            .all(|element| resolve_generics_for_key(element, env, budget)),
        DataType::Intersection(parts) => parts
            .iter_mut()
            .all(|part| resolve_generics_for_key(part, env, budget)),
        DataType::Struct(strct) => resolve_fields_generics_for_key(&mut strct.fields, env, budget),
        DataType::Enum(enm) => enm
            .variants
            .iter_mut()
            .all(|(_, variant)| resolve_fields_generics_for_key(&mut variant.fields, env, budget)),
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Reference { generics, .. } => generics
                .iter_mut()
                .all(|(_, dt)| resolve_generics_for_key(dt, env, budget)),
            NamedReferenceType::Inline { dt, .. } => resolve_generics_for_key(dt, env, budget),
            NamedReferenceType::Recursive(_) => true,
        },
        // A `Phased` override wraps datatypes that may themselves contain
        // `Generic` placeholders (e.g. `Phased<T, Inner>` used as a generic
        // argument), so its contents must be resolved for the key too; a
        // rebuilt opaque with equal contents still compares/hashes equal.
        DataType::Reference(Reference::Opaque(reference)) => {
            match reference.downcast_ref::<PhasedTy>() {
                Some(phased) => {
                    let mut serialize = phased.serialize.clone();
                    let mut deserialize = phased.deserialize.clone();
                    if !resolve_generics_for_key(&mut serialize, env, budget)
                        || !resolve_generics_for_key(&mut deserialize, env, budget)
                    {
                        return false;
                    }
                    *dt = DataType::Reference(Reference::opaque(PhasedTy {
                        serialize,
                        deserialize,
                    }));
                    true
                }
                // Other opaque references are exporter-specific leaves.
                None => true,
            }
        }
        DataType::Primitive(_) => true,
    }
}

fn resolve_fields_generics_for_key(
    fields: &mut Fields,
    env: Option<&GenericEnv<'_>>,
    budget: &mut usize,
) -> bool {
    match fields {
        Fields::Unit => true,
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .iter_mut()
            .filter_map(|field| field.ty.as_mut())
            .all(|ty| resolve_generics_for_key(ty, env, budget)),
        Fields::Named(named) => named
            .fields
            .iter_mut()
            .filter_map(|(_, field)| field.ty.as_mut())
            .all(|ty| resolve_generics_for_key(ty, env, budget)),
    }
}

/// `seen` tracks named references currently being resolved on this flatten
/// check so a cyclic type graph terminates instead of recursing forever. This
/// mirrors the `seen`/`checked_references` discipline used elsewhere in this
/// module, but is scoped to a single field's flatten check rather than shared
/// with the outer traversal, since a reference already validated elsewhere
/// still needs its shape checked here.
fn validate_flatten_target(
    ty: &DataType,
    types: &Types,
    path: &str,
    seen: &mut HashSet<Reference>,
    env: Option<&GenericEnv<'_>>,
    directions: FlattenDirections,
) -> Result<(), Error> {
    match ty {
        // Maps merge directly.
        DataType::Map(_) => Ok(()),
        // Enums are accepted for all four serde representations: external,
        // internal, and adjacent tagging always produce map-shaped output
        // when flattened (verified against serde_json 1.x - even an
        // externally tagged unit variant flattens as `"Variant": null`). An
        // untagged enum only flattens when the active variant's payload is
        // map-shaped, but rejecting it statically would break the common
        // pattern of flattening `serde_json::Value` (an untagged enum) for
        // extra fields, so it is deliberately accepted. Container
        // conversions still substitute the wire type, though, so any
        // declared conversion targets are checked instead.
        DataType::Enum(enm) => {
            let attrs = SerdeContainerAttrs::from_attributes(&enm.attributes)?;
            if let Some((serialize_wire, deserialize_wire)) =
                conversion_wire_targets(attrs.as_ref())
            {
                validate_conversion_wires(
                    serialize_wire,
                    deserialize_wire,
                    types,
                    path,
                    seen,
                    env,
                    directions,
                )?;
            }

            Ok(())
        }
        // An intersection is a structural merge of object-like types (see
        // `DataType::Intersection`'s docs), so it is map-shaped by
        // construction. It can't come out of a derive - specta-serde only
        // produces it *after* validation, when lowering flatten itself - but
        // a manually built one must not be false-positived on.
        DataType::Intersection(_) => Ok(()),
        // A flattened `Option<T>` (including nested `Option<Option<T>>`)
        // contributes nothing when absent and validates as `T` when present.
        DataType::Nullable(inner) => {
            validate_flatten_target(inner, types, path, seen, env, directions)
        }
        // `()` and other zero-element tuples serialize as nothing, so
        // flattening one is a harmless no-op (verified against serde_json).
        DataType::Tuple(tuple) if tuple.elements.is_empty() => Ok(()),
        DataType::Struct(strct) => {
            let attrs = SerdeContainerAttrs::from_attributes(&strct.attributes)?;

            // `#[serde(into/from/try_from)]` substitute the serde *wire* type
            // for the declared shape, per direction, so the wire types are
            // what serde actually flattens - and only for directions that are
            // actually live. A live direction without a conversion keeps the
            // declared shape, so the declared-shape checks below run for
            // exactly the remaining uncovered live directions. (Unified mode
            // separately rejects one-sided conversions during the rewrite -
            // see `select_conversion_target` in lib.rs - which this mirrors.)
            let directions = match conversion_wire_targets(attrs.as_ref()) {
                Some((serialize_wire, deserialize_wire)) => {
                    let remaining = validate_conversion_wires(
                        serialize_wire,
                        deserialize_wire,
                        types,
                        path,
                        seen,
                        env,
                        directions,
                    )?;

                    if !remaining.serialize && !remaining.deserialize {
                        return Ok(());
                    }

                    remaining
                }
                None => directions,
            };

            // `#[serde(transparent)]` structs delegate (de)serialization
            // straight to their single non-skipped field, so serde sees that
            // field's shape - not a struct/tuple wrapper - when flattened.
            if attrs.is_some_and(|attrs| attrs.transparent) {
                let inner_fields = match &strct.fields {
                    Fields::Unit => return Ok(()),
                    Fields::Unnamed(unnamed) => unnamed
                        .fields
                        .iter()
                        .filter_map(|f| f.ty.as_ref())
                        .collect::<Vec<_>>(),
                    Fields::Named(named) => named
                        .fields
                        .iter()
                        .filter_map(|(_, f)| f.ty.as_ref())
                        .collect::<Vec<_>>(),
                };

                return match inner_fields.as_slice() {
                    // Not exactly one live field: leave it be rather than
                    // false-positive on a shape we can't confidently resolve.
                    [only] => validate_flatten_target(only, types, path, seen, env, directions),
                    _ => Ok(()),
                };
            }

            match &strct.fields {
                Fields::Unit | Fields::Named(_) => Ok(()),
                Fields::Unnamed(unnamed) => match unnamed.fields.as_slice() {
                    // A tuple struct with exactly one declared field is a
                    // newtype: serde delegates (de)serialization straight to
                    // the inner value (no `#[serde(transparent)]` needed), so
                    // the flatten target is the inner value's shape.
                    [single] => match &single.ty {
                        Some(ty) => validate_flatten_target(ty, types, path, seen, env, directions),
                        // The field is skipped, so its type is unknowable
                        // here (serde still delegates to it when
                        // serializing); don't false-positive.
                        None => Ok(()),
                    },
                    // Zero or multiple declared fields serialize as a
                    // sequence (`[]` / `[a, b]`), even when skips leave one
                    // live field - verified against serde_json 1.x.
                    _ => Err(Error::invalid_flatten_target(
                        path.to_string(),
                        "tuple structs serialize as a sequence; serde can only flatten structs and maps",
                    )),
                },
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            let key = Reference::Named(reference.clone());
            if !seen.insert(key.clone()) {
                return Ok(());
            }

            let result = match &reference.inner {
                // Inline datatypes were written at the use site, so they
                // resolve generic placeholders against the current scope.
                NamedReferenceType::Inline { dt, .. } => {
                    validate_flatten_target(dt, types, path, seen, env, directions)
                }
                // The resolved definition's placeholders resolve against this
                // reference's concrete arguments, which themselves were
                // written in the current scope - hence `parent: env`.
                NamedReferenceType::Reference { generics, .. } => types
                    .get(reference)
                    .and_then(|ndt| ndt.ty.as_ref())
                    .map_or(Ok(()), |ty| {
                        validate_flatten_target(
                            ty,
                            types,
                            path,
                            seen,
                            Some(&GenericEnv {
                                map: generics,
                                parent: env,
                            }),
                            directions,
                        )
                    }),
                // A recursive-inline marker is a cycle we've already checked.
                NamedReferenceType::Recursive(_) => Ok(()),
            };
            seen.remove(&key);

            result
        }
        // A generic placeholder is knowable whenever the enclosing definition
        // was reached through a reference carrying concrete arguments:
        // substitute and validate the argument in the scope it was written
        // in. A bare placeholder (validating the generic definition itself)
        // stays accepted - serde allows it whenever the instantiation is
        // struct/map-shaped, so rejecting it would be a false positive.
        DataType::Generic(generic) => {
            match env.and_then(|env| env.map.iter().find(|(param, _)| param == generic)) {
                Some((_, arg)) => validate_flatten_target(
                    arg,
                    types,
                    path,
                    seen,
                    env.and_then(|env| env.parent),
                    directions,
                ),
                None => Ok(()),
            }
        }
        DataType::Reference(Reference::Opaque(reference)) => {
            // An explicit `specta_serde::Phased<Serialize, Deserialize>`
            // override is stored as an opaque reference wrapping both phase
            // shapes; each *live* phase must be a valid flatten target. A
            // phase killed by a one-sided skip never flattens (`PhasesFormat`
            // drops the field from that phase before flatten lowering), so
            // its shape is unreachable and must not be validated. Each side
            // is also only ever exported/rewritten *for its own phase*, so it
            // is validated in that direction alone: e.g. the serialize
            // shape's deserialize-facing details (a raw shape behind an
            // `into`-only conversion, a `from` wire) are unreachable.
            match reference.downcast_ref::<PhasedTy>() {
                Some(phased) => {
                    if directions.serialize {
                        validate_flatten_target(
                            &phased.serialize,
                            types,
                            path,
                            seen,
                            env,
                            FlattenDirections::SERIALIZE_ONLY,
                        )?;
                    }
                    if directions.deserialize {
                        validate_flatten_target(
                            &phased.deserialize,
                            types,
                            path,
                            seen,
                            env,
                            FlattenDirections::DESERIALIZE_ONLY,
                        )?;
                    }

                    Ok(())
                }
                // Any other opaque reference is exporter-specific; its shape
                // is unknowable here, so don't false-positive on it.
                None => Ok(()),
            }
        }
        DataType::List(_) | DataType::Tuple(_) | DataType::Primitive(_) => {
            Err(Error::invalid_flatten_target(
                path.to_string(),
                "serde can only flatten structs and maps, but this field's type serializes as a sequence, tuple, or scalar value",
            ))
        }
    }
}

/// Validates the reachable conversion wire types, each in its own direction
/// only: an `into` wire is only ever serialized and a `from`/`try_from` wire
/// only ever deserialized (`select_conversion_target` in `lib.rs` picks by
/// phase), so e.g. a serialize wire's own deserialize-only conversions are
/// unreachable. A wire for a direction that isn't live is skipped entirely.
///
/// Returns the live directions left *uncovered* by a wire - the ones the
/// caller's declared shape still serves.
fn validate_conversion_wires(
    serialize_wire: Option<&DataType>,
    deserialize_wire: Option<&DataType>,
    types: &Types,
    path: &str,
    seen: &mut HashSet<Reference>,
    env: Option<&GenericEnv<'_>>,
    directions: FlattenDirections,
) -> Result<FlattenDirections, Error> {
    if directions.serialize
        && let Some(wire) = serialize_wire
    {
        validate_flatten_target(
            wire,
            types,
            path,
            seen,
            env,
            FlattenDirections::SERIALIZE_ONLY,
        )?;
    }
    if directions.deserialize
        && let Some(wire) = deserialize_wire
    {
        validate_flatten_target(
            wire,
            types,
            path,
            seen,
            env,
            FlattenDirections::DESERIALIZE_ONLY,
        )?;
    }

    Ok(FlattenDirections {
        serialize: directions.serialize && serialize_wire.is_none(),
        deserialize: directions.deserialize && deserialize_wire.is_none(),
    })
}

/// The serde wire types substituted by container conversions, as
/// `(serialize, deserialize)` targets: `#[serde(into = ...)]` replaces the
/// serialize shape and `#[serde(from/try_from = ...)]` the deserialize shape.
/// Returns `None` when the container declares no conversions. Mirrors the
/// per-direction selection in `select_conversion_target` (`lib.rs`).
fn conversion_wire_targets(
    attrs: Option<&SerdeContainerAttrs>,
) -> Option<(Option<&DataType>, Option<&DataType>)> {
    let attrs = attrs?;
    let serialize = attrs.resolved_into.as_ref();
    let deserialize = attrs
        .resolved_from
        .as_ref()
        .or(attrs.resolved_try_from.as_ref());

    (serialize.is_some() || deserialize.is_some()).then_some((serialize, deserialize))
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

fn validate_enum(
    enm: &Enum,
    types: &Types,
    path: String,
    mode: ApplyMode,
    declared_deserializes: bool,
    env: Option<&GenericEnv<'_>>,
) -> Result<(), Error> {
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
    validate_other_variant(enm, &path, &repr, mode, declared_deserializes)?;
    validate_adjacent_collapsed_newtype_variants(enm, &path, &repr, mode)?;

    if matches!(repr, EnumRepr::Internal { .. }) {
        validate_internally_tag_enum(enm, types, path, &mut HashSet::new(), env)?;
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
    declared_deserializes: bool,
) -> Result<(), Error> {
    let mut other_variants = Vec::new();
    for (name, variant) in &enm.variants {
        if variant_is_rendered(variant)?
            && SerdeVariantAttrs::from_attributes(&variant.attributes)?
                .is_some_and(|attrs| attrs.other)
        {
            other_variants.push((name, variant));
        }
    }

    if other_variants.is_empty() {
        return Ok(());
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

    if mode == ApplyMode::Unified
        && declared_deserializes
        && !externally_tagged_unit_enum(enm, repr)?
    {
        return Err(Error::invalid_phased_type_usage(
            path,
            "`#[serde(other)]` requires `PhasesFormat` because a shared tag type cannot exclude known variants soundly",
        ));
    }

    Ok(())
}

fn externally_tagged_unit_enum(enm: &Enum, repr: &EnumRepr) -> Result<bool, Error> {
    if !matches!(repr, EnumRepr::External) {
        return Ok(false);
    }

    for (_, variant) in &enm.variants {
        let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if attrs
            .as_ref()
            .is_some_and(|attrs| attrs.skip_serializing && attrs.skip_deserializing)
        {
            continue;
        }

        if !matches!(variant.fields, Fields::Unit)
            || attrs.as_ref().is_some_and(|attrs| attrs.untagged)
        {
            return Ok(false);
        }
    }

    Ok(true)
}

fn validate_internally_tag_enum(
    enm: &Enum,
    types: &Types,
    path: String,
    seen: &mut HashSet<Reference>,
    env: Option<&GenericEnv<'_>>,
) -> Result<(), Error> {
    for (variant_name, variant) in &enm.variants {
        validate_internally_tag_variant(enm, variant_name, variant, types, &path, seen, env)?;
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
    env: Option<&GenericEnv<'_>>,
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

            validate_internally_tag_enum_datatype(
                first_field,
                types,
                path,
                variant_name,
                seen,
                env,
                true,
            )
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
    env: Option<&GenericEnv<'_>>,
    reject_contextual_generic_argument: bool,
) -> Result<(), Error> {
    match ty {
        DataType::Map(_) => Ok(()),
        DataType::Struct(strct)
            if SerdeContainerAttrs::from_attributes(&strct.attributes)?
                .is_some_and(|attrs| attrs.transparent) =>
        {
            let mut live_fields = Vec::new();
            match &strct.fields {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    for field in &unnamed.fields {
                        let attrs = SerdeFieldAttrs::from_attributes(&field.attributes)?;
                        if attrs
                            .as_ref()
                            .is_some_and(|attrs| attrs.skip_serializing || attrs.skip_deserializing)
                        {
                            continue;
                        }
                        if let Some(ty) = &field.ty {
                            live_fields.push(ty);
                        }
                    }
                }
                Fields::Named(named) => {
                    for (_, field) in &named.fields {
                        let attrs = SerdeFieldAttrs::from_attributes(&field.attributes)?;
                        if attrs
                            .as_ref()
                            .is_some_and(|attrs| attrs.skip_serializing || attrs.skip_deserializing)
                        {
                            continue;
                        }
                        if let Some(ty) = &field.ty {
                            live_fields.push(ty);
                        }
                    }
                }
            }

            match live_fields.as_slice() {
                [ty] => validate_internally_tag_enum_datatype(
                    ty,
                    types,
                    path,
                    variant_name,
                    seen,
                    env,
                    reject_contextual_generic_argument,
                ),
                [] => Ok(()),
                _ => Err(Error::invalid_internally_tagged_enum(
                    path,
                    variant_name,
                    "payload cannot be merged with an internal tag",
                )),
            }
        }
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit | Fields::Named(_) => Ok(()),
            Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                unnamed.fields[0].ty.as_ref().map_or(Ok(()), |ty| {
                    validate_internally_tag_enum_datatype(
                        ty,
                        types,
                        path,
                        variant_name,
                        seen,
                        env,
                        reject_contextual_generic_argument,
                    )
                })
            }
            Fields::Unnamed(_) => Err(Error::invalid_internally_tagged_enum(
                path,
                variant_name,
                "payload cannot be merged with an internal tag",
            )),
        },
        DataType::Reference(Reference::Named(reference)) => {
            let key = resolved_reference_key(reference, env);
            if !seen.insert(key.clone()) {
                return Ok(());
            }

            let path = inner_reference_path(path, reference, types);
            let result = match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    let reject_contextual_generic_argument =
                        reject_contextual_generic_argument && datatype_changes_under_env(dt, env);
                    validate_internally_tag_enum_datatype(
                        dt,
                        types,
                        &path,
                        variant_name,
                        seen,
                        env,
                        reject_contextual_generic_argument,
                    )
                }
                NamedReferenceType::Reference { generics, .. } => {
                    let reject_contextual_generic_argument = reject_contextual_generic_argument
                        && generics
                            .iter()
                            .any(|(_, argument)| datatype_changes_under_env(argument, env));
                    types
                        .get(reference)
                        .and_then(|ndt| ndt.ty.as_ref())
                        .map_or(Ok(()), |ty| {
                            validate_internally_tag_enum_datatype(
                                ty,
                                types,
                                &path,
                                variant_name,
                                seen,
                                Some(&GenericEnv {
                                    map: generics,
                                    parent: env,
                                }),
                                reject_contextual_generic_argument,
                            )
                        })
                }
                NamedReferenceType::Recursive(_) => Ok(()),
            };
            seen.remove(&key);

            result
        }
        DataType::Enum(enm) => match EnumRepr::from_attrs(&enm.attributes)? {
            EnumRepr::Untagged => {
                validate_internally_tag_enum(enm, types, path.to_string(), seen, env)
            }
            EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } => Ok(()),
        },
        DataType::Tuple(tuple) if tuple.elements.is_empty() => Ok(()),
        DataType::Generic(generic) => {
            match env.and_then(|env| env.map.iter().find(|(param, _)| param == generic)) {
                Some((_, argument)) => {
                    if reject_contextual_generic_argument
                        && crate::internal_tag_payload_requires_contextual_rewrite(argument, types)?
                    {
                        return Err(Error::invalid_internally_tagged_enum(
                            path,
                            variant_name,
                            "a concrete generic payload requires context-sensitive enum encoding; use a non-generic wrapper variant",
                        ));
                    }

                    validate_internally_tag_enum_datatype(
                        argument,
                        types,
                        path,
                        variant_name,
                        seen,
                        env.and_then(|env| env.parent),
                        reject_contextual_generic_argument,
                    )
                }
                None => Ok(()),
            }
        }
        DataType::Reference(Reference::Opaque(_))
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

fn datatype_changes_under_env(ty: &DataType, env: Option<&GenericEnv<'_>>) -> bool {
    let mut resolved = ty.clone();
    let mut budget = RESOLVED_KEY_NODE_BUDGET;
    resolve_generics_for_key(&mut resolved, env, &mut budget) && resolved != *ty
}

/// Extends `path` with the name of the type behind a named reference, so an
/// error surfaced while validating what that reference resolves to (for
/// example an untagged enum nested inside an internally tagged variant's
/// payload) names the actual inner type instead of repeating the outer
/// container's path, which would misattribute the error (e.g. blaming a
/// variant that doesn't exist on the outer enum).
fn inner_reference_path(path: &str, reference: &NamedReference, types: &Types) -> String {
    match types.get(reference) {
        Some(ndt) => format!("{path} -> {}", ndt.name),
        None => path.to_string(),
    }
}

fn named_reference_ty<'a>(reference: &'a NamedReference, types: &'a Types) -> Option<&'a DataType> {
    match &reference.inner {
        NamedReferenceType::Inline { dt, .. } => Some(dt),
        NamedReferenceType::Reference { .. } => types.get(reference)?.ty.as_ref(),
        NamedReferenceType::Recursive(_) => None,
    }
}
