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
    },
};

mod inflection;
mod parser;
mod repr;

pub use inflection::RenameRule;
pub use parser::{
    ConversionType, SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs, merge_container_attrs,
    merge_field_attrs, merge_variant_attrs,
};

pub fn apply(types: TypeCollection) -> TypeCollection {
    types.map(|mut ty| {
        rename_datatype_fields(ty.ty_mut());
        ty
    })
}

pub fn apply_phases(types: TypeCollection) -> TypeCollection {
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

    out.iter_mut(|ndt| {
        let Some(mode) = rewrite_plan.get(&TypeKey::from_ndt(ndt)).copied() else {
            return;
        };

        rewrite_datatype_for_phase(ndt.ty_mut(), mode, &types, &generated, &split_types);
    });

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

    out
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
) {
    match ty {
        DataType::Struct(s) => {
            rewrite_fields_for_phase(s.fields_mut(), mode, original_types, generated, split_types)
        }
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                rewrite_fields_for_phase(
                    variant.fields_mut(),
                    mode,
                    original_types,
                    generated,
                    split_types,
                );
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types);
            }
        }
        DataType::List(list) => {
            rewrite_datatype_for_phase(list.ty_mut(), mode, original_types, generated, split_types)
        }
        DataType::Map(map) => {
            rewrite_datatype_for_phase(
                map.key_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
            );
            rewrite_datatype_for_phase(
                map.value_ty_mut(),
                mode,
                original_types,
                generated,
                split_types,
            );
        }
        DataType::Nullable(inner) => {
            rewrite_datatype_for_phase(inner, mode, original_types, generated, split_types)
        }
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced_ndt) = reference.get(original_types) else {
                return;
            };
            let key = TypeKey::from_ndt(referenced_ndt);
            let Some(target) = generated.get(&key) else {
                return;
            };

            let generics = reference
                .generics()
                .iter()
                .map(|(generic, dt)| {
                    let mut dt = dt.clone();
                    rewrite_datatype_for_phase(
                        &mut dt,
                        mode,
                        original_types,
                        generated,
                        split_types,
                    );
                    (generic.clone(), dt)
                })
                .collect::<Vec<_>>();

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
}

fn rewrite_fields_for_phase(
    fields: &mut Fields,
    mode: PhaseRewrite,
    original_types: &TypeCollection,
    generated: &HashMap<TypeKey, GeneratedTypes>,
    split_types: &HashSet<TypeKey>,
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                apply_field_attrs(field);
                rewrite_field_for_phase(field, mode, original_types, generated, split_types);
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

                rewrite_field_for_phase(field, mode, original_types, generated, split_types);
            }
        }
    }
}

fn rewrite_field_for_phase(
    field: &mut Field,
    mode: PhaseRewrite,
    original_types: &TypeCollection,
    generated: &HashMap<TypeKey, GeneratedTypes>,
    split_types: &HashSet<TypeKey>,
) {
    if let Some(ty) = field.ty_mut() {
        rewrite_datatype_for_phase(ty, mode, original_types, generated, split_types);
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

fn rename_datatype_fields(ty: &mut DataType) {
    match ty {
        DataType::Struct(s) => rename_fields(s.fields_mut()),
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                rename_fields(variant.fields_mut());
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rename_datatype_fields(ty);
            }
        }
        DataType::List(list) => rename_datatype_fields(list.ty_mut()),
        DataType::Map(map) => {
            rename_datatype_fields(map.key_ty_mut());
            rename_datatype_fields(map.value_ty_mut());
        }
        DataType::Nullable(inner) => rename_datatype_fields(inner),
        DataType::Primitive(_) | DataType::Reference(_) => {}
    }
}

fn rename_fields(fields: &mut Fields) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                apply_field_attrs(field);
                rename_field_type(field);
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
                            panic!(
                                "specta-serde: incompatible field rename for both-phase export on field '{name}': serialize={serialize:?}, deserialize={deserialize:?}",
                            );
                        }
                    }
                }

                rename_field_type(field);
            }
        }
    }
}

fn rename_field_type(field: &mut Field) {
    if let Some(ty) = field.ty_mut() {
        rename_datatype_fields(ty);
    }
}

fn apply_field_attrs(field: &mut Field) {
    let flatten = field
        .attributes()
        .get::<SerdeFieldAttrs>()
        .is_some_and(|attrs| attrs.flatten);
    field.set_flatten(flatten);
}
