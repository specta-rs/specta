//! Helpers for generating [Type::reference] implementations.

use specta::TypeCollection;

use specta::datatype::{DataType, Field, Fields, Generic, NamedDataType};

#[doc(hidden)] // TODO: Make this private
pub fn inline_and_flatten_ndt(ndt: &mut NamedDataType, types: &TypeCollection) {
    inner(ndt.ty_mut(), types, false, false, &[], 0);
}

pub(crate) fn inline(dt: &mut DataType, types: &TypeCollection) {
    inner(dt, types, false, true, &[], 0)
}

fn field(
    f: &mut Field,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &[(Generic, DataType)],
    depth: usize,
) {
    // TODO: truely_force_inline
    if f.inline() {
        if let Some(ty) = f.ty_mut() {
            inner(ty, types, true, truely_force_inline, generics, depth + 1)
        }
    }

    if let Some(ty) = f.ty_mut() {
        resolve_generics(ty, &generics);
    }
}

fn fields(
    f: &mut Fields,
    types: &TypeCollection,
    truely_force_inline: bool,
    generics: &[(Generic, DataType)],
    depth: usize,
) {
    match f {
        Fields::Unit => {}
        Fields::Unnamed(f) => {
            for f in f.fields_mut() {
                field(f, types, truely_force_inline, generics, depth);
            }
        }
        Fields::Named(f) => {
            for (_, f) in f.fields_mut() {
                field(f, types, truely_force_inline, generics, depth);
            }
        }
    }
}

fn inner(
    dt: &mut DataType,
    types: &TypeCollection,
    force_inline: bool,
    truely_force_inline: bool,
    generics: &[(Generic, DataType)],
    depth: usize,
) {
    // TODO: Can we be smart enough to determine loops, instead of just trying X times and bailing out????
    //      -> Would be more efficient but much harder. Would make the error messages much better though.
    if depth == 25 {
        // TODO: Return a `Result` instead of panicing
        // TODO: Detect which types are the cycle and report it
        panic!("Type recursion limit exceeded!");
    }

    match dt {
        DataType::List(l) => {
            inner(
                l.ty_mut(),
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            );
        }
        DataType::Map(map) => {
            inner(
                map.key_ty_mut(),
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            );
            inner(
                map.value_ty_mut(),
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            );
        }
        DataType::Nullable(d) => {
            inner(
                d,
                types,
                false, // truely_force_inline,
                truely_force_inline,
                generics,
                depth + 1,
            );
        }
        DataType::Struct(s) => {
            fields(s.fields_mut(), types, truely_force_inline, &generics, depth);
        }
        DataType::Enum(e) => {
            for (_, v) in e.variants_mut() {
                fields(v.fields_mut(), types, truely_force_inline, &generics, depth);
            }
        }
        DataType::Tuple(t) => {
            for e in t.elements_mut() {
                inner(e, types, false, truely_force_inline, generics, depth + 1);
            }
        }
        DataType::Generic(g) => {
            let mut ty = generics
                .iter()
                .find(|(ge, _)| ge == g)
                .map(|(_, dt)| dt)
                .unwrap()
                .clone(); // TODO: Properly handle this error

            if truely_force_inline {
                inner(
                    &mut ty,
                    types,
                    false,
                    truely_force_inline,
                    &[], // TODO: What should this be?
                    depth + 1,
                );
                *dt = ty;
            }
        }
        DataType::Reference(r) => {
            if r.inline() || force_inline || truely_force_inline {
                // TODO: Should we error here? Might get hit for `specta_typescript::Any`
                if let Some(ty) = types.get(r.sid()) {
                    let mut ty = ty.ty().clone();
                    inner(
                        &mut ty,
                        types,
                        false,
                        truely_force_inline,
                        &r.generics()
                            .iter()
                            .cloned()
                            .map(|(g, mut dt)| {
                                resolve_generics(&mut dt, generics);
                                (g, dt)
                            })
                            .collect::<Vec<_>>(),
                        depth + 1,
                    );
                    *dt = ty;
                }
            }
        }
        _ => {}
    }
}

/// Following all `DataType::Reference`'s filling in any `DataType::Generic`'s with the correct value.
fn resolve_generics(dt: &mut DataType, generics: &[(Generic, DataType)]) {
    // TODO: This could so only re-alloc if the type has a generics that needs replacing.
    match dt {
        DataType::List(l) => {
            resolve_generics(l.ty_mut(), generics);
        }
        DataType::Map(m) => {
            resolve_generics(m.key_ty_mut(), generics);
            resolve_generics(m.value_ty_mut(), generics);
        }
        DataType::Nullable(d) => {
            resolve_generics(d, generics);
        }
        // DataType::Struct(s) => DataType::Struct(super::StructType {
        //     generics: todo!(),
        //     fields: todo!(),
        //     ..s,
        // })
        // DataType::Enum(e) => todo!(),
        DataType::Tuple(t) => {
            for dt in t.elements_mut() {
                resolve_generics(dt, generics);
            }
        }
        DataType::Reference(r) => {
            for (_, dt) in r.generics_mut() {
                resolve_generics(dt, generics);
            }
        }
        DataType::Generic(g) => {
            // This method is run when not inlining so for `export` we do expect `DataType::Generic`.
            // TODO: Functions main documentation should explain this.
            *dt = generics
                .iter()
                .find(|(ge, _)| ge == g)
                .map(|(_, dt)| dt.clone())
                .unwrap_or(DataType::Generic(g.clone()));
        }
        _ => {}
    }
}
