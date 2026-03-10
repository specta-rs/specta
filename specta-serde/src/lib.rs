//! [Serde](https://serde.rs) support for Specta
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use std::borrow::Cow;

use specta::{
    TypeCollection,
    datatype::{DataType, Field, Fields},
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
