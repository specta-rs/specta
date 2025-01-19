//! Helpers for generating [Type::reference] implementations.

use crate::{datatype::{EnumType, EnumVariant, Field, Fields, List, Map, NamedFields, StructType, TupleType, UnnamedFields}, Type, TypeCollection};

use super::{reference::Reference, DataType, GenericType};

// TODO: Can/should we merge these? Deduplicate the code?

// We need to go to any depth to deal with generics.
fn inline_generics(types: &mut TypeCollection, generics: &[(GenericType, DataType)], dt: DataType) -> DataType {
    match dt {
        DataType::Any | DataType::Unknown |  DataType::Primitive(..) | DataType::Literal(..) => dt,
        DataType::List(list) => DataType::List(List {
            ty: Box::new(inline_generics(types, generics, (*list.ty).clone())),
            length: list.length,
            unique: list.unique,
        }),
        DataType::Map(map) => DataType::Map(Map {
            key_ty: Box::new(inline_generics(types, generics, (*map.key_ty).clone())),
            value_ty: Box::new(inline_generics(types, generics, (*map.value_ty).clone())),
        }),
        DataType::Nullable(data_type) => DataType::Nullable(Box::new(inline_generics(types, generics, *data_type))),
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
                        ty: f.ty.map(|ty| inline_generics(types, generics, ty))
                    }).collect(),
                }),
                Fields::Named(named_fields) => Fields::Named(NamedFields {
                    tag: named_fields.tag,
                    fields: named_fields.fields.into_iter().map(|(k, f)| (k, Field {
                        optional: f.optional,
                        flatten: f.flatten,
                        deprecated: f.deprecated,
                        docs: f.docs,
                        ty: f.ty.map(|ty| inline_generics(types, generics, ty))
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
                            ty: f.ty.map(|ty| inline_generics(types, generics, ty))
                        }).collect(),
                    }),
                    Fields::Named(f) => Fields::Named(NamedFields {
                        tag: f.tag,
                        fields: f.fields.into_iter().map(|(k, f)| (k, Field {
                            optional: f.optional,
                            flatten: f.flatten,
                            deprecated: f.deprecated,
                            docs: f.docs,
                            ty: f.ty.map(|ty| inline_generics(types, generics, ty))
                        })).collect(),
                    })
                }
            })).collect::<Vec<_>>(),
        }),
        DataType::Tuple(t) => DataType::Tuple(TupleType {
            elements: t.elements.into_iter().map(|ty| inline_generics(types, generics, ty)).collect(),
        }),
        DataType::Reference(r) => DataType::Reference(Reference {
            sid: r.sid,
            generics: r.generics.into_iter().map(|t| inline_generics(types, generics, t)).collect()
        }),
        DataType::Generic(ref g) => {
            generics.iter().find(|(generic_type, _)| *generic_type == *g)
                .map(|(_, ty)| ty.clone())
                .unwrap_or(dt) // TODO: Is this good? This is the multi-phase inlining so maybe okay?
        }
    }
}

/// If the top-level type is a `DataType::Reference`, inline it and then resolve all generics.
pub fn inline_reference<T: Type>(types: &mut TypeCollection) -> DataType {
    match T::definition(types) {
        DataType::Reference(r) => {
            let ty = types.get(r.sid).unwrap(); // TODO: Error handling
            let g = ty.inner.generics().unwrap(); // TODO: This should be handled properly

            // TODO: Error if the size of both items doesn't match
            let generics = std::iter::zip(g.iter().cloned(), r.generics.iter().cloned()).collect::<Vec<_>>();

            inline_generics(types, &generics, ty.inner.clone())
        },
        dt => dt
    }
}

/// TODO: Finish and document this. It only inlines the first level of references.
pub fn inline<T: Type>(types: &mut TypeCollection, generics: &[(GenericType, DataType)]) -> DataType {
    fn inner(types: &mut TypeCollection, generics: &[(GenericType, DataType)], dt: DataType, i: i8) -> DataType {
        if i == 1 {
            return inline_generics(types, generics, dt);
            // return inline_generics(types, generics, dt);
        }

        match dt {
            DataType::Any | DataType::Unknown |  DataType::Primitive(..) | DataType::Literal(..) => dt,
            DataType::List(list) => DataType::List(List {
                ty: Box::new(inner(types, generics, (*list.ty).clone(), i + 1)),
                length: list.length,
                unique: list.unique,
            }),
            DataType::Map(map) => DataType::Map(Map {
                key_ty: Box::new(inner(types, generics, (*map.key_ty).clone(), i + 1)),
                value_ty: Box::new(inner(types, generics, (*map.value_ty).clone(), i + 1)),
            }),
            DataType::Nullable(data_type) => DataType::Nullable(Box::new(inner(types, generics, *data_type, i + 1))),
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
                            ty: f.ty.map(|ty| inner(types, generics, ty, i + 1))
                        }).collect(),
                    }),
                    Fields::Named(named_fields) => Fields::Named(NamedFields {
                        tag: named_fields.tag,
                        fields: named_fields.fields.into_iter().map(|(k, f)| (k, Field {
                            optional: f.optional,
                            flatten: f.flatten,
                            deprecated: f.deprecated,
                            docs: f.docs,
                            ty: f.ty.map(|ty| inner(types, generics, ty, i + 1))
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
                                ty: f.ty.map(|ty| inner(types, generics, ty, i + 1))
                            }).collect(),
                        }),
                        Fields::Named(f) => Fields::Named(NamedFields {
                            tag: f.tag,
                            fields: f.fields.into_iter().map(|(k, f)| (k, Field {
                                optional: f.optional,
                                flatten: f.flatten,
                                deprecated: f.deprecated,
                                docs: f.docs,
                                ty: f.ty.map(|ty| inner(types, generics, ty, i + 1))
                            })).collect(),
                        })
                    }
                })).collect::<Vec<_>>(),
            }),
            DataType::Tuple(t) => DataType::Tuple(TupleType {
                elements: t.elements.into_iter().map(|ty| inner(types, generics, ty, i + 1)).collect(),
            }),
            DataType::Reference(r) => {
                // TODO: Error handling
                let Some(ty) = types.map.get(&r.sid).unwrap() else {
                    panic!("detected a recursive inline"); // TODO: Better error message
                };

                let g = ty.inner.generics().unwrap(); // TODO: This should be handled properly

                // TODO: Error if the size of both items doesn't match
                let todo = std::iter::zip(g.iter().cloned(), r.generics.iter().cloned()).collect::<Vec<_>>();

                println!("R {:?} {:?}", r, todo); // TODO

                // println!("TODO {:?}", todo); // TODO

                // TODO: Join `todo` and `generics`

                inner(types, &todo, ty.inner.clone(), i + 1)
            }
            DataType::Generic(g) => {
                println!("G {:?} {:?}", g, generics); // TODO

                generics.iter().find(|(generic_type, _)| *generic_type == g)
                    .map(|(_, ty)| ty.clone())
                    .unwrap()
            }
        }
    }

    let dt = T::definition(types);
    inner(types, generics, dt, 0)
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
