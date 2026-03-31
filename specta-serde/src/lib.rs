//! [Serde](https://serde.rs) support for Specta.
//!
//! # Choosing a mode
//!
//! - Use [`apply`] when serde behavior is symmetric and a single exported shape
//!   should work for both serialization and deserialization.
//! - Use [`apply_phases`] when serde behavior differs by direction (for example
//!   deserialize-widening enums, asymmetric conversion attributes, or explicit
//!   [`Phased`] overrides).
//!
//! # `serde_with` and `#[serde(with = ...)]`
//!
//! `serde_with` is supported through the same mechanism as raw serde codec
//! attributes because it expands to serde metadata (`with`, `serialize_with`,
//! `deserialize_with`).
//!
//! When codecs change the wire type, add an explicit Specta override:
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use specta::Type;
//!
//! #[derive(Type, Serialize, Deserialize)]
//! struct Digest {
//!     #[serde(with = "hex_bytes")]
//!     #[specta(type = String)]
//!     value: Vec<u8>,
//! }
//! ```
//!
//! If serialize and deserialize shapes are different, use [`Phased`] and
//! [`apply_phases`].
//!
//! This is required because a single unified type graph cannot represent two
//! different directional wire shapes at once.
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use serde_with::{OneOrMany, serde_as};
//! use specta::{Type, Types};
//!
//! #[derive(Type, Serialize, Deserialize)]
//! #[serde(untagged)]
//! enum OneOrManyString {
//!     One(String),
//!     Many(Vec<String>),
//! }
//!
//! #[serde_as]
//! #[derive(Type, Serialize, Deserialize)]
//! struct Filters {
//!     #[serde_as(as = "OneOrMany<_>")]
//!     #[specta(type = specta_serde::Phased<Vec<String>, OneOrManyString>)]
//!     tags: Vec<String>,
//! }
//!
//! let types = Types::default().register::<Filters>();
//! let phased_types = specta_serde::apply_phases(types)?;
//! ```
//!
//! As an alternative to codec attributes, `#[serde(into = ...)]`,
//! `#[serde(from = ...)]`, and `#[serde(try_from = ...)]` often produce better
//! type inference because the wire type is modeled as an explicit Rust type:
//!
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use specta::Type;
//!
//! #[derive(Type, Serialize, Deserialize)]
//! struct UserWire {
//!     id: String,
//! }
//!
//! #[derive(Type, Clone, Serialize, Deserialize)]
//! #[serde(into = "UserWire")]
//! struct UserInto {
//!     id: String,
//! }
//!
//! #[derive(Type, Clone, Serialize, Deserialize)]
//! #[serde(from = "UserWire")]
//! struct UserFrom {
//!     id: String,
//! }
//!
//! #[derive(Type, Clone, Serialize, Deserialize)]
//! #[serde(try_from = "UserWire")]
//! struct UserTryFrom {
//!     id: String,
//! }
//! ```
//!
//! See `examples/basic-ts/src/main.rs` for a complete exporter example using
//! [`apply`] and [`apply_phases`].
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
};

use specta::{
    ResolvedTypes, Types,
    datatype::{
        DataType, Enum, Field, Fields, NamedDataType, Primitive, Reference, Struct, Tuple,
        UnnamedFields, Variant,
    },
};

mod error;
mod inflection;
mod parser;
mod phased;
mod repr;
mod validate;

use inflection::RenameRule;
use parser::{SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs};
use phased::PhasedTy;

pub use error::Error;
pub use phased::{Phased, phased};

/// Selects which directional type shape to use after [`apply_phases`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The shape used when Rust serializes data to the wire.
    Serialize,
    /// The shape used when Rust deserializes data from the wire.
    Deserialize,
}

#[doc(hidden)]
pub mod internal {
    pub use crate::error::Result;
    pub use crate::inflection::RenameRule;
    pub use crate::parser::{
        ConversionType, SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs,
    };
}

use error::Result;
use repr::EnumRepr;

/// Validates whether a given [`DataType`] is a valid Serde-type.
///
/// When using [`apply`]/[`apply_phases`] all [`NamedDataType`]s are validated automatically, however if you need to export a [`DataType`] directly this is required to validate the top-level type.
///
/// For example if you try and export `HashMap<InvalidKey, MyGenericType<()>>`, [`apply`]/[`apply_phases`] can validate `MyGenericType` but it doesn't see the top-level `HashMap`'s generics so it can't validate them.
///
/// This is *only* required if your using the primitives from your language exporter.
pub fn validate(dt: &DataType, types: &ResolvedTypes) -> Result<()> {
    validate::validate_datatype_for_mode(dt, types.as_types(), validate::ApplyMode::Unified)
}

/// Applies serde transformations in unified mode.
///
/// Unified mode produces a single transformed type graph that must satisfy both
/// serialization and deserialization behavior. This is the simplest mode and is
/// usually what exporters want when serde behavior is symmetric.
///
/// Returns [`ResolvedTypes`] because serde rewrites may
/// alter type shapes.
///
/// Returns an [`Error`] when serde metadata introduces phase-only differences
/// that cannot be represented as one shape (for example `#[serde(other)]`,
/// identifier enums, asymmetric conversion attributes, `skip_serializing_if`,
/// or explicit [`Phased`] overrides).
///
/// Use [`apply_phases`] when your serialize and deserialize wire shapes differ.
pub fn apply(types: Types) -> Result<ResolvedTypes> {
    validate::validate_for_mode(&types, validate::ApplyMode::Unified)?;

    let mut out = types.clone();
    let generated = HashMap::<TypeIdentity, SplitGeneratedTypes>::new();
    let split_types = HashSet::<TypeIdentity>::new();
    let mut rewrite_err = None;

    out.iter_mut(|ndt| {
        if rewrite_err.is_some() {
            return;
        }

        let ndt_name = ndt.name().to_string();

        if let Err(err) = rewrite_datatype_for_phase(
            ndt.ty_mut(),
            PhaseRewrite::Unified,
            &types,
            &generated,
            &split_types,
            Some(ndt_name.as_str()),
        ) {
            rewrite_err = Some(err);
        }
    });

    if let Some(err) = rewrite_err {
        return Err(err);
    }

    Ok(ResolvedTypes::from_resolved_types(out))
}

