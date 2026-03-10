//! [Serde](https://serde.rs) support for Specta
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
    TypeCollection,
    datatype::{
        DataType, Enum, EnumVariant, Field, Fields, NamedDataType, NamedDataTypeBuilder, Reference,
        Struct, Tuple,
    },
    internal,
};

mod error;
mod inflection;
mod parser;
mod repr;

pub use error::{Error, Result};
pub use inflection::RenameRule;
pub use parser::{
    ConversionType, SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs, merge_container_attrs,
    merge_field_attrs, merge_variant_attrs,
};
use repr::EnumRepr;

pub fn apply(types: TypeCollection) -> Result<TypeCollection> {
    let mut out = types;
    let mut rewrite_err = None;

    out.iter_mut(|ndt| {
        if rewrite_err.is_some() {
            return;
        }

        if let Err(err) = rename_datatype_fields(ndt.ty_mut()) {
            rewrite_err = Some(err);
        }
    });

    if let Some(err) = rewrite_err {
        return Err(err);
    }

    Ok(out)
}

pub fn apply_phases(types: TypeCollection) -> Result<TypeCollection> {
    let originals = types.into_unsorted_iter().cloned().collect::<Vec<_>>();
    let mut dependencies = HashMap::<TypeKey, HashSet<TypeKey>>::new();
    let mut reverse_dependencies = HashMap::<TypeKey, HashSet<TypeKey>>::new();

    for original in &originals {
        let key = TypeKey::from_ndt(original);
        let mut deps = HashSet::new();
        collect_dependencies(original.ty(), &types, &mut deps);
        for dep in &deps {
            reverse_dependencies
                .entry(dep.clone())
                .or_default()
                .insert(key.clone());
        }
        dependencies.insert(key, deps);
    }

    let mut split_types = originals
        .iter()
        .filter(|ndt| has_local_phase_difference(ndt.ty()))
        .map(TypeKey::from_ndt)
        .collect::<HashSet<_>>();

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
    let mut generated = HashMap::<TypeKey, GeneratedTypes>::new();
    let mut rewrite_plan = HashMap::<TypeKey, PhaseRewrite>::new();

    for original in &originals {
        let key = TypeKey::from_ndt(original);

        if split_types.contains(&key) {
            let serialize_ndt = build_from_original(
                original,
                format!("{}_Serialize", original.name()),
                original.generics().to_vec(),
                original.ty().clone(),
                &types,
                &mut out,
            );
            let deserialize_ndt = build_from_original(
                original,
                format!("{}_Deserialize", original.name()),
                original.generics().to_vec(),
                original.ty().clone(),
                &types,
                &mut out,
            );

            rewrite_plan.insert(TypeKey::from_ndt(&serialize_ndt), PhaseRewrite::Serialize);
            rewrite_plan.insert(
                TypeKey::from_ndt(&deserialize_ndt),
                PhaseRewrite::Deserialize,
            );
            generated.insert(
                key,
                GeneratedTypes::Split {
                    serialize: serialize_ndt,
                    deserialize: deserialize_ndt,
                },
            );
        } else {
            rewrite_plan.insert(key.clone(), PhaseRewrite::Unified);
            generated.insert(key, GeneratedTypes::Unified(original.clone()));
        }
    }

    let mut rewrite_err = None;
    out.iter_mut(|ndt| {
        if rewrite_err.is_some() {
            return;
        }

        let Some(mode) = rewrite_plan.get(&TypeKey::from_ndt(ndt)).copied() else {
            return;
        };

        if let Err(err) =
            rewrite_datatype_for_phase(ndt.ty_mut(), mode, &types, &generated, &split_types)
        {
            rewrite_err = Some(err);
        }
    });

    if let Some(err) = rewrite_err {
        return Err(err);
    }

    out.iter_mut(|ndt| {
        let key = TypeKey::from_ndt(ndt);
        if !split_types.contains(&key) {
            return;
        }

        let Some(GeneratedTypes::Split {
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

        let mut serialize_variant = EnumVariant::unnamed().build();
        if let Fields::Unnamed(fields) = serialize_variant.fields_mut() {
            fields
                .fields_mut()
                .push(Field::new(serialize.reference(generic_args.clone()).into()));
        }

        let mut deserialize_variant = EnumVariant::unnamed().build();
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

    debug_assert_eq!(dependencies.len(), originals.len());

    Ok(out)
}

#[derive(Debug, Clone, Copy)]
enum PhaseRewrite {
    Unified,
    Serialize,
    Deserialize,
}

#[derive(Debug, Clone)]
enum GeneratedTypes {
    Unified(NamedDataType),
    Split {
        serialize: NamedDataType,
        deserialize: NamedDataType,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TypeKey {
    name: String,
    module_path: String,
}

impl TypeKey {
    fn from_ndt(ty: &specta::datatype::NamedDataType) -> Self {
        Self {
            name: ty.name().to_string(),
            module_path: ty.module_path().to_string(),
        }
    }
}

fn rewrite_datatype_for_phase(
    ty: &mut DataType,
    mode: PhaseRewrite,
    original_types: &TypeCollection,
    generated: &HashMap<TypeKey, GeneratedTypes>,
    split_types: &HashSet<TypeKey>,
) -> Result<()> {
    match ty {
        DataType::Struct(s) => {
            rewrite_fields_for_phase(s.fields_mut(), mode, original_types, generated, split_types)?
        }
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                rewrite_fields_for_phase(
                    variant.fields_mut(),
                    mode,
                    original_types,
                    generated,
                    split_types,
                )?;
            }

            rewrite_enum_repr_for_phase(e, mode, original_types)?;
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types)?;
            }
        }
        DataType::List(list) => {
            rewrite_datatype_for_phase(list.ty_mut(), mode, original_types, generated, split_types)?
        }
        DataType::Map(map) => {
            rewrite_datatype_for_phase(
                map.key_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
            )?;
            rewrite_datatype_for_phase(
                map.value_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
            )?;
        }
        DataType::Nullable(inner) => {
            rewrite_datatype_for_phase(inner, mode, original_types, generated, split_types)?
        }
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced_ndt) = reference.get(original_types) else {
                return Ok(());
            };
            let key = TypeKey::from_ndt(referenced_ndt);
            let Some(target) = generated.get(&key) else {
                return Ok(());
            };

            let mut generics = Vec::with_capacity(reference.generics().len());
            for (generic, dt) in reference.generics() {
                let mut dt = dt.clone();
                rewrite_datatype_for_phase(&mut dt, mode, original_types, generated, split_types)?;
                generics.push((generic.clone(), dt));
            }

            let mut new_reference = match (target, mode) {
                (GeneratedTypes::Unified(target), _) => target.reference(generics),
                (GeneratedTypes::Split { serialize, .. }, PhaseRewrite::Unified) => {
                    debug_assert!(
                        !split_types.contains(&key),
                        "Unified mode should not reference split types"
                    );
                    serialize.reference(generics)
                }
                (GeneratedTypes::Split { serialize, .. }, PhaseRewrite::Serialize) => {
                    serialize.reference(generics)
                }
                (GeneratedTypes::Split { deserialize, .. }, PhaseRewrite::Deserialize) => {
                    deserialize.reference(generics)
                }
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
    original_types: &TypeCollection,
    generated: &HashMap<TypeKey, GeneratedTypes>,
    split_types: &HashSet<TypeKey>,
) -> Result<()> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                apply_field_attrs(field);
                rewrite_field_for_phase(field, mode, original_types, generated, split_types)?;
            }
        }
        Fields::Named(named) => {
            for (name, field) in named.fields_mut() {
                apply_field_attrs(field);

                if let Some(serde_attrs) = field.attributes().get::<SerdeFieldAttrs>() {
                    let rename = match mode {
                        PhaseRewrite::Serialize => serde_attrs.rename_serialize.as_deref(),
                        PhaseRewrite::Deserialize => serde_attrs.rename_deserialize.as_deref(),
                        PhaseRewrite::Unified => serde_attrs
                            .rename_serialize
                            .as_deref()
                            .or(serde_attrs.rename_deserialize.as_deref()),
                    };

                    if let Some(rename) = rename {
                        *name = Cow::Owned(rename.to_string());
                    }
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
    original_types: &TypeCollection,
    generated: &HashMap<TypeKey, GeneratedTypes>,
    split_types: &HashSet<TypeKey>,
) -> Result<()> {
    if let Some(ty) = field.ty_mut() {
        rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types)?;
    }

    Ok(())
}

fn rewrite_enum_repr_for_phase(
    e: &mut Enum,
    mode: PhaseRewrite,
    original_types: &TypeCollection,
) -> Result<()> {
    let repr = enum_repr_from_attrs(e.attributes())?;
    if matches!(repr, EnumRepr::Untagged) {
        return Ok(());
    }

    let container_attrs = e.attributes().get::<SerdeContainerAttrs>().cloned();
    let variants = std::mem::take(e.variants_mut());
    let mut transformed = Vec::with_capacity(variants.len());
    for (variant_name, variant) in variants {
        let serialized_name =
            serialized_variant_name(&variant_name, &variant, &container_attrs, mode)?;
        let transformed_variant = match &repr {
            EnumRepr::External => transform_external_variant(serialized_name.clone(), &variant)?,
            EnumRepr::Internal { tag } => transform_internal_variant(
                serialized_name.clone(),
                tag.as_ref(),
                &variant,
                original_types,
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
                )?
            }
            EnumRepr::Untagged => unreachable!(),
        };

        transformed.push((Cow::Owned(serialized_name), transformed_variant));
    }

    *e.variants_mut() = transformed;

    Ok(())
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
    variant: &EnumVariant,
    container_attrs: &Option<SerdeContainerAttrs>,
    mode: PhaseRewrite,
) -> Result<String> {
    let variant_attrs = variant.attributes().get::<SerdeVariantAttrs>();

    if let Some(rename) = select_phase_string(
        mode,
        variant_attrs.and_then(|attrs| attrs.rename_serialize.as_deref()),
        variant_attrs.and_then(|attrs| attrs.rename_deserialize.as_deref()),
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

fn transform_external_variant(
    serialized_name: String,
    variant: &EnumVariant,
) -> Result<EnumVariant> {
    Ok(match variant.fields() {
        Fields::Unit => clone_variant_with_unnamed_fields(
            variant,
            vec![Field::new(string_literal_datatype(serialized_name))],
        ),
        _ => {
            let payload = variant_payload_datatype(variant)
                .ok_or_else(|| Error::invalid_external_tagged_variant(serialized_name.clone()))?;

            clone_variant_with_named_fields(
                variant,
                vec![(Cow::Owned(serialized_name), Field::new(payload))],
            )
        }
    })
}

fn transform_adjacent_variant(
    serialized_name: String,
    tag: &str,
    content: &str,
    variant: &EnumVariant,
) -> Result<EnumVariant> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(string_literal_datatype(serialized_name.clone())),
    )];

    if !matches!(variant.fields(), Fields::Unit) {
        let payload = variant_payload_datatype(variant)
            .ok_or_else(|| Error::invalid_adjacent_tagged_variant(serialized_name.clone()))?;
        fields.push((Cow::Owned(content.to_string()), Field::new(payload)));
    }

    Ok(clone_variant_with_named_fields(variant, fields))
}

