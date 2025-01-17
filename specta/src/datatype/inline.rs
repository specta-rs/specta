//! Helpers for generating [Type::reference] implementations.

use crate::{datatype::{EnumType, EnumVariant, Field, Fields, List, Map, NamedFields, StructType, TupleType, UnnamedFields}, Type, TypeCollection};

use super::DataType;

/// TODO: Finish and document this. It only inlines the first level of references.
pub fn inline<T: Type>(types: &mut TypeCollection) -> DataType {
    fn inner(types: &mut TypeCollection, dt: DataType, i: i8) -> DataType {
        if i == 1 {
            return dt;
        }

        match dt {
            DataType::Any | DataType::Unknown |  DataType::Primitive(..) | DataType::Literal(..) => dt,
            DataType::List(list) => DataType::List(List {
                ty: Box::new(inner(types, (*list.ty).clone(), i + 1)),
                length: list.length,
                unique: list.unique,
            }),
            DataType::Map(map) => DataType::Map(Map {
                key_ty: Box::new(inner(types, (*map.key_ty).clone(), i + 1)),
                value_ty: Box::new(inner(types, (*map.value_ty).clone(), i + 1)),
            }),
            DataType::Nullable(data_type) => DataType::Nullable(Box::new(inner(types, *data_type, i + 1))),
            DataType::Struct(s) => DataType::Struct(StructType {
                name: s.name,
                sid: s.sid,
                generics: s.generics,
                fields: match s.fields {
                    Fields::Unit => Fields::Unit,
                    Fields::Unnamed(unnamed_fields) => Fields::Unnamed(UnnamedFields {
                        fields: unnamed_fields.fields.into_iter().map(|f| Field {
                            optional: f.optional,
                            flatten: f.flatten,
                            deprecated: f.deprecated,
                            docs: f.docs,
                            ty: f.ty.map(|ty| inner(types, ty, i + 1))
                        }).collect(),
                    }),
                    Fields::Named(named_fields) => Fields::Named(NamedFields {
                        tag: named_fields.tag,
                        fields: named_fields.fields.into_iter().map(|(k, f)| (k, Field {
                            optional: f.optional,
                            flatten: f.flatten,
                            deprecated: f.deprecated,
                            docs: f.docs,
                            ty: f.ty.map(|ty| inner(types, ty, i + 1))
                        })).collect(),
                    })
                }
            }),
            DataType::Enum(e) => DataType::Enum(EnumType {
                name: e.name,
                sid: e.sid,
                skip_bigint_checks: e.skip_bigint_checks,
                repr: e.repr,
                generics: e.generics,
                variants: e.variants.into_iter().map(|(k, v)| (k, EnumVariant {
                    skip: v.skip,
                    docs: v.docs,
                    deprecated: v.deprecated,
                    fields: match v.fields {
                        Fields::Unit => Fields::Unit,
                        Fields::Unnamed(f) => Fields::Unnamed(UnnamedFields {
                            fields: f.fields.into_iter().map(|f| Field {
                                optional: f.optional,
                                flatten: f.flatten,
                                deprecated: f.deprecated,
                                docs: f.docs,
                                ty: f.ty.map(|ty| inner(types, ty, i + 1))
                            }).collect(),
                        }),
                        Fields::Named(f) => Fields::Named(NamedFields {
                            tag: f.tag,
                            fields: f.fields.into_iter().map(|(k, f)| (k, Field {
                                optional: f.optional,
                                flatten: f.flatten,
                                deprecated: f.deprecated,
                                docs: f.docs,
                                ty: f.ty.map(|ty| inner(types, ty, i + 1))
                            })).collect(),
                        })
                    }
                })).collect::<Vec<_>>(),
            }),
            DataType::Tuple(t) => DataType::Tuple(TupleType {
                elements: t.elements.into_iter().map(|ty| inner(types, ty, i + 1)).collect(),
            }),
            DataType::Reference(r) => {
                let ty = types.get(r.sid).unwrap(); // TODO: Error handling
                inner(types, ty.inner.clone(), i + 1)
            }
            DataType::Generic(g) => todo!(),
        }
    }

    let dt = T::definition(types);
    inner(types, dt, 0)
}

// /// TODO: Finish and document this. It only inlines the first level of references.
// pub fn inline_ish<T: Type>(types: &mut TypeCollection) -> DataType {
//     let dt = T::definition(types);
//     match dt {
//         DataType::Reference(r) => {
//             let ty = types.get(r.sid).unwrap(); // TODO: Error handling
//             ty.inner.clone()
//         }
//         _ => dt
//     }
// }

// /// TODO: Finish and document this
// /// TODO: Rename
// pub fn complete_inline<T: Type>(types: &mut TypeCollection) -> DataType {
//     fn inner(types: &mut TypeCollection, dt: DataType) -> DataType {
//         match dt {
//             DataType::Any | DataType::Unknown |  DataType::Primitive(..) | DataType::Literal(..) => dt,
//             DataType::List(list) => inner(types, (*list.ty).clone()),
//             DataType::Map(map) => todo!(),
//             DataType::Nullable(data_type) => todo!(),
//             DataType::Struct(s) => DataType::Struct(StructType {
//                 name: s.name,
//                 sid: s.sid,
//                 generics: s.generics,
//                 fields: match s.fields {
//                     Fields::Unit => Fields::Unit,
//                     Fields::Unnamed(unnamed_fields) => Fields::Unnamed(UnnamedFields {
//                         fields: unnamed_fields.fields.into_iter().map(|f| Field {
//                             optional: f.optional,
//                             flatten: f.flatten,
//                             deprecated: f.deprecated,
//                             docs: f.docs,
//                             ty: f.ty.map(|ty| inner(types, ty))
//                         }).collect(),
//                     }),
//                     Fields::Named(named_fields) => Fields::Named(NamedFields {
//                         tag: named_fields.tag,
//                         fields: named_fields.fields.into_iter().map(|(k, f)| (k, Field {
//                             optional: f.optional,
//                             flatten: f.flatten,
//                             deprecated: f.deprecated,
//                             docs: f.docs,
//                             ty: f.ty.map(|ty| inner(types, ty))
//                         })).collect(),
//                     })
//                 }
//             }),
//             DataType::Enum(enum_type) => todo!(),
//             DataType::Tuple(t) => DataType::Tuple(TupleType {
//                 elements: t.elements.into_iter().map(|ty| inner(types, ty)).collect(),
//             }),
//             DataType::Reference(r) => {
//                 // assert_eq!(r.generics.len(), 0, "Generics not supported, yet"); // TODO

//                 let ty = types.get(r.sid).unwrap(); // TODO: Error handling
//                 inner(types, ty.inner.clone())
//             }
//             DataType::Generic(generic_type) => todo!(),
//         }
//     }

//     let dt = T::definition(types);
//     inner(types, dt)
// }