/// Applies serde transformations in split-phase mode.
///
/// Phase mode preserves directional differences by rewriting affected named
/// types into paired `*_Serialize` and `*_Deserialize` types and then updating
/// references accordingly. This allows exporters to represent serde behavior
/// that is asymmetric between serialization and deserialization.
///
/// Returns [`ResolvedTypes`] because serde rewrites may
/// alter type shapes.
///
/// Use this when working with deserialize-widening attributes like
/// `#[serde(other)]`/identifier enums, asymmetric conversion attributes, or
/// explicit [`Phased`] overrides.
pub fn apply_phases(types: Types) -> Result<ResolvedTypes> {
    validate::validate_for_mode(&types, validate::ApplyMode::Phases)?;

    let originals = types.into_unsorted_iter().collect::<Vec<_>>();
    let mut dependencies = HashMap::<TypeIdentity, HashSet<TypeIdentity>>::new();
    let mut reverse_dependencies = HashMap::<TypeIdentity, HashSet<TypeIdentity>>::new();

    for original in &originals {
        let key = TypeIdentity::from_ndt(original);
        let mut deps = HashSet::new();
        collect_dependencies(original.ty(), &types, &mut deps)?;
        for dep in &deps {
            reverse_dependencies
                .entry(dep.clone())
                .or_default()
                .insert(key.clone());
        }
        dependencies.insert(key, deps);
    }

    let mut split_types = HashSet::new();
    for ndt in &originals {
        if has_local_phase_difference(ndt.ty())? {
            split_types.insert(TypeIdentity::from_ndt(ndt));
        }
    }

    let mut queue = VecDeque::from_iter(split_types.iter().cloned());
    while let Some(key) = queue.pop_front() {
        if let Some(dependents) = reverse_dependencies.get(&key) {
            for dependent in dependents {
                if split_types.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    let mut out = types.clone();
    let mut generated = HashMap::<TypeIdentity, SplitGeneratedTypes>::new();
    let mut generated_types = HashSet::<TypeIdentity>::new();

    for original in &originals {
        let key = TypeIdentity::from_ndt(original);

        if split_types.contains(&key) {
            let serialize_ndt = build_from_original(
                original,
                format!("{}_Serialize", original.name()),
                original.generics().to_vec(),
                original.ty().clone(),
                &types,
            );

            let deserialize_ndt = build_from_original(
                original,
                format!("{}_Deserialize", original.name()),
                original.generics().to_vec(),
                original.ty().clone(),
                &types,
            );

            generated.insert(
                key,
                SplitGeneratedTypes {
                    serialize: serialize_ndt,
                    deserialize: Box::new(deserialize_ndt),
                },
            );
        }
    }

    for original in &originals {
        let key = TypeIdentity::from_ndt(original);

        if !split_types.contains(&key) {
            continue;
        }

        let Some(mut generated_types_for_phase) = generated.get(&key).cloned() else {
            continue;
        };

        rewrite_datatype_for_phase(
            generated_types_for_phase.serialize.ty_mut(),
            PhaseRewrite::Serialize,
            &types,
            &generated,
            &split_types,
            Some(original.name().as_ref()),
        )?;

        rewrite_datatype_for_phase(
            generated_types_for_phase.deserialize.ty_mut(),
            PhaseRewrite::Deserialize,
            &types,
            &generated,
            &split_types,
            Some(original.name().as_ref()),
        )?;

        generated.insert(key, generated_types_for_phase);
    }

    for generated_types_for_phase in generated.values() {
        generated_types.insert(TypeIdentity::from_ndt(&generated_types_for_phase.serialize));
        generated_types.insert(TypeIdentity::from_ndt(
            &generated_types_for_phase.deserialize,
        ));
        generated_types_for_phase.serialize.register(&mut out);
        generated_types_for_phase.deserialize.register(&mut out);
    }

    let mut rewrite_err = None;
    out.iter_mut(|ndt| {
        if rewrite_err.is_some() {
            return;
        }

        let ndt_name = ndt.name().to_string();
        let key = TypeIdentity::from_ndt(ndt);

        if split_types.contains(&key) || generated_types.contains(&key) {
            return;
        }

        if let Err(err) = rewrite_datatype_for_phase(
            ndt.ty_mut(),
            PhaseRewrite::Unified,
            &types,
            &generated,
            &split_types,
            Some(ndt_name.as_str()),
        ) {
            rewrite_err = Some(err);
        }
    });

    if let Some(err) = rewrite_err {
        return Err(err);
    }

    out.iter_mut(|ndt| {
        let key = TypeIdentity::from_ndt(ndt);
        if !split_types.contains(&key) {
            return;
        }

        let Some(SplitGeneratedTypes {
            serialize,
            deserialize,
        }) = generated.get(&key)
        else {
            return;
        };

        let generic_args = ndt
            .generics()
            .iter()
            .map(|(generic, _)| (generic.clone(), generic.clone().into()))
            .collect::<Vec<_>>();

        let mut serialize_variant = Variant::unnamed().build();
        if let Fields::Unnamed(fields) = serialize_variant.fields_mut() {
            fields
                .fields_mut()
                .push(Field::new(serialize.reference(generic_args.clone()).into()));
        }

        let mut deserialize_variant = Variant::unnamed().build();
        if let Fields::Unnamed(fields) = deserialize_variant.fields_mut() {
            fields
                .fields_mut()
                .push(Field::new(deserialize.reference(generic_args).into()));
        }

        let mut wrapper = Enum::new();
        wrapper
            .variants_mut()
            .push((Cow::Borrowed("Serialize"), serialize_variant));
        wrapper
            .variants_mut()
            .push((Cow::Borrowed("Deserialize"), deserialize_variant));

        ndt.set_ty(DataType::Enum(wrapper));
    });
    Ok(ResolvedTypes::from_resolved_types(out))
}

/// Rewrites a [`DataType`] to the requested directional shape after [`apply_phases`].
///
/// This is useful for exporter integrations that need deserialize-specific input
/// types and serialize-specific output types while still exporting against the
/// resolved type graph returned by [`apply_phases`].
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use specta::{Type, Types, datatype::{DataType, Reference}};
/// use specta_serde::{Phase, Phased, apply_phases, select_phase_datatype};
///
/// #[derive(Type, Serialize, Deserialize)]
/// #[serde(untagged)]
/// enum OneOrManyString {
///     One(String),
///     Many(Vec<String>),
/// }
///
/// #[derive(Type, Serialize, Deserialize)]
/// struct Filters {
///     #[specta(type = Phased<Vec<String>, OneOrManyString>)]
///     tags: Vec<String>,
/// }
///
/// let mut types = Types::default();
/// let dt = Filters::definition(&mut types);
/// let resolved = apply_phases(types)?;
///
/// let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
/// let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);
///
/// let DataType::Reference(Reference::Named(serialize_reference)) = &serialize else {
///     panic!("expected named serialize reference");
/// };
/// let DataType::Reference(Reference::Named(deserialize_reference)) = &deserialize else {
///     panic!("expected named deserialize reference");
/// };
///
/// assert_eq!(
///     serialize_reference.get(resolved.as_types()).unwrap().name(),
///     "Filters_Serialize"
/// );
/// assert_eq!(
///     deserialize_reference.get(resolved.as_types()).unwrap().name(),
///     "Filters_Deserialize"
/// );
/// # Ok::<(), specta_serde::Error>(())
/// ```
pub fn select_phase_datatype(dt: &DataType, types: &ResolvedTypes, phase: Phase) -> DataType {
    let mut dt = dt.clone();
    select_phase_datatype_inner(&mut dt, types.as_types(), phase);
    dt
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseRewrite {
    Unified,
    Serialize,
    Deserialize,
}

fn select_phase_datatype_inner(ty: &mut DataType, types: &Types, phase: Phase) {
    if let Some(resolved) = select_explicit_phased_type(ty, phase) {
        *ty = resolved;
        select_phase_datatype_inner(ty, types, phase);
        return;
    }

    match ty {
        DataType::Struct(s) => select_phase_fields(s.fields_mut(), types, phase),
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                select_phase_fields(variant.fields_mut(), types, phase);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                select_phase_datatype_inner(ty, types, phase);
            }
        }
        DataType::List(list) => select_phase_datatype_inner(list.ty_mut(), types, phase),
        DataType::Map(map) => {
            select_phase_datatype_inner(map.key_ty_mut(), types, phase);
            select_phase_datatype_inner(map.value_ty_mut(), types, phase);
        }
        DataType::Nullable(inner) => select_phase_datatype_inner(inner, types, phase),
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced_ndt) = reference.get(types) else {
                return;
            };

            let generics = reference
                .generics()
                .iter()
                .map(|(generic, dt)| {
                    let mut dt = dt.clone();
                    select_phase_datatype_inner(&mut dt, types, phase);
                    (generic.clone(), dt)
                })
                .collect::<Vec<_>>();

            let target_ndt =
                select_split_type_variant(referenced_ndt, types, phase).unwrap_or(referenced_ndt);

            let mut new_reference = target_ndt.reference(generics);
            if reference.inline() {
                new_reference = new_reference.inline();
            }

            *ty = DataType::Reference(new_reference);
        }
        DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_))
        | DataType::Primitive(_) => {}
    }
}

