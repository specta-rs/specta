//! Interop for Specta v1. This will not be in the final v2 release but is good for the meantime.

use std::borrow::Cow;

use specta1::NamedDataType;

use crate::{
    datatype::{DataType, DeprecatedType, LiteralType, PrimitiveType},
    TypeCollection,
};

/// Allow for conversion between Specta v2 and v1 data types.
pub fn specta_v2_to_v1(datatype: DataType) -> specta1::DataType {
    match datatype {
        DataType::Any => specta1::DataType::Any,
        DataType::Unknown => panic!("Specta v1 does not support unknown types"),
        DataType::Primitive(p) => specta1::DataType::Primitive(match p {
            PrimitiveType::i8 => specta1::PrimitiveType::i8,
            PrimitiveType::i16 => specta1::PrimitiveType::i16,
            PrimitiveType::i32 => specta1::PrimitiveType::i32,
            PrimitiveType::i64 => specta1::PrimitiveType::i64,
            PrimitiveType::i128 => specta1::PrimitiveType::i128,
            PrimitiveType::isize => specta1::PrimitiveType::isize,
            PrimitiveType::u8 => specta1::PrimitiveType::u8,
            PrimitiveType::u16 => specta1::PrimitiveType::u16,
            PrimitiveType::u32 => specta1::PrimitiveType::u32,
            PrimitiveType::u64 => specta1::PrimitiveType::u64,
            PrimitiveType::u128 => specta1::PrimitiveType::u128,
            PrimitiveType::usize => specta1::PrimitiveType::usize,
            PrimitiveType::f32 => specta1::PrimitiveType::f32,
            PrimitiveType::f64 => specta1::PrimitiveType::f64,
            PrimitiveType::bool => specta1::PrimitiveType::bool,
            PrimitiveType::char => specta1::PrimitiveType::char,
            PrimitiveType::String => specta1::PrimitiveType::String,
        }),
        DataType::Literal(l) => specta1::DataType::Literal(match l {
            LiteralType::i8(v) => specta1::LiteralType::i8(v),
            LiteralType::i16(v) => specta1::LiteralType::i16(v),
            LiteralType::i32(v) => specta1::LiteralType::i32(v),
            LiteralType::u8(v) => specta1::LiteralType::u8(v),
            LiteralType::u16(v) => specta1::LiteralType::u16(v),
            LiteralType::u32(v) => specta1::LiteralType::u32(v),
            LiteralType::f32(v) => specta1::LiteralType::f32(v),
            LiteralType::f64(v) => specta1::LiteralType::f64(v),
            LiteralType::bool(v) => specta1::LiteralType::bool(v),
            LiteralType::String(v) => specta1::LiteralType::String(v),
            LiteralType::char(_) => panic!("Specta v1 does not support char literals"),
            LiteralType::None => specta1::LiteralType::None,
        }),
        DataType::List(l) => specta1::DataType::List(Box::new(specta_v2_to_v1(l.ty().clone()))),
        DataType::Map(m) => specta1::DataType::Record(Box::new((
            specta_v2_to_v1(m.key_ty().clone()),
            specta_v2_to_v1(m.value_ty().clone()),
        ))),
        DataType::Nullable(n) => specta1::DataType::Nullable(Box::new(specta_v2_to_v1(*n))),
        DataType::Struct(s) => specta1::DataType::Object(specta1::ObjectType {
            tag: match s.tag() {
                Some(v) => Some(match v {
                    Cow::Borrowed(v) => v,
                    Cow::Owned(v) => String::leak(v.clone()),
                }),
                None => None,
            },
            generics: s
                .generics
                .into_iter()
                .map(|g| match g.0 {
                    Cow::Borrowed(v) => v,
                    Cow::Owned(v) => String::leak(v),
                })
                .collect(),
            fields: match s.fields {
                crate::datatype::StructFields::Unit => vec![],
                crate::datatype::StructFields::Unnamed(f) => f
                    .fields
                    .into_iter()
                    .map(|f| specta1::ObjectField {
                        key: "", // TODO: Imagine an unnamed struct field having a name
                        optional: f.optional,
                        flatten: f.flatten,
                        ty: specta_v2_to_v1(f.ty.unwrap_or(DataType::Unknown)),
                    })
                    .collect(),
                crate::datatype::StructFields::Named(f) => f
                    .fields
                    .into_iter()
                    .map(|(name, f)| specta1::ObjectField {
                        key: match name {
                            Cow::Borrowed(v) => v,
                            Cow::Owned(v) => String::leak(v),
                        },
                        optional: f.optional,
                        flatten: f.flatten,
                        ty: specta_v2_to_v1(f.ty.unwrap_or(DataType::Unknown)),
                    })
                    .collect(),
            },
        }),
        DataType::Enum(e) => {
            let generics = e
                .generics
                .into_iter()
                .map(|v| match v.0 {
                    Cow::Borrowed(v) => v,
                    Cow::Owned(v) => String::leak(v),
                })
                .collect::<Vec<_>>();

            specta1::DataType::Enum(match e.repr {
                crate::datatype::EnumRepr::Untagged => specta1::EnumType::Untagged {
                    generics: generics.clone(),
                    variants: e
                        .variants
                        .into_iter()
                        .map(|(name, v)| match v.inner() {
                            crate::datatype::EnumVariants::Unit => specta1::EnumVariant::Unit,
                            crate::datatype::EnumVariants::Named(f) => {
                                specta1::EnumVariant::Named(specta1::ObjectType {
                                    generics: generics.clone(),
                                    fields: f
                                        .fields
                                        .iter()
                                        .map(|(name, f)| specta1::ObjectField {
                                            key: match name {
                                                Cow::Borrowed(v) => v,
                                                Cow::Owned(v) => String::leak(v.clone()),
                                            },
                                            optional: f.optional,
                                            flatten: f.flatten,
                                            ty: specta_v2_to_v1(
                                                f.ty.clone().unwrap_or(DataType::Unknown),
                                            ),
                                        })
                                        .collect(),
                                    tag: f.tag.clone().map(|v| match v {
                                        Cow::Borrowed(v) => v,
                                        Cow::Owned(v) => String::leak(v),
                                    }),
                                })
                            }
                            crate::datatype::EnumVariants::Unnamed(f) => {
                                specta1::EnumVariant::Unnamed(specta1::TupleType {
                                    generics: generics.clone(),
                                    fields: f
                                        .fields
                                        .clone()
                                        .into_iter()
                                        .map(|f| specta_v2_to_v1(f.ty.unwrap_or(DataType::Unknown)))
                                        .collect(),
                                })
                            }
                        })
                        .collect::<Vec<_>>(),
                },
                crate::datatype::EnumRepr::External => specta1::EnumType::Tagged {
                    variants: e
                        .variants
                        .into_iter()
                        .map(|(name, v)| {
                            (
                                match name {
                                    Cow::Borrowed(v) => v,
                                    Cow::Owned(v) => String::leak(v),
                                },
                                match v.inner {
                                    crate::datatype::EnumVariants::Unit => {
                                        specta1::EnumVariant::Unit
                                    }
                                    crate::datatype::EnumVariants::Named(f) => {
                                        specta1::EnumVariant::Named(specta1::ObjectType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|(name, f)| specta1::ObjectField {
                                                    key: match name {
                                                        Cow::Borrowed(v) => v,
                                                        Cow::Owned(v) => String::leak(v),
                                                    },
                                                    optional: f.optional,
                                                    flatten: f.flatten,
                                                    ty: specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    ),
                                                })
                                                .collect(),
                                            tag: f.tag.map(|v| match v {
                                                Cow::Borrowed(v) => v,
                                                Cow::Owned(v) => String::leak(v),
                                            }),
                                        })
                                    }
                                    crate::datatype::EnumVariants::Unnamed(f) => {
                                        specta1::EnumVariant::Unnamed(specta1::TupleType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|f| {
                                                    specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    )
                                                })
                                                .collect(),
                                        })
                                    }
                                },
                            )
                        })
                        .collect(),
                    generics,
                    repr: specta1::EnumRepr::External,
                },
                crate::datatype::EnumRepr::Internal { tag } => specta1::EnumType::Tagged {
                    generics: generics.clone(),
                    variants: e
                        .variants
                        .into_iter()
                        .map(|(name, v)| {
                            (
                                match name {
                                    Cow::Borrowed(v) => v,
                                    Cow::Owned(v) => String::leak(v),
                                },
                                match v.inner {
                                    crate::datatype::EnumVariants::Unit => {
                                        specta1::EnumVariant::Unit
                                    }
                                    crate::datatype::EnumVariants::Named(f) => {
                                        specta1::EnumVariant::Named(specta1::ObjectType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|(name, f)| specta1::ObjectField {
                                                    key: match name {
                                                        Cow::Borrowed(v) => v,
                                                        Cow::Owned(v) => String::leak(v),
                                                    },
                                                    optional: f.optional,
                                                    flatten: f.flatten,
                                                    ty: specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    ),
                                                })
                                                .collect(),
                                            tag: f.tag.map(|v| match v {
                                                Cow::Borrowed(v) => v,
                                                Cow::Owned(v) => String::leak(v),
                                            }),
                                        })
                                    }
                                    crate::datatype::EnumVariants::Unnamed(f) => {
                                        specta1::EnumVariant::Unnamed(specta1::TupleType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|f| {
                                                    specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    )
                                                })
                                                .collect(),
                                        })
                                    }
                                },
                            )
                        })
                        .collect(),
                    repr: specta1::EnumRepr::Internal {
                        tag: match tag {
                            Cow::Borrowed(v) => v,
                            Cow::Owned(v) => String::leak(v),
                        },
                    },
                },
                crate::datatype::EnumRepr::Adjacent { tag, content } => specta1::EnumType::Tagged {
                    generics: generics.clone(),
                    repr: specta1::EnumRepr::Adjacent {
                        tag: match tag {
                            Cow::Borrowed(v) => v,
                            Cow::Owned(v) => String::leak(v),
                        },
                        content: match content {
                            Cow::Borrowed(v) => v,
                            Cow::Owned(v) => String::leak(v),
                        },
                    },
                    variants: e
                        .variants
                        .clone()
                        .into_iter()
                        .map(|(name, v)| {
                            (
                                match name {
                                    Cow::Borrowed(v) => v,
                                    Cow::Owned(v) => String::leak(v),
                                },
                                match v.inner {
                                    crate::datatype::EnumVariants::Unit => {
                                        specta1::EnumVariant::Unit
                                    }
                                    crate::datatype::EnumVariants::Named(f) => {
                                        specta1::EnumVariant::Named(specta1::ObjectType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|(name, f)| specta1::ObjectField {
                                                    key: match name {
                                                        Cow::Borrowed(v) => v,
                                                        Cow::Owned(v) => String::leak(v),
                                                    },
                                                    optional: f.optional,
                                                    flatten: f.flatten,
                                                    ty: specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    ),
                                                })
                                                .collect(),
                                            tag: f.tag.map(|v| match v {
                                                Cow::Borrowed(v) => v,
                                                Cow::Owned(v) => String::leak(v),
                                            }),
                                        })
                                    }
                                    crate::datatype::EnumVariants::Unnamed(f) => {
                                        specta1::EnumVariant::Unnamed(specta1::TupleType {
                                            generics: generics.clone(),
                                            fields: f
                                                .fields
                                                .into_iter()
                                                .map(|f| {
                                                    specta_v2_to_v1(
                                                        f.ty.unwrap_or(DataType::Unknown),
                                                    )
                                                })
                                                .collect(),
                                        })
                                    }
                                },
                            )
                        })
                        .collect(),
                },
            })
        }
        DataType::Tuple(t) => specta1::DataType::Tuple(specta1::TupleType {
            fields: t.elements.into_iter().map(specta_v2_to_v1).collect(),
            generics: vec![],
        }),
        DataType::Reference(r) => specta1::DataType::Reference(specta1::DataTypeReference {
            name: match r.name {
                Cow::Borrowed(v) => v,
                Cow::Owned(v) => String::leak(v),
            },
            sid: specta1::r#type::internal_sid_hash("specta1", "", r.sid.type_name),
            generics: r
                .generics
                .into_iter()
                .map(|(_, d)| specta_v2_to_v1(d))
                .collect(),
        }),
        DataType::Generic(g) => specta1::DataType::Generic(specta1::GenericType(match g.0 {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => String::leak(s),
        })),
    }
}

