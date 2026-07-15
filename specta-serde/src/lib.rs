//! [Serde](https://serde.rs) support for Specta.
//!
//! # Choosing a mode
//!
//! - Use [`Format`] when serde behavior is symmetric and a single exported shape
//!   should work for both serialization and deserialization.
//! - Use [`PhasesFormat`] when serde behavior differs by direction (for example
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
//! [`PhasesFormat`].
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
//! let phased_types = specta_typescript::Typescript::default()
//!     .export(&types, specta_serde::PhasesFormat)?;
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
//! [`Format`] and [`PhasesFormat`].
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
    FormatError, Types,
    datatype::{
        DataType, Enum, Field, Fields, NamedDataType, NamedReference, NamedReferenceType,
        Primitive, Reference, Struct, Tuple, UnnamedFields, Variant,
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
use repr::EnumRepr;

const SERDE_NEWTYPE_SKIP_IGNORED: &str = "specta:serde_newtype_skip_ignored";

pub use error::Error;
pub use phased::{Phased, phased};

/// Selects which directional type shape to use with [`PhasesFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// The shape used when Rust serializes data to the wire.
    Serialize,
    /// The shape used when Rust deserializes data from the wire.
    Deserialize,
}

/// Applies serde-aware rewrites to a single shared type graph.
///
/// Use this when the serialized and deserialized wire shape can be represented
/// by the same exported schema. Exporters should pass this formatter to Specta's
/// formatting hook, for example
/// `specta_typescript::Typescript::default().export(&types, specta_serde::Format)`.
///
/// This formatter validates the graph for unified export and applies serde
/// container, variant, and field behavior that affects the exported shape, such
/// as renames, tagging, defaults, flattening, and compatible conversion attrs.
///
/// When a single safe type can contain both directions, this formatter widens
/// the shared shape. For example, conditional serialization makes a field
/// optional and aliases add accepted names. Use [`PhasesFormat`] when consumers
/// need exact directional shapes or when the two directions cannot be safely
/// unified.
pub struct Format;

impl specta::Format for Format {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, FormatError> {
        validate::validate_for_mode(types, validate::ApplyMode::Unified)?;

        let mut out = types.clone();
        let generated = HashMap::<TypeIdentity, SplitGeneratedTypes>::new();
        let split_types = HashSet::<TypeIdentity>::new();
        let mut rewrite_err = None;

        out.iter_mut(|ndt| {
            if rewrite_err.is_some() {
                return;
            }

            let ndt_name = ndt.name.to_string();

            // Compute the container rename before `rewrite_datatype_for_phase` runs: some
            // rewrites (e.g. enum representation lowering) clear the container's serde
            // attributes once applied, so the rename must be read from the untouched type.
            if let Err(err) = rewrite_named_type_for_phase(ndt, PhaseRewrite::Unified) {
                rewrite_err = Some(err);
                return;
            }

            if let Some(ty) = ndt.ty.as_mut()
                && let Err(err) = rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Unified,
                    types,
                    &generated,
                    &split_types,
                    Some(ndt_name.as_str()),
                )
            {
                rewrite_err = Some(err);
            }
        });

        if let Some(err) = rewrite_err {
            return Err(Box::new(err));
        }

        Ok(Cow::Owned(out))
    }

    fn map_type(&'_ self, types: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, FormatError> {
        if datatype_is_registered_definition(types, dt) {
            return Ok(Cow::Owned(dt.clone()));
        }

        validate::validate_datatype_for_mode(dt, types, validate::ApplyMode::Unified)?;

        let mut dt = dt.clone();
        rewrite_datatype_for_phase(
            &mut dt,
            PhaseRewrite::Unified,
            types,
            &HashMap::new(),
            &HashSet::new(),
            None,
        )?;

        Ok(Cow::Owned(dt))
    }
}

/// Applies serde-aware rewrites while preserving separate serialize and
/// deserialize shapes.
///
/// Use this when serde metadata makes the wire format directional, such as
/// asymmetric renames, directional skips, `#[serde(default)]` (fields may be
/// omitted on deserialize but are always emitted on serialize),
/// `#[serde(with = ...)]`-style codecs, `#[serde(into = ...)]`/`#[serde(from = ...)]`,
/// or explicit [`Phased`] overrides.
///
/// Exporters should pass this formatter to Specta's formatting hook, for
/// example
/// `specta_typescript::Typescript::default().export(&types, specta_serde::PhasesFormat)`.
///
/// The transformed type graph includes `*_Serialize` and `*_Deserialize` named
/// types for definitions that need to diverge, while unchanged definitions stay
/// shared. Inline datatype rendering uses the serialize-facing shape; use
/// [`select_phase_datatype`] to inspect either direction explicitly.
pub struct PhasesFormat;

impl specta::Format for PhasesFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, FormatError> {
        validate::validate_for_mode(types, validate::ApplyMode::Phases)?;

        let originals = types.into_unsorted_iter().collect::<Vec<_>>();
        let mut dependencies = HashMap::<TypeIdentity, HashSet<TypeIdentity>>::new();
        let mut reverse_dependencies = HashMap::<TypeIdentity, HashSet<TypeIdentity>>::new();

        for original in &originals {
            let key = TypeIdentity::from_ndt(original);
            let mut deps = HashSet::new();
            if let Some(ty) = &original.ty {
                collect_dependencies(ty, types, &mut deps)?;
            }
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
            if ndt
                .ty
                .as_ref()
                .is_some_and(|ty| has_local_phase_difference(ty).unwrap_or(false))
            {
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
                let serialize_ndt = build_from_original(original, PhaseRewrite::Serialize)?;

                let deserialize_ndt = build_from_original(original, PhaseRewrite::Deserialize)?;

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

            let mut rewrite_err = None;
            if let Some(ty) = generated_types_for_phase.serialize.ty.as_mut()
                && let Err(err) = rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Serialize,
                    types,
                    &generated,
                    &split_types,
                    Some(original.name.as_ref()),
                )
            {
                rewrite_err = Some(err);
            }
            if let Some(err) = rewrite_err.take() {
                return Err(Box::new(err));
            }

            if let Some(ty) = generated_types_for_phase.deserialize.ty.as_mut()
                && let Err(err) = rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Deserialize,
                    types,
                    &generated,
                    &split_types,
                    Some(original.name.as_ref()),
                )
            {
                rewrite_err = Some(err);
            }
            if let Some(err) = rewrite_err {
                return Err(Box::new(err));
            }

            generated.insert(key, generated_types_for_phase);
        }

        for generated_types_for_phase in generated.values_mut() {
            let serialize =
                register_generated_type(&mut out, generated_types_for_phase.serialize.clone());
            let deserialize = Box::new(register_generated_type(
                &mut out,
                (*generated_types_for_phase.deserialize).clone(),
            ));

            generated_types.insert(TypeIdentity::from_ndt(&serialize));
            generated_types.insert(TypeIdentity::from_ndt(&deserialize));

            generated_types_for_phase.serialize = serialize;
            generated_types_for_phase.deserialize = deserialize;
        }

        let registered_generated = generated.clone();
        for generated_types_for_phase in generated.values_mut() {
            if let Some(ty) = generated_types_for_phase.serialize.ty.as_mut() {
                rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Serialize,
                    types,
                    &registered_generated,
                    &split_types,
                    Some(generated_types_for_phase.serialize.name.as_ref()),
                )?;
            }

            if let Some(ty) = generated_types_for_phase.deserialize.ty.as_mut() {
                rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Deserialize,
                    types,
                    &registered_generated,
                    &split_types,
                    Some(generated_types_for_phase.deserialize.name.as_ref()),
                )?;
            }
        }

        out.iter_mut(|ndt| {
            for generated_types_for_phase in generated.values() {
                if ndt.name == generated_types_for_phase.serialize.name {
                    ndt.ty = generated_types_for_phase.serialize.ty.clone();
                    return;
                }

                if ndt.name == generated_types_for_phase.deserialize.name {
                    ndt.ty = generated_types_for_phase.deserialize.ty.clone();
                    return;
                }
            }
        });

        let mut rewrite_err = None;
        out.iter_mut(|ndt| {
            if rewrite_err.is_some() {
                return;
            }

            let ndt_name = ndt.name.to_string();
            let key = TypeIdentity::from_ndt(ndt);

            if split_types.contains(&key) || generated_types.contains(&key) {
                return;
            }

            // As above: apply the container rename before `rewrite_datatype_for_phase`
            // mutates (and, for some enum representations, clears) the container's
            // serde attributes.
            if let Err(err) = rewrite_named_type_for_phase(ndt, PhaseRewrite::Unified) {
                rewrite_err = Some(err);
                return;
            }

            if let Some(ty) = ndt.ty.as_mut()
                && let Err(err) = rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Unified,
                    types,
                    &generated,
                    &split_types,
                    Some(ndt_name.as_str()),
                )
            {
                rewrite_err = Some(err);
            }
        });

        if let Some(err) = rewrite_err {
            return Err(Box::new(err));
        }

        let mut rewrite_err = None;
        out.iter_mut(|ndt| {
            if rewrite_err.is_some() {
                return;
            }

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

            // The wrapper is the split type's public name, so a *symmetric*
            // (effective) container rename must apply to it too; `ndt.ty` is
            // still the untouched original here, so the attrs are intact.
            // Authored-distinct per-phase renames keep the Rust name: there
            // is no single user-authored name to give the wrapper.
            let wrapper_rename = match symmetric_container_rename(ndt) {
                Ok(rename) => rename,
                Err(err) => {
                    rewrite_err = Some(err);
                    return;
                }
            };

            let generic_args = ndt
                .generics
                .iter()
                .map(|generic| {
                    let generic = specta::datatype::Generic::new(generic.name.clone());
                    (generic.clone(), generic.into())
                })
                .collect::<Vec<_>>();

            let mut serialize_variant = Variant::unnamed().build();
            if let Fields::Unnamed(fields) = &mut serialize_variant.fields {
                fields
                    .fields
                    .push(Field::new(serialize.reference(generic_args.clone()).into()));
            }

            let mut deserialize_variant = Variant::unnamed().build();
            if let Fields::Unnamed(fields) = &mut deserialize_variant.fields {
                fields
                    .fields
                    .push(Field::new(deserialize.reference(generic_args).into()));
            }

            let mut wrapper = Enum::default();
            wrapper
                .variants
                .push((Cow::Borrowed("Serialize"), serialize_variant));
            wrapper
                .variants
                .push((Cow::Borrowed("Deserialize"), deserialize_variant));

            ndt.ty = Some(DataType::Enum(wrapper));
            if let Some(rename) = wrapper_rename {
                ndt.name = Cow::Owned(rename);
            }
        });

        if let Some(err) = rewrite_err {
            return Err(Box::new(err));
        }

        Ok(Cow::Owned(out))
    }

    fn map_type(&'_ self, types: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, FormatError> {
        if datatype_is_registered_definition(types, dt) {
            return Ok(Cow::Owned(dt.clone()));
        }

        let mut selected = select_phase_datatype(dt, types, Phase::Serialize);

        // Only the serialize phase is selected and rewritten here, so
        // validation must treat deserialize-only shapes (e.g. a
        // `#[serde(flatten, skip_serializing)]` field, dropped by the
        // serialize rewrite) as unreachable.
        validate::validate_datatype_for_mode_shallow(
            &selected,
            types,
            validate::ApplyMode::Phases,
            Phase::Serialize,
        )?;

        rewrite_datatype_for_phase(
            &mut selected,
            PhaseRewrite::Serialize,
            types,
            &HashMap::new(),
            &HashSet::new(),
            None,
        )?;

        Ok(Cow::Owned(selected))
    }
}

fn datatype_is_registered_definition(types: &Types, dt: &DataType) -> bool {
    types
        .into_unsorted_iter()
        .any(|ndt| ndt.ty.as_ref() == Some(dt))
}

/// Rewrites a [`DataType`] to the requested directional shape for [`PhasesFormat`].
///
/// This is useful for exporter integrations that need deserialize-specific input
/// types and serialize-specific output types while still exporting against the
/// resolved type graph produced by the `map_types` callback from
/// [`PhasesFormat`].
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use specta::{Format as _, Type, Types, datatype::{DataType, Reference}};
/// use specta_serde::{Phase, Phased, PhasesFormat, select_phase_datatype};
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
/// let format = PhasesFormat;
/// let resolved = format.map_types(&types)
///     .expect("PhasesFormat should succeed")
///     .into_owned();
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
///     resolved.get(serialize_reference).unwrap().name,
///     "Filters_Serialize"
/// );
/// assert_eq!(
///     resolved.get(deserialize_reference).unwrap().name,
///     "Filters_Deserialize"
/// );
/// # Ok::<(), specta_serde::Error>(())
/// ```
pub fn select_phase_datatype(dt: &DataType, types: &Types, phase: Phase) -> DataType {
    let mut dt = dt.clone();
    select_phase_datatype_inner(&mut dt, types, phase);
    dt
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseRewrite {
    Unified,
    Serialize,
    Deserialize,
}

fn select_phase_datatype_inner(ty: &mut DataType, types: &Types, phase: Phase) {
    if let Some(resolved) = select_split_wrapper_variant(ty, phase) {
        *ty = resolved;
        select_phase_datatype_inner(ty, types, phase);
        return;
    }

    if let Some(resolved) = select_explicit_phased_type(ty, phase) {
        *ty = resolved;
        select_phase_datatype_inner(ty, types, phase);
        return;
    }

    match ty {
        DataType::Struct(s) => select_phase_fields(&mut s.fields, types, phase),
        DataType::Enum(e) => {
            for (_, variant) in &mut e.variants {
                select_phase_fields(&mut variant.fields, types, phase);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in &mut tuple.elements {
                select_phase_datatype_inner(ty, types, phase);
            }
        }
        DataType::List(list) => select_phase_datatype_inner(&mut list.ty, types, phase),
        DataType::Map(map) => {
            select_phase_datatype_inner(map.key_ty_mut(), types, phase);
            select_phase_datatype_inner(map.value_ty_mut(), types, phase);
        }
        DataType::Intersection(types_) => {
            for ty in types_ {
                select_phase_datatype_inner(ty, types, phase);
            }
        }
        DataType::Nullable(inner) => select_phase_datatype_inner(inner, types, phase),
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Inline { dt, .. } = &mut reference.inner {
                select_phase_datatype_inner(dt, types, phase);
                return;
            }

            let Some(referenced_ndt) = types.get(reference) else {
                return;
            };
            for (_, dt) in named_reference_generics_mut(reference) {
                select_phase_datatype_inner(dt, types, phase);
            }

            if let Some(mut selected) = referenced_ndt
                .ty
                .as_ref()
                .and_then(|ty| select_split_wrapper_variant(ty, phase))
            {
                select_phase_datatype_inner(&mut selected, types, phase);
                *ty = selected;
                return;
            }

            let target_ndt =
                select_split_type_variant(referenced_ndt, types, phase).unwrap_or(referenced_ndt);
            let Reference::Named(new_reference) =
                target_ndt.reference(named_reference_generics(reference).to_vec())
            else {
                unreachable!("named types always produce named references")
            };
            *reference = new_reference;
        }
        DataType::Reference(Reference::Opaque(_))
        | DataType::Generic(_)
        | DataType::Primitive(_) => {}
    }
}

fn select_phase_fields(fields: &mut Fields, types: &Types, phase: Phase) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(ty) = field.ty.as_mut() {
                    select_phase_datatype_inner(ty, types, phase);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(ty) = field.ty.as_mut() {
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

fn select_split_wrapper_variant(ty: &DataType, phase: Phase) -> Option<DataType> {
    let DataType::Enum(wrapper) = ty else {
        return None;
    };

    if wrapper.variants.len() != 2 {
        return None;
    }

    let variant_name = match phase {
        Phase::Serialize => "Serialize",
        Phase::Deserialize => "Deserialize",
    };

    let (_, variant) = wrapper
        .variants
        .iter()
        .find(|(name, _)| name == variant_name)?;
    let Fields::Unnamed(fields) = &variant.fields else {
        return None;
    };

    let [field] = &fields.fields[..] else {
        return None;
    };

    field.ty.clone()
}

fn select_split_type_variant<'a>(
    ndt: &'a NamedDataType,
    types: &'a Types,
    phase: Phase,
) -> Option<&'a NamedDataType> {
    let Some(DataType::Enum(wrapper)) = &ndt.ty else {
        return None;
    };

    if wrapper.variants.len() != 2 {
        return None;
    }

    let variant_name = match phase {
        Phase::Serialize => "Serialize",
        Phase::Deserialize => "Deserialize",
    };

    let (_, variant) = wrapper
        .variants
        .iter()
        .find(|(name, _)| name == variant_name)?;
    let Fields::Unnamed(fields) = &variant.fields else {
        return None;
    };
    let [field] = &fields.fields[..] else {
        return None;
    };
    let Some(DataType::Reference(Reference::Named(reference))) = field.ty.as_ref() else {
        return None;
    };

    types.get(reference)
}

fn named_reference_generics(
    reference: &NamedReference,
) -> &[(specta::datatype::Generic, DataType)] {
    match &reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics,
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => &[],
    }
}