fn select_phase_fields(fields: &mut Fields, types: &Types, phase: Phase) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in fields.fields_mut() {
                if let Some(ty) = field.ty_mut() {
                    select_phase_datatype_inner(ty, types, phase);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in fields.fields_mut() {
                if let Some(ty) = field.ty_mut() {
                    select_phase_datatype_inner(ty, types, phase);
                }
            }
        }
    }
}

fn select_explicit_phased_type(ty: &DataType, phase: Phase) -> Option<DataType> {
    let DataType::Reference(Reference::Opaque(reference)) = ty else {
        return None;
    };
    let phased = reference.downcast_ref::<PhasedTy>()?;

    Some(match phase {
        Phase::Serialize => phased.serialize.clone(),
        Phase::Deserialize => phased.deserialize.clone(),
    })
}

fn select_split_type_variant<'a>(
    ndt: &'a NamedDataType,
    types: &'a Types,
    phase: Phase,
) -> Option<&'a NamedDataType> {
    let DataType::Enum(wrapper) = ndt.ty() else {
        return None;
    };

    let variant_name = match phase {
        Phase::Serialize => "Serialize",
        Phase::Deserialize => "Deserialize",
    };

    let (_, variant) = wrapper
        .variants()
        .iter()
        .find(|(name, _)| name == variant_name)?;
    let Fields::Unnamed(fields) = variant.fields() else {
        return None;
    };
    let field = fields.fields().first()?;
    let Some(DataType::Reference(Reference::Named(reference))) = field.ty() else {
        return None;
    };

    reference.get(types)
}

#[derive(Debug, Clone)]
struct SplitGeneratedTypes {
    serialize: NamedDataType,
    deserialize: Box<NamedDataType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TypeIdentity {
    name: String,
    module_path: String,
    file: &'static str,
    line: u32,
    column: u32,
}

impl TypeIdentity {
    fn from_ndt(ty: &specta::datatype::NamedDataType) -> Self {
        let location = ty.location();
        Self {
            name: ty.name().to_string(),
            module_path: ty.module_path().to_string(),
            file: location.file(),
            line: location.line(),
            column: location.column(),
        }
    }
}

fn rewrite_datatype_for_phase(
    ty: &mut DataType,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
    container_name: Option<&str>,
) -> Result<()> {
    if let Some(resolved) = resolve_phased_type(ty, mode, "type")? {
        *ty = resolved;
    }

    if let Some(converted) = conversion_datatype_for_mode(ty, mode)?
        && converted != *ty
    {
        *ty = converted;
        return rewrite_datatype_for_phase(
            ty,
            mode,
            original_types,
            generated,
            split_types,
            container_name,
        );
    }

    match ty {
        DataType::Struct(s) => {
            let container_default = SerdeContainerAttrs::from_attributes(s.attributes())?
                .is_some_and(|attrs| attrs.default);
            let container_rename_all = container_rename_all_rule(
                s.attributes(),
                mode,
                "struct rename_all",
                container_name.unwrap_or("<anonymous struct>"),
            )?;

            rewrite_fields_for_phase(
                s.fields_mut(),
                mode,
                original_types,
                generated,
                split_types,
                container_rename_all,
                container_default,
                false,
            )?;
            rewrite_struct_repr_for_phase(s, mode, container_name)?;
        }
        DataType::Enum(e) => {
            filter_enum_variants_for_phase(e, mode)?;
            let container_attrs = SerdeContainerAttrs::from_attributes(e.attributes())?;

            for (variant_name, variant) in e.variants_mut() {
                let rename_rule =
                    enum_variant_field_rename_rule(&container_attrs, variant, mode, variant_name)?;

                rewrite_fields_for_phase(
                    variant.fields_mut(),
                    mode,
                    original_types,
                    generated,
                    split_types,
                    rename_rule,
                    false,
                    true,
                )?;
            }

            if rewrite_identifier_enum_for_phase(e, mode, original_types, generated, split_types)? {
                return Ok(());
            }

            rewrite_enum_repr_for_phase(e, mode, original_types)?;
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types, None)?;
            }
        }
        DataType::List(list) => rewrite_datatype_for_phase(
            list.ty_mut(),
            mode,
            original_types,
            generated,
            split_types,
            None,
        )?,
        DataType::Map(map) => {
            rewrite_datatype_for_phase(
                map.key_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
                None,
            )?;
            rewrite_datatype_for_phase(
                map.value_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
                None,
            )?;
        }
        DataType::Nullable(inner) => {
            rewrite_datatype_for_phase(inner, mode, original_types, generated, split_types, None)?
        }
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced_ndt) = reference.get(original_types) else {
                return Ok(());
            };
            let key = TypeIdentity::from_ndt(referenced_ndt);

            let mut generics = Vec::with_capacity(reference.generics().len());
            for (generic, dt) in reference.generics() {
                let mut dt = dt.clone();
                rewrite_datatype_for_phase(
                    &mut dt,
                    mode,
                    original_types,
                    generated,
                    split_types,
                    None,
                )?;
                generics.push((generic.clone(), dt));
            }

            if !split_types.contains(&key) {
                let mut new_reference = referenced_ndt.reference(generics);
                if reference.inline() {
                    new_reference = new_reference.inline();
                }
                *ty = DataType::Reference(new_reference);
                return Ok(());
            }

            let Some(target) = generated.get(&key) else {
                return Ok(());
            };

            let mut new_reference = match mode {
                PhaseRewrite::Unified => {
                    unreachable!("unified mode should not reference split types")
                }
                PhaseRewrite::Serialize => target.serialize.reference(generics),
                PhaseRewrite::Deserialize => target.deserialize.reference(generics),
            };

            if reference.inline() {
                new_reference = new_reference.inline();
            }

            *ty = DataType::Reference(new_reference);
        }
        DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_))
        | DataType::Primitive(_) => {}
    }

    Ok(())
}

fn rewrite_fields_for_phase(
    fields: &mut Fields,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
    rename_all_rule: Option<RenameRule>,
    container_default: bool,
    preserve_skipped_unnamed_fields: bool,
) -> Result<()> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                if should_skip_field_for_mode(field, mode)? {
                    if preserve_skipped_unnamed_fields {
                        *field = skipped_field_marker(field);
                    }

                    continue;
                }

                apply_field_attrs(field, mode, container_default)?;
                rewrite_field_for_phase(field, mode, original_types, generated, split_types)?;
            }

            if !preserve_skipped_unnamed_fields {
                unnamed.fields_mut().retain(|field| field.ty().is_some());
            }
        }
        Fields::Named(named) => {
            let mut skip_err = None;
            named
                .fields_mut()
                .retain(|(_, field)| match should_skip_field_for_mode(field, mode) {
                    Ok(skip) => !skip,
                    Err(err) => {
                        skip_err = Some(err);
                        true
                    }
                });
            if let Some(err) = skip_err {
                return Err(err);
            }

            for (name, field) in named.fields_mut() {
                apply_field_attrs(field, mode, container_default)?;

                if let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(field.attributes())? {
                    let rename = select_phase_string(
                        mode,
                        serde_attrs.rename_serialize.as_deref(),
                        serde_attrs.rename_deserialize.as_deref(),
                        "field rename",
                        name,
                    )?;

                    if let Some(rename) = rename {
                        *name = Cow::Owned(rename.to_string());
                    } else if let Some(rule) = rename_all_rule {
                        *name = Cow::Owned(rule.apply_to_field(name));
                    }
                } else if let Some(rule) = rename_all_rule {
                    *name = Cow::Owned(rule.apply_to_field(name));
                }

                rewrite_field_for_phase(field, mode, original_types, generated, split_types)?;
            }
        }
    }

    Ok(())
}

