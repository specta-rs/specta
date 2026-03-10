//! [Serde](https://serde.rs) support for Specta
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use std::{borrow::Cow, collections::HashMap};

use specta::{
    datatype::{DataType, Enum, EnumVariant, Field, Fields, NamedDataTypeBuilder, Reference},
    TypeCollection,
};

mod inflection;
mod parser;
mod repr;

pub use inflection::RenameRule;
pub use parser::{
    merge_container_attrs, merge_field_attrs, merge_variant_attrs, ConversionType,
    SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs,
};

pub fn apply(types: TypeCollection) -> TypeCollection {
    types.map(|mut ty| {
        rename_datatype_fields(ty.ty_mut());
        ty
    })
}

pub fn apply_phases(types: TypeCollection) -> TypeCollection {
    let originals = types.into_unsorted_iter().cloned().collect::<Vec<_>>();
    let mut out = TypeCollection::default();
    let mut phase_types = HashMap::new();

    for original in &originals {
        let key = TypeKey::from_ndt(original);
        let generics = original.generics().to_vec();

        let mut serialize_builder = NamedDataTypeBuilder::new(
            format!("{}_Serialize", original.name()),
            generics.clone(),
            original.ty().clone(),
        )
        .docs(original.docs().clone())
        .module_path(original.module_path().clone());
        if let Some(deprecated) = original.deprecated().cloned() {
            serialize_builder = serialize_builder.deprecated(deprecated);
        }
        if !original.requires_reference(&types) {
            serialize_builder = serialize_builder.inline();
        }

        let mut deserialize_builder = NamedDataTypeBuilder::new(
            format!("{}_Deserialize", original.name()),
            generics,
            original.ty().clone(),
        )
        .docs(original.docs().clone())
        .module_path(original.module_path().clone());
        if let Some(deprecated) = original.deprecated().cloned() {
            deserialize_builder = deserialize_builder.deprecated(deprecated);
        }
        if !original.requires_reference(&types) {
            deserialize_builder = deserialize_builder.inline();
        }

        let serialize_ndt = serialize_builder.build(&mut out);
        let deserialize_ndt = deserialize_builder.build(&mut out);

        phase_types.insert(key, (serialize_ndt, deserialize_ndt));
    }

    out.iter_mut(|ndt| {
        let base_name = if let Some(name) = ndt.name().strip_suffix("_Serialize") {
            Some((name, Phase::Serialize))
        } else if let Some(name) = ndt.name().strip_suffix("_Deserialize") {
            Some((name, Phase::Deserialize))
        } else {
            None
        };

        let Some((base_name, phase)) = base_name else {
            return;
        };

        let key = TypeKey {
            name: base_name.to_string(),
            module_path: ndt.module_path().to_string(),
        };

        if phase_types.contains_key(&key) {
            rewrite_datatype_for_phase(ndt.ty_mut(), phase, &types, &phase_types);
        }
    });

    for original in &originals {
        let key = TypeKey::from_ndt(original);
        let Some((serialize_ndt, deserialize_ndt)) = phase_types.get(&key) else {
            continue;
        };

        let generic_args = original
            .generics()
            .iter()
            .map(|(generic, _)| (generic.clone(), generic.clone().into()))
            .collect::<Vec<_>>();

        let serialize_variant = EnumVariant::unnamed()
            .field(Field::new(
                serialize_ndt.reference(generic_args.clone()).into(),
            ))
            .build();
        let deserialize_variant = EnumVariant::unnamed()
            .field(Field::new(deserialize_ndt.reference(generic_args).into()))
            .build();

        let mut wrapper = Enum::new();
        wrapper
            .variants_mut()
            .push((Cow::Borrowed("Serialize"), serialize_variant));
        wrapper
            .variants_mut()
            .push((Cow::Borrowed("Deserialize"), deserialize_variant));

        let mut wrapper_builder = NamedDataTypeBuilder::new(
            original.name().clone(),
            original.generics().to_vec(),
            DataType::Enum(wrapper),
        )
        .docs(original.docs().clone())
        .module_path(original.module_path().clone());
        if let Some(deprecated) = original.deprecated().cloned() {
            wrapper_builder = wrapper_builder.deprecated(deprecated);
        }
        if !original.requires_reference(&types) {
            wrapper_builder = wrapper_builder.inline();
        }

        let _ = wrapper_builder.build(&mut out);
    }

    out
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Serialize,
    Deserialize,
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
    phase: Phase,
    original_types: &TypeCollection,
    phase_types: &HashMap<
        TypeKey,
        (
            specta::datatype::NamedDataType,
            specta::datatype::NamedDataType,
        ),
    >,
) {
    match ty {
        DataType::Struct(s) => {
            rewrite_fields_for_phase(s.fields_mut(), phase, original_types, phase_types)
        }
        DataType::Enum(e) => {
            for (_, variant) in e.variants_mut() {
                rewrite_fields_for_phase(variant.fields_mut(), phase, original_types, phase_types);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in tuple.elements_mut() {
                rewrite_datatype_for_phase(ty, phase, original_types, phase_types);
            }
        }
        DataType::List(list) => {
            rewrite_datatype_for_phase(list.ty_mut(), phase, original_types, phase_types)
        }
        DataType::Map(map) => {
            rewrite_datatype_for_phase(map.key_ty_mut(), phase, original_types, phase_types);
            rewrite_datatype_for_phase(map.value_ty_mut(), phase, original_types, phase_types);
        }
        DataType::Nullable(inner) => {
            rewrite_datatype_for_phase(inner, phase, original_types, phase_types)
        }
        DataType::Reference(Reference::Named(reference)) => {
            let Some(referenced_ndt) = reference.get(original_types) else {
                return;
            };
            let key = TypeKey::from_ndt(referenced_ndt);
            let Some((serialize_ty, deserialize_ty)) = phase_types.get(&key) else {
                return;
            };

            let generics = reference
                .generics()
                .iter()
                .map(|(generic, dt)| {
                    let mut dt = dt.clone();
                    rewrite_datatype_for_phase(&mut dt, phase, original_types, phase_types);
                    (generic.clone(), dt)
                })
                .collect::<Vec<_>>();

            let mut new_reference = match phase {
                Phase::Serialize => serialize_ty.reference(generics),
                Phase::Deserialize => deserialize_ty.reference(generics),
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
    phase: Phase,
    original_types: &TypeCollection,
    phase_types: &HashMap<
        TypeKey,
        (
            specta::datatype::NamedDataType,
            specta::datatype::NamedDataType,
        ),
    >,
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in unnamed.fields_mut() {
                rewrite_field_for_phase(field, phase, original_types, phase_types);
            }
        }
        Fields::Named(named) => {
            for (name, field) in named.fields_mut() {
                if let Some(serde_attrs) = field.attributes().get::<SerdeFieldAttrs>() {
                    let rename = match phase {
                        Phase::Serialize => serde_attrs
                            .rename_serialize
                            .as_deref()
                            .or(serde_attrs.rename_deserialize.as_deref()),
                        Phase::Deserialize => serde_attrs
                            .rename_deserialize
                            .as_deref()
                            .or(serde_attrs.rename_serialize.as_deref()),
                    };

                    if let Some(rename) = rename {
                        *name = Cow::Owned(rename.to_string());
                    }
                }

                rewrite_field_for_phase(field, phase, original_types, phase_types);
            }
        }
    }
}

fn rewrite_field_for_phase(
    field: &mut Field,
    phase: Phase,
    original_types: &TypeCollection,
    phase_types: &HashMap<
        TypeKey,
        (
            specta::datatype::NamedDataType,
            specta::datatype::NamedDataType,
        ),
    >,
) {
    if let Some(ty) = field.ty_mut() {
        rewrite_datatype_for_phase(ty, phase, original_types, phase_types);
    }
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
                rename_field_type(field);
            }
        }
        Fields::Named(named) => {
            for (name, field) in named.fields_mut() {
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