fn named_reference_generics_mut(
    reference: &mut NamedReference,
) -> &mut [(specta::datatype::Generic, DataType)] {
    match &mut reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics,
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => &mut [],
    }
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
        let location = ty.location;
        Self {
            name: ty.name.to_string(),
            module_path: ty.module_path.to_string(),
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
) -> Result<(), Error> {
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
            let container_attrs = SerdeContainerAttrs::from_attributes(&s.attributes)?;
            let container_default = container_attrs.as_ref().is_some_and(|attrs| attrs.default);
            let container_transparent = container_attrs.is_some_and(|attrs| attrs.transparent);
            let container_rename_all = container_rename_all_rule(
                &s.attributes,
                mode,
                "struct rename_all",
                container_name.unwrap_or("<anonymous struct>"),
            )?;

            // A `#[serde(transparent)]` struct serializes as its sole live
            // field's bare value regardless of declared arity, so it is
            // exempt from the declared-arity tuple rewrite below.
            let original_unnamed_arity = match &s.fields {
                Fields::Unnamed(unnamed) if !container_transparent => Some(unnamed.fields.len()),
                Fields::Unit | Fields::Named(_) | Fields::Unnamed(_) => None,
            };
            let conditional_omission_applies = !container_transparent
                && !matches!(&s.fields, Fields::Unnamed(unnamed) if unnamed.fields.len() == 1);

            rewrite_fields_for_phase(
                &mut s.fields,
                mode,
                original_types,
                generated,
                split_types,
                container_rename_all,
                container_default,
                // Generated arity-preserving payloads keep their skipped
                // `ty: None` slots across later passes; see
                // `PRESERVED_ARITY_PAYLOAD_MARKER`.
                s.attributes.contains_key(PRESERVED_ARITY_PAYLOAD_MARKER),
                conditional_omission_applies,
            )?;

            // A declared-multi-field tuple struct stays an array even when
            // `#[serde(skip)]` reduces it to exactly one live field: serde
            // still serializes it as `[value]`, not a bare `value`. The
            // latter is only correct for a *genuine* single-field (newtype)
            // struct, which `Fields::Unnamed` already renders bare by
            // design. Rewriting the live field(s) into a `DataType::Tuple`
            // forces array rendering regardless of the live count.
            // (Exporters must render named `Tuple` definitions -- see
            // specta-swift's `export_type`, which treats them as tuple
            // structs.)
            let live_unnamed_types = match (original_unnamed_arity, &s.fields) {
                (Some(arity), Fields::Unnamed(unnamed))
                    if arity > 1 && unnamed.fields.len() == 1 =>
                {
                    Some(
                        unnamed
                            .fields
                            .iter()
                            .filter_map(|f| f.ty.clone())
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            };
            if let Some(live_unnamed_types) = live_unnamed_types {
                *ty = DataType::Tuple(Tuple::new(live_unnamed_types));
                return Ok(());
            }

            rewrite_struct_repr_for_phase(s, mode, container_name)?;
            normalize_container_attrs_for_phase(&mut s.attributes, mode)?;
            if let Some(intersection) = lower_flattened_struct(s, mode)? {
                *ty = intersection;
                return Ok(());
            }
            if let Some(intersection) = lower_field_aliases_for_phase(&mut s.fields, mode)? {
                *ty = intersection;
            }
        }
        DataType::Enum(e) => {
            filter_enum_variants_for_phase(e, mode)?;
            let container_attrs = SerdeContainerAttrs::from_attributes(&e.attributes)?;
            let repr = EnumRepr::from_attrs(&e.attributes)?;

            for (variant_name, variant) in &mut e.variants {
                let rename_rule =
                    enum_variant_field_rename_rule(&container_attrs, variant, mode, variant_name)?;
                let conditional_omission_applies = !matches!(
                    &variant.fields,
                    Fields::Unnamed(unnamed) if unnamed.fields.len() == 1
                );

                rewrite_fields_for_phase(
                    &mut variant.fields,
                    mode,
                    original_types,
                    generated,
                    split_types,
                    rename_rule,
                    false,
                    true,
                    conditional_omission_applies,
                )?;

                let has_flattened_aliases = matches!(&variant.fields, Fields::Named(named)
                    if named.fields.iter().any(|(_, field)| field_is_flattened(field))
                        && named.fields.iter().any(|(_, field)| field_has_aliases(field)));

                let lowered = if has_flattened_aliases {
                    if matches!(&repr, EnumRepr::Internal { .. }) {
                        None
                    } else {
                        let mut strct = Struct::unit();
                        std::mem::swap(&mut strct.fields, &mut variant.fields);
                        lower_flattened_struct(&mut strct, mode)?
                    }
                } else {
                    lower_field_aliases_for_phase(&mut variant.fields, mode)?
                };

                if let Some(lowered) = lowered {
                    variant.fields = Variant::unnamed().field(Field::new(lowered)).build().fields;
                }
            }

            if rewrite_identifier_enum_for_phase(e, mode, original_types, generated, split_types)? {
                return Ok(());
            }

            rewrite_enum_repr_for_phase(e, mode, original_types)?;
            normalize_enum_attrs_for_phase(e, mode)?;
        }
        DataType::Tuple(tuple) => {
            for ty in &mut tuple.elements {
                rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types, None)?;
            }
        }
        DataType::List(list) => rewrite_datatype_for_phase(
            &mut list.ty,
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
        DataType::Intersection(types_) => {
            for ty in types_ {
                rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types, None)?;
            }
        }
        DataType::Nullable(inner) => {
            rewrite_datatype_for_phase(inner, mode, original_types, generated, split_types, None)?
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Inline { dt, .. } = &mut reference.inner {
                rewrite_datatype_for_phase(dt, mode, original_types, generated, split_types, None)?;
            }

            let Some(referenced_ndt) = original_types.get(reference) else {
                return Ok(());
            };
            let key = TypeIdentity::from_ndt(referenced_ndt);
            for (_, dt) in named_reference_generics_mut(reference) {
                rewrite_datatype_for_phase(dt, mode, original_types, generated, split_types, None)?;
            }

            if !split_types.contains(&key) {
                return Ok(());
            }

            let Some(target) = generated.get(&key) else {
                return Ok(());
            };

            let Reference::Named(reference_from_target) = (match mode {
                PhaseRewrite::Unified => {
                    unreachable!("unified mode should not reference split types")
                }
                PhaseRewrite::Serialize => target
                    .serialize
                    .reference(named_reference_generics(reference).to_vec()),
                PhaseRewrite::Deserialize => target
                    .deserialize
                    .reference(named_reference_generics(reference).to_vec()),
            }) else {
                unreachable!("named types always produce named references")
            };
            *reference = reference_from_target;
        }
        DataType::Reference(Reference::Opaque(_))
        | DataType::Generic(_)
        | DataType::Primitive(_) => {}
    }

    Ok(())
}

fn lower_flattened_struct(
    strct: &mut Struct,
    mode: PhaseRewrite,
) -> Result<Option<DataType>, Error> {
    let Fields::Named(named) = &mut strct.fields else {
        return Ok(None);
    };

    let has_flattened = named
        .fields
        .iter()
        .any(|(_, field)| field_is_flattened(field));
    if !has_flattened {
        return Ok(None);
    }

    let fields = std::mem::take(&mut named.fields);
    let mut base = Struct::named();
    let mut mandatory = Vec::new();
    let mut optional = Vec::new();

    for (name, field) in fields {
        if field_is_flattened(&field) {
            if let Some(ty) = field.ty {
                // `#[serde(flatten)]` on an `Option<T>` (or `Option<Option<T>>`,
                // which serde flattens identically) contributes nothing when
                // `None` and `T`'s fields when `Some`. Track it separately so
                // it can become a union branch instead of being merged
                // unconditionally into the intersection - see
                // `flatten_intersection_with_optionals` for why.
                let (ty, was_nullable) = strip_nullable(ty);
                if field.attributes.contains_key(CONDITIONAL_OMISSION_MARKER) || was_nullable {
                    optional.push(ty);
                } else {
                    mandatory.push(ty);
                }
            }
        } else {
            base.field_mut(name, field);
        }
    }

    let mut base = match base.build() {
        DataType::Struct(base) => base,
        _ => unreachable!("Struct::named always builds a struct"),
    };
    if matches!(&base.fields, Fields::Named(named) if !named.fields.is_empty()) {
        base.attributes = strct.attributes.clone();
        let base = lower_field_aliases_for_phase(&mut base.fields, mode)?
            .unwrap_or(DataType::Struct(base));
        mandatory.insert(0, base);
    }

    Ok(Some(flatten_intersection_with_optionals(
        mandatory, optional,
    )))
}

/// Strips zero or more layers of `Nullable` (i.e. `Option`) off a `DataType`,
/// reporting whether at least one layer was present. `Option<Option<T>>`
/// flattens the same way `Option<T>` does, so nested `Nullable`s all collapse
/// to a single "this part is optional" flag.
fn strip_nullable(mut ty: DataType) -> (DataType, bool) {
    let mut was_nullable = false;
    while let DataType::Nullable(inner) = ty {
        was_nullable = true;
        ty = *inner;
    }
    (ty, was_nullable)
}

/// Builds the `DataType` for a flattened intersection where some parts can be
/// absent because they are `Option<T>` or use `skip_serializing_if`.
///
/// Serde contributes nothing for a flattened `Option<T>` field when it's
/// `None`, and merges `T`'s fields when it's `Some`. Naively intersecting the
/// field's `Nullable(T)` type (i.e. `Base & T | null`) is wrong in both
/// directions: the wire value is never bare `null`, and legitimate `None`
/// output (base fields only) wouldn't satisfy `T`'s required fields.
///
/// Instead, every optional part becomes a branch in a union: for each subset
/// of the optional parts we emit `mandatory & <subset>` (an empty subset with
/// no mandatory parts becomes an empty struct), so the result is exactly the
/// set of shapes serde can actually produce. With no optional parts this
/// degrades to the plain intersection that existed before this function was
/// introduced.
///
/// We deliberately build the union as the *outermost* type (`(Base & Inner) |
/// Base` rather than `Base & (Inner | {})`): TypeScript's `&` binds tighter
/// than `|`, so an intersection nested inside a union never needs parens,
/// whereas `specta-typescript`'s `RenderMode::Normal` intersection renderer
/// joins parts with `" & "` without wrapping union members in parens.
///
/// The branch without an optional part leans on TypeScript's structural
/// typing when *deserializing*: a value carrying only a strict subset of a
/// part's required fields still satisfies the part-less branch, even though
/// serde would reject it. Guarding against that would need `field?: never`
/// markers for each of the part's fields (like the untagged-enum lowering
/// emits), but the part is usually an unresolved [`Reference`] whose fields
/// aren't known here.
fn flatten_intersection_with_optionals(
    mandatory: Vec<DataType>,
    optional: Vec<DataType>,
) -> DataType {
    if optional.is_empty() {
        return DataType::Intersection(mandatory);
    }

    let branch_count = 1usize << optional.len();
    let mut variants = Vec::with_capacity(branch_count);
    for mask in (0..branch_count).rev() {
        let mut parts = mandatory.clone();
        for (idx, part) in optional.iter().enumerate() {
            if mask & (1 << idx) != 0 {
                parts.push(part.clone());
            }
        }

        let branch_ty = match parts.len() {
            0 => Struct::named().build(),
            1 => parts.remove(0),
            _ => DataType::Intersection(parts),
        };
        variants.push((
            Cow::Borrowed(""),
            Variant::unnamed().field(Field::new(branch_ty)).build(),
        ));
    }

    let mut union = Enum::default();
    union.variants = variants;
    // This synthetic present/absent union is already in its final exported
    // shape; see `ENUM_REPR_REWRITTEN_MARKER`. Without this, a later rewrite
    // pass (e.g. `PhasesFormat`'s second pass over split generated types, or
    // simply walking into this enum while it's nested inside another
    // variant's payload) would treat this union as an unrewritten
    // user-authored enum and externally tag it, double-wrapping (or, for
    // unnamed/empty variant names as here, erroring on export).
    union.attributes.insert(ENUM_REPR_REWRITTEN_MARKER, true);
    DataType::Enum(union)
}

fn lower_field_aliases_for_phase(
    fields: &mut Fields,
    mode: PhaseRewrite,
) -> Result<Option<DataType>, Error> {
    if !matches!(mode, PhaseRewrite::Unified | PhaseRewrite::Deserialize) {
        return Ok(None);
    }

    let Fields::Named(named) = fields else {
        return Ok(None);
    };

    if !named
        .fields
        .iter()
        .any(|(_, field)| field_has_aliases(field))
    {
        return Ok(None);
    }

    let mut base = Struct::named();
    let mut parts = Vec::new();

    for (name, field) in std::mem::take(&mut named.fields) {
        let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
            base.field_mut(name, field);
            continue;
        };

        if attrs.aliases.is_empty() {
            base.field_mut(name, field);
            continue;
        }

        let mut accepted_names = Vec::with_capacity(attrs.aliases.len() + 1);
        accepted_names.push(name);
        accepted_names.extend(attrs.aliases.into_iter().map(Cow::Owned));
        parts.push(alias_field_union(accepted_names, field));
    }

    let base = match base.build() {
        DataType::Struct(base) => base,
        _ => unreachable!("Struct::named always builds a struct"),
    };

    if matches!(&base.fields, Fields::Named(named) if !named.fields.is_empty()) {
        parts.insert(0, DataType::Struct(base));
    }

    Ok(Some(DataType::Intersection(parts)))
}

fn field_has_aliases(field: &Field) -> bool {
    SerdeFieldAttrs::from_attributes(&field.attributes)
        .ok()
        .flatten()
        .is_some_and(|attrs| !attrs.aliases.is_empty())
}

fn alias_field_union(names: Vec<Cow<'static, str>>, field: Field) -> DataType {
    let mut aliases = Enum::default();
    let empty_variant = Variant::unnamed().build();

    for name in names {
        let mut field = field.clone();
        field.attributes.remove(parser::FIELD_ALIASES);

        aliases.variants.push((
            Cow::Borrowed(""),
            clone_variant_with_unnamed_fields(
                &empty_variant,
                vec![Field::new(named_fields_datatype(vec![(name, field)]))],
            ),
        ));
    }

    // This synthetic union is already in its final exported shape; see
    // `ENUM_REPR_REWRITTEN_MARKER`.
    aliases.attributes.insert(ENUM_REPR_REWRITTEN_MARKER, true);

    DataType::Enum(aliases)
}

fn field_is_flattened(field: &Field) -> bool {
    SerdeFieldAttrs::from_attributes(&field.attributes)
        .ok()
        .flatten()
        .is_some_and(|attrs| attrs.flatten)
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
    conditional_omission_applies: bool,
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            // Compute this before rewriting consumes `skip_serializing_if`.
            // If skipped slots collapse a declared tuple to one live field,
            // retaining the markers is what lets the renderer keep both the
            // sequence shape and that field's trailing optional marker.
            let preserve_conditional_skip_slots =
                unnamed_conditional_skip_slots_need_preserving(unnamed, mode)?;

            for field in &mut unnamed.fields {
                if should_skip_field_for_mode(field, mode)? {
                    // Always demote to a `ty: None` marker: an explicit
                    // `skip_serializing, skip_deserializing` pair keeps
                    // `field.ty` populated (unlike bare `#[serde(skip)]`,
                    // which the macro erases), and the retain below keys on
                    // `ty`, so a populated skipped field would otherwise
                    // stay in the exported tuple even though serde never
                    // puts it on the wire.
                    *field = skipped_field_marker(field);
                    continue;
                }

                apply_field_attrs(field, mode, container_default)?;
                rewrite_field_for_phase(
                    field,
                    mode,
                    original_types,
                    generated,
                    split_types,
                    conditional_omission_applies,
                )?;
            }

            if !preserve_skipped_unnamed_fields
                && !preserve_conditional_skip_slots
                && !unnamed_skip_slots_need_preserving(unnamed, container_default)?
            {
                unnamed.fields.retain(|field| field.ty.as_ref().is_some());
            }
        }
        Fields::Named(named) => {
            let mut skip_err = None;
            named
                .fields
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

            for (name, field) in &mut named.fields {
                apply_field_attrs(field, mode, container_default)?;

                let serde_attrs = SerdeFieldAttrs::from_attributes(&field.attributes)?;
                let key =
                    phase_field_key(name.as_ref(), serde_attrs.as_ref(), rename_all_rule, mode)?;
                if key != name.as_ref() {
                    *name = Cow::Owned(key);
                }

                rewrite_field_for_phase(
                    field,
                    mode,
                    original_types,
                    generated,
                    split_types,
                    conditional_omission_applies,
                )?;
            }
        }
    }

    Ok(())
}