fn transform_internal_variant(
    serialized_name: String,
    tag: &str,
    variant: &EnumVariant,
    original_types: &TypeCollection,
) -> Result<EnumVariant> {
    let mut fields = vec![(
        Cow::Owned(tag.to_string()),
        Field::new(string_literal_datatype(serialized_name.clone())),
    )];

    match variant.fields() {
        Fields::Unit => {}
        Fields::Named(named) => {
            fields.extend(named.fields().iter().cloned());
        }
        Fields::Unnamed(unnamed) => {
            let non_skipped = unnamed
                .fields()
                .iter()
                .filter_map(|field| field.ty().cloned())
                .collect::<Vec<_>>();

            if unnamed.fields().len() != 1 || non_skipped.len() != 1 {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "tuple variant must have exactly one non-skipped field",
                ));
            }

            let payload_ty = non_skipped.into_iter().next().expect("checked above");
            if !is_internal_tag_compatible(&payload_ty, original_types, &mut HashSet::new()) {
                return Err(Error::invalid_internally_tagged_variant(
                    serialized_name,
                    "payload cannot be merged with a tag",
                ));
            }

            if !matches!(&payload_ty, DataType::Tuple(tuple) if tuple.elements().is_empty()) {
                let mut flattened = Field::new(payload_ty);
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
        .push((Cow::Owned(value), EnumVariant::unit()));
    DataType::Enum(value_enum)
}

fn variant_payload_datatype(variant: &EnumVariant) -> Option<DataType> {
    match variant.fields() {
        Fields::Unit => Some(DataType::Tuple(Tuple::new(vec![]))),
        Fields::Named(named) => {
            let mut out = Struct::unit();
            out.set_fields(Fields::Named(named.clone()));
            Some(DataType::Struct(out))
        }
        Fields::Unnamed(unnamed) => {
            let fields = unnamed
                .fields()
                .iter()
                .filter_map(|field| field.ty().cloned())
                .collect::<Vec<_>>();

            match fields.as_slice() {
                [] if unnamed.fields().is_empty() => Some(DataType::Tuple(Tuple::new(vec![]))),
                [] => None,
                [single] if unnamed.fields().len() == 1 => Some(single.clone()),
                _ => Some(DataType::Tuple(Tuple::new(fields))),
            }
        }
    }
}

fn clone_variant_with_named_fields(
    original: &EnumVariant,
    fields: Vec<(Cow<'static, str>, Field)>,
) -> EnumVariant {
    let mut transformed = original.clone();
    transformed.set_fields(internal::construct::fields_named(
        fields,
        specta::datatype::Attributes::default(),
    ));
    transformed
}

fn clone_variant_with_unnamed_fields(original: &EnumVariant, fields: Vec<Field>) -> EnumVariant {
    let mut transformed = original.clone();
    transformed.set_fields(internal::construct::fields_unnamed(
        fields,
        specta::datatype::Attributes::default(),
    ));
    transformed
}

fn is_internal_tag_compatible(
    ty: &DataType,
    original_types: &TypeCollection,
    seen: &mut HashSet<TypeKey>,
) -> bool {
    match ty {
        DataType::Map(_) => true,
        DataType::Struct(strct) => matches!(strct.fields(), Fields::Named(_)),
        DataType::Tuple(tuple) => tuple.elements().is_empty(),
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced) = reference.get(original_types) else {
                return false;
            };

            let key = TypeKey::from_ndt(referenced);
            if !seen.insert(key.clone()) {
                return true;
            }

            let compatible = is_internal_tag_compatible(referenced.ty(), original_types, seen);
            seen.remove(&key);
            compatible
        }
        DataType::Enum(enm) => {
            enm.attributes()
                .get::<SerdeContainerAttrs>()
                .is_some_and(|attrs| attrs.untagged)
                && enm.variants().iter().all(|(_, variant)| {
                    is_internal_variant_compatible(variant, original_types, seen)
                })
        }
        DataType::Primitive(_)
        | DataType::List(_)
        | DataType::Nullable(_)
        | DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_)) => false,
    }
}

