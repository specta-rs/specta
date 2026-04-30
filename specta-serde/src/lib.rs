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
/// If serde metadata produces different serialize and deserialize shapes, this
/// formatter returns an error instead of guessing. In that case, use
/// [`PhasesFormat`].
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

            if let Some(ty) = ndt.ty.as_mut() {
                if rewrite_err.is_some() {
                    return;
                }

                if let Err(err) = rewrite_datatype_for_phase(
                    ty,
                    PhaseRewrite::Unified,
                    types,
                    &generated,
                    &split_types,
                    Some(ndt_name.as_str()),
                ) {
                    rewrite_err = Some(err);
                }
            }

            if rewrite_err.is_some() {
                return;
            }

            if let Err(err) = rewrite_named_type_for_phase(ndt, PhaseRewrite::Unified) {
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
/// asymmetric renames, directional skips, `#[serde(with = ...)]`-style codecs,
/// `#[serde(into = ...)]`/`#[serde(from = ...)]`, or explicit [`Phased`]
/// overrides.
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

            rewrite_named_type_for_phase(
                &mut generated_types_for_phase.serialize,
                PhaseRewrite::Serialize,
            )?;

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

            rewrite_named_type_for_phase(
                &mut generated_types_for_phase.deserialize,
                PhaseRewrite::Deserialize,
            )?;

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
                return;
            }

            if let Err(err) = rewrite_named_type_for_phase(ndt, PhaseRewrite::Unified) {
                rewrite_err = Some(err);
            }
        });

        if let Some(err) = rewrite_err {
            return Err(Box::new(err));
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
        });
        Ok(Cow::Owned(out))
    }

    fn map_type(&'_ self, types: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, FormatError> {
        if datatype_is_registered_definition(types, dt) {
            return Ok(Cow::Owned(dt.clone()));
        }

        let mut selected = select_phase_datatype(dt, types, Phase::Serialize);

        validate::validate_datatype_for_mode_shallow(
            &selected,
            types,
            validate::ApplyMode::Phases,
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
enum PhaseRewrite {
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
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive => &[],
    }
}

fn named_reference_generics_mut(
    reference: &mut NamedReference,
) -> &mut [(specta::datatype::Generic, DataType)] {
    match &mut reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics,
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive => &mut [],
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
            let container_default = SerdeContainerAttrs::from_attributes(&s.attributes)?
                .is_some_and(|attrs| attrs.default);
            let container_rename_all = container_rename_all_rule(
                &s.attributes,
                mode,
                "struct rename_all",
                container_name.unwrap_or("<anonymous struct>"),
            )?;

            rewrite_fields_for_phase(
                &mut s.fields,
                mode,
                original_types,
                generated,
                split_types,
                container_rename_all,
                container_default,
                false,
            )?;
            rewrite_struct_repr_for_phase(s, mode, container_name)?;
            if let Some(intersection) = lower_flattened_struct(s)? {
                *ty = intersection;
            }
        }
        DataType::Enum(e) => {
            filter_enum_variants_for_phase(e, mode)?;
            let container_attrs = SerdeContainerAttrs::from_attributes(&e.attributes)?;

            for (variant_name, variant) in &mut e.variants {
                let rename_rule =
                    enum_variant_field_rename_rule(&container_attrs, variant, mode, variant_name)?;

                rewrite_fields_for_phase(
                    &mut variant.fields,
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

fn lower_flattened_struct(strct: &mut Struct) -> Result<Option<DataType>, Error> {
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
    let mut parts = Vec::new();

    for (name, field) in fields {
        if field_is_flattened(&field) {
            if let Some(ty) = field.ty {
                parts.push(ty);
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
        parts.insert(0, DataType::Struct(base));
    }

    Ok(Some(DataType::Intersection(parts)))
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
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
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

                if let Some(serde_attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? {
                    let rename = select_phase_string(
                        mode,
                        serde_attrs.rename_serialize.as_deref(),
                        serde_attrs.rename_deserialize.as_deref(),
                        "field rename",
                        name.as_ref(),
                    )?;

                    if let Some(rename) = rename {
                        *name = Cow::Owned(rename.to_string());
                    } else if let Some(rule) = rename_all_rule {
                        *name = Cow::Owned(rule.apply_to_field(name.as_ref()));
                    }
                } else if let Some(rule) = rename_all_rule {
                    *name = Cow::Owned(rule.apply_to_field(name.as_ref()));
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
) -> Result<(), Error> {
    if let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)?
        && attrs.skip_serializing_if.is_some()
    {
        if let PhaseRewrite::Serialize = mode {
            field.optional = true;
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
    let Some(attrs) = SerdeFieldAttrs::from_attributes(&field.attributes)? else {
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

fn unnamed_has_effective_payload(unnamed: &UnnamedFields) -> bool {
    unnamed_live_field_count(unnamed) != 0
}

fn unnamed_fields_all_skipped(unnamed: &UnnamedFields) -> bool {
    !unnamed.fields.is_empty() && !unnamed_has_effective_payload(unnamed)
}

fn rewrite_enum_repr_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &Types,
) -> Result<(), Error> {
    if enum_repr_already_rewritten(e) {
        return Ok(());
    }

    let repr = EnumRepr::from_attrs(&e.attributes)?;
    if matches!(repr, EnumRepr::Untagged) {
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
        let mut transformed_variant = match &repr {
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

        transformed_variant.attributes = Default::default();
        transformed.push((Cow::Owned(serialized_name), transformed_variant));
    }

    e.variants = transformed;
    e.attributes = Default::default();

    Ok(())
}

fn enum_repr_already_rewritten(e: &Enum) -> bool {
    e.attributes.is_empty()
        && !e.variants.is_empty()
        && e.variants.iter().all(|(name, variant)| {
            variant.attributes.is_empty() && variant_repr_already_rewritten(name, variant)
        })
}

fn variant_repr_already_rewritten(name: &str, variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) if fields.fields.len() == 1 => fields
            .fields
            .first()
            .and_then(|field| field.ty.as_ref())
            .is_some_and(is_generated_string_literal_datatype),
        Fields::Named(fields) => fields.fields.iter().any(|(field_name, field)| {
            field_name == name
                || field
                    .ty
                    .as_ref()
                    .is_some_and(is_generated_string_literal_datatype)
        }),
        _ => false,
    }
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
        Cow::Borrowed("__specta_identifier_index"),
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
        variants.push((
            Cow::Borrowed("__specta_identifier_other"),
            identifier_union_variant(fallback_ty),
        ));
    }

    e.variants = variants;
    Ok(true)
}

fn container_rename_all_rule(
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

fn enum_variant_field_rename_rule(
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

fn serialized_variant_name(
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
) -> Result<Variant, Error> {
    let skipped_only_unnamed = match &variant.fields {
        Fields::Unnamed(unnamed) => unnamed_fields_all_skipped(unnamed),
        Fields::Unit | Fields::Named(_) => false,
    };

    Ok(match &variant.fields {
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
) -> Result<Variant, Error> {
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
            fields.extend(named.fields.iter().cloned());
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
            let payload_ty = payload_field.ty.clone().expect("checked above");
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
                return Ok(clone_variant_with_unnamed_fields(
                    variant,
                    vec![Field::new(DataType::Intersection(vec![
                        named_fields_datatype(fields),
                        payload_ty,
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

fn variant_has_effective_payload(variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unit => false,
        Fields::Named(named) => !&named.fields.is_empty(),
        Fields::Unnamed(unnamed) => unnamed_has_effective_payload(unnamed),
    }
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
                [] => Some(Field::new(DataType::Tuple(Tuple::new(vec![])))),
                [single] if original_unnamed_len == 1 => Some((*single).clone()),
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

fn internal_tag_payload_compatibility(
    ty: &DataType,
    original_types: &Types,
    seen: &mut HashSet<TypeIdentity>,
) -> Result<Option<bool>, Error> {
    match ty {
        DataType::Map(_) => Ok(Some(false)),
        DataType::Struct(strct) => {
            if SerdeContainerAttrs::from_attributes(&strct.attributes)?
                .is_some_and(|attrs| attrs.transparent)
            {
                let payload_fields = match &strct.fields {
                    Fields::Unit => return Ok(Some(true)),
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
                        return Ok(Some(true));
                    }

                    return Ok(None);
                };

                return internal_tag_payload_compatibility(inner_ty, original_types, seen);
            }

            Ok(match &strct.fields {
                Fields::Named(named) => Some(
                    named
                        .fields
                        .iter()
                        .all(|(_, field)| field.ty.as_ref().is_none()),
                ),
                Fields::Unit | Fields::Unnamed(_) => None,
            })
        }
        DataType::Tuple(tuple) => Ok(tuple.elements.is_empty().then_some(true)),
        DataType::Intersection(types) => {
            let mut is_effectively_empty = true;

            for ty in types {
                let Some(part_empty) =
                    internal_tag_payload_compatibility(ty, original_types, seen)?
                else {
                    return Ok(None);
                };

                is_effectively_empty &= part_empty;
            }

            Ok(Some(is_effectively_empty))
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Inline { dt, .. } = &reference.inner {
                return internal_tag_payload_compatibility(dt, original_types, seen);
            }

            let Some(referenced) = original_types.get(reference) else {
                return Ok(None);
            };
            let Some(referenced_ty) = referenced.ty.as_ref() else {
                return Ok(None);
            };

            let key = TypeIdentity::from_ndt(referenced);
            if !seen.insert(key.clone()) {
                return Ok(Some(false));
            }

            let compatible =
                internal_tag_payload_compatibility(referenced_ty, original_types, seen);
            seen.remove(&key);
            compatible
        }
        DataType::Enum(enm) => match EnumRepr::from_attrs(&enm.attributes) {
            Ok(EnumRepr::Untagged) => {
                let mut is_effectively_empty = true;
                for (_, variant) in &enm.variants {
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
        | DataType::Reference(Reference::Opaque(_))
        | DataType::Generic(_) => Ok(None),
    }
}

fn internal_tag_variant_payload_compatibility(
    variant: &Variant,
    original_types: &Types,
    seen: &mut HashSet<TypeIdentity>,
) -> Result<Option<bool>, Error> {
    match &variant.fields {
        Fields::Unit => Ok(Some(true)),
        Fields::Named(named) => Ok(Some(
            named
                .fields
                .iter()
                .all(|(_, field)| field.ty.as_ref().is_none()),
        )),
        Fields::Unnamed(unnamed) => {
            if unnamed.fields.len() != 1 {
                return Ok(None);
            }

            unnamed
                .fields
                .iter()
                .find_map(|field| field.ty.as_ref())
                .map_or(Ok(None), |ty| {
                    internal_tag_payload_compatibility(ty, original_types, seen)
                })
        }
    }
}

fn has_local_phase_difference(dt: &DataType) -> Result<bool, Error> {
    match dt {
        DataType::Struct(s) => Ok(container_has_local_difference(&s.attributes)?
            || fields_have_local_difference(&s.fields)?),
        DataType::Enum(e) => Ok(container_has_local_difference(&e.attributes)?
            || e.variants
                .iter()
                .try_fold(false, |has_difference, (_, variant)| {
                    if has_difference {
                        return Ok(true);
                    }

                    Ok(variant_has_local_difference(variant)?
                        || fields_have_local_difference(&variant.fields)?)
                })?),
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
            unnamed
                .fields
                .iter()
                .try_fold(false, |has_difference, field| {
                    if has_difference {
                        return Ok(true);
                    }

                    field
                        .ty
                        .as_ref()
                        .map_or(Ok(false), has_local_phase_difference)
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

                    Ok(field_has_local_difference(field)?
                        || field
                            .ty
                            .as_ref()
                            .map_or(Ok(false), has_local_phase_difference)?)
                })
        }
    }
}

fn field_has_local_difference(field: &Field) -> Result<bool, Error> {
    Ok(SerdeFieldAttrs::from_attributes(&field.attributes)?
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

fn variant_has_local_difference(variant: &Variant) -> Result<bool, Error> {
    Ok(SerdeVariantAttrs::from_attributes(&variant.attributes)?
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
) -> Result<(), Error> {
    match dt {
        DataType::Struct(s) => {
            collect_conversion_dependencies(&s.attributes, types, deps)?;
            collect_fields_dependencies(&s.fields, types, deps)?;
        }
        DataType::Enum(e) => {
            collect_conversion_dependencies(&e.attributes, types, deps)?;
            for (_, variant) in &e.variants {
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
                if let Some(ty) = field.ty.as_ref() {
                    collect_dependencies(ty, types, deps)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &named.fields {
                if let Some(ty) = field.ty.as_ref() {
                    collect_dependencies(ty, types, deps)?;
                }
            }
        }
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

fn split_type_name(original: &NamedDataType, mode: PhaseRewrite) -> Result<String, Error> {
    let suffix = match mode {
        PhaseRewrite::Serialize => "Serialize",
        PhaseRewrite::Deserialize => "Deserialize",
        PhaseRewrite::Unified => return Ok(original.name.to_string()),
    };

    let base_name = original
        .ty
        .as_ref()
        .map(|ty| renamed_type_name_for_phase(ty, mode, original.name.as_ref()))
        .transpose()?
        .flatten()
        .unwrap_or_else(|| original.name.to_string());

    Ok(format!("{base_name}_{suffix}"))
}

fn renamed_type_name_for_phase(
    ty: &DataType,
    mode: PhaseRewrite,
    current_name: &str,
) -> Result<Option<String>, Error> {
    let DataType::Struct(strct) = ty else {
        return Ok(None);
    };
    let Some(attrs) = SerdeContainerAttrs::from_attributes(&strct.attributes)? else {
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

        assert!(!field_has_skip_serializing_if(&serialize, &resolved, "nickname"));
        assert!(!field_has_skip_serializing_if(&deserialize, &resolved, "nickname"));
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
            .find_map(|(name, field)| (name == field_name).then(|| field.ty.as_ref()).flatten())
            .expect("field should exist")
    }

    fn field_has_skip_serializing_if(dt: &DataType, types: &Types, field_name: &str) -> bool {
        let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
            panic!("expected named reference");
        };
        let named = reference.get(types).expect("reference should resolve");
        let DataType::Struct(strct) = &named.ty else {
            panic!("expected struct type");
        };
        let specta::datatype::Fields::Named(fields) = &strct.fields else {
            panic!("expected named fields");
        };
        fields
            .fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, field)| field.attributes.contains_key(parser::FIELD_SKIP_SERIALIZING_IF))
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