/// The effective wire key of a named field for `mode`: the explicit
/// directional `rename` if set, else the phase's `rename_all` /
/// `rename_all_fields` rule applied to the default name, else the default
/// name. This is the single source of truth for field keys — unified-mode
/// validation compares the `Serialize` and `Deserialize` results of this
/// same function, so validation and rewriting cannot drift.
pub(crate) fn phase_field_key(
    field_name: &str,
    serde_attrs: Option<&SerdeFieldAttrs>,
    rename_all_rule: Option<RenameRule>,
    mode: PhaseRewrite,
) -> Result<String, Error> {
    if let Some(attrs) = serde_attrs
        && let Some(rename) = select_phase_string(
            mode,
            attrs.rename_serialize.as_deref(),
            attrs.rename_deserialize.as_deref(),
            "field rename",
            field_name,
        )?
    {
        return Ok(rename.to_string());
    }

    Ok(match rename_all_rule {
        Some(rule) => rule.apply_to_field(field_name),
        None => field_name.to_string(),
    })
}

fn rewrite_field_for_phase(
    field: &mut Field,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
    conditional_omission_applies: bool,
) -> Result<(), Error> {
    if let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)?
        && attrs.skip_serializing_if.is_some()
    {
        if conditional_omission_applies
            && matches!(mode, PhaseRewrite::Unified | PhaseRewrite::Serialize)
        {
            field.optional = true;
            if attrs.flatten {
                field.attributes.insert(CONDITIONAL_OMISSION_MARKER, true);
            }

            if mode == PhaseRewrite::Serialize
                && attrs.skip_serializing_if.as_deref() == Some("Option::is_none")
                && let Some(DataType::Nullable(inner)) = field.ty.take()
            {
                field.ty = Some(*inner);
            }
        }
        // The attribute is meaningless on phase-split fields: the _Serialize
        // variant already has `optional = true`, and the _Deserialize variant
        // treats the field as present-or-default. Leaving it attached makes
        // `validate_datatype_for_mode(_, _, ApplyMode::Unified)` reject the
        // already-split variant — a footgun for downstream callers (e.g.
        // tauri-specta's `validate_exported_command`) that run unified
        // validation on the post-`apply_phases` graph.
        field.attributes.remove(parser::FIELD_SKIP_SERIALIZING_IF);
    }

    if mode != PhaseRewrite::Unified {
        // Directional attributes were consumed while producing this
        // phase-specific shape: renames are already applied to the field name
        // by the caller, and fields skipped in this phase were already
        // dropped. Strip them for the same reason as `skip_serializing_if`
        // above: unified-mode validation now rejects one-sided renames/skips,
        // and it must keep accepting the already-split shapes.
        for key in [
            parser::FIELD_RENAME_SERIALIZE,
            parser::FIELD_RENAME_DESERIALIZE,
            parser::FIELD_SKIP_SERIALIZING,
            parser::FIELD_SKIP_DESERIALIZING,
        ] {
            field.attributes.remove(key);
        }
    }

    if let Some(ty) = field.ty.clone()
        && let Some(resolved) = resolve_phased_type(&ty, mode, "field")?
    {
        field.ty = Some(resolved);
    }

    if let Some(ty) = field.ty.as_mut() {
        rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types, None)?;
    }

    Ok(())
}

/// Strips or normalizes directional container serde attrs that were consumed
/// while producing a phase-specific shape, so that unified-mode validation
/// (which rejects one-sided renames) keeps accepting the already-split shapes
/// for downstream callers that validate the post-`apply_phases` graph.
fn normalize_container_attrs_for_phase(
    attrs: &mut specta::datatype::Attributes,
    mode: PhaseRewrite,
) -> Result<(), Error> {
    if mode == PhaseRewrite::Unified {
        return Ok(());
    }

    let Some(parsed) = SerdeContainerAttrs::from_attributes(attrs)? else {
        return Ok(());
    };

    // `rename_all` / `rename_all_fields` have already been applied to the
    // field names by `rewrite_fields_for_phase`.
    for key in [
        parser::CONTAINER_RENAME_ALL_SERIALIZE,
        parser::CONTAINER_RENAME_ALL_DESERIALIZE,
        parser::CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
        parser::CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
    ] {
        attrs.remove(key);
    }

    // The exported name was already fixed from the pre-rewrite attributes
    // (`split_type_name` via `build_from_original`), but keep the container
    // rename normalized to the phase-selected value (rather than dropping it)
    // so the split shape stays self-describing and idempotent: re-inspecting
    // it finds a symmetric rename matching the phase, not a one-sided one.
    let rename = match mode {
        PhaseRewrite::Serialize => parsed.rename_serialize,
        PhaseRewrite::Deserialize => parsed.rename_deserialize,
        PhaseRewrite::Unified => unreachable!("handled above"),
    };
    attrs.remove(parser::CONTAINER_RENAME_SERIALIZE);
    attrs.remove(parser::CONTAINER_RENAME_DESERIALIZE);
    if let Some(rename) = rename {
        attrs.insert(parser::CONTAINER_RENAME_SERIALIZE, rename.clone());
        attrs.insert(parser::CONTAINER_RENAME_DESERIALIZE, rename);
    }

    Ok(())
}

/// Enum counterpart of [`normalize_container_attrs_for_phase`]: tagged enum
/// reprs are rebuilt without the original variant attrs by
/// [`rewrite_enum_repr_for_phase`], but untagged enums keep their
/// `DataType::Enum` shape (the repr rewrite returns early for them) and
/// variant-level `#[serde(untagged)]` variants keep their attrs via
/// [`clone_variant_with_unnamed_fields`], so the consumed directional attrs
/// must be stripped here as well.
fn normalize_enum_attrs_for_phase(e: &mut Enum, mode: PhaseRewrite) -> Result<(), Error> {
    if mode == PhaseRewrite::Unified {
        return Ok(());
    }

    normalize_container_attrs_for_phase(&mut e.attributes, mode)?;

    for (_, variant) in &mut e.variants {
        // Variant renames were already consumed by `serialized_variant_name`
        // (and are wire-irrelevant for untagged variants), `rename_all` was
        // applied to the variant's field names by `rewrite_fields_for_phase`,
        // and variants skipped in this phase were already dropped by
        // `filter_enum_variants_for_phase`.
        for key in [
            parser::VARIANT_RENAME_SERIALIZE,
            parser::VARIANT_RENAME_DESERIALIZE,
            parser::VARIANT_RENAME_ALL_SERIALIZE,
            parser::VARIANT_RENAME_ALL_DESERIALIZE,
            parser::VARIANT_SKIP_SERIALIZING,
            parser::VARIANT_SKIP_DESERIALIZING,
        ] {
            variant.attributes.remove(key);
        }
    }

    Ok(())
}

fn rewrite_struct_repr_for_phase(
    strct: &mut Struct,
    mode: PhaseRewrite,
    container_name: Option<&str>,
) -> Result<(), Error> {
    let Some((tag, rename_serialize, rename_deserialize)) =
        SerdeContainerAttrs::from_attributes(&strct.attributes)?.map(|attrs| {
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

    let Fields::Named(named) = &mut strct.fields else {
        return Ok(());
    };

    if named.fields.iter().any(|(name, field)| {
        name.as_ref() == tag
            && field
                .ty
                .as_ref()
                .is_some_and(is_generated_string_literal_datatype)
    }) {
        return Ok(());
    }

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

    named.fields.insert(
        0,
        (
            Cow::Owned(tag.to_string()),
            Field::new(string_literal_datatype(serialized_name)),
        ),
    );

    Ok(())
}

fn should_skip_field_for_mode(field: &Field, mode: PhaseRewrite) -> Result<bool, Error> {
    if field.attributes.contains_key(SERDE_NEWTYPE_SKIP_IGNORED) {
        return Ok(false);
    }
    let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
        return Ok(false);
    };

    Ok(match mode {
        PhaseRewrite::Serialize => attrs.skip_serializing,
        PhaseRewrite::Deserialize => attrs.skip_deserializing,
        PhaseRewrite::Unified => attrs.skip_serializing || attrs.skip_deserializing,
    })
}

/// Marker recording that a field's *declared Rust type* was `Option<T>`.
/// serde's `missing_field` helper special-cases `Option` on deserialize (a
/// missing key yields `None`), which changes what an adjacently tagged
/// newtype variant requires for its `content` key -- both when the sole
/// field is skipped (collapse handling) and when it is live (a missing
/// `content` deserializes as `None` even without any skip attrs).
///
/// Written exclusively by the `Type` derive from the real field syntax. It
/// is deliberately NOT inferred from the exported datatype here: a
/// `#[specta(type = Option<...>)]` override produces a
/// [`DataType::Nullable`] that serde knows nothing about (and vice versa),
/// so the exported shape is not evidence of serde's `Option` behavior.
/// Hand-built datatypes without the marker conservatively keep `content`
/// required, which is exact for non-`Option` fields and merely conservative
/// for writers of actual `Option` fields (serde accepts an explicit
/// `content: null` for those too).
const NULLABLE_FIELD: &str = "specta:nullable";

/// Whether a tuple struct's skipped `ty: None` slots must survive the
/// rewrite so the declared arity reaches the renderer. serde keeps the
/// sequence representation for skip-reduced tuple structs
/// (`S(#[serde(skip)] u8, #[serde(default)] u8)` serializes to `[2]` and
/// accepts `[]`), so when a default — field-level on a surviving element or
/// container-level — makes elements omittable, collapsing to a bare newtype
/// would lose both the array shape and the deserialize `?`. The condition is
/// deliberately phase-symmetric (attrs, not the phase-computed `optional`
/// flag) so both split halves keep the same arity. Without a default in
/// play the historical collapsed rendering is kept.
fn unnamed_skip_slots_need_preserving(
    unnamed: &UnnamedFields,
    container_default: bool,
) -> Result<bool, Error> {
    if unnamed.fields.len() <= 1 || unnamed.fields.iter().all(|field| field.ty.is_some()) {
        return Ok(false);
    }

    if container_default {
        return Ok(true);
    }

    for field in unnamed_live_fields(unnamed) {
        if SerdeFieldAttrs::from_attributes(&field.attributes)?.is_some_and(|attrs| attrs.default) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn unnamed_conditional_skip_slots_need_preserving(
    unnamed: &UnnamedFields,
    mode: PhaseRewrite,
) -> Result<bool, Error> {
    if unnamed.fields.len() <= 1 {
        return Ok(false);
    }

    let mut has_skipped_slot = false;
    let mut has_live_conditional_omission = false;
    for field in &unnamed.fields {
        let skipped = field.ty.is_none() || should_skip_field_for_mode(field, mode)?;
        has_skipped_slot |= skipped;
        has_live_conditional_omission |= !skipped
            && (field.optional
                || SerdeFieldAttrs::from_attributes(&field.attributes)?
                    .is_some_and(|attrs| attrs.skip_serializing_if.is_some()));
    }

    Ok(has_skipped_slot && has_live_conditional_omission)
}

fn skipped_field_marker(field: &Field) -> Field {
    let mut skipped = Field::default();
    skipped.optional = field.optional;
    skipped.deprecated = field.deprecated.clone();
    skipped.docs = field.docs.clone();
    skipped.attributes = field.attributes.clone();
    skipped
}

fn unnamed_live_fields(unnamed: &UnnamedFields) -> impl Iterator<Item = &Field> {
    unnamed.fields.iter().filter(|field| field.ty.is_some())
}

fn unnamed_live_field_count(unnamed: &UnnamedFields) -> usize {
    unnamed_live_fields(unnamed).count()
}

/// Marker attribute key recorded on an [`Enum`]'s
/// [`Attributes`](specta::datatype::Attributes) once it is in its final
/// serde representation, so a later rewrite pass (e.g. [`PhasesFormat`]'s
/// second pass over split generated types) doesn't transform it again. It is
/// set when [`rewrite_enum_repr_for_phase`] rewrites an enum, and at
/// construction by the synthetic-enum builders ([`string_literal_datatype`],
/// [`alias_field_union`], [`rewrite_identifier_enum_for_phase`]) whose output
/// is already final.
///
/// This replaces a former shape-sniffing heuristic
/// (`enum_repr_already_rewritten`) that tried to detect "already rewritten"
/// enums by inspecting variant shapes. That heuristic was unsound: every
/// shape the transform produces is also a valid shape a user can author
/// directly (for example a single unnamed `&'static str` field lowers to
/// exactly the `DataType::Primitive(Primitive::str)` that the transform
/// emits for a widened tag), so it could both skip enums that had never been
/// rewritten and re-transform enums that had. Tracking the rewrite
/// explicitly makes the check exact.
const ENUM_REPR_REWRITTEN_MARKER: &str = "specta_serde:enum_repr_rewritten";

/// Marker retained on a rewritten deserialize-only `#[serde(other)]` branch.
/// Exporters use it to distinguish the widened catch-all from authored
/// variant-level untagged branches with an equally broad string field.
const ENUM_OTHER_VARIANT_MARKER: &str = "specta_serde:variant_other";

/// Marker attribute recorded on the payload structs generated by
/// [`variant_payload_field`] when skipped `ty: None` slots are kept to
/// preserve the variant's declared tuple arity (so a skip-reduced single
/// live element still renders as a sequence). Later rewrite passes honor it
/// by keeping those marker slots instead of applying the default
/// live-fields-only retain for unnamed structs.
const PRESERVED_ARITY_PAYLOAD_MARKER: &str = "specta_serde:preserved_arity_payload";

/// Marks flattened fields whose payload may be absent from serialization.
/// `Field::optional` cannot carry this distinction because serde defaults can
/// also set it, while defaults do not make flattened payloads optional.
const CONDITIONAL_OMISSION_MARKER: &str = "specta_serde:conditional_omission";

fn rewrite_enum_repr_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &Types,
) -> Result<(), Error> {
    if e.attributes.contains_key(ENUM_REPR_REWRITTEN_MARKER) {
        return Ok(());
    }

    let repr = EnumRepr::from_attrs(&e.attributes)?;
    if matches!(repr, EnumRepr::Untagged) {
        rewrite_container_untagged_unit_variants(e)?;
        // Mark like every other branch so idempotency across passes comes
        // from explicit tracking, never from shape reasoning (the unit
        // rewrite happens to be shape-idempotent, but that's exactly the
        // kind of invariant this marker exists to not rely on). Unlike the
        // other branches we keep the remaining attributes: the untagged repr
        // attribute must stay visible to later consumers (e.g.
        // `internal_tag_payload_compatibility` unions an untagged payload's
        // variants instead of rejecting it).
        e.attributes.insert(ENUM_REPR_REWRITTEN_MARKER, true);
        return Ok(());
    }

    let container_attrs = SerdeContainerAttrs::from_attributes(&e.attributes)?;
    let variants = std::mem::take(&mut e.variants);
    let mut transformed = Vec::with_capacity(variants.len());
    for (variant_name, variant) in variants {
        if variant.skip {
            continue;
        }

        let variant_attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if variant_attrs
            .as_ref()
            .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
        {
            continue;
        }

        if variant_attrs.as_ref().is_some_and(|attrs| attrs.untagged) {
            let mut transformed_variant = transform_untagged_variant(&variant)?;
            // Clear attributes like the other transformed variants below, so
            // later passes (which still filter and walk variants before the
            // marker check) see a plain, already-rewritten variant.
            transformed_variant.attributes = Default::default();
            transformed.push((Cow::Owned(variant_name.into_owned()), transformed_variant));
            continue;
        }

        let serialized_name =
            serialized_variant_name(&variant_name, &variant, &container_attrs, mode)?;
        let aliases = variant_attrs
            .as_ref()
            .filter(|_| matches!(mode, PhaseRewrite::Unified | PhaseRewrite::Deserialize))
            .map(|attrs| attrs.aliases.as_slice())
            .unwrap_or(&[]);
        let names = std::iter::once(serialized_name).chain(aliases.iter().cloned());

        for serialized_name in names {
            let widen_tag = matches!(mode, PhaseRewrite::Unified | PhaseRewrite::Deserialize)
                && variant_attrs.as_ref().is_some_and(|attrs| attrs.other);
            let mut transformed_variant = match &repr {
                EnumRepr::External => {
                    transform_external_variant(serialized_name.clone(), &variant, widen_tag, mode)?
                }
                EnumRepr::Internal { tag } => transform_internal_variant(
                    serialized_name.clone(),
                    tag.as_ref(),
                    &variant,
                    original_types,
                    widen_tag,
                    mode,
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
                        mode,
                    )?
                }
                EnumRepr::Untagged => unreachable!(),
            };

            transformed_variant.attributes = Default::default();
            if widen_tag {
                transformed_variant
                    .attributes
                    .insert(ENUM_OTHER_VARIANT_MARKER, true);
            }
            transformed.push((Cow::Owned(serialized_name), transformed_variant));
        }
    }

    e.variants = transformed;
    e.attributes = Default::default();
    e.attributes.insert(ENUM_REPR_REWRITTEN_MARKER, true);

    Ok(())
}

fn rewrite_identifier_enum_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &Types,
    generated: &HashMap<TypeIdentity, SplitGeneratedTypes>,
    split_types: &HashSet<TypeIdentity>,
) -> Result<bool, Error> {
    let Some(attrs) = SerdeContainerAttrs::from_attributes(&e.attributes)? else {
        return Ok(false);
    };

    if !attrs.variant_identifier && !attrs.field_identifier {
        return Ok(false);
    }

    if mode != PhaseRewrite::Deserialize {
        return Ok(false);
    }

    let container_attrs = SerdeContainerAttrs::from_attributes(&e.attributes)?;
    let mut variants = Vec::new();
    let mut seen = HashSet::new();

    for (variant_name, variant) in e.variants.iter() {
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

        if let Some(variant_attrs) = SerdeVariantAttrs::from_attributes(&variant.attributes)? {
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
        Cow::Borrowed(""),
        identifier_union_variant(DataType::Primitive(specta::datatype::Primitive::u32)),
    ));

    if attrs.field_identifier
        && let Some((_, fallback)) = &e.variants.last()
        && let Fields::Unnamed(unnamed) = &fallback.fields
        && let Some(field) = unnamed.fields.first()
        && let Some(ty) = field.ty.as_ref()
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
        variants.push((Cow::Borrowed(""), identifier_union_variant(fallback_ty)));
    }

    e.attributes = Default::default();
    // This synthetic identifier union is already in its final exported
    // shape; see `ENUM_REPR_REWRITTEN_MARKER`.
    e.attributes.insert(ENUM_REPR_REWRITTEN_MARKER, true);
    e.variants = variants;
    Ok(true)
}