pub fn specta_v2_type_map_to_v1_type_defs(defs: TypeCollection, type_map: &mut specta1::TypeDefs) {
    for (sid, dt) in defs.into_iter() {
        let dtv1 = specta_v2_to_v1(dt.inner.clone());
        let sid = specta1::r#type::internal_sid_hash("specta1", "", sid.type_name);
        type_map.insert(
            sid,
            Some(NamedDataType {
                name: match dt.name.clone() {
                    Cow::Borrowed(v) => v,
                    Cow::Owned(v) => String::leak(v),
                },
                sid: Some(sid),
                impl_location: None,
                comments: Vec::leak(vec![match dt.docs.clone() {
                    Cow::Borrowed(v) => v,
                    Cow::Owned(v) => String::leak(v),
                }]),
                export: None,
                deprecated: dt.deprecated.clone().map(|v| match v {
                    DeprecatedType::Deprecated => "",
                    DeprecatedType::DeprecatedWithSince {  note, .. } => match note {
                        Cow::Borrowed(v) => v,
                        Cow::Owned(v) => String::leak(v),
                    }
                }),
                item: match dtv1 {
                    specta1::DataType::Object(o) => specta1::NamedDataTypeItem::Object(o),
                    specta1::DataType::Enum(e) => specta1::NamedDataTypeItem::Enum(e),
                    specta1::DataType::Tuple(t) => specta1::NamedDataTypeItem::Tuple(t),
                    _ => unreachable!("Specta v1 doesn't support named types that aren't an object, enum or tuple!"),
                }
            }),
        );
    }
}
