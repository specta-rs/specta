use std::collections::HashSet;

use specta::{
    Types,
    datatype::{
        DataType, Enum, Field, Fields, Generic, NamedReference, NamedReferenceType, Reference,
        Variant,
    },
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
                    for (idx, (field, _)) in unnamed
                        .fields
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, field)| field.ty.as_ref().map(|ty| (idx, (field, ty))))
                    {
                        validate_field_attributes(field, format!("{path}[{idx}]"), mode)?;
                    }
                    for (idx, (_, ty)) in unnamed
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
                        )?;
                    }
                }
                Fields::Named(named) => {
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
                            FlattenDirections::LIVE,
                        )?;
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
                let variant_directions = FlattenDirections::for_variant(variant)?;
                match &variant.fields {
                    Fields::Unit => {}
                    Fields::Named(named) => {
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
                        for (idx, (_, ty)) in
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

    validate_flatten_target(ty, types, path, &mut HashSet::new(), None, directions)
}

/// Which directions of a flattened field actually hit the wire. A direction
/// killed by a one-sided serde skip doesn't need a flattenable shape, which
/// matters wherever the two directions' shapes can diverge: explicit
/// [`PhasedTy`] overrides and container conversion targets.
#[derive(Clone, Copy)]
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
/// Each time [`validate_flatten_target`] follows a named reference that
/// carries concrete generic arguments, it pushes a new frame mapping the
/// definition's parameters to those arguments. The arguments themselves were
/// written one scope up, so when a placeholder resolves to an argument, the
/// argument is validated against this frame's `parent`. Substitution is lazy
/// (no datatype is ever rewritten), which keeps the `seen` keys syntactic and
/// guarantees termination even for non-regular recursive generics like
/// `struct N<T>(Box<N<Option<T>>>)`.
struct GenericEnv<'a> {
    map: &'a [(Generic, DataType)],
    parent: Option<&'a GenericEnv<'a>>,
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
        validate_internally_tag_enum(enm, types, path, &mut HashSet::new())?;
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
                let path = inner_reference_path(path, reference, types);
                validate_internally_tag_enum_datatype(ty, types, &path, variant_name, seen)
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