pub(crate) fn container_rename_all_rule(
    attrs: &specta::datatype::Attributes,
    mode: PhaseRewrite,
    context: &str,
    container_name: &str,
) -> Result<Option<RenameRule>, Error> {
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

pub(crate) fn enum_variant_field_rename_rule(
    container_attrs: &Option<SerdeContainerAttrs>,
    variant: &Variant,
    mode: PhaseRewrite,
    variant_name: &str,
) -> Result<Option<RenameRule>, Error> {
    let variant_attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;

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
    if let Fields::Unnamed(fields) = &mut variant.fields {
        fields.fields.push(Field::new(ty));
    }
    variant
}

fn transform_untagged_variant(variant: &Variant) -> Result<Variant, Error> {
    let payload = variant_payload_field(variant)
        .ok_or_else(|| Error::invalid_external_tagged_variant("<untagged variant>"))?;
    Ok(clone_variant_with_unnamed_fields(variant, vec![payload]))
}

/// Serde serializes a unit variant of a container-level `#[serde(untagged)]`
/// enum as `null` -- no discriminant is ever written for untagged enums --
/// but the exporter's serde-agnostic default for a `Fields::Unit` variant is
/// the variant's name as a string literal (e.g. `"A"`). Rewrite unit variants
/// into the same `null`-rendering shape [`transform_untagged_variant`]
/// already produces for skip-all-fields variants.
///
/// Only `Fields::Unit` variants are touched. [`transform_untagged_variant`]
/// (by way of [`variant_payload_field`]) is *not* shape-preserving for a
/// zero-arg tuple variant (`Variant()`): serde serializes that as `[]`, but
/// `variant_payload_field` maps it to the same empty-tuple shape as a unit
/// variant, which would wrongly turn `[]` into `null`. Struct, newtype, and
/// tuple variants (including zero-arg ones) already match serde's untagged
/// wire shape under the exporter's defaults, so they're left untouched here.
///
/// A second pass (e.g. `PhasesFormat`'s cross-reference resolution pass)
/// never reaches this rewrite again: the caller sets
/// `ENUM_REPR_REWRITTEN_MARKER` after applying it. (It also happens to be
/// shape-idempotent - a transformed unit variant becomes `Fields::Unnamed`,
/// which no longer matches `Fields::Unit` - but the marker is the invariant
/// idempotency relies on.)
fn rewrite_container_untagged_unit_variants(e: &mut Enum) -> Result<(), Error> {
    for (_, variant) in &mut e.variants {
        if matches!(variant.fields, Fields::Unit) {
            *variant = transform_untagged_variant(variant)?;
        }
    }

    Ok(())
}

fn filter_enum_variants_for_phase(e: &mut Enum, mode: PhaseRewrite) -> Result<(), Error> {
    let mut filter_err = None;
    e.variants.retain(|(_, variant)| {
        if variant.skip {
            return false;
        }

        match SerdeVariantAttrs::from_attributes(&variant.attributes) {
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

pub(crate) fn serialized_variant_name(
    variant_name: &str,
    variant: &Variant,
    container_attrs: &Option<SerdeContainerAttrs>,
    mode: PhaseRewrite,
) -> Result<String, Error> {
    let variant_attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;

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
) -> Result<Option<&'a str>, Error> {
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
) -> Result<Option<RenameRule>, Error> {
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

fn resolve_phased_type(
    ty: &DataType,
    mode: PhaseRewrite,
    path: &str,
) -> Result<Option<DataType>, Error> {
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
                "`specta_serde::Phased<Serialize, Deserialize>` requires `PhasesFormat`",
            ));
        }
        PhaseRewrite::Serialize => Some(phased.serialize.clone()),
        PhaseRewrite::Deserialize => Some(phased.deserialize.clone()),
    })
}

fn conversion_datatype_for_mode(
    ty: &DataType,
    mode: PhaseRewrite,
) -> Result<Option<DataType>, Error> {
    let attrs = match ty {
        DataType::Struct(s) => &s.attributes,
        DataType::Enum(e) => &e.attributes,
        _ => return Ok(None),
    };

    select_conversion_target(attrs, mode)
}

fn select_conversion_target(
    attrs: &specta::datatype::Attributes,
    mode: PhaseRewrite,
) -> Result<Option<DataType>, Error> {
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
                resolved
                    .and_then(|attrs| {
                        attrs
                            .into
                            .as_ref()
                            .map(|v| format!("into({})", v.type_src))
                            .or_else(|| {
                                attrs.from.as_ref().map(|v| format!("from({})", v.type_src))
                            })
                            .or_else(|| {
                                attrs
                                    .try_from
                                    .as_ref()
                                    .map(|v| format!("try_from({})", v.type_src))
                            })
                    })
                    .unwrap_or_else(|| "<container>".to_string()),
                resolved.and_then(|attrs| attrs.into.as_ref().map(|v| v.type_src.clone())),
                resolved.and_then(|attrs| {
                    attrs.from.as_ref().map(|v| v.type_src.clone()).or_else(|| {
                        attrs
                            .try_from
                            .as_ref()
                            .map(|v| format!("try_from({})", v.type_src))
                    })
                }),
            )),
        },
    }
}

fn transform_external_variant(
    serialized_name: String,
    variant: &Variant,
    widen_tag: bool,
    mode: PhaseRewrite,
) -> Result<Variant, Error> {
    // Only a genuine newtype variant (declared arity 1) collapses to a unit
    // string when its sole field is skipped. A declared-multi-field tuple
    // variant stays a tuple variant even when *serde* skips reduce it to 0
    // live fields -- serde still requires (and emits) an empty array payload,
    // so it must render as `{ Name: [] }`, not a bare string literal. A
    // payload hidden by `#[specta(skip)]` instead follows the hidden-field
    // convention (the bare string, like the pre-rewrite behavior): serde
    // still transports the values, so fabricating `[]` would be wrong.
    let collapses_to_unit =
        variant_collapses_to_unit(variant) || variant_payload_is_hidden(variant, mode)?;

    Ok(match &variant.fields {
        Fields::Unit => clone_variant_with_unnamed_fields(
            variant,
            vec![Field::new(if widen_tag {
                DataType::Primitive(Primitive::str)
            } else {
                string_literal_datatype(serialized_name)
            })],
        ),
        _ if collapses_to_unit => clone_variant_with_unnamed_fields(
            variant,
            vec![Field::new(if widen_tag {
                DataType::Primitive(Primitive::str)
            } else {
                string_literal_datatype(serialized_name)
            })],
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
    mode: PhaseRewrite,
) -> Result<Variant, Error> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(if widen_tag {
            DataType::Primitive(Primitive::str)
        } else {
            string_literal_datatype(serialized_name.clone())
        }),
    )];

    // Every non-unit variant kind carries a `content` key, even when the
    // payload itself is empty: `Foo {}` -> `content: {}`, `Foo()` -> `content:
    // []`, and a declared-multi-field tuple reduced to 0 live fields by
    // `#[serde(skip)]` -> `content: []`. Only genuine unit variants omit it in
    // both phases; newtype variants collapsed to unit by a skipped sole field
    // omit it on serialize only (see below).
    if variant_payload_is_hidden(variant, mode)? {
        // Hidden-field convention: serde still transports the
        // `#[specta(skip)]`ped values under `content`, but the user asked to
        // hide them, so `content` is omitted entirely (fabricating `[]`
        // would claim a wire shape serde rejects).
    } else if !variant_collapses_to_unit(variant) {
        let mut payload = variant_payload_field(variant)
            .ok_or_else(|| Error::invalid_adjacent_tagged_variant(serialized_name.clone()))?;
        // serde's adjacent deserializer routes a missing `content` key
        // through `missing_field`, which yields `None` for a *newtype*
        // `Option` payload even without any skip attrs -- `{"t":"V"}`
        // deserializes to `V(None)` while the serializer always emits
        // `content`. Deserialize-facing shapes therefore keep `content`
        // optional, mirroring `field_is_optional_for_mode`'s treatment of
        // missing-input-allowed (`#[serde(default)]`) fields: optional in
        // the deserialize and unified phases, required in serialize.
        if matches!(mode, PhaseRewrite::Deserialize | PhaseRewrite::Unified)
            && sole_live_field_is_nullable(variant)
        {
            payload.optional = true;
        }
        fields.push((Cow::Owned(content.to_string()), payload));
    } else if !matches!(variant.fields, Fields::Unit) && sole_field_is_serde_skipped(variant, mode)?
    {
        // A newtype variant collapsed to unit by a serde-skipped sole field
        // is asymmetric under adjacent tagging: serde's serializer omits
        // `content` entirely (like a unit variant), but its deserializer
        // still requires `content` to be present and exactly `null` --
        // UNLESS the skipped field is an `Option`, which serde's
        // `missing_field` helper deserializes as `None` when the key is
        // absent, making `content` genuinely optional on deserialize too.
        // `DataType::Tuple(vec![])` renders as `null`.
        //
        // A sole field hidden WITHOUT a serde skip (`#[specta(skip)]` or a
        // hand-built `Field { ty: None, .. }`) does not take this branch:
        // serde still transports the payload symmetrically, the skip merely
        // hides it from the export, so `content` is omitted in every mode
        // (the `else` fall-through), matching the hidden-field convention.
        let skipped_nullable = skipped_sole_field_is_nullable(variant);
        match mode {
            PhaseRewrite::Serialize => {}
            PhaseRewrite::Deserialize => {
                let mut field = Field::new(DataType::Tuple(Tuple::new(vec![])));
                field.optional = skipped_nullable;
                fields.push((Cow::Owned(content.to_string()), field));
            }
            PhaseRewrite::Unified => {
                // For `Option` fields `content?: null` is exact for both
                // directions. The non-`Option` case is rejected up front by
                // `validate_adjacent_collapsed_newtype_variants` (unified
                // mode can't represent its ser/de asymmetry), so this branch
                // is defensive best-effort for datatypes that bypassed
                // validation: it accepts serde's serialize output (no key)
                // while still letting callers write the `content: null` the
                // deserializer demands.
                let mut field = Field::new(DataType::Tuple(Tuple::new(vec![])));
                field.optional = true;
                fields.push((Cow::Owned(content.to_string()), field));
            }
        }
    }

    Ok(clone_variant_with_named_fields(variant, fields))
}

fn transform_internal_variant(
    serialized_name: String,
    tag: &str,
    variant: &Variant,
    original_types: &Types,
    widen_tag: bool,
    mode: PhaseRewrite,
) -> Result<Variant, Error> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(if widen_tag {
            DataType::Primitive(Primitive::str)
        } else {
            string_literal_datatype(serialized_name.clone())
        }),
    )];

    match &variant.fields {
        Fields::Unit => {}
        Fields::Named(named) => {
            // If any named field is `#[serde(flatten)]`, serde merges its
            // contents at the variant's top level alongside the tag. Mirror
            // the unnamed-payload path: build an Intersection of `{tag}`,
            // each flattened field's payload type, and (if any) a struct of
            // the remaining non-flattened fields. Without this, the flatten
            // attribute survives to the typescript exporter, which writes
            // the field literally as `inner: T` instead of merging it.
            let has_flattened = named
                .fields
                .iter()
                .any(|(_, field)| field_is_flattened(field));
            if has_flattened {
                let mut mandatory_parts: Vec<DataType> = Vec::new();
                let mut optional_parts: Vec<DataType> = Vec::new();
                let mut leftover: Vec<(Cow<'static, str>, Field)> = Vec::new();
                for (name, field) in named.fields.iter().cloned() {
                    if field_is_flattened(&field) {
                        if let Some(ty) = field.ty {
                            // See `flatten_intersection_with_optionals` on why
                            // a flattened `Option<T>` field needs to become a
                            // union branch rather than an unconditional
                            // intersection part.
                            let (ty, was_nullable) = strip_nullable(ty);
                            if field.attributes.contains_key(CONDITIONAL_OMISSION_MARKER)
                                || was_nullable
                            {
                                optional_parts.push(ty);
                            } else {
                                mandatory_parts.push(ty);
                            }
                        }
                    } else {
                        leftover.push((name, field));
                    }
                }
                let mut mandatory = Vec::with_capacity(mandatory_parts.len() + 2);
                mandatory.push(named_fields_datatype(fields));
                if !leftover.is_empty() {
                    let DataType::Struct(mut leftover) = named_fields_datatype(leftover) else {
                        unreachable!("named_fields_datatype always builds a struct")
                    };
                    mandatory.push(
                        lower_field_aliases_for_phase(&mut leftover.fields, mode)?
                            .unwrap_or(DataType::Struct(leftover)),
                    );
                }
                mandatory.extend(mandatory_parts);
                return Ok(clone_variant_with_unnamed_fields(
                    variant,
                    vec![Field::new(flatten_intersection_with_optionals(
                        mandatory,
                        optional_parts,
                    ))],
                ));
            }
            fields.extend(named.fields.iter().cloned());
        }
        Fields::Unnamed(unnamed) => {
            if variant_payload_is_hidden(variant, mode)? {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "a payload hidden only from Specta cannot be represented inside an internal tag",
                ));
            }

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
            let payload_ty = payload_field.ty.clone().expect("checked above");
            let Some(payload) = internal_tag_payload_compatibility(
                &payload_ty,
                original_types,
                &mut HashSet::new(),
                mode,
            )?
            else {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "payload cannot be merged with a tag",
                ));
            };

            if !payload.is_effectively_empty {
                return Ok(clone_variant_with_unnamed_fields(
                    variant,
                    vec![Field::new(DataType::Intersection(vec![
                        named_fields_datatype(fields),
                        payload.replacement.unwrap_or(payload_ty),
                    ]))],
                ));
            }
        }
    }

    Ok(clone_variant_with_named_fields(variant, fields))
}