fn is_internal_variant_compatible(
    variant: &EnumVariant,
    original_types: &TypeCollection,
    seen: &mut HashSet<TypeKey>,
) -> bool {
    match variant.fields() {
        Fields::Unit => true,
        Fields::Named(_) => true,
        Fields::Unnamed(unnamed) => {
            if unnamed.fields().len() != 1 {
                return false;
            }

            unnamed
                .fields()
                .iter()
                .find_map(|field| field.ty())
                .is_some_and(|ty| is_internal_tag_compatible(ty, original_types, seen))
        }
    }
}

fn has_local_phase_difference(dt: &DataType) -> bool {
    match dt {
        DataType::Struct(s) => fields_have_local_difference(s.fields()),
        DataType::Enum(e) => e
            .variants()
            .iter()
            .any(|(_, variant)| fields_have_local_difference(variant.fields())),
        DataType::Tuple(tuple) => tuple.elements().iter().any(has_local_phase_difference),
        DataType::List(list) => has_local_phase_difference(list.ty()),
        DataType::Map(map) => {
            has_local_phase_difference(map.key_ty()) || has_local_phase_difference(map.value_ty())
        }
        DataType::Nullable(inner) => has_local_phase_difference(inner),
        DataType::Primitive(_) | DataType::Reference(_) => false,
    }
}

