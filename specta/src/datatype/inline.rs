//! Helpers for generating [Type::reference] implementations.

use std::collections::HashMap;

use crate::TypeCollection;

use super::{DataType, Field, Fields, Generic, NamedDataType};

#[doc(hidden)] // TODO: Move this into Specta Typescript
pub fn inline_and_flatten_ndt(dt: NamedDataType, types: &TypeCollection) -> NamedDataType {
    NamedDataType {
        inner: inline_and_flatten(dt.inner, types),
        ..dt
    }
}

#[doc(hidden)] // TODO: Move this into Specta Typescript
pub fn inline_and_flatten(dt: DataType, types: &TypeCollection) -> DataType {
    inner(dt, types, false, false, &Default::default(), 0)
}

#[doc(hidden)] // TODO: Move this into Specta Typescript
pub fn inline(dt: DataType, types: &TypeCollection) -> DataType {
    inner(dt, types, false, true, &Default::default(), 0)
}

fn field(
    f: Field,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &HashMap<Generic, DataType>,
    depth: usize,
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
                    depth + 1,
                )),
                ..f
            };
        }
    }

    return Field {
        ty: f.ty.map(|ty| resolve_generics(ty, &generics)),
        ..f
    };
}

fn fields(
    f: Fields,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &HashMap<Generic, DataType>,
    depth: usize,
) -> Fields {
    match f {
        Fields::Unnamed(f) => Fields::Unnamed(super::UnnamedFields {
            fields: f
                .fields
                .into_iter()
                .map(|f| field(f, types, truely_force_inline, generics, depth))
                .collect(),
        }),
        Fields::Named(f) => Fields::Named(super::NamedFields {
            fields: f
                .fields
                .into_iter()
                .map(|(n, f)| (n, field(f, types, truely_force_inline, generics, depth)))
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
    generics: &HashMap<Generic, DataType>,
    depth: usize,
) -> DataType {
    // TODO: Can we be smart enough to determine loops, instead of just trying X times and bailing out????
    //      -> Would be more efficent but much harder. Would make the error messages much better though.
    if depth == 25 {
        // TODO: Return a `Result` instead of panicing
        // TODO: Detect which types are the cycle and report it
        panic!("Type recursion limit exceeded!");
    }

    match dt {
        DataType::List(l) => DataType::List(super::List {
            ty: Box::new(inner(
                *l.ty,
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            )),
            length: l.length,
            unique: l.unique,
        }),
        DataType::Map(map) => DataType::Map(super::Map {
            key_ty: Box::new(inner(
                *map.key_ty,
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            )),
            value_ty: Box::new(inner(
                *map.value_ty,
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            )),
        }),
        DataType::Nullable(d) => DataType::Nullable(Box::new(inner(
            *d,
            types,
            false, // truely_force_inline,
            truely_force_inline,
            generics,
            depth + 1,
        ))),
        DataType::Struct(s) => DataType::Struct(super::Struct {
            fields: fields(s.fields, types, truely_force_inline, &generics, depth),
            ..s
        }),
        DataType::Enum(e) => DataType::Enum(super::Enum {
            variants: e
                .variants
                .into_iter()
                .map(|(n, t)| {
                    (
                        n,
                        super::EnumVariant {
                            fields: fields(t.fields, types, truely_force_inline, &generics, depth),
                            ..t
                        },
                    )
                })
                .collect(),
            ..e
        }),
        DataType::Tuple(t) => DataType::Tuple(super::Tuple {
            elements: t
                .elements
                .into_iter()
                .map(|e| inner(e, types, false, truely_force_inline, generics, depth + 1))
                .collect(),
        }),
        DataType::Generic(g) => {
            let ty = generics.get(&g).expect("dun goof").clone();

            if truely_force_inline {
                inner(
                    ty.clone(),
                    types,
                    false,
                    truely_force_inline,
                    &Default::default(), // TODO: What should this be?
                    depth + 1,
                )
            } else {
                ty
            }
        }
        DataType::Reference(r) => {
            if r.inline() || force_inline || truely_force_inline {
                let ty = types.get(r.sid()).expect("dun goof");

                inner(
                    ty.inner.clone(),
                    types,
                    false,
                    truely_force_inline,
                    &r.generics
                        .clone()
                        .into_iter()
                        .map(|(g, dt)| (g, resolve_generics(dt, generics)))
                        .collect(),
                    depth + 1,
                )
            } else {
                DataType::Reference(r)
            }
        }
        v => v,
    }
}

/// Following all `DataType::Reference`'s filling in any `DataType::Generic`'s with the correct value.
fn resolve_generics(dt: DataType, generics: &HashMap<Generic, DataType>) -> DataType {
    // TODO: This could so only re-alloc if the type has a generics that needs replacing.
    match dt {
        DataType::List(l) => DataType::List(super::List {
            ty: Box::new(resolve_generics(*l.ty, generics)),
            ..l
        }),
        DataType::Map(m) => DataType::Map(super::Map {
            key_ty: Box::new(resolve_generics(*m.key_ty, generics)),
            value_ty: Box::new(resolve_generics(*m.value_ty, generics)),
        }),
        DataType::Nullable(d) => DataType::Nullable(Box::new(resolve_generics(*d, generics))),
        // DataType::Struct(s) => DataType::Struct(super::StructType {
        //     generics: todo!(),
        //     fields: todo!(),
        //     ..s,
        // })
        // DataType::Enum(e) => todo!(),
        DataType::Tuple(t) => DataType::Tuple(super::Tuple {
            elements: t
                .elements
                .into_iter()
                .map(|dt| resolve_generics(dt, generics))
                .collect(),
        }),
        DataType::Reference(r) => DataType::Reference(super::Reference {
            generics: r
                .generics
                .into_iter()
                .map(|(g, dt)| (g, resolve_generics(dt, generics)))
                .collect(),
            ..r
        }),
        DataType::Generic(g) => {
            // This method is run when not inlining so for `export` we do expect `DataType::Generic`.
            // TODO: Functions main documentation nshould explain this.
            generics.get(&g).cloned().unwrap_or(DataType::Generic(g))
        }
        v => v,
    }
}