fn named_fields_datatype(fields: Vec<(Cow<'static, str>, Field)>) -> DataType {
    let mut builder = Struct::named();
    for (name, field) in fields {
        builder = builder.field(name, field);
    }

    builder.build()
}

fn string_literal_datatype(value: String) -> DataType {
    let mut value_enum = Enum::default();
    value_enum
        .variants
        .push((Cow::Owned(value), Variant::unit()));
    // This synthetic single-variant literal (e.g. a tag field's type) is
    // already in its final exported shape; see `ENUM_REPR_REWRITTEN_MARKER`.
    value_enum
        .attributes
        .insert(ENUM_REPR_REWRITTEN_MARKER, true);
    DataType::Enum(value_enum)
}

fn is_generated_string_literal_datatype(ty: &DataType) -> bool {
    let DataType::Enum(e) = ty else {
        return false;
    };

    let Some((_, variant)) = e.variants.first() else {
        return false;
    };

    if e.variants.len() != 1 {
        return false;
    }

    match &variant.fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) if fields.fields.len() == 1 => fields
            .fields
            .first()
            .and_then(|field| field.ty.as_ref())
            .is_some_and(is_generated_string_literal_datatype),
        _ => false,
    }
}

/// A variant/struct whose declared arity collapses to serde's unit
/// representation: genuine unit fields, and newtype (declared arity 1)
/// unnamed fields whose sole field has been skipped. Every other shape
/// (including empty tuples/structs and multi-field tuples reduced to 0 or 1
/// live fields by `#[serde(skip)]`) still serializes with a payload -- an
/// empty tuple variant serializes as `[]`, an empty struct variant as `{}`,
/// and a declared-multi-field tuple stays a sequence even when skips reduce
/// it to 0 or 1 live fields.
fn variant_collapses_to_unit(variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unit => true,
        Fields::Unnamed(unnamed) => {
            unnamed.fields.len() == 1 && unnamed_live_field_count(unnamed) == 0
        }
        Fields::Named(_) => false,
    }
}

/// Whether a tuple variant's payload is *hidden* rather than serde-skipped:
/// zero live fields, at least one of which lacks a serde skip for the given
/// phase (`#[specta(skip)]` or a hand-built `Field { ty: None, .. }`). serde
/// still transports those values on the wire, so no payload shape is known
/// to the export -- the variant follows the hidden-field convention (payload
/// omitted) instead of fabricating an `[]` serde would reject. A genuinely
/// empty tuple variant (`V()`) and an all-serde-skipped one really are `[]`
/// on the wire and are NOT hidden.
fn variant_payload_is_hidden(variant: &Variant, mode: PhaseRewrite) -> Result<bool, Error> {
    let Fields::Unnamed(unnamed) = &variant.fields else {
        return Ok(false);
    };
    if unnamed.fields.is_empty() || unnamed_live_field_count(unnamed) != 0 {
        return Ok(false);
    }

    for field in &unnamed.fields {
        if !should_skip_field_for_mode(field, mode)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Whether a collapsed newtype variant's sole field carries a *serde* skip
/// for the given phase, as opposed to being hidden by `#[specta(skip)]` (or a
/// hand-built `Field { ty: None, .. }`), which serde knows nothing about.
fn sole_field_is_serde_skipped(variant: &Variant, mode: PhaseRewrite) -> Result<bool, Error> {
    match &variant.fields {
        Fields::Unnamed(unnamed) => match unnamed.fields.as_slice() {
            [field] => should_skip_field_for_mode(field, mode),
            _ => Ok(false),
        },
        Fields::Unit | Fields::Named(_) => Ok(false),
    }
}

/// Whether a newtype variant's *live* sole payload was declared as
/// `Option<T>` (see [`NULLABLE_FIELD`]). Declared arity must be 1: a
/// multi-field tuple variant deserializes as a sequence, so serde's
/// `missing_field` `Option` special case never applies to it.
fn sole_live_field_is_nullable(variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unnamed(unnamed) => match unnamed.fields.as_slice() {
            [field] => field.ty.is_some() && field.attributes.contains_key(NULLABLE_FIELD),
            _ => false,
        },
        Fields::Unit | Fields::Named(_) => false,
    }
}

/// Whether a collapsed newtype variant's skipped sole field was declared as
/// `Option<T>` (see [`NULLABLE_FIELD`]; the attribute survives on the
/// `ty: None` placeholder produced by [`skipped_field_marker`]).
fn skipped_sole_field_is_nullable(variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .first()
            .is_some_and(|field| field.attributes.contains_key(NULLABLE_FIELD)),
        Fields::Unit | Fields::Named(_) => false,
    }
}

/// A [`DataType`] that always renders as an empty array (`[]`), as opposed to
/// `DataType::Tuple(Tuple::new(vec![]))`, which renders as `null` because it
/// also represents Rust's unit type `()`.
fn empty_array_datatype() -> DataType {
    Struct::unnamed().build()
}

