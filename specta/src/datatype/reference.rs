//! Helpers for generating [Type::reference] implementations.

use std::collections::HashMap;

use crate::{
    datatype::{Field, Fields, GenericType},
    SpectaID, TypeCollection,
};

use super::{DataType, NamedDataType};

/// A reference datatype.
///
/// TODO: Explain how to construct this.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
    pub(crate) generics: Vec<DataType>,
    // When creating a reference we generate a `DataType` replacing all `DataType::Generic` with the current generics.
    // This allows us to avoid a runtime find-and-replace on it.
    pub(crate) dt: Box<DataType>,
    // TODO: This is a Typescript-specific thing
    pub(crate) inline: bool,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(
        sid: SpectaID,
        generics: impl Into<Vec<DataType>>,
        dt: DataType,
        inline: bool,
    ) -> Self {
        Self {
            sid,
            generics: generics.into(),
            dt: Box::new(dt),
            inline,
        }
    }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn generics(&self) -> &[DataType] {
        &self.generics
    }

    pub fn inline(&self) -> bool {
        self.inline
    }

    pub fn datatype(&self) -> &DataType {
        &self.dt
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}

#[doc(hidden)] // TODO: Move this into Specta Typescript
pub fn inline_and_flatten_ndt(dt: NamedDataType, types: &TypeCollection) -> NamedDataType {
    NamedDataType {
        inner: inline_and_flatten(dt.inner, types),
        ..dt
    }
}

#[doc(hidden)] // TODO: Move this into Specta Typescript
pub fn inline_and_flatten(dt: DataType, types: &TypeCollection) -> DataType {
    inner(
        dt,
        types,
        false,
        false,
        &Default::default(),
        &Default::default(),
    )
}

pub fn inline(dt: DataType, types: &TypeCollection) -> DataType {
    inner(
        dt,
        types,
        false,
        true,
        &Default::default(),
        &Default::default(),
    )
}

fn field(
    f: Field,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &HashMap<GenericType, DataType>,
) -> Field {
    // TODO: truely_force_inline
    if f.flatten() || f.inline() {
        if let Some(ty) = &f.ty {
            return Field {
                ty: Some(inner(
                    ty.clone(),
                    types,
                    true,
                    truely_force_inline,
                    generics,
                    &Default::default(),
                )),
                ..f
            };
        }
    }

    f
}

fn fields(
    f: Fields,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &HashMap<GenericType, DataType>,
) -> Fields {
    match f {
        Fields::Unnamed(f) => Fields::Unnamed(super::UnnamedFields {
            fields: f
                .fields
                .into_iter()
                .map(|f| field(f, types, truely_force_inline, generics))
                .collect(),
        }),
        Fields::Named(f) => Fields::Named(super::NamedFields {
            fields: f
                .fields
                .into_iter()
                .map(|(n, f)| (n, field(f, types, truely_force_inline, generics)))
                .collect(),
            ..f
        }),
        v => v,
    }
}

fn inner(
    dt: DataType,
    types: &TypeCollection,
    force_inline: bool,
    truely_force_inline: bool,
    generics: &HashMap<GenericType, DataType>,
    known_references: &Vec<DataType>,
) -> DataType {
    match dt {
        DataType::List(l) => DataType::List(super::List {
            ty: Box::new(inner(
                *l.ty,
                types,
                truely_force_inline,
                truely_force_inline,
                generics,
                &Default::default(),
            )),
            length: l.length,
            unique: l.unique,
        }),
        DataType::Map(map) => DataType::Map(super::Map {
            key_ty: Box::new(inner(
                *map.key_ty,
                types,
                truely_force_inline,
                truely_force_inline,
                generics,
                &Default::default(),
            )),
            value_ty: Box::new(inner(
                *map.value_ty,
                types,
                truely_force_inline,
                truely_force_inline,
                generics,
                &Default::default(),
            )),
        }),
        DataType::Nullable(d) => DataType::Nullable(Box::new(inner(
            *d,
            types,
            truely_force_inline,
            truely_force_inline,
            generics,
            &Default::default(),
        ))),
        DataType::Struct(s) => {
            let generics = s
                .generics
                .iter()
                .cloned()
                .zip(known_references.iter().cloned())
                .collect::<HashMap<_, _>>();

            DataType::Struct(super::StructType {
                fields: fields(s.fields, types, truely_force_inline, &generics),
                ..s
            })
        }
        DataType::Enum(e) => {
            let generics = e
                .generics
                .iter()
                .cloned()
                .zip(known_references.iter().cloned())
                .collect::<HashMap<_, _>>();

            DataType::Enum(super::EnumType {
                variants: e
                    .variants
                    .into_iter()
                    .map(|(n, t)| {
                        (
                            n,
                            super::EnumVariant {
                                fields: fields(t.fields, types, truely_force_inline, &generics),
                                ..t
                            },
                        )
                    })
                    .collect(),
                ..e
            })
        }
        DataType::Tuple(t) => DataType::Tuple(super::TupleType {
            elements: t
                .elements
                .into_iter()
                .map(|e| {
                    inner(
                        e,
                        types,
                        false,
                        truely_force_inline,
                        generics,
                        &Default::default(),
                    )
                })
                .collect(),
        }),
        DataType::Generic(g) => generics.get(&g).expect("dun goof").clone(),
        DataType::Reference(r) => {
            if r.inline() || force_inline || truely_force_inline {
                inner(
                    *r.dt.clone(),
                    types,
                    truely_force_inline, // TODO:  false,
                    truely_force_inline,
                    &Default::default(),
                    &r.generics,
                )
            } else {
                DataType::Reference(r)
            }
        }
        v => v,
    }
}