fn rewrite_field_for_phase(
    field: &mut Field,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
) -> Result<()> {
    if let Some(attrs) = SerdeFieldAttrs::from_attributes(field.attributes())?
        && let PhaseRewrite::Serialize = mode
        && attrs.skip_serializing_if.is_some()
    {
        field.set_optional(true);
    }

    if let Some(ty) = field.ty().cloned()
        && let Some(resolved) = resolve_phased_type(&ty, mode, "field")?
    {
        field.set_ty(resolved);
    }

    if let Some(ty) = field.ty_mut() {
        rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types, None)?;
    }

    Ok(())
}

fn rewrite_struct_repr_for_phase(
    strct: &mut Struct,
    mode: PhaseRewrite,
    container_name: Option<&str>,
) -> Result<()> {
    let Some((tag, rename_serialize, rename_deserialize)) =
        SerdeContainerAttrs::from_attributes(strct.attributes())?.map(|attrs| {
            (
                attrs.tag.clone(),
                attrs.rename_serialize.clone(),
                attrs.rename_deserialize.clone(),
            )
        })
    else {
        return Ok(());
    };

    let Some(tag) = tag.as_deref() else {
        return Ok(());
    };

    let serialized_name = match select_phase_string(
        mode,
        rename_serialize.as_deref(),
        rename_deserialize.as_deref(),
        "struct rename",
        container_name.unwrap_or("<anonymous struct>"),
    )? {
        Some(rename) => rename.to_string(),
        None => container_name
            .map(str::to_owned)
            .ok_or_else(|| {
                Error::invalid_phased_type_usage(
                    "<anonymous struct>",
                    "`#[serde(tag = ...)]` on structs requires either a named type or `#[serde(rename = ...)]`",
                )
            })?,
    };

    let Fields::Named(named) = strct.fields_mut() else {
        return Ok(());
    };

    named.fields_mut().insert(
        0,
        (
            Cow::Owned(tag.to_string()),
            Field::new(string_literal_datatype(serialized_name)),
        ),
    );

    Ok(())
}

fn should_skip_field_for_mode(field: &Field, mode: PhaseRewrite) -> Result<bool> {
    let Some(attrs) = SerdeFieldAttrs::from_attributes(field.attributes())? else {
        return Ok(false);
    };

    Ok(match mode {
        PhaseRewrite::Serialize => attrs.skip_serializing,
        PhaseRewrite::Deserialize => attrs.skip_deserializing,
        PhaseRewrite::Unified => attrs.skip_serializing || attrs.skip_deserializing,
    })
}

fn skipped_field_marker(field: &Field) -> Field {
    let mut skipped = Field::default();
    skipped.set_optional(field.optional());
    skipped.set_flatten(field.flatten());
    skipped.set_deprecated(field.deprecated().cloned());
    skipped.set_docs(field.docs().clone());
    skipped.set_inline(field.inline());
    skipped.set_type_overridden(field.type_overridden());
    skipped.set_attributes(field.attributes().clone());
    skipped
}

fn unnamed_live_fields(unnamed: &UnnamedFields) -> impl Iterator<Item = &Field> {
    unnamed.fields().iter().filter(|field| field.ty().is_some())
}

fn unnamed_live_field_count(unnamed: &UnnamedFields) -> usize {
    unnamed_live_fields(unnamed).count()
}

fn unnamed_has_effective_payload(unnamed: &UnnamedFields) -> bool {
    unnamed_live_field_count(unnamed) != 0
}

fn unnamed_fields_all_skipped(unnamed: &UnnamedFields) -> bool {
    !unnamed.fields().is_empty() && !unnamed_has_effective_payload(unnamed)
}

fn rewrite_enum_repr_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &Types,
) -> Result<()> {
    let repr = enum_repr_from_attrs(e.attributes())?;
    if matches!(repr, EnumRepr::Untagged) {
        return Ok(());
    }

    let container_attrs = SerdeContainerAttrs::from_attributes(e.attributes())?;
    let variants = std::mem::take(e.variants_mut());
    let mut transformed = Vec::with_capacity(variants.len());
    for (variant_name, variant) in variants {
        if variant.skip() {
            continue;
        }

        let variant_attrs = SerdeVariantAttrs::from_attributes(variant.attributes())?;
        if variant_attrs
            .as_ref()
            .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
        {
            continue;
        }

        if variant_attrs.as_ref().is_some_and(|attrs| attrs.untagged) {
            transformed.push((
                Cow::Owned(variant_name.into_owned()),
                transform_untagged_variant(&variant)?,
            ));
            continue;
        }

        let serialized_name =
            serialized_variant_name(&variant_name, &variant, &container_attrs, mode)?;
        let widen_tag =
            mode == PhaseRewrite::Deserialize && variant_attrs.is_some_and(|attrs| attrs.other);
        let transformed_variant = match &repr {
            EnumRepr::External => transform_external_variant(serialized_name.clone(), &variant)?,
            EnumRepr::Internal { tag } => transform_internal_variant(
                serialized_name.clone(),
                tag.as_ref(),
                &variant,
                original_types,
                widen_tag,
            )?,
            EnumRepr::Adjacent { tag, content } => {
                if tag == content {
                    return Err(Error::invalid_enum_representation(
                        "serde adjacent tagging requires distinct `tag` and `content` field names",
                    ));
                }

                transform_adjacent_variant(
                    serialized_name.clone(),
                    tag.as_ref(),
                    content.as_ref(),
                    &variant,
                    widen_tag,
                )?
            }
            EnumRepr::Untagged => unreachable!(),
        };

        transformed.push((Cow::Owned(serialized_name), transformed_variant));
    }

    *e.variants_mut() = transformed;

    Ok(())
}

fn rewrite_identifier_enum_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
) -> Result<bool> {
    let Some(attrs) = SerdeContainerAttrs::from_attributes(e.attributes())? else {
        return Ok(false);
    };

    if !attrs.variant_identifier && !attrs.field_identifier {
        return Ok(false);
    }

    if mode != PhaseRewrite::Deserialize {
        return Ok(false);
    }

    let container_attrs = SerdeContainerAttrs::from_attributes(e.attributes())?;
    let mut variants = Vec::new();
    let mut seen = HashSet::new();

    for (variant_name, variant) in e.variants().iter() {
        let serialized_name = serialized_variant_name(
            variant_name,
            variant,
            &container_attrs,
            PhaseRewrite::Deserialize,
        )?;

        if seen.insert(serialized_name.clone()) {
            variants.push((
                Cow::Owned(serialized_name.clone()),
                identifier_union_variant(string_literal_datatype(serialized_name)),
            ));
        }

        if let Some(variant_attrs) = SerdeVariantAttrs::from_attributes(variant.attributes())? {
            for alias in &variant_attrs.aliases {
                if seen.insert(alias.clone()) {
                    variants.push((
                        Cow::Owned(alias.clone()),
                        identifier_union_variant(string_literal_datatype(alias.clone())),
                    ));
                }
            }
        }
    }

    variants.push((
        Cow::Borrowed("__specta_identifier_index"),
        identifier_union_variant(DataType::Primitive(specta::datatype::Primitive::u32)),
    ));

    if attrs.field_identifier
        && let Some((_, fallback)) = e.variants().last()
        && let Fields::Unnamed(unnamed) = fallback.fields()
        && let Some(field) = unnamed.fields().first()
        && let Some(ty) = field.ty()
    {
        let mut fallback_ty = ty.clone();
        rewrite_datatype_for_phase(
            &mut fallback_ty,
            mode,
            original_types,
            generated,
            split_types,
            None,
        )?;
        variants.push((
            Cow::Borrowed("__specta_identifier_other"),
            identifier_union_variant(fallback_ty),
        ));
    }

    *e.variants_mut() = variants;
    Ok(true)
}