fn fields_have_local_difference(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(unnamed) => unnamed
            .fields()
            .iter()
            .any(|field| field.ty().is_some_and(has_local_phase_difference)),
        Fields::Named(named) => named.fields().iter().any(|(_, field)| {
            field_has_local_difference(field) || field.ty().is_some_and(has_local_phase_difference)
        }),
    }
}

fn field_has_local_difference(field: &Field) -> bool {
    field
        .attributes()
        .get::<SerdeFieldAttrs>()
        .map(|attrs| attrs.rename_serialize.as_deref() != attrs.rename_deserialize.as_deref())
        .unwrap_or_default()
}

fn collect_dependencies(dt: &DataType, types: &TypeCollection, deps: &mut HashSet<TypeKey>) {
    match dt {
        DataType::Struct(s) => collect_fields_dependencies(s.fields(), types, deps),
        DataType::Enum(e) => {
            for (_, variant) in e.variants() {
                collect_fields_dependencies(variant.fields(), types, deps);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements() {
                collect_dependencies(ty, types, deps);
            }
        }
        DataType::List(list) => collect_dependencies(list.ty(), types, deps),
        DataType::Map(map) => {
            collect_dependencies(map.key_ty(), types, deps);
            collect_dependencies(map.value_ty(), types, deps);
        }
        DataType::Nullable(inner) => collect_dependencies(inner, types, deps),
        DataType::Reference(Reference::Named(reference)) => {
            if let Some(referenced) = reference.get(types) {
                deps.insert(TypeKey::from_ndt(referenced));
            }

            for (_, generic) in reference.generics() {
                collect_dependencies(generic, types, deps);
            }
        }
        DataType::Primitive(_)
        | DataType::Reference(Reference::Generic(_))
        | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn collect_fields_dependencies(
    fields: &Fields,
    types: &TypeCollection,
    deps: &mut HashSet<TypeKey>,
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields() {
                if let Some(ty) = field.ty() {
                    collect_dependencies(ty, types, deps);
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in named.fields() {
                if let Some(ty) = field.ty() {
                    collect_dependencies(ty, types, deps);
                }
            }
        }
    }
}

fn build_from_original(
    original: &NamedDataType,
    name: impl Into<Cow<'static, str>>,
    generics: Vec<(specta::datatype::GenericReference, Cow<'static, str>)>,
    ty: DataType,
    types: &TypeCollection,
    out: &mut TypeCollection,
) -> NamedDataType {
    let mut builder = NamedDataTypeBuilder::new(name, generics, ty)
        .docs(original.docs().clone())
        .module_path(original.module_path().clone());
    if let Some(deprecated) = original.deprecated().cloned() {
        builder = builder.deprecated(deprecated);
    }
    if !original.requires_reference(types) {
        builder = builder.inline();
    }
    builder.build(out)
}

fn rename_datatype_fields(ty: &mut DataType) -> Result<()> {
    match ty {
        DataType::Struct(s) => rename_fields(s.fields_mut())?,
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                rename_fields(variant.fields_mut())?;
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rename_datatype_fields(ty)?;
            }
        }
        DataType::List(list) => rename_datatype_fields(list.ty_mut())?,
        DataType::Map(map) => {
            rename_datatype_fields(map.key_ty_mut())?;
            rename_datatype_fields(map.value_ty_mut())?;
        }
        DataType::Nullable(inner) => rename_datatype_fields(inner)?,
        DataType::Primitive(_) | DataType::Reference(_) => {}
    }

    Ok(())
}

fn rename_fields(fields: &mut Fields) -> Result<()> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                apply_field_attrs(field);
                rename_field_type(field)?;
            }
        }
        Fields::Named(named) => {
            for (name, field) in named.fields_mut() {
                apply_field_attrs(field);

                if let Some(serde_attrs) = field.attributes().get::<SerdeFieldAttrs>() {
                    match (
                        serde_attrs.rename_serialize.as_deref(),
                        serde_attrs.rename_deserialize.as_deref(),
                    ) {
                        (None, None) => {}
                        (Some(serialize), Some(deserialize)) if serialize == deserialize => {
                            *name = Cow::Owned(serialize.to_string());
                        }
                        (serialize, deserialize) => {
                            return Err(Error::incompatible_rename(
                                "field rename",
                                name.to_string(),
                                serialize.map(ToString::to_string),
                                deserialize.map(ToString::to_string),
                            ));
                        }
                    }
                }

                rename_field_type(field)?;
            }
        }
    }

    Ok(())
}

fn rename_field_type(field: &mut Field) -> Result<()> {
    if let Some(ty) = field.ty_mut() {
        rename_datatype_fields(ty)?;
    }

    Ok(())
}

fn apply_field_attrs(field: &mut Field) {
    let flatten = field
        .attributes()
        .get::<SerdeFieldAttrs>()
        .is_some_and(|attrs| attrs.flatten);
    field.set_flatten(flatten);
}
