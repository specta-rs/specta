//! Helpers for generating [Type::reference] implementations.

use crate::{datatype::{Field, Fields, NamedFields, StructType, UnnamedFields}, SpectaID, Type, TypeCollection};

use super::{DataType, DataTypeReference, GenericType};

/// A reference datatype.
///
/// TODO: Explain how to construct this.
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(sid: SpectaID) -> Self {
        Self { sid }
    }

    // pub fn new(sid: SpectaID) -> Self {
    //     //     if type_map.map.get(&sid).is_none() {
    //     //         type_map.map.entry(sid).or_insert(None);
    //     //         let dt = T::definition_named_data_type(type_map);
    //     //         type_map.map.insert(sid, Some(dt));
    //     //     }

    //     Self {
    //         sid,
    //     }
    // }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn to_datatype(&self, generics: impl Into<Vec<(GenericType, DataType)>>) -> DataType {
        DataType::Reference(DataTypeReference {
            sid: self.sid,
            generics: generics.into(),
        })
    }
}

/// TODO: Finish and document this
/// TODO: Move somewhere else
pub fn inline<T: Type>(types: &mut TypeCollection) -> DataType {
    fn inner(types: &mut TypeCollection, dt: DataType) -> DataType {
        match dt {
            DataType::Any | DataType::Unknown |  DataType::Primitive(..) | DataType::Literal(..) => dt,
            DataType::List(list) => inner(types, (*list.ty).clone()),
            DataType::Map(map) => todo!(),
            DataType::Nullable(data_type) => todo!(),
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
                            ty: f.ty.map(|ty| inner(types, ty))
                        }).collect(),
                    }),
                    Fields::Named(named_fields) => Fields::Named(NamedFields {
                        tag: named_fields.tag,
                        fields: named_fields.fields.into_iter().map(|(k, f)| (k, Field {
                            optional: f.optional,
                            flatten: f.flatten,
                            deprecated: f.deprecated,
                            docs: f.docs,
                            ty: f.ty.map(|ty| inner(types, ty))
                        })).collect(),
                    })
                }
            }),
            DataType::Enum(enum_type) => todo!(),
            DataType::Tuple(tuple_type) => todo!(),
            DataType::Reference(r) => {
                assert_eq!(r.generics.len(), 0, "Generics not supported, yet"); // TODO

                let ty = types.get(r.sid).unwrap(); // TODO: Error handling
                inner(types, ty.inner.clone())
            }
            DataType::Generic(generic_type) => todo!(),
        }
    }

    let dt = T::definition(types);
    inner(types, dt)
}

pub fn reference_or_inline<T: Type + ?Sized>(
    types: &mut TypeCollection,
    generics: &[DataType],
) -> DataType {
    // match T::reference(types, generics) {
    //     Some(reference) => {
    //         // TODO: Fix generics
    //         reference.to_datatype(vec![])
    //     },
    //     None => T::inline(types, Generics::Provided(generics)),
    // }
    todo!();
}

// TODO: Remove this?
// pub fn reference<T: NamedType>(
//     type_map: &mut TypeCollection,
// ) -> Reference {
//     T::definition(type_map);
//     Reference {
//         sid: T::sid(),
//         // inner: DataType::Reference(DataTypeReference {
//         //     sid,
//         //     // TODO: Make this work
//         //     generics: Default::default()
//         // }),
//     }
// }

// // TODO: Remove this?
// pub fn reference(
//     type_map: &mut TypeCollection,
//     sid: SpectaID,
//     definition_named_data_type: fn(&mut TypeCollection) -> NamedDataType,
// ) -> Reference {
//     if type_map.map.get(&sid).is_none() {
//         type_map.map.entry(sid).or_insert(None);
//         let dt = definition_named_data_type(type_map);
//         type_map.map.insert(sid, Some(dt));
//     }

//     Reference {
//         sid,
//     }
// }