fn container_rename_all_rule(
    attrs: &specta::datatype::Attributes,
    mode: PhaseRewrite,
    context: &str,
    container_name: &str,
) -> Result<Option<RenameRule>> {
    let attrs = SerdeContainerAttrs::from_attributes(attrs)?;

    select_phase_rule(
        mode,
        attrs.as_ref().and_then(|attrs| attrs.rename_all_serialize),
        attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_deserialize),
        context,
        container_name,
    )
}

fn enum_variant_field_rename_rule(
    container_attrs: &Option<SerdeContainerAttrs>,
    variant: &Variant,
    mode: PhaseRewrite,
    variant_name: &str,
) -> Result<Option<RenameRule>> {
    let variant_attrs = SerdeVariantAttrs::from_attributes(variant.attributes())?;

    let variant_rule = select_phase_rule(
        mode,
        variant_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_serialize),
        variant_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_deserialize),
        "enum variant rename_all",
        variant_name,
    )?;

    if variant_rule.is_some() {
        return Ok(variant_rule);
    }

    select_phase_rule(
        mode,
        container_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_fields_serialize),
        container_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_fields_deserialize),
        "enum rename_all_fields",
        variant_name,
    )
}

fn identifier_union_variant(ty: DataType) -> Variant {
    let mut variant = Variant::unnamed().build();
    if let Fields::Unnamed(fields) = variant.fields_mut() {
        fields.fields_mut().push(Field::new(ty));
    }
    variant
}

fn transform_untagged_variant(variant: &Variant) -> Result<Variant> {
    let payload = variant_payload_field(variant)
        .ok_or_else(|| Error::invalid_external_tagged_variant("<untagged variant>"))?;
    Ok(clone_variant_with_unnamed_fields(variant, vec![payload]))
}

fn filter_enum_variants_for_phase(e: &mut Enum, mode: PhaseRewrite) -> Result<()> {
    let mut filter_err = None;
    e.variants_mut().retain(|(_, variant)| {
        if variant.skip() {
            return false;
        }

        match SerdeVariantAttrs::from_attributes(variant.attributes()) {
            Ok(Some(attrs)) => !variant_is_skipped_for_mode(&attrs, mode),
            Ok(None) => true,
            Err(err) => {
                filter_err = Some(err);
                true
            }
        }
    });

    if let Some(err) = filter_err {
        return Err(err);
    }

    Ok(())
}

fn variant_is_skipped_for_mode(attrs: &SerdeVariantAttrs, mode: PhaseRewrite) -> bool {
    match mode {
        PhaseRewrite::Serialize => attrs.skip_serializing,
        PhaseRewrite::Deserialize => attrs.skip_deserializing,
        PhaseRewrite::Unified => attrs.skip_serializing || attrs.skip_deserializing,
    }
}

fn enum_repr_from_attrs(attrs: &specta::datatype::Attributes) -> Result<EnumRepr> {
    let Some(container_attrs) = SerdeContainerAttrs::from_attributes(attrs)? else {
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
                tag: Cow::Owned(tag.to_string()),
                content: Cow::Owned(content.to_string()),
            },
            (Some(tag), None) => EnumRepr::Internal {
                tag: Cow::Owned(tag.to_string()),
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

fn serialized_variant_name(
    variant_name: &str,
    variant: &Variant,
    container_attrs: &Option<SerdeContainerAttrs>,
    mode: PhaseRewrite,
) -> Result<String> {
    let variant_attrs = SerdeVariantAttrs::from_attributes(variant.attributes())?;

    if let Some(rename) = select_phase_string(
        mode,
        variant_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_serialize.as_deref()),
        variant_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_deserialize.as_deref()),
        "enum variant rename",
        variant_name,
    )? {
        return Ok(rename.to_string());
    }

    Ok(select_phase_rule(
        mode,
        container_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_serialize),
        container_attrs
            .as_ref()
            .and_then(|attrs| attrs.rename_all_deserialize),
        "enum rename_all",
        variant_name,
    )?
    .map_or_else(
        || variant_name.to_string(),
        |rule| rule.apply_to_variant(variant_name),
    ))
}

fn select_phase_string<'a>(
    mode: PhaseRewrite,
    serialize: Option<&'a str>,
    deserialize: Option<&'a str>,
    context: &str,
    name: &str,
) -> Result<Option<&'a str>> {
    Ok(match mode {
        PhaseRewrite::Serialize => serialize,
        PhaseRewrite::Deserialize => deserialize,
        PhaseRewrite::Unified => match (serialize, deserialize) {
            (Some(serialize), Some(deserialize)) if serialize != deserialize => {
                return Err(Error::incompatible_rename(
                    context.to_string(),
                    name,
                    Some(serialize.to_string()),
                    Some(deserialize.to_string()),
                ));
            }
            (serialize, deserialize) => serialize.or(deserialize),
        },
    })
}

fn select_phase_rule(
    mode: PhaseRewrite,
    serialize: Option<RenameRule>,
    deserialize: Option<RenameRule>,
    context: &str,
    name: &str,
) -> Result<Option<RenameRule>> {
    Ok(match mode {
        PhaseRewrite::Serialize => serialize,
        PhaseRewrite::Deserialize => deserialize,
        PhaseRewrite::Unified => match (serialize, deserialize) {
            (Some(serialize), Some(deserialize)) if serialize != deserialize => {
                return Err(Error::incompatible_rename(
                    context.to_string(),
                    name,
                    Some(format!("{serialize:?}")),
                    Some(format!("{deserialize:?}")),
                ));
            }
            (serialize, deserialize) => serialize.or(deserialize),
        },
    })
}

fn resolve_phased_type(ty: &DataType, mode: PhaseRewrite, path: &str) -> Result<Option<DataType>> {
    let DataType::Reference(Reference::Opaque(reference)) = ty else {
        return Ok(None);
    };
    let Some(phased) = reference.downcast_ref::<PhasedTy>() else {
        return Ok(None);
    };

    Ok(match mode {
        // Note that we won't hit this if `TSerialize == TDeserialize` because it will just return `T` directly in the `impl Type for Phased<...>`
        PhaseRewrite::Unified => {
            return Err(Error::invalid_phased_type_usage(
                path,
                "`specta_serde::Phased<Serialize, Deserialize>` requires `apply_phases`",
            ));
        }
        PhaseRewrite::Serialize => Some(phased.serialize.clone()),
        PhaseRewrite::Deserialize => Some(phased.deserialize.clone()),
    })
}

fn conversion_datatype_for_mode(ty: &DataType, mode: PhaseRewrite) -> Result<Option<DataType>> {
    let attrs = match ty {
        DataType::Struct(s) => s.attributes(),
        DataType::Enum(e) => e.attributes(),
        _ => return Ok(None),
    };

    select_conversion_target(attrs, mode)
}

