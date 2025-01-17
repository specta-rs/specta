//! Helpers for generating [Type::reference] implementations.

use crate::{datatype::{Field, Fields, NamedFields, StructType, UnnamedFields}, SpectaID, Type, TypeCollection};

use super::{DataType,  GenericType};

/// A reference datatype.
///
/// TODO: Explain how to construct this.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
    pub(crate) generics: Vec<(GenericType, DataType)>,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(sid: SpectaID, generics: impl Into<Vec<(GenericType, DataType)>>) -> Self {
        Self { sid, generics: generics.into() }
    }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn generics(&self) -> &[(GenericType, DataType)] {
        &self.generics
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
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