fn variant_payload_field(variant: &Variant) -> Option<Field> {
    match &variant.fields {
        Fields::Unit => Some(Field::new(DataType::Tuple(Tuple::new(vec![])))),
        Fields::Named(named) => {
            let mut out = Struct::named();
            for (name, field) in named.fields.iter().cloned() {
                out.field_mut(name, field);
            }
            Some(Field::new(out.build()))
        }
        Fields::Unnamed(unnamed) => {
            let original_unnamed_len = unnamed.fields.len();

            let non_skipped = unnamed_live_fields(unnamed).collect::<Vec<_>>();

            match non_skipped.as_slice() {
                // A newtype (declared arity 1) whose sole field is skipped
                // collapses to serde's *unit* payload (`null`), unlike
                // zero-arg / multi-field all-skipped tuples which stay `[]`.
                [] if original_unnamed_len == 1 => {
                    Some(Field::new(DataType::Tuple(Tuple::new(vec![]))))
                }
                [] => Some(Field::new(empty_array_datatype())),
                [single] if original_unnamed_len == 1 => Some((*single).clone()),
                // A bare `Tuple` has no `Field`s, so it cannot carry the
                // `optional` flag a phase rewrite sets on a defaulted
                // trailing element. Keep an unnamed struct in that case so
                // exporters can render the trailing `?` — including the
                // skipped `ty: None` marker slots, so the declared arity
                // survives and a skip-reduced single live element still
                // renders as a sequence (`[number?]`) rather than
                // collapsing to a bare newtype.
                _ if non_skipped.iter().any(|field| field.optional) => {
                    let mut out = Struct::unnamed();
                    for field in &unnamed.fields {
                        out.field_mut(field.clone());
                    }
                    let mut built = out.build();
                    if let DataType::Struct(strct) = &mut built {
                        strct
                            .attributes
                            .insert(PRESERVED_ARITY_PAYLOAD_MARKER, true);
                    }
                    Some(Field::new(built))
                }
                _ => Some(Field::new(DataType::Tuple(Tuple::new(
                    non_skipped
                        .iter()
                        .filter_map(|field| field.ty.clone())
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
    transformed.skip = original.skip;
    transformed.docs = original.docs.clone();
    transformed.deprecated = original.deprecated.clone();
    transformed.attributes = original.attributes.clone();
    transformed
}

fn clone_variant_with_unnamed_fields(original: &Variant, fields: Vec<Field>) -> Variant {
    let mut builder = Variant::unnamed();
    for field in fields {
        builder = builder.field(field);
    }

    let mut transformed = builder.build();
    transformed.skip = original.skip;
    transformed.docs = original.docs.clone();
    transformed.deprecated = original.deprecated.clone();
    transformed.attributes = original.attributes.clone();
    transformed
}

struct InternalTagPayloadCompatibility {
    is_effectively_empty: bool,
    replacement: Option<DataType>,
}

impl InternalTagPayloadCompatibility {
    fn empty() -> Self {
        Self {
            is_effectively_empty: true,
            replacement: None,
        }
    }

    fn merge_as_is() -> Self {
        Self {
            is_effectively_empty: false,
            replacement: None,
        }
    }
}

fn internal_tag_payload_compatibility(
    ty: &DataType,
    original_types: &Types,
    seen: &mut HashSet<Reference>,
    mode: PhaseRewrite,
) -> Result<Option<InternalTagPayloadCompatibility>, Error> {
    if let Some(converted) = conversion_datatype_for_mode(ty, mode)?
        && converted != *ty
    {
        let Some(mut compatibility) =
            internal_tag_payload_compatibility(&converted, original_types, seen, mode)?
        else {
            return Ok(None);
        };
        if !compatibility.is_effectively_empty && compatibility.replacement.is_none() {
            compatibility.replacement = Some(converted);
        }
        return Ok(Some(compatibility));
    }

    match ty {
        DataType::Map(_) | DataType::Generic(_) => {
            Ok(Some(InternalTagPayloadCompatibility::merge_as_is()))
        }
        DataType::Struct(strct) => {
            if SerdeContainerAttrs::from_attributes(&strct.attributes)?
                .is_some_and(|attrs| attrs.transparent)
            {
                let has_hidden_wire_data =
                    match &strct.fields {
                        Fields::Unit => Ok(false),
                        Fields::Unnamed(unnamed) => unnamed.fields.iter().try_fold(
                            false,
                            |hidden, field| -> Result<_, Error> {
                                Ok(hidden
                                    || (field.ty.is_none()
                                        && !should_skip_field_for_mode(field, mode)?))
                            },
                        ),
                        Fields::Named(named) => named.fields.iter().try_fold(
                            false,
                            |hidden, (_, field)| -> Result<_, Error> {
                                Ok(hidden
                                    || (field.ty.is_none()
                                        && !should_skip_field_for_mode(field, mode)?))
                            },
                        ),
                    }?;
                if has_hidden_wire_data {
                    return Ok(None);
                }

                let payload_fields = match &strct.fields {
                    Fields::Unit => return Ok(Some(InternalTagPayloadCompatibility::empty())),
                    Fields::Unnamed(unnamed) => unnamed
                        .fields
                        .iter()
                        .filter_map(|field| field.ty.as_ref())
                        .collect::<Vec<_>>(),
                    Fields::Named(named) => named
                        .fields
                        .iter()
                        .filter_map(|(_, field)| field.ty.as_ref())
                        .collect::<Vec<_>>(),
                };

                let [inner_ty] = payload_fields.as_slice() else {
                    if payload_fields.is_empty() {
                        return Ok(Some(InternalTagPayloadCompatibility::empty()));
                    }

                    return Ok(None);
                };

                return internal_tag_payload_compatibility(inner_ty, original_types, seen, mode);
            }

            match &strct.fields {
                Fields::Unit => Ok(Some(InternalTagPayloadCompatibility::empty())),
                Fields::Named(named) => {
                    let mut is_effectively_empty = true;
                    for (_, field) in &named.fields {
                        if field.ty.is_some() {
                            is_effectively_empty = false;
                        } else if !should_skip_field_for_mode(field, mode)? {
                            return Ok(None);
                        }
                    }
                    Ok(Some(InternalTagPayloadCompatibility {
                        is_effectively_empty,
                        replacement: None,
                    }))
                }
                // Serde's newtype structs delegate their payload back into
                // `TaggedSerializer`. Empty and multi-field tuple structs use
                // the unsupported tuple-struct path instead.
                Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                    let Some(inner_ty) = unnamed.fields[0].ty.as_ref() else {
                        return if should_skip_field_for_mode(&unnamed.fields[0], mode)? {
                            Ok(Some(InternalTagPayloadCompatibility::empty()))
                        } else {
                            Ok(None)
                        };
                    };
                    internal_tag_payload_compatibility(inner_ty, original_types, seen, mode)
                }
                Fields::Unnamed(_) => Ok(None),
            }
        }
        DataType::Tuple(tuple) => Ok(tuple
            .elements
            .is_empty()
            .then(InternalTagPayloadCompatibility::empty)),
        DataType::Intersection(types) => {
            let mut is_effectively_empty = true;
            let mut replacements = Vec::with_capacity(types.len());
            let mut was_rewritten = false;

            for ty in types {
                let Some(part) =
                    internal_tag_payload_compatibility(ty, original_types, seen, mode)?
                else {
                    return Ok(None);
                };

                is_effectively_empty &= part.is_effectively_empty;
                was_rewritten |= part.replacement.is_some();
                replacements.push(part.replacement.unwrap_or_else(|| ty.clone()));
            }

            Ok(Some(InternalTagPayloadCompatibility {
                is_effectively_empty,
                replacement: was_rewritten.then_some(DataType::Intersection(replacements)),
            }))
        }
        DataType::Reference(Reference::Named(reference)) => {
            let referenced_ty = match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => (**dt).clone(),
                NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => {
                    let Some(ty) = original_types
                        .get(reference)
                        .and_then(|referenced| referenced.ty.as_ref())
                    else {
                        return Ok(None);
                    };
                    ty.clone()
                }
            };
            let mut referenced_ty = referenced_ty;
            substitute_generics(&mut referenced_ty, named_reference_generics(reference));

            let key = Reference::Named(reference.clone());
            if !seen.insert(key.clone()) {
                return Ok(Some(InternalTagPayloadCompatibility::merge_as_is()));
            }

            let mut compatible =
                internal_tag_payload_compatibility(&referenced_ty, original_types, seen, mode)?;
            seen.remove(&key);
            if let Some(replacement) = compatible
                .as_mut()
                .and_then(|payload| payload.replacement.as_mut())
            {
                substitute_generics(replacement, named_reference_generics(reference));
            }
            Ok(compatible)
        }
        DataType::Enum(enm)
            if enm.attributes.contains_key(ENUM_REPR_REWRITTEN_MARKER)
                && !matches!(
                    EnumRepr::from_attrs(&enm.attributes),
                    Ok(EnumRepr::Untagged)
                ) =>
        {
            Ok(Some(contextualize_rewritten_external_units(
                enm,
                original_types,
                seen,
                mode,
            )?))
        }
        DataType::Enum(enm) => match EnumRepr::from_attrs(&enm.attributes) {
            Ok(EnumRepr::Untagged) => {
                let mut is_effectively_empty = true;
                let mut rewritten = enm.clone();
                let mut changed = false;
                for ((_, variant), (_, rewritten_variant)) in
                    enm.variants.iter().zip(&mut rewritten.variants)
                {
                    let Some(variant_payload) = internal_tag_variant_payload_compatibility(
                        variant,
                        original_types,
                        seen,
                        mode,
                    )?
                    else {
                        return Ok(None);
                    };

                    is_effectively_empty &= variant_payload.is_effectively_empty;
                    let replacement = variant_payload.replacement.or_else(|| {
                        variant_payload
                            .is_effectively_empty
                            .then(|| named_fields_datatype(Vec::new()))
                    });
                    if let Some(replacement) = replacement {
                        *rewritten_variant = clone_variant_with_unnamed_fields(
                            rewritten_variant,
                            vec![Field::new(replacement)],
                        );
                        changed = true;
                    }
                }

                Ok(Some(InternalTagPayloadCompatibility {
                    is_effectively_empty,
                    replacement: changed.then_some(DataType::Enum(rewritten)),
                }))
            }
            Ok(EnumRepr::External) => {
                if !external_enum_requires_contextual_rewrite(enm, mode)? {
                    return Ok(Some(InternalTagPayloadCompatibility::merge_as_is()));
                }
                Ok(Some(InternalTagPayloadCompatibility {
                    is_effectively_empty: false,
                    replacement: Some(external_enum_tagged_payload_datatype(
                        enm,
                        original_types,
                        mode,
                    )?),
                }))
            }
            Ok(EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. }) => {
                Ok(Some(InternalTagPayloadCompatibility::merge_as_is()))
            }
            Err(_) => Ok(None),
        },
        DataType::Primitive(_)
        | DataType::List(_)
        | DataType::Nullable(_)
        | DataType::Reference(Reference::Opaque(_)) => Ok(None),
    }
}

pub(crate) fn internal_tag_payload_requires_contextual_rewrite(
    ty: &DataType,
    types: &Types,
) -> Result<bool, Error> {
    enum TransparentPayload<'a> {
        NotTransparent,
        Payload(&'a DataType),
        Contextual,
    }

    fn transparent_payload(
        strct: &Struct,
        mode: PhaseRewrite,
    ) -> Result<TransparentPayload<'_>, Error> {
        if !SerdeContainerAttrs::from_attributes(&strct.attributes)?
            .is_some_and(|attrs| attrs.transparent)
        {
            return Ok(TransparentPayload::NotTransparent);
        }

        let fields = match &strct.fields {
            Fields::Unit => Vec::new(),
            Fields::Unnamed(unnamed) => unnamed.fields.iter().collect::<Vec<_>>(),
            Fields::Named(named) => named
                .fields
                .iter()
                .map(|(_, field)| field)
                .collect::<Vec<_>>(),
        };
        let mut live = Vec::new();
        for field in fields {
            if should_skip_field_for_mode(field, mode)? {
                continue;
            }
            let Some(ty) = &field.ty else {
                return Ok(TransparentPayload::Contextual);
            };
            live.push(ty);
        }

        Ok(match live.as_slice() {
            [ty] => TransparentPayload::Payload(ty),
            _ => TransparentPayload::Contextual,
        })
    }

    fn empty_payload_uses_unit_encoding(
        ty: &DataType,
        types: &Types,
        seen: &mut HashSet<Reference>,
        mode: PhaseRewrite,
    ) -> Result<bool, Error> {
        match ty {
            DataType::Struct(strct) => match transparent_payload(strct, mode)? {
                TransparentPayload::Payload(ty) => {
                    empty_payload_uses_unit_encoding(ty, types, seen, mode)
                }
                TransparentPayload::Contextual => Ok(true),
                TransparentPayload::NotTransparent => match &strct.fields {
                    Fields::Unit => Ok(true),
                    Fields::Named(_) => Ok(false),
                    Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                        unnamed.fields[0].ty.as_ref().map_or(Ok(true), |ty| {
                            empty_payload_uses_unit_encoding(ty, types, seen, mode)
                        })
                    }
                    Fields::Unnamed(_) => Ok(true),
                },
            },
            DataType::Tuple(tuple) => Ok(tuple.elements.is_empty()),
            DataType::Reference(Reference::Named(reference)) => {
                let key = Reference::Named(reference.clone());
                if !seen.insert(key.clone()) {
                    return Ok(false);
                }
                let referenced_ty = match &reference.inner {
                    NamedReferenceType::Inline { dt, .. } => Some((**dt).clone()),
                    NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => {
                        types
                            .get(reference)
                            .and_then(|ndt| ndt.ty.as_ref())
                            .cloned()
                    }
                };
                let result = match referenced_ty {
                    Some(mut ty) => {
                        substitute_generics(&mut ty, named_reference_generics(reference));
                        empty_payload_uses_unit_encoding(&ty, types, seen, mode)
                    }
                    None => Ok(true),
                };
                seen.remove(&key);
                result
            }
            DataType::Enum(enm)
                if matches!(EnumRepr::from_attrs(&enm.attributes)?, EnumRepr::Untagged) =>
            {
                for (_, variant) in &enm.variants {
                    match &variant.fields {
                        Fields::Unit => return Ok(true),
                        Fields::Named(_) => {}
                        Fields::Unnamed(_) => {
                            let Some(payload) =
                                variant_payload_field(variant).and_then(|field| field.ty)
                            else {
                                return Ok(true);
                            };
                            if empty_payload_uses_unit_encoding(&payload, types, seen, mode)? {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            DataType::Intersection(parts) => {
                for part in parts {
                    if empty_payload_uses_unit_encoding(part, types, seen, mode)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            DataType::Map(_) => Ok(false),
            DataType::Enum(_)
            | DataType::Primitive(_)
            | DataType::List(_)
            | DataType::Nullable(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => Ok(true),
        }
    }

    fn inner(
        ty: &DataType,
        types: &Types,
        seen: &mut HashSet<Reference>,
        mode: PhaseRewrite,
    ) -> Result<bool, Error> {
        if let Some(converted) = conversion_datatype_for_mode(ty, mode)?
            && converted != *ty
        {
            return inner(&converted, types, seen, mode);
        }

        match ty {
            DataType::Struct(strct) => match transparent_payload(strct, mode)? {
                TransparentPayload::Payload(ty) => inner(ty, types, seen, mode),
                TransparentPayload::Contextual => Ok(true),
                TransparentPayload::NotTransparent => match &strct.fields {
                    Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                        let field = &unnamed.fields[0];
                        match &field.ty {
                            Some(ty) => inner(ty, types, seen, mode),
                            None => Ok(!should_skip_field_for_mode(field, mode)?),
                        }
                    }
                    Fields::Unit => Ok(true),
                    Fields::Named(_) | Fields::Unnamed(_) => Ok(false),
                },
            },
            DataType::Reference(Reference::Named(reference)) => {
                let key = Reference::Named(reference.clone());
                if !seen.insert(key.clone()) {
                    return Ok(false);
                }

                let referenced_ty = match &reference.inner {
                    NamedReferenceType::Inline { dt, .. } => Some((**dt).clone()),
                    NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => {
                        types
                            .get(reference)
                            .and_then(|ndt| ndt.ty.as_ref())
                            .cloned()
                    }
                };
                let result = match referenced_ty {
                    Some(mut ty) => {
                        substitute_generics(&mut ty, named_reference_generics(reference));
                        inner(&ty, types, seen, mode)
                    }
                    None => Ok(false),
                };
                seen.remove(&key);
                result
            }
            DataType::Enum(enm)
                if matches!(EnumRepr::from_attrs(&enm.attributes)?, EnumRepr::Untagged) =>
            {
                for (_, variant) in &enm.variants {
                    if variant.skip {
                        continue;
                    }
                    let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
                    if attrs
                        .as_ref()
                        .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
                    {
                        continue;
                    }

                    let Some(payload) = internal_tag_variant_payload_compatibility(
                        variant,
                        types,
                        &mut HashSet::new(),
                        mode,
                    )?
                    else {
                        return Ok(true);
                    };
                    if payload.replacement.is_some() {
                        return Ok(true);
                    }
                    if !payload.is_effectively_empty {
                        continue;
                    }

                    match &variant.fields {
                        Fields::Unit => return Ok(true),
                        Fields::Named(_) => {}
                        Fields::Unnamed(unnamed) => {
                            let [field] = unnamed.fields.as_slice() else {
                                return Ok(true);
                            };
                            if should_skip_field_for_mode(field, mode)? {
                                return Ok(true);
                            }
                            let Some(ty) = &field.ty else {
                                return Ok(true);
                            };
                            if empty_payload_uses_unit_encoding(
                                ty,
                                types,
                                &mut HashSet::new(),
                                mode,
                            )? {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(false)
            }
            DataType::Enum(enm)
                if matches!(EnumRepr::from_attrs(&enm.attributes)?, EnumRepr::External) =>
            {
                for (_, variant) in &enm.variants {
                    if variant.skip {
                        continue;
                    }
                    let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
                    if attrs
                        .as_ref()
                        .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
                    {
                        continue;
                    }
                    if attrs.as_ref().is_some_and(|attrs| attrs.other) {
                        return Ok(true);
                    }
                    if attrs.as_ref().is_some_and(|attrs| attrs.untagged) {
                        let compatibility = internal_tag_variant_payload_compatibility(
                            variant,
                            types,
                            &mut HashSet::new(),
                            mode,
                        )?;
                        let requires_rewrite = match compatibility {
                            None => true,
                            Some(payload) if payload.replacement.is_some() => true,
                            Some(payload) if payload.is_effectively_empty => {
                                match &variant.fields {
                                    Fields::Unit => true,
                                    Fields::Named(_) => false,
                                    Fields::Unnamed(unnamed) => {
                                        let [field] = unnamed.fields.as_slice() else {
                                            return Ok(true);
                                        };
                                        if should_skip_field_for_mode(field, mode)? {
                                            return Ok(true);
                                        }
                                        let Some(ty) = &field.ty else {
                                            return Ok(true);
                                        };
                                        empty_payload_uses_unit_encoding(
                                            ty,
                                            types,
                                            &mut HashSet::new(),
                                            mode,
                                        )?
                                    }
                                }
                            }
                            Some(_) => false,
                        };
                        if requires_rewrite {
                            return Ok(true);
                        }
                        continue;
                    }

                    match &variant.fields {
                        Fields::Unit => return Ok(true),
                        Fields::Unnamed(unnamed) => {
                            if let [field] = unnamed.fields.as_slice()
                                && (field.ty.is_none() || should_skip_field_for_mode(field, mode)?)
                            {
                                return Ok(true);
                            }
                        }
                        Fields::Named(_) => {}
                    }
                }
                Ok(false)
            }
            DataType::Intersection(parts) => {
                for part in parts {
                    if inner(part, types, seen, mode)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            DataType::Tuple(tuple) => Ok(tuple.elements.is_empty()),
            DataType::Enum(_)
            | DataType::Map(_)
            | DataType::List(_)
            | DataType::Nullable(_)
            | DataType::Primitive(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => Ok(false),
        }
    }

    if inner(ty, types, &mut HashSet::new(), PhaseRewrite::Serialize)? {
        return Ok(true);
    }
    inner(ty, types, &mut HashSet::new(), PhaseRewrite::Deserialize)
}

fn contextualize_rewritten_external_units(
    enm: &Enum,
    original_types: &Types,
    seen: &mut HashSet<Reference>,
    mode: PhaseRewrite,
) -> Result<InternalTagPayloadCompatibility, Error> {
    let mut rewritten = enm.clone();
    let mut changed = false;

    for (name, variant) in &mut rewritten.variants {
        let Fields::Unnamed(fields) = &variant.fields else {
            continue;
        };
        let [field] = fields.fields.as_slice() else {
            continue;
        };
        let Some(ty) = field.ty.as_ref() else {
            continue;
        };
        if matches!(ty, DataType::Primitive(Primitive::str)) {
            return Err(Error::invalid_internally_tagged_variant(
                name.clone(),
                "an external enum payload with `#[serde(other)]` cannot be represented generically inside an internal tag",
            ));
        }
        if matches!(ty, DataType::Tuple(tuple) if tuple.elements.is_empty()) {
            *variant = clone_variant_with_unnamed_fields(
                variant,
                vec![Field::new(named_fields_datatype(Vec::new()))],
            );
            variant.attributes = Default::default();
            changed = true;
            continue;
        }
        if !is_generated_string_literal_datatype(ty) {
            let Some(compatibility) =
                internal_tag_payload_compatibility(ty, original_types, seen, mode)?
            else {
                return Err(Error::invalid_internally_tagged_variant(
                    name.clone(),
                    "an inline untagged payload cannot be merged with an internal tag",
                ));
            };
            if compatibility.is_effectively_empty {
                *variant = clone_variant_with_unnamed_fields(
                    variant,
                    vec![Field::new(named_fields_datatype(Vec::new()))],
                );
                changed = true;
            } else if let Some(replacement) = compatibility.replacement {
                *variant =
                    clone_variant_with_unnamed_fields(variant, vec![Field::new(replacement)]);
                changed = true;
            }
            continue;
        }

        *variant = clone_variant_with_named_fields(
            variant,
            vec![(
                name.clone(),
                Field::new(DataType::Tuple(Tuple::new(vec![]))),
            )],
        );
        variant.attributes = Default::default();
        changed = true;
    }

    if changed {
        Ok(InternalTagPayloadCompatibility {
            is_effectively_empty: false,
            replacement: Some(DataType::Enum(rewritten)),
        })
    } else {
        Ok(InternalTagPayloadCompatibility::merge_as_is())
    }
}

fn external_enum_requires_contextual_rewrite(
    enm: &Enum,
    mode: PhaseRewrite,
) -> Result<bool, Error> {
    for (_, variant) in &enm.variants {
        if variant.skip {
            continue;
        }
        let attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if attrs
            .as_ref()
            .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
        {
            continue;
        }
        if attrs
            .as_ref()
            .is_some_and(|attrs| attrs.other || attrs.untagged)
        {
            return Ok(true);
        }

        match &variant.fields {
            Fields::Unit => return Ok(true),
            Fields::Unnamed(unnamed) => {
                if let [field] = unnamed.fields.as_slice()
                    && (field.ty.is_none() || should_skip_field_for_mode(field, mode)?)
                {
                    return Ok(true);
                }
            }
            Fields::Named(_) => {}
        }
    }

    Ok(false)
}

// An externally tagged enum normally renders a unit variant as a string, but
// serde's `TaggedSerializer` always writes the inner discriminant as a map
// entry. Build the contextual representation used only inside the outer
// internally tagged newtype variant.
fn external_enum_tagged_payload_datatype(
    enm: &Enum,
    original_types: &Types,
    mode: PhaseRewrite,
) -> Result<DataType, Error> {
    let container_attrs = SerdeContainerAttrs::from_attributes(&enm.attributes)?;
    let mut transformed = Enum::default();

    for (variant_name, original_variant) in &enm.variants {
        let mut variant = original_variant.clone();
        if variant.skip {
            continue;
        }

        let variant_attrs = SerdeVariantAttrs::from_attributes(&variant.attributes)?;
        if variant_attrs
            .as_ref()
            .is_some_and(|attrs| variant_is_skipped_for_mode(attrs, mode))
        {
            continue;
        }

        let rename_rule =
            enum_variant_field_rename_rule(&container_attrs, &variant, mode, variant_name)?;
        rewrite_fields_for_phase(
            &mut variant.fields,
            mode,
            original_types,
            &HashMap::new(),
            &HashSet::new(),
            rename_rule,
            false,
            true,
        )?;

        if let Some(aliases) = lower_field_aliases_for_phase(&mut variant.fields, mode)? {
            variant.fields =
                clone_variant_with_unnamed_fields(&variant, vec![Field::new(aliases)]).fields;
        } else if matches!(
            &variant.fields,
            Fields::Named(fields) if fields.fields.iter().any(|(_, field)| field_is_flattened(field))
        ) {
            let mut payload = Struct::unit();
            payload.fields = variant.fields.clone();
            let payload = lower_flattened_struct(&mut payload)?
                .expect("a named payload containing flatten must be lowered");
            variant.fields =
                clone_variant_with_unnamed_fields(&variant, vec![Field::new(payload)]).fields;
        }

        if variant_payload_is_hidden(&variant, mode)? {
            return Err(Error::invalid_internally_tagged_variant(
                variant_name.clone(),
                "a payload hidden only from Specta cannot be represented inside an internal tag",
            ));
        }

        if variant_attrs.as_ref().is_some_and(|attrs| attrs.untagged) {
            let payload = match &variant.fields {
                Fields::Unit => named_fields_datatype(Vec::new()),
                _ => {
                    let payload = variant_payload_field(&variant)
                        .ok_or_else(|| {
                            Error::invalid_external_tagged_variant(variant_name.clone())
                        })?
                        .ty
                        .expect("variant payload fields always have a datatype");
                    let Some(compatibility) = internal_tag_payload_compatibility(
                        &payload,
                        original_types,
                        &mut HashSet::new(),
                        mode,
                    )?
                    else {
                        return Err(Error::invalid_internally_tagged_variant(
                            variant_name.clone(),
                            "untagged payload cannot be merged with a tag",
                        ));
                    };

                    if compatibility.is_effectively_empty {
                        named_fields_datatype(Vec::new())
                    } else {
                        compatibility.replacement.unwrap_or(payload)
                    }
                }
            };
            let mut transformed_variant =
                clone_variant_with_unnamed_fields(&variant, vec![Field::new(payload)]);
            transformed_variant.attributes = Default::default();
            transformed
                .variants
                .push((variant_name.clone(), transformed_variant));
            continue;
        }

        let serialized_name =
            serialized_variant_name(variant_name, &variant, &container_attrs, mode)?;
        let aliases = variant_attrs
            .as_ref()
            .filter(|_| mode == PhaseRewrite::Deserialize)
            .map(|attrs| attrs.aliases.as_slice())
            .unwrap_or(&[]);

        for name in std::iter::once(serialized_name).chain(aliases.iter().cloned()) {
            let widen_tag = mode == PhaseRewrite::Deserialize
                && variant_attrs.as_ref().is_some_and(|attrs| attrs.other);
            if widen_tag {
                return Err(Error::invalid_internally_tagged_variant(
                    name,
                    "an external enum payload with `#[serde(other)]` cannot be represented generically inside an internal tag",
                ));
            }

            let payload = match &variant.fields {
                Fields::Unit => Field::new(DataType::Tuple(Tuple::new(vec![]))),
                _ => variant_payload_field(&variant)
                    .ok_or_else(|| Error::invalid_external_tagged_variant(name.clone()))?,
            };
            let mut transformed_variant = clone_variant_with_named_fields(
                &variant,
                vec![(Cow::Owned(name.clone()), payload)],
            );
            transformed_variant.attributes = Default::default();
            transformed
                .variants
                .push((Cow::Owned(name), transformed_variant));
        }
    }

    transformed
        .attributes
        .insert(ENUM_REPR_REWRITTEN_MARKER, true);
    Ok(DataType::Enum(transformed))
}

fn substitute_generics(ty: &mut DataType, generics: &[(specta::datatype::Generic, DataType)]) {
    match ty {
        DataType::Generic(generic) => {
            if let Some((_, concrete)) = generics.iter().find(|(candidate, _)| candidate == generic)
            {
                *ty = concrete.clone();
            }
        }
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, generics),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_field_generics(&mut variant.fields, generics);
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                substitute_generics(element, generics);
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Intersection(parts) => {
            for part in parts {
                substitute_generics(part, generics);
            }
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Reference(Reference::Named(reference)) => {
            for (_, argument) in named_reference_generics_mut(reference) {
                substitute_generics(argument, generics);
            }
            if let NamedReferenceType::Inline { dt, .. } = &mut reference.inner {
                substitute_generics(dt, generics);
            }
        }
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(
    fields: &mut Fields,
    generics: &[(specta::datatype::Generic, DataType)],
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
    }
}

fn internal_tag_variant_payload_compatibility(
    variant: &Variant,
    original_types: &Types,
    seen: &mut HashSet<Reference>,
    mode: PhaseRewrite,
) -> Result<Option<InternalTagPayloadCompatibility>, Error> {
    match &variant.fields {
        Fields::Unit => Ok(Some(InternalTagPayloadCompatibility::empty())),
        Fields::Named(named) => Ok(Some(InternalTagPayloadCompatibility {
            is_effectively_empty: named
                .fields
                .iter()
                .all(|(_, field)| field.ty.as_ref().is_none()),
            replacement: None,
        })),
        Fields::Unnamed(unnamed) => {
            if unnamed.fields.len() != 1 {
                return Ok(None);
            }

            let field = &unnamed.fields[0];
            if should_skip_field_for_mode(field, mode)? {
                return Ok(Some(InternalTagPayloadCompatibility::empty()));
            }
            field.ty.as_ref().map_or(Ok(None), |ty| {
                internal_tag_payload_compatibility(ty, original_types, seen, mode)
            })
        }
    }
}

fn has_local_phase_difference(dt: &DataType) -> Result<bool, Error> {
    match dt {
        // `#[serde(default)]` on a struct container widens every field
        // to optional on deserialize while serialize still always emits
        // them, so it forces a phase split just like a field-level default.
        // serde only supports this attribute on structs (not enums), so it
        // is checked here rather than inside `container_has_local_difference`,
        // which is shared with `DataType::Enum` below. Like a field-level
        // default, it only matters for fields that appear on the deserialize
        // wire: with no such field (an empty struct, or one whose fields are
        // all `#[serde(skip)]`-ped cache/state) both phases render the same
        // shape, and splitting would only add a redundant identical type pair
        // (and force every dependent to split too).
        DataType::Struct(s) => Ok(container_has_local_difference(&s.attributes)?
            || (struct_has_deserialize_wire_field(&s.fields)?
                && SerdeContainerAttrs::from_attributes(&s.attributes)?
                    .is_some_and(|attrs| attrs.default))
            || fields_have_local_difference(&s.fields)?),
        DataType::Enum(e) => {
            let adjacent = matches!(
                EnumRepr::from_attrs(&e.attributes)?,
                EnumRepr::Adjacent { .. }
            );

            Ok(container_has_local_difference(&e.attributes)?
                || e.variants
                    .iter()
                    .try_fold(false, |has_difference, (_, variant)| {
                        if has_difference {
                            return Ok(true);
                        }

                        // A variant removed from both phases renders in neither
                        // half, so none of its own or its payload's attrs can
                        // constitute a phase difference.
                        if variant_is_dead_in_both_phases(variant)? {
                            return Ok(false);
                        }

                        Ok(variant_has_local_difference(variant)?
                            || fields_have_local_difference(&variant.fields)?
                            || (adjacent && adjacent_content_is_phase_asymmetric(variant)?))
                    })?)
        }
        DataType::Tuple(tuple) => tuple.elements.iter().try_fold(false, |has_difference, ty| {
            if has_difference {
                return Ok(true);
            }

            has_local_phase_difference(ty)
        }),
        DataType::List(list) => has_local_phase_difference(&list.ty),
        DataType::Map(map) => Ok(has_local_phase_difference(map.key_ty())?
            || has_local_phase_difference(map.value_ty())?),
        DataType::Intersection(types_) => types_.iter().try_fold(false, |has_difference, ty| {
            if has_difference {
                return Ok(true);
            }

            has_local_phase_difference(ty)
        }),
        DataType::Nullable(inner) => has_local_phase_difference(inner),
        DataType::Reference(Reference::Opaque(reference)) => {
            Ok(reference.downcast_ref::<PhasedTy>().is_some())
        }
        DataType::Primitive(_)
        | DataType::Reference(Reference::Named(_))
        | DataType::Generic(_) => Ok(false),
    }
}

/// Whether at least one field survives into the exported deserialize shape —
/// the only fields a container `#[serde(default)]` can widen (a
/// `skip_deserializing` field's default is applied invisibly, off the wire,
/// a `#[specta(skip)]` field (`ty: None`) is hidden from the export, and
/// serde never applies defaults to `flatten` fields: their keys stay
/// required on deserialize).
///
/// For tuple structs, serde fills every missing trailing element from the
/// container's `Default` instance (a shorter array — even `[]` — is
/// accepted), so any live element counts. A newtype keeps its bare-value
/// representation, which has nothing to omit.
fn struct_has_deserialize_wire_field(fields: &Fields) -> Result<bool, Error> {
    match fields {
        Fields::Unit => Ok(false),
        Fields::Named(named) => {
            for (_, field) in &named.fields {
                if field.ty.is_some()
                    && !SerdeFieldAttrs::from_attributes(&field.attributes)?
                        .is_some_and(|attrs| attrs.skip_deserializing || attrs.flatten)
                {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.fields.len() == 1 {
                return Ok(false);
            }

            for field in &unnamed.fields {
                if field.ty.is_some()
                    && !SerdeFieldAttrs::from_attributes(&field.attributes)?
                        .is_some_and(|attrs| attrs.skip_deserializing)
                {
                    return Ok(true);
                }
            }

            Ok(false)
        }
    }
}

fn container_has_local_difference(attrs: &specta::datatype::Attributes) -> Result<bool, Error> {
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

fn fields_have_local_difference(fields: &Fields) -> Result<bool, Error> {
    match fields {
        Fields::Unit => Ok(false),
        Fields::Unnamed(unnamed) => {
            // A single unnamed field is a newtype: serde represents it as
            // the bare inner value, leaving no container position to omit,
            // so `#[serde(default)]` on it is inert on the wire.
            let newtype = unnamed.fields.len() == 1;
            let trailing_default_from = unnamed_trailing_default_run(unnamed)?;

            unnamed
                .fields
                .iter()
                .enumerate()
                .try_fold(false, |has_difference, (idx, field)| {
                    if has_difference {
                        return Ok(true);
                    }

                    single_field_has_local_difference(field, newtype || idx < trailing_default_from)
                })
        }
        Fields::Named(named) => {
            named
                .fields
                .iter()
                .try_fold(false, |has_difference, (_, field)| {
                    if has_difference {
                        return Ok(true);
                    }

                    single_field_has_local_difference(field, false)
                })
        }
    }
}

/// An adjacently tagged newtype variant whose sole field is skipped in *both*
/// phases collapses asymmetrically: serde's serializer omits the `content` key
/// entirely (like a unit variant), while its deserializer still requires
/// `content: null`. Such enums must split under [`PhasesFormat`] even though
/// the skip attribute itself is symmetric.
fn adjacent_content_is_phase_asymmetric(variant: &Variant) -> Result<bool, Error> {
    if variant.skip {
        return Ok(false);
    }

    // `#[serde(untagged)]` variants bypass the tag/content representation
    // entirely, so there is no `content` key to be asymmetric about.
    if let Some(attrs) = SerdeVariantAttrs::from_attributes(&variant.attributes)?
        && (attrs.untagged
            || (variant_is_skipped_for_mode(&attrs, PhaseRewrite::Serialize)
                && variant_is_skipped_for_mode(&attrs, PhaseRewrite::Deserialize)))
    {
        return Ok(false);
    }

    let Fields::Unnamed(unnamed) = &variant.fields else {
        return Ok(false);
    };
    let [field] = unnamed.fields.as_slice() else {
        return Ok(false);
    };

    // A skipped `Option` sole field is *not* asymmetric: serde's
    // `missing_field` helper deserializes a missing `content` key as `None`,
    // so the unified `content?: null` shape is exact for both phases and no
    // split is needed. `Option`-ness is read only from the marker the derive
    // records from the real field syntax; see [`NULLABLE_FIELD`].
    if field.attributes.contains_key(NULLABLE_FIELD) {
        return Ok(false);
    }

    // Only symmetric *serde* skips exhibit the ser/de content asymmetry. A
    // field hidden by `#[specta(skip)]` alone (or a hand-built
    // `Field { ty: None, .. }`) is invisible to serde -- the wire carries the
    // payload symmetrically -- so `ty` absence is deliberately not evidence.
    Ok(should_skip_field_for_mode(field, PhaseRewrite::Serialize)?
        && should_skip_field_for_mode(field, PhaseRewrite::Deserialize)?)
}

fn single_field_has_local_difference(field: &Field, default_is_inert: bool) -> Result<bool, Error> {
    // A field removed from both phases renders in neither half, so no attr
    // on it — nor anything inside its (inline) datatype — can constitute a
    // phase difference.
    if field_is_dead_in_both_phases(field)? {
        return Ok(false);
    }

    Ok(field_has_local_difference(field, default_is_inert)?
        || field
            .ty
            .as_ref()
            .map_or(Ok(false), has_local_phase_difference)?)
}

/// A field absent from BOTH exported phases: `#[specta(skip)]` (and bare
/// `#[serde(skip)]`, which the specta macro special-cases the same way)
/// erase `ty`, and the explicit `skip_serializing, skip_deserializing` pair
/// is removed from each rewrite. Such a field renders nowhere, so it can
/// neither contribute a phase difference nor tie its container to a
/// dependency's split. A one-sided skip keeps the field in one phase and
/// stays direction-relevant.
/// The declared index from which every remaining live unnamed field carries
/// `#[serde(default)]`. Sequence elements can only be omitted from the END,
/// so only defaults in this trailing run are observable on the deserialize
/// wire. serde_derive enforces the suffix rule at compile time ("field must
/// have #[serde(default)] because previous field N has #[serde(default)]");
/// this gates hand-built datatypes the same way, mirroring the renderer's
/// trailing-only optional handling.
fn unnamed_trailing_default_run(unnamed: &UnnamedFields) -> Result<usize, Error> {
    let mut run_start = 0;
    for (idx, field) in unnamed.fields.iter().enumerate() {
        // Dead fields never reach the wire, so they neither extend nor
        // break the trailing run.
        if field_is_dead_in_both_phases(field)? {
            continue;
        }

        if !SerdeFieldAttrs::from_attributes(&field.attributes)?.is_some_and(|attrs| attrs.default)
        {
            run_start = idx + 1;
        }
    }

    Ok(run_start)
}

fn field_is_dead_in_both_phases(field: &Field) -> Result<bool, Error> {
    if field.attributes.contains_key(SERDE_NEWTYPE_SKIP_IGNORED) {
        return Ok(false);
    }
    if field.ty.is_none() {
        return Ok(true);
    }

    Ok(SerdeFieldAttrs::from_attributes(&field.attributes)?
        .is_some_and(|attrs| attrs.skip_serializing && attrs.skip_deserializing))
}

/// The variant counterpart of [`field_is_dead_in_both_phases`]:
/// [`filter_enum_variants_for_phase`] drops such a variant from both
/// generated halves — either via the specta-level [`Variant::skip`] flag
/// (dropped unconditionally) or the serde skip-flag pair.
fn variant_is_dead_in_both_phases(variant: &Variant) -> Result<bool, Error> {
    Ok(variant.skip
        || SerdeVariantAttrs::from_attributes(&variant.attributes)?
            .is_some_and(|attrs| attrs.skip_serializing && attrs.skip_deserializing))
}

fn field_has_local_difference(field: &Field, default_is_inert: bool) -> Result<bool, Error> {
    let ignored_skip = field.attributes.contains_key(SERDE_NEWTYPE_SKIP_IGNORED);
    Ok(SerdeFieldAttrs::from_attributes(&field.attributes)?
        .map(|attrs| {
            attrs.rename_serialize.as_deref() != attrs.rename_deserialize.as_deref()
                || !attrs.aliases.is_empty()
                // `#[serde(default)]` only widens the deserialize shape
                // (absent fields fall back to `Default::default()`); serde
                // always emits the field on serialize. It is therefore only
                // a phase difference when the field actually appears in the
                // exported deserialize shape: with `skip_deserializing`
                // (including via full `skip`, e.g. `#[serde(skip, default =
                // "...")]` cache fields) the default is applied invisibly,
                // and a `#[specta(skip)]` field (`ty: None`) is hidden from
                // both exported phases entirely. serde also never applies
                // `default` to a `flatten` field — base-only input still
                // fails with "missing field", so the flattened keys are
                // equally required in both phases. The caller flags newtype
                // fields, where the bare wire value leaves nothing to omit.
                // An asymmetric serde skip still splits via the skip check
                // below.
                || (attrs.default
                    && !default_is_inert
                    && !attrs.skip_deserializing
                    && !attrs.flatten
                    && field.ty.is_some())
                || (!ignored_skip && attrs.skip_serializing != attrs.skip_deserializing)
                || attrs.skip_serializing_if.is_some()
                || attrs.has_serialize_with
                || attrs.has_deserialize_with
                || attrs.has_with
        })
        .unwrap_or_default())
}

fn variant_has_local_difference(variant: &Variant) -> Result<bool, Error> {
    Ok(SerdeVariantAttrs::from_attributes(&variant.attributes)?
        .map(|attrs| {
            attrs.rename_serialize.as_deref() != attrs.rename_deserialize.as_deref()
                || attrs.rename_all_serialize != attrs.rename_all_deserialize
                || !attrs.aliases.is_empty()
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
) -> Result<(), Error> {
    match dt {
        DataType::Struct(s) => {
            collect_conversion_dependencies(&s.attributes, types, deps)?;
            collect_fields_dependencies(&s.fields, types, deps)?;
        }
        DataType::Enum(e) => {
            collect_conversion_dependencies(&e.attributes, types, deps)?;
            for (_, variant) in &e.variants {
                // A variant removed from both phases never appears in either
                // half, so its payload cannot tie the enum to a phase split.
                if variant_is_dead_in_both_phases(variant)? {
                    continue;
                }

                collect_fields_dependencies(&variant.fields, types, deps)?;
            }
        }
        DataType::Tuple(tuple) => {
            for ty in &tuple.elements {
                collect_dependencies(ty, types, deps)?;
            }
        }
        DataType::List(list) => collect_dependencies(&list.ty, types, deps)?,
        DataType::Map(map) => {
            collect_dependencies(map.key_ty(), types, deps)?;
            collect_dependencies(map.value_ty(), types, deps)?;
        }
        DataType::Intersection(types_) => {
            for ty in types_ {
                collect_dependencies(ty, types, deps)?;
            }
        }
        DataType::Nullable(inner) => collect_dependencies(inner, types, deps)?,
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Inline { dt, .. } = &reference.inner {
                collect_dependencies(dt, types, deps)?;
            }

            if let Some(referenced) = types.get(reference) {
                deps.insert(TypeIdentity::from_ndt(referenced));
            }

            for (_, generic) in named_reference_generics(reference) {
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
        DataType::Primitive(_) | DataType::Generic(_) => {}
    }

    Ok(())
}

fn collect_conversion_dependencies(
    attrs: &specta::datatype::Attributes,
    types: &Types,
    deps: &mut HashSet<TypeIdentity>,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &unnamed.fields {
                collect_field_dependencies(field, types, deps)?;
            }
        }
        Fields::Named(named) => {
            for (_, field) in &named.fields {
                collect_field_dependencies(field, types, deps)?;
            }
        }
    }

    Ok(())
}

fn collect_field_dependencies(
    field: &Field,
    types: &Types,
    deps: &mut HashSet<TypeIdentity>,
) -> Result<(), Error> {
    // See `field_is_dead_in_both_phases`: such a field renders in neither
    // half, so it cannot tie its container to a referenced type's phase
    // split. A one-sided skip keeps the field in one phase, so it must
    // still propagate (its skip asymmetry splits the container regardless).
    if field_is_dead_in_both_phases(field)? {
        return Ok(());
    }

    if let Some(ty) = field.ty.as_ref() {
        collect_dependencies(ty, types, deps)?;
    }

    Ok(())
}

fn build_from_original(
    original: &NamedDataType,
    mode: PhaseRewrite,
) -> Result<NamedDataType, Error> {
    let mut ndt = original.clone();
    ndt.name = Cow::Owned(split_type_name(original, mode)?);

    Ok(ndt)
}

fn register_generated_type(types: &mut Types, generated: NamedDataType) -> NamedDataType {
    NamedDataType::new(generated.name.clone(), types, move |_, ndt| {
        ndt.docs = generated.docs;
        ndt.deprecated = generated.deprecated;
        ndt.module_path = generated.module_path;
        ndt.location = generated.location;
        ndt.generics = generated.generics;
        ndt.ty = generated.ty;
    })
}

fn rewrite_named_type_for_phase(ndt: &mut NamedDataType, mode: PhaseRewrite) -> Result<(), Error> {
    if let Some(ty) = &ndt.ty
        && let Some(rename) = renamed_type_name_for_phase(ty, mode, ndt.name.as_ref())?
    {
        ndt.name = Cow::Owned(rename);
    }

    Ok(())
}

/// The single *effective* container rename of a type whose serialize and
/// deserialize renames agree (defaulting a missing side to the type's own
/// name), or `None` when the phases have distinct names or no rename at all.
fn symmetric_container_rename(ndt: &NamedDataType) -> Result<Option<String>, Error> {
    let Some(ty) = &ndt.ty else {
        return Ok(None);
    };

    let original_name = ndt.name.as_ref();
    let serialize_rename = renamed_type_name_for_phase(ty, PhaseRewrite::Serialize, original_name)?;
    let deserialize_rename =
        renamed_type_name_for_phase(ty, PhaseRewrite::Deserialize, original_name)?;

    let effective_serialize = serialize_rename.as_deref().unwrap_or(original_name);
    let effective_deserialize = deserialize_rename.as_deref().unwrap_or(original_name);

    Ok(
        (effective_serialize == effective_deserialize && effective_serialize != original_name)
            .then(|| effective_serialize.to_string()),
    )
}

fn split_type_name(original: &NamedDataType, mode: PhaseRewrite) -> Result<String, Error> {
    let suffix = match mode {
        PhaseRewrite::Serialize => "Serialize",
        PhaseRewrite::Deserialize => "Deserialize",
        PhaseRewrite::Unified => return Ok(original.name.to_string()),
    };

    let rename_for = |phase: PhaseRewrite| {
        original
            .ty
            .as_ref()
            .map(|ty| renamed_type_name_for_phase(ty, phase, original.name.as_ref()))
            .transpose()
            .map(Option::flatten)
    };
    let serialize_rename = rename_for(PhaseRewrite::Serialize)?;
    let deserialize_rename = rename_for(PhaseRewrite::Deserialize)?;

    // Compare *effective* names, defaulting a missing side to the type's own
    // name, mirroring `validate_container_attributes`: a one-sided rename
    // equal to the original name (e.g. `rename(serialize = "Foo")` on `Foo`)
    // is a no-op, not an authored phase-specific name.
    let original_name = original.name.as_ref();
    let effective_serialize = serialize_rename.as_deref().unwrap_or(original_name);
    let effective_deserialize = deserialize_rename.as_deref().unwrap_or(original_name);
    let renames_differ = effective_serialize != effective_deserialize;

    // A rename is an *authored* phase-specific name when the effective names
    // differ and it isn't the original name (which the phased wrapper type
    // already occupies). Authored names are the user's explicit export names
    // and are used verbatim; anything else gets a generated `_{suffix}` name.
    fn authored<'a>(
        rename: &'a Option<String>,
        renames_differ: bool,
        original_name: &str,
    ) -> Option<&'a str> {
        rename
            .as_deref()
            .filter(|rename| renames_differ && *rename != original_name)
    }

    let (rename, other_rename) = match mode {
        PhaseRewrite::Serialize => (&serialize_rename, &deserialize_rename),
        _ => (&deserialize_rename, &serialize_rename),
    };

    if let Some(rename) = authored(rename, renames_differ, original_name) {
        return Ok(rename.to_string());
    }

    let base_name = rename.as_deref().unwrap_or(original_name);
    let mut name = format!("{base_name}_{suffix}");
    // The sibling phase's authored name may equal this generated name (e.g.
    // `rename(serialize = "Foo_Deserialize")` on `Foo`). The authored name
    // wins verbatim, so disambiguate the generated one by appending its phase
    // suffix again; the result can't collide with the authored name (it
    // strictly extends it) or the wrapper's original name.
    if authored(other_rename, renames_differ, original_name) == Some(name.as_str()) {
        name = format!("{name}_{suffix}");
    }
    Ok(name)
}

fn renamed_type_name_for_phase(
    ty: &DataType,
    mode: PhaseRewrite,
    current_name: &str,
) -> Result<Option<String>, Error> {
    let attributes = match ty {
        DataType::Struct(strct) => &strct.attributes,
        DataType::Enum(e) => &e.attributes,
        _ => return Ok(None),
    };
    let Some(attrs) = SerdeContainerAttrs::from_attributes(attributes)? else {
        return Ok(None);
    };

    Ok(select_phase_string(
        mode,
        attrs.rename_serialize.as_deref(),
        attrs.rename_deserialize.as_deref(),
        "container rename",
        current_name,
    )?
    .map(str::to_string))
}

fn apply_field_attrs(
    field: &mut Field,
    mode: PhaseRewrite,
    container_default: bool,
) -> Result<(), Error> {
    if field.attributes.contains_key(SERDE_NEWTYPE_SKIP_IGNORED) {
        return Ok(());
    }
    let mut optional = field.optional;
    if let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? {
        if field_is_optional_for_mode(Some(&attrs), container_default, mode) {
            optional = true;
        }
    } else if field_is_optional_for_mode(None, container_default, mode) {
        optional = true;
    }
    field.optional = optional;

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
    #![allow(clippy::panic)]

    use serde::{Deserialize, Serialize};
    use specta::{Format as _, Type, Types, datatype::DataType};

    use super::{
        Phase, Phased, PhasesFormat, parser, select_phase_datatype,
        validate::{ApplyMode, validate_datatype_for_mode},
    };

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

    #[derive(Type, Serialize, Deserialize)]
    struct WithSkipIf {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    #[derive(Type, Serialize, Deserialize)]
    struct WithDirectionalFieldAttrs {
        #[serde(rename(serialize = "serName"))]
        field_a: String,
        #[serde(skip_serializing)]
        field_b: String,
    }

    #[derive(Type, Serialize, Deserialize)]
    #[serde(
        rename(serialize = "DirectionalContainerSer"),
        rename_all(serialize = "camelCase")
    )]
    struct WithDirectionalContainerAttrs {
        field_one: String,
    }

    #[derive(Type, Serialize, Deserialize)]
    #[serde(untagged)]
    enum UntaggedWithDirectionalVariantAttrs {
        A(String),
        #[serde(skip_deserializing)]
        B(u32),
        #[serde(rename(serialize = "CSer"))]
        C(bool),
    }

    #[derive(Type, Serialize, Deserialize)]
    #[serde(untagged, rename_all(serialize = "camelCase"))]
    enum UntaggedWithDirectionalContainerAttrs {
        A { field_one: String },
    }

    #[test]
    fn selects_split_named_reference_for_each_phase() {
        let mut types = specta::Types::default();
        let dt = Filters::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "Filters_Serialize");
        assert_named_reference(&deserialize, &resolved, "Filters_Deserialize");
    }

    #[test]
    fn rewrites_nested_generics_for_each_phase() {
        let mut types = specta::Types::default();
        let dt = FilterList::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "FilterList_Serialize");
        assert_named_reference(&deserialize, &resolved, "FilterList_Deserialize");

        let serialize_inner = named_field_type(&serialize, &resolved, "items");
        let deserialize_inner = named_field_type(&deserialize, &resolved, "items");

        assert_named_reference(
            list_item_type(serialize_inner),
            &resolved,
            "Filters_Serialize",
        );
        assert_named_reference(
            list_item_type(deserialize_inner),
            &resolved,
            "Filters_Deserialize",
        );
    }

    #[test]
    fn preserves_unsplit_types() {
        let mut types = specta::Types::default();
        let dt = Plain::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "Plain");
        assert_named_reference(&deserialize, &resolved, "Plain");
    }

    #[test]
    fn clears_skip_serializing_if_attribute_after_phase_split() {
        let mut types = specta::Types::default();
        let dt = WithSkipIf::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert!(!field_has_skip_serializing_if(
            &serialize, &resolved, "nickname"
        ));
        assert!(!field_has_skip_serializing_if(
            &deserialize,
            &resolved,
            "nickname"
        ));
    }

    #[test]
    fn skip_serializing_if_option_is_none_omits_null_in_serialize_phase() {
        let mut types = specta::Types::default();
        let dt = WithSkipIf::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        let serialize_field = named_field(&serialize, &resolved, "nickname");
        let deserialize_field = named_field(&deserialize, &resolved, "nickname");

        assert!(serialize_field.optional);
        assert!(!matches!(
            serialize_field.ty.as_ref(),
            Some(DataType::Nullable(_))
        ));
        assert!(matches!(
            deserialize_field.ty.as_ref(),
            Some(DataType::Nullable(_))
        ));
    }

    #[test]
    fn phase_split_field_passes_unified_mode_validation() {
        // Regression test for the interaction with downstream callers (e.g.
        // tauri-specta's `validate_exported_command`) that run unified-mode
        // validation on the post-`apply_phases` graph. Before clearing the
        // attribute on phase-split fields, this would error with
        // "skip_serializing_if requires format_phases because unified mode
        // cannot represent conditional omission".
        let mut types = specta::Types::default();
        let dt = WithSkipIf::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        validate_datatype_for_mode(&serialize, &resolved, ApplyMode::Unified)
            .expect("Unified validation should accept phase-split _Serialize variant");
        validate_datatype_for_mode(&deserialize, &resolved, ApplyMode::Unified)
            .expect("Unified validation should accept phase-split _Deserialize variant");
    }

    #[test]
    fn phase_split_directional_attrs_pass_unified_mode_validation() {
        // Same downstream contract as above, but for the one-sided directional
        // renames/skips that unified-mode validation rejects: once
        // `PhasesFormat` has split a type, the phase-specific shapes must not
        // keep carrying the consumed directional serde attrs, otherwise
        // unified-mode validation on the post-`apply_phases` graph rejects the
        // already-split types.
        let mut types = specta::Types::default();
        let field_dt = WithDirectionalFieldAttrs::definition(&mut types);
        let container_dt = WithDirectionalContainerAttrs::definition(&mut types);
        let resolved = formatted_phases(types);

        for (name, dt) in [
            ("field attrs", &field_dt),
            ("container attrs", &container_dt),
        ] {
            for phase in [Phase::Serialize, Phase::Deserialize] {
                let phased = select_phase_datatype(dt, &resolved, phase);
                validate_datatype_for_mode(&phased, &resolved, ApplyMode::Unified)
                    .unwrap_or_else(|err| {
                        panic!(
                            "Unified validation should accept phase-split {name} {phase:?} shape: {err}"
                        )
                    });
            }
        }
    }

    #[test]
    fn phase_split_untagged_enum_directional_attrs_pass_unified_mode_validation() {
        // Untagged enums keep their `DataType::Enum` shape through a phase
        // split (`rewrite_enum_repr_for_phase` returns early for them), so the
        // consumed directional variant/container attrs must be stripped from
        // the kept variants too, or unified-mode validation of the
        // post-`apply_phases` graph rejects the already-split enums.
        let mut types = specta::Types::default();
        let variant_dt = UntaggedWithDirectionalVariantAttrs::definition(&mut types);
        let container_dt = UntaggedWithDirectionalContainerAttrs::definition(&mut types);
        let resolved = formatted_phases(types);

        for (name, dt) in [
            ("variant attrs", &variant_dt),
            ("container attrs", &container_dt),
        ] {
            for phase in [Phase::Serialize, Phase::Deserialize] {
                let phased = select_phase_datatype(dt, &resolved, phase);
                validate_datatype_for_mode(&phased, &resolved, ApplyMode::Unified)
                    .unwrap_or_else(|err| {
                        panic!(
                            "Unified validation should accept phase-split untagged enum {name} {phase:?} shape: {err}"
                        )
                    });
            }
        }
    }

    #[test]
    fn resolves_explicit_phased_datatypes_without_named_types() {
        let mut types = specta::Types::default();
        let dt = <Phased<String, Vec<String>>>::definition(&mut types);
        let resolved = formatted_phases(types);

        let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
        let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

        assert_named_reference(&serialize, &resolved, "String");
        assert_named_reference(&deserialize, &resolved, "Vec");
    }

    fn assert_named_reference(dt: &DataType, types: &Types, expected_name: &str) {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };

        let actual = types
            .get(reference)
            .expect("reference should resolve")
            .name
            .as_ref();

        assert_eq!(actual, expected_name);
    }

    fn named_field_type<'a>(dt: &'a DataType, types: &'a Types, field_name: &str) -> &'a DataType {
        named_field(dt, types, field_name)
            .ty
            .as_ref()
            .expect("field should have a type")
    }

    fn named_field<'a>(
        dt: &'a DataType,
        types: &'a Types,
        field_name: &str,
    ) -> &'a specta::datatype::Field {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };

        let named = types.get(reference).expect("reference should resolve");
        let Some(DataType::Struct(strct)) = &named.ty else {
            panic!("expected struct type");
        };
        let specta::datatype::Fields::Named(fields) = &strct.fields else {
            panic!("expected named fields");
        };

        fields
            .fields
            .iter()
            .find_map(|(name, field)| (name == field_name).then_some(field))
            .expect("field should exist")
    }

    fn field_has_skip_serializing_if(dt: &DataType, types: &Types, field_name: &str) -> bool {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };
        let named = types.get(reference).expect("reference should resolve");
        let Some(DataType::Struct(strct)) = &named.ty else {
            panic!("expected struct type");
        };
        let specta::datatype::Fields::Named(fields) = &strct.fields else {
            panic!("expected named fields");
        };
        fields
            .fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, field)| {
                field
                    .attributes
                    .contains_key(parser::FIELD_SKIP_SERIALIZING_IF)
            })
            .expect("field should exist")
    }

    fn first_generic_type(dt: &DataType) -> &DataType {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference with generics");
        };

        let specta::datatype::NamedReferenceType::Reference { generics, .. } = &reference.inner
        else {
            panic!("expected named reference with generics");
        };

        generics
            .first()
            .map(|(_, dt)| dt)
            .expect("expected first generic type")
    }

    fn list_item_type(dt: &DataType) -> &DataType {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected inline list reference");
        };

        let specta::datatype::NamedReferenceType::Inline { dt, .. } = &reference.inner else {
            return first_generic_type(dt);
        };

        let DataType::List(list) = dt.as_ref() else {
            panic!("expected inline list");
        };

        &list.ty
    }

    fn formatted_phases(types: Types) -> Types {
        let format = PhasesFormat;
        format
            .map_types(&types)
            .expect("PhasesFormat should succeed")
            .into_owned()
    }
}