fn select_conversion_target(
    attrs: &specta::datatype::Attributes,
    mode: PhaseRewrite,
) -> Result<Option<DataType>> {
    let parsed = SerdeContainerAttrs::from_attributes(attrs)?;
    let resolved = parsed.as_ref();

    let serialize_target = resolved.and_then(|v| v.resolved_into.as_ref());
    let deserialize_target =
        resolved.and_then(|v| v.resolved_from.as_ref().or(v.resolved_try_from.as_ref()));

    match mode {
        PhaseRewrite::Serialize => Ok(serialize_target.cloned()),
        PhaseRewrite::Deserialize => Ok(deserialize_target.cloned()),
        PhaseRewrite::Unified => match (serialize_target, deserialize_target) {
            (None, None) => Ok(None),
            (Some(serialize), Some(deserialize)) if serialize == deserialize => {
                Ok(Some(serialize.clone()))
            }
            _ => Err(Error::incompatible_conversion(
                "container conversion",
                conversion_name(attrs)?,
                serialize_conversion_name(parsed.as_ref()),
                deserialize_conversion_name(parsed.as_ref()),
            )),
        },
    }
}

fn conversion_name(attrs: &specta::datatype::Attributes) -> Result<String> {
    Ok(SerdeContainerAttrs::from_attributes(attrs)?
        .and_then(|attrs| {
            attrs
                .into
                .as_ref()
                .map(|v| format!("into({})", v.type_src))
                .or_else(|| attrs.from.as_ref().map(|v| format!("from({})", v.type_src)))
                .or_else(|| {
                    attrs
                        .try_from
                        .as_ref()
                        .map(|v| format!("try_from({})", v.type_src))
                })
        })
        .unwrap_or_else(|| "<container>".to_string()))
}

fn serialize_conversion_name(attrs: Option<&SerdeContainerAttrs>) -> Option<String> {
    attrs.and_then(|attrs| attrs.into.as_ref().map(|v| v.type_src.clone()))
}

fn deserialize_conversion_name(attrs: Option<&SerdeContainerAttrs>) -> Option<String> {
    attrs.and_then(|attrs| {
        attrs.from.as_ref().map(|v| v.type_src.clone()).or_else(|| {
            attrs
                .try_from
                .as_ref()
                .map(|v| format!("try_from({})", v.type_src))
        })
    })
}

fn transform_external_variant(serialized_name: String, variant: &Variant) -> Result<Variant> {
    let skipped_only_unnamed = match variant.fields() {
        Fields::Unnamed(unnamed) => unnamed_fields_all_skipped(unnamed),
        Fields::Unit | Fields::Named(_) => false,
    };

    Ok(match variant.fields() {
        Fields::Unit => clone_variant_with_unnamed_fields(
            variant,
            vec![Field::new(string_literal_datatype(serialized_name))],
        ),
        _ if skipped_only_unnamed => clone_variant_with_unnamed_fields(
            variant,
            vec![Field::new(string_literal_datatype(serialized_name))],
        ),
        _ => {
            let payload = variant_payload_field(variant)
                .ok_or_else(|| Error::invalid_external_tagged_variant(serialized_name.clone()))?;

            clone_variant_with_named_fields(variant, vec![(Cow::Owned(serialized_name), payload)])
        }
    })
}

fn transform_adjacent_variant(
    serialized_name: String,
    tag: &str,
    content: &str,
    variant: &Variant,
    widen_tag: bool,
) -> Result<Variant> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(if widen_tag {
            DataType::Primitive(Primitive::str)
        } else {
            string_literal_datatype(serialized_name.clone())
        }),
    )];

    if variant_has_effective_payload(variant) {
        let payload = variant_payload_field(variant)
            .ok_or_else(|| Error::invalid_adjacent_tagged_variant(serialized_name.clone()))?;
        fields.push((Cow::Owned(content.to_string()), payload));
    }

    Ok(clone_variant_with_named_fields(variant, fields))
}

fn transform_internal_variant(
    serialized_name: String,
    tag: &str,
    variant: &Variant,
    original_types: &Types,
    widen_tag: bool,
) -> Result<Variant> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(if widen_tag {
            DataType::Primitive(Primitive::str)
        } else {
            string_literal_datatype(serialized_name.clone())
        }),
    )];

    match variant.fields() {
        Fields::Unit => {}
        Fields::Named(named) => {
            fields.extend(named.fields().iter().cloned());
        }
        Fields::Unnamed(unnamed) => {
            let live_field_count = unnamed_live_field_count(unnamed);

            if live_field_count == 0 {
                return Ok(clone_variant_with_named_fields(variant, fields));
            }

            let non_skipped = unnamed_live_fields(unnamed).collect::<Vec<_>>();

            if live_field_count != 1 {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "tuple variant must have exactly one non-skipped field",
                ));
            }

            let payload_field = non_skipped
                .into_iter()
                .next()
                .expect("checked above")
                .clone();
            let payload_ty = payload_field.ty().cloned().expect("checked above");
            let Some(payload_is_effectively_empty) = internal_tag_payload_compatibility(
                &payload_ty,
                original_types,
                &mut HashSet::new(),
            )?
            else {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "payload cannot be merged with a tag",
                ));
            };

            if !payload_is_effectively_empty {
                let mut flattened = payload_field;
                flattened.set_flatten(true);
                fields.push((Cow::Borrowed("__specta_internal_payload"), flattened));
            }
        }
    }

    Ok(clone_variant_with_named_fields(variant, fields))
}

fn string_literal_datatype(value: String) -> DataType {
    let mut value_enum = Enum::new();
    value_enum
        .variants_mut()
        .push((Cow::Owned(value), Variant::unit()));
    DataType::Enum(value_enum)
}

fn variant_has_effective_payload(variant: &Variant) -> bool {
    match variant.fields() {
        Fields::Unit => false,
        Fields::Named(named) => !named.fields().is_empty(),
        Fields::Unnamed(unnamed) => unnamed_has_effective_payload(unnamed),
    }
}

fn variant_payload_field(variant: &Variant) -> Option<Field> {
    match variant.fields() {
        Fields::Unit => Some(Field::new(DataType::Tuple(Tuple::new(vec![])))),
        Fields::Named(named) => {
            let mut out = Struct::named();
            for (name, field) in named.fields().iter().cloned() {
                out.field_mut(name, field);
            }
            Some(Field::new(out.build()))
        }
        Fields::Unnamed(unnamed) => {
            let original_unnamed_len = unnamed.fields().len();

            let non_skipped = unnamed_live_fields(unnamed).collect::<Vec<_>>();

            match non_skipped.as_slice() {
                [] => Some(Field::new(DataType::Tuple(Tuple::new(vec![])))),
                [single] if original_unnamed_len == 1 => Some((*single).clone()),
                _ => Some(Field::new(DataType::Tuple(Tuple::new(
                    non_skipped
                        .iter()
                        .filter_map(|field| field.ty().cloned())
                        .collect(),
                )))),
            }
        }
    }
}

fn clone_variant_with_named_fields(
    original: &Variant,
    fields: Vec<(Cow<'static, str>, Field)>,
) -> Variant {
    let mut builder = Variant::named();
    for (name, field) in fields {
        builder = builder.field(name, field);
    }

    let mut transformed = builder.build();
    transformed.set_skip(original.skip());
    transformed.set_docs(original.docs().clone());
    transformed.set_deprecated(original.deprecated().cloned());
    transformed.set_type_overridden(original.type_overridden());
    *transformed.attributes_mut() = original.attributes().clone();
    transformed
}

fn clone_variant_with_unnamed_fields(original: &Variant, fields: Vec<Field>) -> Variant {
    let mut builder = Variant::unnamed();
    for field in fields {
        builder = builder.field(field);
    }

    let mut transformed = builder.build();
    transformed.set_skip(original.skip());
    transformed.set_docs(original.docs().clone());
    transformed.set_deprecated(original.deprecated().cloned());
    transformed.set_type_overridden(original.type_overridden());
    *transformed.attributes_mut() = original.attributes().clone();
    transformed
}

fn internal_tag_payload_compatibility(
    ty: &DataType,
    original_types: &Types,
    seen: &mut HashSet<TypeIdentity>,
) -> Result<Option<bool>> {
    match ty {
        DataType::Map(_) => Ok(Some(false)),
        DataType::Struct(strct) => {
            if SerdeContainerAttrs::from_attributes(strct.attributes())?
                .is_some_and(|attrs| attrs.transparent)
            {
                let payload_fields = match strct.fields() {
                    Fields::Unit => return Ok(Some(true)),
                    Fields::Unnamed(unnamed) => unnamed
                        .fields()
                        .iter()
                        .filter_map(Field::ty)
                        .collect::<Vec<_>>(),
                    Fields::Named(named) => named
                        .fields()
                        .iter()
                        .filter_map(|(_, field)| field.ty())
                        .collect::<Vec<_>>(),
                };

                let [inner_ty] = payload_fields.as_slice() else {
                    if payload_fields.is_empty() {
                        return Ok(Some(true));
                    }

                    return Ok(None);
                };

                return internal_tag_payload_compatibility(inner_ty, original_types, seen);
            }

            Ok(match strct.fields() {
                Fields::Named(named) => {
                    Some(named.fields().iter().all(|(_, field)| field.ty().is_none()))
                }
                Fields::Unit | Fields::Unnamed(_) => None,
            })
        }
        DataType::Tuple(tuple) => Ok(tuple.elements().is_empty().then_some(true)),
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced) = reference.get(original_types) else {
                return Ok(None);
            };

            let key = TypeIdentity::from_ndt(referenced);
            if !seen.insert(key.clone()) {
                return Ok(Some(false));
            }

            let compatible =
                internal_tag_payload_compatibility(referenced.ty(), original_types, seen);
            seen.remove(&key);
            compatible
        }
        DataType::Enum(enm) => match enum_repr_from_attrs(enm.attributes()) {
            Ok(EnumRepr::Untagged) => {
                let mut is_effectively_empty = true;
                for (_, variant) in enm.variants() {
                    let Some(variant_empty) =
                        internal_tag_variant_payload_compatibility(variant, original_types, seen)?
                    else {
                        return Ok(None);
                    };

                    is_effectively_empty &= variant_empty;
                }

                Ok(Some(is_effectively_empty))
            }
            Ok(EnumRepr::External | EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. }) => {
                Ok(Some(false))
            }
            Err(_) => Ok(None),
        },
        DataType::Primitive(_)
        | DataType::List(_)
        | DataType::Nullable(_)
        | DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_)) => Ok(None),
    }
}

fn internal_tag_variant_payload_compatibility(
    variant: &Variant,
    original_types: &Types,
    seen: &mut HashSet<TypeIdentity>,
) -> Result<Option<bool>> {
    match variant.fields() {
        Fields::Unit => Ok(Some(true)),
        Fields::Named(named) => Ok(Some(
            named.fields().iter().all(|(_, field)| field.ty().is_none()),
        )),
        Fields::Unnamed(unnamed) => {
            if unnamed.fields().len() != 1 {
                return Ok(None);
            }

            unnamed
                .fields()
                .iter()
                .find_map(|field| field.ty())
                .map_or(Ok(None), |ty| {
                    internal_tag_payload_compatibility(ty, original_types, seen)
                })
        }
    }
}

fn has_local_phase_difference(dt: &DataType) -> Result<bool> {
    match dt {
        DataType::Struct(s) => Ok(container_has_local_difference(s.attributes())?
            || fields_have_local_difference(s.fields())?),
        DataType::Enum(e) => Ok(container_has_local_difference(e.attributes())?
            || e.variants()
                .iter()
                .try_fold(false, |has_difference, (_, variant)| {
                    if has_difference {
                        return Ok(true);
                    }

                    Ok(variant_has_local_difference(variant)?
                        || fields_have_local_difference(variant.fields())?)
                })?),
        DataType::Tuple(tuple) => tuple
            .elements()
            .iter()
            .try_fold(false, |has_difference, ty| {
                if has_difference {
                    return Ok(true);
                }

                has_local_phase_difference(ty)
            }),
        DataType::List(list) => has_local_phase_difference(list.ty()),
        DataType::Map(map) => Ok(has_local_phase_difference(map.key_ty())?
            || has_local_phase_difference(map.value_ty())?),
        DataType::Nullable(inner) => has_local_phase_difference(inner),
        DataType::Reference(Reference::Opaque(reference)) => {
            Ok(reference.downcast_ref::<PhasedTy>().is_some())
        }
        DataType::Primitive(_)
        | DataType::Reference(Reference::Named(_))
        | DataType::Reference(Reference::Generic(_)) => Ok(false),
    }
}

fn container_has_local_difference(attrs: &specta::datatype::Attributes) -> Result<bool> {
    let Some(conversions) = SerdeContainerAttrs::from_attributes(attrs)? else {
        return Ok(false);
    };

    Ok(conversions.resolved_into.as_ref()
        != conversions
            .resolved_from
            .as_ref()
            .or(conversions.resolved_try_from.as_ref())
        || conversions.rename_serialize != conversions.rename_deserialize
        || conversions.rename_all_serialize != conversions.rename_all_deserialize
        || conversions.rename_all_fields_serialize != conversions.rename_all_fields_deserialize
        || conversions.variant_identifier
        || conversions.field_identifier)
}

fn fields_have_local_difference(fields: &Fields) -> Result<bool> {
    match fields {
        Fields::Unit => Ok(false),
        Fields::Unnamed(unnamed) => {
            unnamed
                .fields()
                .iter()
                .try_fold(false, |has_difference, field| {
                    if has_difference {
                        return Ok(true);
                    }

                    field.ty().map_or(Ok(false), has_local_phase_difference)
                })
        }
        Fields::Named(named) => {
            named
                .fields()
                .iter()
                .try_fold(false, |has_difference, (_, field)| {
                    if has_difference {
                        return Ok(true);
                    }

                    Ok(field_has_local_difference(field)?
                        || field.ty().map_or(Ok(false), has_local_phase_difference)?)
                })
        }
    }
}

fn field_has_local_difference(field: &Field) -> Result<bool> {
    Ok(SerdeFieldAttrs::from_attributes(field.attributes())?
        .map(|attrs| {
            attrs.rename_serialize.as_deref() != attrs.rename_deserialize.as_deref()
                || attrs.skip_serializing != attrs.skip_deserializing
                || attrs.skip_serializing_if.is_some()
                || attrs.has_serialize_with
                || attrs.has_deserialize_with
                || attrs.has_with
        })
        .unwrap_or_default())
}

fn variant_has_local_difference(variant: &Variant) -> Result<bool> {
    Ok(SerdeVariantAttrs::from_attributes(variant.attributes())?
        .map(|attrs| {
            attrs.rename_serialize.as_deref() != attrs.rename_deserialize.as_deref()
                || attrs.rename_all_serialize != attrs.rename_all_deserialize
                || attrs.skip_serializing != attrs.skip_deserializing
                || attrs.has_serialize_with
                || attrs.has_deserialize_with
                || attrs.has_with
                || attrs.other
        })
        .unwrap_or_default())
}

fn collect_dependencies(
    dt: &DataType,
    types: &Types,
    deps: &mut HashSet<TypeIdentity>,
) -> Result<()> {
    match dt {
        DataType::Struct(s) => {
            collect_conversion_dependencies(s.attributes(), types, deps)?;
            collect_fields_dependencies(s.fields(), types, deps)?;
        }
        DataType::Enum(e) => {
            collect_conversion_dependencies(e.attributes(), types, deps)?;
            for (_, variant) in e.variants() {
                collect_fields_dependencies(variant.fields(), types, deps)?;
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements() {
                collect_dependencies(ty, types, deps)?;
            }
        }
        DataType::List(list) => collect_dependencies(list.ty(), types, deps)?,
        DataType::Map(map) => {
            collect_dependencies(map.key_ty(), types, deps)?;
            collect_dependencies(map.value_ty(), types, deps)?;
        }
        DataType::Nullable(inner) => collect_dependencies(inner, types, deps)?,
        DataType::Reference(Reference::Named(reference)) => {
            if let Some(referenced) = reference.get(types) {
                deps.insert(TypeIdentity::from_ndt(referenced));
            }

            for (_, generic) in reference.generics() {
                collect_dependencies(generic, types, deps)?;
            }
        }
        DataType::Reference(Reference::Opaque(_)) => {
            if let DataType::Reference(Reference::Opaque(reference)) = dt
                && let Some(phased) = reference.downcast_ref::<PhasedTy>()
            {
                collect_dependencies(&phased.serialize, types, deps)?;
                collect_dependencies(&phased.deserialize, types, deps)?;
            }
        }
        DataType::Primitive(_) | DataType::Reference(Reference::Generic(_)) => {}
    }

    Ok(())
}

fn collect_conversion_dependencies(
    attrs: &specta::datatype::Attributes,
    types: &Types,
    deps: &mut HashSet<TypeIdentity>,
) -> Result<()> {
    let Some(conversions) = SerdeContainerAttrs::from_attributes(attrs)? else {
        return Ok(());
    };

    for conversion in [
        conversions.resolved_into.as_ref(),
        conversions.resolved_from.as_ref(),
        conversions.resolved_try_from.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        collect_dependencies(conversion, types, deps)?;
    }

    Ok(())
}

fn collect_fields_dependencies(
    fields: &Fields,
    types: &Types,
    deps: &mut HashSet<TypeIdentity>,
) -> Result<()> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields() {
                if let Some(ty) = field.ty() {
                    collect_dependencies(ty, types, deps)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in named.fields() {
                if let Some(ty) = field.ty() {
                    collect_dependencies(ty, types, deps)?;
                }
            }
        }
    }

    Ok(())
}

fn build_from_original(
    original: &NamedDataType,
    name: impl Into<Cow<'static, str>>,
    generics: Vec<(specta::datatype::GenericReference, Cow<'static, str>)>,
    ty: DataType,
    types: &Types,
) -> NamedDataType {
    let mut ndt = if original.requires_reference(types) {
        NamedDataType::new(name, generics, ty)
    } else {
        NamedDataType::new_inline(name, generics, ty)
    };

    ndt.set_docs(original.docs().clone());
    ndt.set_location(original.location());
    ndt.set_module_path(original.module_path().clone());
    ndt.set_deprecated(original.deprecated().cloned());

    ndt
}

fn apply_field_attrs(field: &mut Field, mode: PhaseRewrite, container_default: bool) -> Result<()> {
    let mut flatten = field.flatten();
    let mut optional = field.optional();
    if let Some(attrs) = SerdeFieldAttrs::from_attributes(field.attributes())? {
        flatten = attrs.flatten;
        if field_is_optional_for_mode(Some(&attrs), container_default, mode) {
            optional = true;
        }
    } else if field_is_optional_for_mode(None, container_default, mode) {
        optional = true;
    }
    field.set_flatten(flatten);
    field.set_optional(optional);

    Ok(())
}

fn field_is_optional_for_mode(
    attrs: Option<&SerdeFieldAttrs>,
    container_default: bool,
    mode: PhaseRewrite,
) -> bool {
    match mode {
        PhaseRewrite::Serialize => false,
        PhaseRewrite::Deserialize | PhaseRewrite::Unified => {
            container_default
                || attrs.is_some_and(|attrs| attrs.default || attrs.skip_deserializing)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use specta::{ResolvedTypes, Type, datatype::DataType};

    use super::{Phase, Phased, apply_phases, select_phase_datatype};

    #[derive(Type, Serialize, Deserialize)]
    #[serde(untagged)]
    enum OneOrManyString {
        One(String),
        Many(Vec<String>),
    }

    #[derive(Type, Serialize, Deserialize)]
    struct Filters {
        #[specta(type = Phased<Vec<String>, OneOrManyString>)]
        tags: Vec<String>,
    }

    #[derive(Type, Serialize, Deserialize)]
    struct FilterList {
        items: Vec<Filters>,
    }

    #[derive(Type, Serialize, Deserialize)]
    struct Plain {
        name: String,
    }

    #[test]
    fn selects_split_named_reference_for_each_phase() {
        let mut types = specta::Types::default();
        let dt = Filters::definition(&mut types);
        let resolved = apply_phases(types).expect("apply_phases should succeed");

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "Filters_Serialize");
        assert_named_reference(&deserialize, &resolved, "Filters_Deserialize");
    }

    #[test]
    fn rewrites_nested_generics_for_each_phase() {
        let mut types = specta::Types::default();
        let dt = FilterList::definition(&mut types);
        let resolved = apply_phases(types).expect("apply_phases should succeed");

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "FilterList_Serialize");
        assert_named_reference(&deserialize, &resolved, "FilterList_Deserialize");

        let serialize_inner = named_field_type(&serialize, &resolved, "items");
        let deserialize_inner = named_field_type(&deserialize, &resolved, "items");

        assert_named_reference(
            first_generic_type(serialize_inner),
            &resolved,
            "Filters_Serialize",
        );
        assert_named_reference(
            first_generic_type(deserialize_inner),
            &resolved,
            "Filters_Deserialize",
        );
    }

    #[test]
    fn preserves_unsplit_types() {
        let mut types = specta::Types::default();
        let dt = Plain::definition(&mut types);
        let resolved = apply_phases(types).expect("apply_phases should succeed");

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "Plain");
        assert_named_reference(&deserialize, &resolved, "Plain");
    }

    #[test]
    fn resolves_explicit_phased_datatypes_without_named_types() {
        let mut types = specta::Types::default();
        let dt = <Phased<String, Vec<String>>>::definition(&mut types);
        let resolved = apply_phases(types).expect("apply_phases should succeed");

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "String");
        assert_named_reference(first_generic_type(&deserialize), &resolved, "String");
    }

    fn assert_named_reference(dt: &DataType, types: &ResolvedTypes, expected_name: &str) {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };

        let actual = reference
            .get(types.as_types())
            .expect("reference should resolve")
            .name();

        assert_eq!(actual, expected_name);
    }

    fn named_field_type<'a>(
        dt: &'a DataType,
        types: &'a ResolvedTypes,
        field_name: &str,
    ) -> &'a DataType {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };

        let named = reference
            .get(types.as_types())
            .expect("reference should resolve");
        let DataType::Struct(strct) = named.ty() else {
            panic!("expected struct type");
        };
        let specta::datatype::Fields::Named(fields) = strct.fields() else {
            panic!("expected named fields");
        };

        fields
            .fields()
            .iter()
            .find_map(|(name, field)| (name == field_name).then(|| field.ty()).flatten())
            .expect("field should exist")
    }

    fn first_generic_type(dt: &DataType) -> &DataType {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference with generics");
        };

        reference
            .generics()
            .first()
            .map(|(_, dt)| dt)
            .expect("expected first generic type")
    }
}
