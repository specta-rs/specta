//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are for advanced usecases, you should generally use [Typescript] or [JSDoc] in end-user applications.

use std::{
    borrow::{Borrow, Cow},
    fmt::Write as _,
    iter,
};

use specta::{
    TypeCollection,
    datatype::{
        DataType, DeprecatedType, Enum, Generic, List, Map, NamedDataType, NamedReference,
        OpaqueReference, Primitive, Reference, Tuple,
    },
};

use crate::{BigIntExportBehavior, Branded, Error, Exporter, Layout, legacy::js_doc, opaque};

/// Generate an `export Type = ...` Typescript string for a specific [`NamedDataType`].
///
/// This method leaves the following up to the implementer:
///  - Ensuring all referenced types are exported
///  - Handling multiple type with overlapping names
///  - Transforming the type for your serialization format (Eg. Serde)
///
pub fn export(
    exporter: &dyn AsRef<Exporter>,
    types: &TypeCollection,
    ndt: &NamedDataType,
) -> Result<String, Error> {
    let mut s = String::new();
    export_internal(&mut s, exporter.as_ref(), types, ndt)?;
    Ok(s)
}

pub(crate) fn export_internal(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    ndt: &NamedDataType,
) -> Result<(), Error> {
    if exporter.jsdoc {
        return typedef_internal(s, exporter, types, ndt);
    }

    let generics = (!ndt.generics().is_empty())
        .then(|| {
            iter::once("<")
                .chain(intersperse(ndt.generics().iter().map(|g| g.borrow()), ", "))
                .chain(iter::once(">"))
        })
        .into_iter()
        .flatten();

    // TODO: Modernise this
    let name = crate::legacy::sanitise_type_name(
        crate::legacy::ExportContext {
            cfg: exporter,
            path: vec![],
            is_export: false,
        },
        crate::legacy::NamedLocation::Type,
        &match exporter.layout {
            Layout::ModulePrefixedName => {
                let mut s = ndt.module_path().split("::").collect::<Vec<_>>().join("_");
                s.push('_');
                s.push_str(ndt.name());
                Cow::Owned(s)
            }
            _ => ndt.name().clone(),
        },
    )?;

    js_doc(s, ndt.docs(), ndt.deprecated());

    s.push_str("export type ");
    s.push_str(&name);
    for part in generics {
        s.push_str(part);
    }
    s.push_str(" = ");

    datatype(
        s,
        exporter,
        types,
        ndt.ty(),
        vec![ndt.name().clone()],
        true,
        Some(ndt.name()),
        "\t",
    )?;
    s.push_str(";\n");

    Ok(())
}

/// Generate an inlined Typescript string for a specific [`DataType`].
///
/// This methods leaves all the same things as the [`export`] method up to the user.
///
/// Note that calling this method with a tagged struct or enum may cause the tag to not be exported.
/// The type should be wrapped in a [`NamedDataType`] to provide a proper name.
///
pub fn inline(
    exporter: &dyn AsRef<Exporter>,
    types: &TypeCollection,
    dt: &DataType,
) -> Result<String, Error> {
    let mut s = String::new();
    inline_datatype(
        &mut s,
        exporter.as_ref(),
        types,
        dt,
        vec![],
        false,
        None,
        "",
        0,
        &[],
    )?;
    Ok(s)
}

// This can be used internally to prevent cloning `Typescript` instances.
// Externally this shouldn't be a concern so we don't expose it.
pub(crate) fn typedef_internal(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    dt: &NamedDataType,
) -> Result<(), Error> {
    let generics = (!dt.generics().is_empty())
        .then(|| {
            iter::once("<")
                .chain(intersperse(dt.generics().iter().map(|g| g.borrow()), ", "))
                .chain(iter::once(">"))
        })
        .into_iter()
        .flatten();

    let name = dt.name();
    let type_name = iter::empty()
        .chain([name.as_ref()])
        .chain(generics)
        .collect::<String>();

    s.push_str("/**\n");

    if !dt.docs().is_empty() {
        for line in dt.docs().lines() {
            s.push_str("\t* ");
            s.push_str(line);
            s.push('\n');
        }
        s.push_str("\t*\n");
    }

    if let Some(deprecated) = dt.deprecated() {
        s.push_str("\t* @deprecated");
        if let DeprecatedType::DeprecatedWithSince { note, .. } = deprecated {
            s.push(' ');
            s.push_str(note);
        }
        s.push('\n');
    }

    s.push_str("\t* @typedef {");
    datatype(
        s,
        exporter,
        types,
        dt.ty(),
        vec![dt.name().clone()],
        false,
        Some(dt.name()),
        "\t*\t",
    )?;
    s.push_str("} ");
    s.push_str(&type_name);
    s.push('\n');
    s.push_str("\t*/");

    Ok(())
}

/// Generate an Typescript string to refer to a specific [`DataType`].
///
/// For primitives this will include the literal type but for named type it will contain a reference.
///
/// See [`export`] for the list of things to consider when using this.
pub fn reference(
    exporter: &dyn AsRef<Exporter>,
    types: &TypeCollection,
    r: &Reference,
) -> Result<String, Error> {
    let mut s = String::new();
    reference_dt(&mut s, exporter.as_ref(), types, r, vec![], false, &[])?;
    Ok(s)
}

fn merged_generics(
    parent: &[(Generic, DataType)],
    child: &[(Generic, DataType)],
) -> Vec<(Generic, DataType)> {
    child
        .iter()
        .map(|(generic, dt)| (generic.clone(), resolve_generics_in_datatype(dt, parent)))
        .chain(parent.iter().cloned())
        .collect()
}

fn resolve_generics_in_datatype(dt: &DataType, generics: &[(Generic, DataType)]) -> DataType {
    match dt {
        DataType::Primitive(_) | DataType::Reference(_) => dt.clone(),
        DataType::List(l) => {
            let mut out = l.clone();
            out.set_ty(resolve_generics_in_datatype(l.ty(), generics));
            DataType::List(out)
        }
        DataType::Map(m) => {
            let mut out = m.clone();
            out.set_key_ty(resolve_generics_in_datatype(m.key_ty(), generics));
            out.set_value_ty(resolve_generics_in_datatype(m.value_ty(), generics));
            DataType::Map(out)
        }
        DataType::Nullable(def) => {
            DataType::Nullable(Box::new(resolve_generics_in_datatype(def, generics)))
        }
        DataType::Struct(st) => {
            let mut out = st.clone();
            match out.fields_mut() {
                specta::datatype::Fields::Unit => {}
                specta::datatype::Fields::Unnamed(unnamed) => {
                    for field in unnamed.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = resolve_generics_in_datatype(ty, generics);
                        }
                    }
                }
                specta::datatype::Fields::Named(named) => {
                    for (_, field) in named.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = resolve_generics_in_datatype(ty, generics);
                        }
                    }
                }
            }
            DataType::Struct(out)
        }
        DataType::Enum(en) => {
            let mut out = en.clone();
            for (_, variant) in out.variants_mut() {
                match variant.fields_mut() {
                    specta::datatype::Fields::Unit => {}
                    specta::datatype::Fields::Unnamed(unnamed) => {
                        for field in unnamed.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve_generics_in_datatype(ty, generics);
                            }
                        }
                    }
                    specta::datatype::Fields::Named(named) => {
                        for (_, field) in named.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve_generics_in_datatype(ty, generics);
                            }
                        }
                    }
                }
            }
            DataType::Enum(out)
        }
        DataType::Tuple(t) => {
            let mut out = t.clone();
            for element in out.elements_mut() {
                *element = resolve_generics_in_datatype(element, generics);
            }
            DataType::Tuple(out)
        }
        DataType::Generic(g) => {
            if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                if matches!(resolved_dt, DataType::Generic(inner) if inner == g) {
                    dt.clone()
                } else {
                    resolve_generics_in_datatype(resolved_dt, generics)
                }
            } else {
                dt.clone()
            }
        }
    }
}

// Internal function to handle inlining without cloning DataType nodes
#[allow(clippy::too_many_arguments)]
fn inline_datatype(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    parent_name: Option<&str>,
    prefix: &str,
    depth: usize,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    // Prevent infinite recursion
    if depth == 25 {
        return Err(Error::InvalidName {
            path: location.join("."),
            name: "Type recursion limit exceeded during inline expansion".into(),
        });
    }

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(&exporter.bigint, p, location)?),
        DataType::List(l) => {
            // Inline the list element type
            let mut dt_str = String::new();
            crate::legacy::datatype_inner(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                    is_export,
                },
                &specta::datatype::FunctionReturnType::Value(l.ty().clone()),
                types,
                &mut dt_str,
                generics,
            )?;

            let dt_str = if (dt_str.contains(' ') && !dt_str.ends_with('}'))
                || (dt_str.contains(' ') && (dt_str.contains('&') || dt_str.contains('|')))
            {
                format!("({dt_str})")
            } else {
                dt_str
            };

            if let Some(length) = l.length() {
                s.push('[');
                for n in 0..length {
                    if n != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&dt_str);
                }
                s.push(']');
            } else {
                write!(s, "{dt_str}[]")?;
            }
        }
        DataType::Map(m) => map_dt(s, exporter, types, m, location, is_export, generics)?,
        DataType::Nullable(def) => {
            inline_datatype(
                s,
                exporter,
                types,
                def,
                location,
                is_export,
                parent_name,
                prefix,
                depth + 1,
                generics,
            )?;
            let or_null = " | null";
            if !s.ends_with(&or_null) {
                s.push_str(or_null);
            }
        }
        DataType::Struct(st) => {
            // If we have generics to resolve, handle the struct inline to preserve context
            if !generics.is_empty() {
                use specta::datatype::Fields;
                match st.fields() {
                    Fields::Unit => s.push_str("null"),
                    Fields::Named(named) => {
                        s.push('{');
                        let mut has_field = false;
                        for (key, field) in named.fields() {
                            // Skip fields without a type (e.g., flattened or skipped fields)
                            let Some(field_ty) = field.ty() else {
                                continue;
                            };

                            has_field = true;
                            s.push('\n');
                            s.push_str(prefix);
                            s.push('\t');
                            s.push_str(key);
                            s.push_str(": ");
                            inline_datatype(
                                s,
                                exporter,
                                types,
                                field_ty,
                                location.clone(),
                                is_export,
                                parent_name,
                                prefix,
                                depth + 1,
                                generics,
                            )?;
                            s.push(',');
                        }

                        if has_field {
                            s.push('\n');
                            s.push_str(prefix);
                        }

                        s.push('}');
                    }
                    Fields::Unnamed(_) => {
                        // For unnamed fields, fall back to legacy handling
                        crate::legacy::struct_datatype(
                            crate::legacy::ExportContext {
                                cfg: exporter,
                                path: vec![],
                                is_export,
                            },
                            parent_name,
                            st,
                            types,
                            s,
                            prefix,
                            generics,
                        )?
                    }
                }
            } else {
                // No generics, use legacy path
                crate::legacy::struct_datatype(
                    crate::legacy::ExportContext {
                        cfg: exporter,
                        path: vec![],
                        is_export,
                    },
                    parent_name,
                    st,
                    types,
                    s,
                    prefix,
                    Default::default(),
                )?
            }
        }
        DataType::Enum(e) => enum_dt(s, exporter, types, e, location, is_export, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, location, is_export, generics)?,
        DataType::Reference(r) => {
            // Always inline references when in inline mode
            if let Reference::Named(r) = r
                && let Some(ndt) = r.get(types)
            {
                let combined_generics = merged_generics(generics, r.generics());
                inline_datatype(
                    s,
                    exporter,
                    types,
                    ndt.ty(),
                    location,
                    is_export,
                    parent_name,
                    prefix,
                    depth + 1,
                    &combined_generics,
                )?;
            } else {
                // Fallback to regular reference if type not found
                reference_dt(s, exporter, types, r, location, is_export, generics)?;
            }
        }
        DataType::Generic(g) => {
            // Try to resolve the generic from the generics map
            if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                if matches!(resolved_dt, DataType::Generic(inner) if inner == g) {
                    s.push_str(<Generic as Borrow<str>>::borrow(g));
                    return Ok(());
                }
                // Recursively inline the resolved type
                inline_datatype(
                    s,
                    exporter,
                    types,
                    resolved_dt,
                    location,
                    is_export,
                    parent_name,
                    prefix,
                    depth + 1,
                    generics,
                )?;
            } else {
                // Fallback to placeholder name if not found in the generics map
                // This can happen for unsubstituted generic types
                s.push_str(<Generic as Borrow<str>>::borrow(g));
            }
        }
    }

    Ok(())
}

// TODO: private
#[allow(clippy::too_many_arguments)]
pub(crate) fn datatype(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    parent_name: Option<&str>,
    prefix: &str,
) -> Result<(), Error> {
    datatype_with_generics(
        s,
        exporter,
        types,
        dt,
        location,
        is_export,
        parent_name,
        prefix,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn datatype_with_generics(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    // TODO: Validating the variant from `dt` can be flattened

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(&exporter.bigint, p, location)?),
        DataType::List(l) => list_dt(s, exporter, types, l, location, is_export, generics)?,
        DataType::Map(m) => map_dt(s, exporter, types, m, location, is_export, generics)?,
        DataType::Nullable(def) => {
            // TODO: Replace legacy stuff
            crate::legacy::datatype_inner(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                    is_export,
                },
                &specta::datatype::FunctionReturnType::Value((**def).clone()),
                types,
                s,
                generics,
            )?;

            let or_null = " | null";
            if !s.ends_with(&or_null) {
                s.push_str(or_null);
            }

            // datatype(s, ts, types, &*t, location, state)?;
            // let or_null = " | null";
            // if !s.ends_with(or_null) {
            //     s.push_str(or_null);
            // }
        }
        DataType::Struct(st) => {
            // location.push(st.name().clone());
            // fields_dt(s, ts, types, st.name(), &st.fields(), location, state)?

            crate::legacy::struct_datatype(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                    is_export,
                },
                parent_name,
                st,
                types,
                s,
                prefix,
                generics,
            )?
        }
        DataType::Enum(e) => enum_dt(s, exporter, types, e, location, is_export, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, location, is_export, generics)?,
        DataType::Reference(r) => {
            reference_dt(s, exporter, types, r, location, is_export, generics)?
        }
        DataType::Generic(g) => {
            if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                if matches!(resolved_dt, DataType::Generic(inner) if inner == g) {
                    s.push_str(g.borrow());
                    return Ok(());
                }
                datatype_with_generics(
                    s,
                    exporter,
                    types,
                    resolved_dt,
                    location,
                    is_export,
                    parent_name,
                    prefix,
                    generics,
                )?;
            } else {
                s.push_str(g.borrow());
            }
        }
    };

    Ok(())
}

fn primitive_dt(
    b: &BigIntExportBehavior,
    p: &Primitive,
    location: Vec<Cow<'static, str>>,
) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 | f32 | f16 | f64 => "number",
        usize | isize | i64 | u64 | i128 | u128 => match b {
            BigIntExportBehavior::String => "string",
            BigIntExportBehavior::Number => "number",
            BigIntExportBehavior::BigInt => "bigint",
            BigIntExportBehavior::Fail => {
                return Err(Error::BigIntForbidden {
                    path: location.join("."),
                });
            }
        },
        Primitive::bool => "boolean",
        String | char => "string",
    })
}

fn list_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    l: &List,
    _location: Vec<Cow<'static, str>>,
    // TODO: Remove this???
    is_export: bool,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    // TODO: This is the legacy stuff
    {
        let mut dt = String::new();
        crate::legacy::datatype_inner(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
                is_export,
            },
            &specta::datatype::FunctionReturnType::Value(l.ty().clone()),
            types,
            &mut dt,
            generics,
        )?;

        let dt = if (dt.contains(' ') && !dt.ends_with('}'))
          // This is to do with maintaining order of operations.
          // Eg `{} | {}` must be wrapped in parens like `({} | {})[]` but `{}` doesn't cause `{}[]` is valid
          || (dt.contains(' ') && (dt.contains('&') || dt.contains('|')))
        {
            format!("({dt})")
        } else {
            dt
        };

        if let Some(length) = l.length() {
            s.push('[');

            for n in 0..length {
                if n != 0 {
                    s.push_str(", ");
                }

                s.push_str(&dt);
            }

            s.push(']');
        } else {
            write!(s, "{dt}[]")?;
        }
    }

    //     // We use `T[]` instead of `Array<T>` to avoid issues with circular references.

    //     let mut result = String::new();
    //     datatype(&mut result, ts, types, &l.ty(), location, state)?;
    //     let result = if (result.contains(' ') && !result.ends_with('}'))
    //         // This is to do with maintaining order of operations.
    //         // Eg `{} | {}` must be wrapped in parens like `({} | {})[]` but `{}` doesn't cause `{}[]` is valid
    //         || (result.contains(' ') && (result.contains('&') || result.contains('|')))
    //     {
    //         format!("({result})")
    //     } else {
    //         result
    //     };

    //     match l.length() {
    //         Some(len) => {
    //             s.push_str("[");
    //             iter_with_sep(
    //                 s,
    //                 0..len,
    //                 |s, _| {
    //                     s.push_str(&result);
    //                     Ok(())
    //                 },
    //                 ", ",
    //             )?;
    //             s.push_str("]");
    //         }
    //         None => {
    //             s.push_str(&result);
    //             s.push_str("[]");
    //         }
    //     }

    Ok(())
}

fn map_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    m: &Map,
    _location: Vec<Cow<'static, str>>,
    // TODO: Remove
    is_export: bool,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    {
        fn is_exhaustive(dt: &DataType, types: &TypeCollection) -> bool {
            match dt {
                DataType::Enum(e) => e.variants().iter().filter(|(_, v)| !v.skip()).count() == 0,
                DataType::Reference(Reference::Named(r)) => {
                    if let Some(ndt) = r.get(types) {
                        is_exhaustive(ndt.ty(), types)
                    } else {
                        false
                    }
                }
                DataType::Reference(Reference::Opaque(_)) => false,
                _ => true,
            }
        }

        let is_exhaustive = is_exhaustive(m.key_ty(), types);

        // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
        // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
        if !is_exhaustive {
            s.push_str("Partial<");
        }
        s.push_str("{ [key in ");
        crate::legacy::datatype_inner(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
                is_export: true,
            },
            &specta::datatype::FunctionReturnType::Value(m.key_ty().clone()),
            types,
            s,
            generics,
        )?;
        s.push_str("]: ");
        crate::legacy::datatype_inner(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
                is_export,
            },
            &specta::datatype::FunctionReturnType::Value(m.value_ty().clone()),
            types,
            s,
            generics,
        )?;
        s.push_str(" }");
        if !is_exhaustive {
            s.push('>');
        }
    }
    // assert!(flattening, "todo: map flattening");

    // // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
    // // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
    // s.push_str("Partial<{ [key in ");
    // datatype(s, ts, types, m.key_ty(), location.clone(), state)?;
    // s.push_str("]: ");
    // datatype(s, ts, types, m.value_ty(), location, state)?;
    // s.push_str(" }>");
    Ok(())
}

fn enum_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    e: &Enum,
    _location: Vec<Cow<'static, str>>,
    // TODO: Remove
    is_export: bool,
    prefix: &str,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    // TODO: Drop legacy stuff
    {
        crate::legacy::enum_datatype(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
                is_export,
            },
            e,
            types,
            s,
            prefix,
            generics,
        )?
    }

    //     assert!(!state.flattening, "todo: support for flattening enums"); // TODO

    //     location.push(e.name().clone());

    //     let mut _ts = None;
    //     if e.skip_bigint_checks() {
    //         _ts = Some(Typescript {
    //             bigint: BigIntExportBehavior::Number,
    //             ..ts.clone()
    //         });
    //         _ts.as_ref().expect("set above")
    //     } else {
    //         ts
    //     };

    //     let variants = e.variants().iter().filter(|(_, variant)| !variant.skip());

    //     if variants.clone().next().is_none()
    //     /* is_empty */
    //     {
    //         s.push_str("never");
    //         return Ok(());
    //     }

    //     let mut variants = variants
    //         .into_iter()
    //         .map(|(variant_name, variant)| {
    //             let mut s = String::new();
    //             let mut location = location.clone();
    //             location.push(variant_name.clone());

    //             // TODO
    //             // variant.deprecated()
    //             // variant.docs()

    //             match &e.repr() {
    //                 EnumRepr::Untagged => {
    //                     fields_dt(&mut s, ts, types, variant_name, variant.fields(), location, state)?;
    //                 },
    //                 EnumRepr::External => match variant.fields() {
    //                     Fields::Unit => {
    //                         s.push_str("\"");
    //                         s.push_str(variant_name);
    //                         s.push_str("\"");
    //                     },
    //                     Fields::Unnamed(n) if n.fields().into_iter().filter(|f| f.ty().is_some()).next().is_none() /* is_empty */ => {
    //                         // We detect `#[specta(skip)]` by checking if the unfiltered fields are also empty.
    //                         if n.fields().is_empty() {
    //                             s.push_str("{ ");
    //                             s.push_str(&escape_key(variant_name));
    //                             s.push_str(": [] }");
    //                         } else {
    //                             s.push_str("\"");
    //                             s.push_str(variant_name);
    //                             s.push_str("\"");
    //                         }
    //                     }
    //                     _ => {
    //                         s.push_str("{ ");
    //                         s.push_str(&escape_key(variant_name));
    //                         s.push_str(": ");
    //                         fields_dt(&mut s, ts, types, variant_name, variant.fields(), location, state)?;
    //                         s.push_str(" }");
    //                     }
    //                 }
    //                 EnumRepr::Internal { tag } => {
    //                     // TODO: Unconditionally wrapping in `(` kinda sucks.
    //                     write!(s, "({{ {}: \"{}\"", escape_key(tag), variant_name).expect("infallible");

    //                     match variant.fields() {
    //                         Fields::Unit => {
    //                              s.push_str(" })");
    //                         },
    //                         // Fields::Unnamed(f) if f.fields.iter().filter(|f| f.ty().is_some()).count() == 1 => {
    //                         //     // let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

    //                         //      s.push_str("______"); // TODO

    //                         // //     // if fields.len

    //                         // //     // TODO: Having no fields are skipping is valid
    //                         // //     // TODO: Having more than 1 field is invalid

    //                         // //     // TODO: Check if the field's type is object-like and can be merged.

    //                         //     // todo!();
    //                         // }
    //                         f => {
    //                             // TODO: Cleanup and explain this
    //                             let mut skip_join = false;
    //                             if let Fields::Unnamed(f) = &f {
    //                                 let mut fields = f.fields.iter().filter(|f| f.ty().is_some());
    //                                 if let (Some(v), None) = (fields.next(), fields.next()) {
    //                                     if let Some(DataType::Tuple(tuple)) = &v.ty() {
    //                                         skip_join = tuple.elements().len() == 0;
    //                                     }
    //                                 }
    //                             }

    //                             if skip_join {
    //                                 s.push_str(" })");
    //                             } else {
    //                                 s.push_str(" } & ");

    //                                 // TODO: Can we be smart enough to omit the `{` and `}` if this is an object
    //                                 fields_dt(&mut s, ts, types, variant_name, f, location, state)?;
    //                                 s.push_str(")");
    //                             }

    //                             // match f {
    //                             //     // Checked above
    //                             //     Fields::Unit => unreachable!(),
    //                             //     Fields::Unnamed(unnamed_fields) => unnamed_fields,
    //                             //     Fields::Named(named_fields) => todo!(),
    //                             // }

    //                             // println!("{:?}", f); // TODO: If object we can join in fields like this, else `} & ...`
    //                             // flattened_fields_dt(&mut s, ts, types, variant_name, f, location, false)?; // TODO: Fix `flattening`

    //                         }
    //                     }

    //                 }
    //                 EnumRepr::Adjacent { tag, content } => {
    //                     write!(s, "{{ {}: \"{}\"", escape_key(tag), variant_name).expect("infallible");

    //                     match variant.fields() {
    //                         Fields::Unit => {},
    //                         f => {
    //                             write!(s, "; {}: ", escape_key(content)).expect("infallible");
    //                             fields_dt(&mut s, ts, types, variant_name, f, location, state)?;
    //                         }
    //                     }

    //                     s.push_str(" }");
    //                 }
    //             }

    //             Ok(s)
    //         })
    //         .collect::<Result<Vec<String>, Error>>()?;

    //     // TODO: Instead of deduplicating on the string, we should do it in the AST.
    //     // This would avoid the intermediate `String` allocations and be more reliable.
    //     variants.dedup();

    //     iter_with_sep(
    //         s,
    //         variants,
    //         |s, v| {
    //             s.push_str(&v);
    //             Ok(())
    //         },
    //         " | ",
    //     )?;

    Ok(())
}

// fn fields_dt(
//     s: &mut String,
//     ts: &Typescript,
//     types: &TypeCollection,
//     name: &Cow<'static, str>,
//     f: &Fields,
//     location: Vec<Cow<'static, str>>,
//     state: State,
// ) -> Result<(), Error> {
//     match f {
//         Fields::Unit => {
//             assert!(!state.flattening, "todo: support for flattening enums"); // TODO
//             s.push_str("null")
//         }
//         Fields::Unnamed(f) => {
//             assert!(!state.flattening, "todo: support for flattening enums"); // TODO
//             let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

//             // A single field usually becomes `T`.
//             // but when `#[serde(skip)]` is used it should be `[T]`.
//             if fields.clone().count() == 1 && f.fields.len() == 1 {
//                 return field_dt(
//                     s,
//                     ts,
//                     types,
//                     None,
//                     fields.next().expect("checked above"),
//                     location,
//                     state,
//                 );
//             }

//             s.push_str("[");
//             iter_with_sep(
//                 s,
//                 fields.enumerate(),
//                 |s, (i, f)| {
//                     let mut location = location.clone();
//                     location.push(i.to_string().into());

//                     field_dt(s, ts, types, None, f, location, state)
//                 },
//                 ", ",
//             )?;
//             s.push_str("]");
//         }
//         Fields::Named(f) => {
//             let fields = f.fields().into_iter().filter(|(_, f)| f.ty().is_some());
//             if fields.clone().next().is_none()
//             /* is_empty */
//             {
//                 assert!(!state.flattening, "todo: support for flattening enums"); // TODO

//                 if let Some(tag) = f.tag() {
//                     if !state.flattening {}

//                     write!(s, "{{ {}: \"{name}\" }}", escape_key(tag)).expect("infallible");
//                 } else {
//                     s.push_str("Record<string, never>");
//                 }

//                 return Ok(());
//             }

//             if !state.flattening {
//                 s.push_str("{ ");
//             }
//             if let Some(tag) = &f.tag() {
//                 write!(s, "{}: \"{name}\"; ", escape_key(tag)).expect("infallible");
//             }

//             iter_with_sep(
//                 s,
//                 fields,
//                 |s, (key, f)| {
//                     let mut location = location.clone();
//                     location.push(key.clone());

//                     field_dt(s, ts, types, Some(key), f, location, state)
//                 },
//                 "; ",
//             )?;
//             if !state.flattening {
//                 s.push_str(" }");
//             }
//         }
//     }
//     Ok(())
// }

// // TODO: Remove this to avoid so much duplicate logic
// fn flattened_fields_dt(
//     s: &mut String,
//     ts: &Typescript,
//     types: &TypeCollection,
//     name: &Cow<'static, str>,
//     f: &Fields,
//     location: Vec<Cow<'static, str>>,
//     state: State,
// ) -> Result<(), Error> {
//     match f {
//         Fields::Unit => todo!(), // s.push_str("null"),
//         Fields::Unnamed(f) => {
//             // TODO: Validate flattening?

//             let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

//             // A single field usually becomes `T`.
//             // but when `#[serde(skip)]` is used it should be `[T]`.
//             if fields.clone().count() == 1 && f.fields.len() == 1 {
//                 return field_dt(
//                     s,
//                     ts,
//                     types,
//                     None,
//                     fields.next().expect("checked above"),
//                     location,
//                     state,
//                 );
//             }

//             s.push_str("[");
//             iter_with_sep(
//                 s,
//                 fields.enumerate(),
//                 |s, (i, f)| {
//                     let mut location = location.clone();
//                     location.push(i.to_string().into());

//                     field_dt(s, ts, types, None, f, location, state)
//                 },
//                 ", ",
//             )?;
//             s.push_str("]");
//         }
//         Fields::Named(f) => {
//             let fields = f.fields().into_iter().filter(|(_, f)| f.ty().is_some());
//             if fields.clone().next().is_none()
//             /* is_empty */
//             {
//                 if let Some(tag) = f.tag() {
//                     write!(s, "{{ {}: \"{name}\" }}", escape_key(tag)).expect("infallible");
//                 } else {
//                     s.push_str("Record<string, never>");
//                 }

//                 return Ok(());
//             }

//             // s.push_str("{ "); // TODO
//             if let Some(tag) = &f.tag() {
//                 write!(s, "{}: \"{name}\"; ", escape_key(tag)).expect("infallible");
//             }

//             iter_with_sep(
//                 s,
//                 fields,
//                 |s, (key, f)| {
//                     let mut location = location.clone();
//                     location.push(key.clone());

//                     field_dt(s, ts, types, Some(key), f, location, state)
//                 },
//                 "; ",
//             )?;
//             // s.push_str(" }"); // TODO
//         }
//     }
//     Ok(())
// }

// fn field_dt(
//     s: &mut String,
//     ts: &Typescript,
//     types: &TypeCollection,
//     key: Option<&Cow<'static, str>>,
//     f: &Field,
//     location: Vec<Cow<'static, str>>,
//     state: State,
// ) -> Result<(), Error> {
//     let Some(ty) = f.ty() else {
//         // These should be filtered out before getting here.
//         unreachable!()
//     };

//     // TODO
//     // field.deprecated(),
//     // field.docs(),

//     let ty = if f.inline() {
//         specta::datatype::inline_dt(types, ty.clone())
//     } else {
//         ty.clone()
//     };

//     if !f.flatten() {
//         if let Some(key) = key {
//             s.push_str(&*escape_key(key));
//             // https://github.com/specta-rs/rspc/issues/100#issuecomment-1373092211
//             if f.optional() {
//                 s.push_str("?");
//             }
//             s.push_str(": ");
//         }
//     } else {
//         // TODO: We need to validate the inner type can be flattened safely???

//         //     data

//         //     match ty {
//         //         DataType::Any => todo!(),
//         //         DataType::Unknown => todo!(),
//         //         DataType::Primitive(primitive_type) => todo!(),
//         //         DataType::Literal(literal_type) => todo!(),
//         //         DataType::List(list) => todo!(),
//         //         DataType::Map(map) => todo!(),
//         //         DataType::Nullable(data_type) => todo!(),
//         //         DataType::Struct(st) => {
//         //             // location.push(st.name().clone()); // TODO
//         //             flattened_fields_dt(s, ts, types, st.name(), &st.fields(), location)?
//         //         }

//         //         // flattened_fields_dt(s, ts, types, &ty, location)?,
//         //         DataType::Enum(enum_type) => todo!(),
//         //         DataType::Tuple(tuple_type) => todo!(),
//         //         DataType::Reference(reference) => todo!(),
//         //         DataType::Generic(generic_type) => todo!(),
//         //     };
//     }

//     // TODO: Only flatten when object is inline?

//     datatype(
//         s,
//         ts,
//         types,
//         &ty,
//         location,
//         State {
//             flattening: state.flattening || f.flatten(),
//         },
//     )?;

//     // TODO: This is not always correct but is it ever correct?
//     // If we can't use `?` (Eg. in a tuple) we manually join it.
//     // if key.is_none() && f.optional() {
//     //     s.push_str(" | undefined");
//     // }

//     Ok(())
// }

fn tuple_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    t: &Tuple,
    _location: Vec<Cow<'static, str>>,
    // TODO: Remove
    is_export: bool,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    {
        s.push_str(&crate::legacy::tuple_datatype(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
                is_export,
            },
            t,
            types,
            generics,
        )?);
    }

    // match &t.elements()[..] {
    //     [] => s.push_str("null"),
    //     elems => {
    //         s.push_str("[");
    //         iter_with_sep(
    //             s,
    //             elems.into_iter().enumerate(),
    //             |s, (i, dt)| {
    //                 let mut location = location.clone();
    //                 location.push(i.to_string().into());

    //                 datatype(s, ts, types, &dt, location, state)
    //             },
    //             ", ",
    //         )?;
    //         s.push_str("]");
    //     }
    // }
    Ok(())
}

fn reference_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    r: &Reference,
    location: Vec<Cow<'static, str>>,
    // TODO: Remove
    is_export: bool,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    match r {
        Reference::Named(r) => {
            reference_named_dt(s, exporter, types, r, location, is_export, generics)
        }
        Reference::Opaque(r) => reference_opaque_dt(s, exporter, types, r),
    }
}

fn reference_opaque_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    r: &OpaqueReference,
) -> Result<(), Error> {
    if let Some(def) = r.downcast_ref::<opaque::Define>() {
        s.push_str(&def.0);
        return Ok(());
    } else if r.downcast_ref::<opaque::Any>().is_some() {
        s.push_str("any");
        return Ok(());
    } else if r.downcast_ref::<opaque::Unknown>().is_some() {
        s.push_str("unknown");
        return Ok(());
    } else if r.downcast_ref::<opaque::Never>().is_some() {
        s.push_str("never");
        return Ok(());
    } else if let Some(def) = r.downcast_ref::<Branded>() {
        // TODO: Build onto `s` instead of appending a separate string
        s.push_str(&match def.ty() {
            DataType::Reference(r) => reference(exporter, types, r),
            ty => inline(exporter, types, ty),
        }?);
        s.push_str(r#" & ""#);
        s.push_str(def.brand());
        s.push('"');
        return Ok(());
    }

    return Err(Error::UnsupportedOpaqueReference(r.clone()));
}

fn reference_named_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &TypeCollection,
    r: &NamedReference,
    location: Vec<Cow<'static, str>>,
    // TODO: Remove
    is_export: bool,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    // TODO: Legacy stuff
    {
        let ndt = r.get(types).ok_or_else(|| Error::DanglingNamedReference {
            reference: format!("{r:?}"),
        })?;

        // Check if this reference should be inlined
        if r.inline() {
            // Inline the referenced type directly, resolving generics
            let combined_generics = merged_generics(generics, r.generics());
            return inline_datatype(
                s,
                exporter,
                types,
                ndt.ty(),
                location,
                is_export,
                None,
                "",
                0,
                &combined_generics,
            );
        }

        let name = match exporter.layout {
            Layout::ModulePrefixedName => {
                let mut s = ndt.module_path().split("::").collect::<Vec<_>>().join("_");
                s.push('_');
                s.push_str(ndt.name());
                Cow::Owned(s)
            }
            Layout::Namespaces => {
                if ndt.module_path().is_empty() {
                    ndt.name().clone()
                } else {
                    let mut path =
                        ndt.module_path()
                            .split("::")
                            .fold("$s$.".to_string(), |mut s, segment| {
                                s.push_str(segment);
                                s.push('.');
                                s
                            });
                    path.push_str(ndt.name());
                    Cow::Owned(path)
                }
            }
            Layout::Files => {
                if ndt.module_path().is_empty() {
                    ndt.name().clone()
                } else {
                    let mut path =
                        ndt.module_path()
                            .split("::")
                            .fold(String::new(), |mut s, segment| {
                                s.push_str(segment);
                                s.push('.');
                                s
                            });
                    path.push_str(ndt.name());
                    Cow::Owned(path)
                }
            }
            _ => ndt.name().clone(),
        };

        s.push_str(&name);
        if !r.generics().is_empty() {
            s.push('<');

            for (i, (_, v)) in r.generics().iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }

                crate::legacy::datatype_inner(
                    crate::legacy::ExportContext {
                        cfg: exporter,
                        path: vec![],
                        is_export,
                    },
                    &specta::datatype::FunctionReturnType::Value(v.clone()),
                    types,
                    s,
                    generics,
                )?;
            }

            s.push('>');
        }
    }

    //     let ndt = types
    //         .get(r.sid())
    //         // Should be impossible without a bug in Specta.
    //         .unwrap_or_else(|| panic!("Missing {:?} in `TypeCollection`", r.sid()));

    //     if r.inline() {
    //         todo!("inline reference!");
    //     }

    //     s.push_str(ndt.name());
    //     // TODO: We could possible break this out, the root `export` function also has to emit generics.
    //     match r.generics() {
    //         [] => {}
    //         generics => {
    //             s.push('<');
    //             // TODO: Should we push a location for which generic?
    //             iter_with_sep(
    //                 s,
    //                 generics,
    //                 |s, dt| datatype(s, ts, types, &dt, location.clone(), state),
    //                 ", ",
    //             )?;
    //             s.push('>');
    //         }
    //     }

    Ok(())
}

// fn validate_name(
//     ident: &Cow<'static, str>,
//     location: &Vec<Cow<'static, str>>,
// ) -> Result<(), Error> {
//     // TODO: Use a perfect hash-map for faster lookups?
//     if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
//         return Err(Error::ForbiddenName {
//             path: location.join("."),
//             name,
//         });
//     }

//     if ident.is_empty() {
//         return Err(Error::InvalidName {
//             path: location.join("."),
//             name: ident.clone(),
//         });
//     }

//     if let Some(first_char) = ident.chars().next() {
//         if !first_char.is_alphabetic() && first_char != '_' {
//             return Err(Error::InvalidName {
//                 path: location.join("."),
//                 name: ident.clone(),
//             });
//         }
//     }

//     if ident
//         .find(|c: char| !c.is_alphanumeric() && c != '_')
//         .is_some()
//     {
//         return Err(Error::InvalidName {
//             path: location.join("."),
//             name: ident.clone(),
//         });
//     }

//     Ok(())
// }

// fn escape_key(name: &Cow<'static, str>) -> Cow<'static, str> {
//     let needs_escaping = name
//         .chars()
//         .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
//         && name
//             .chars()
//             .next()
//             .map(|first| !first.is_numeric())
//             .unwrap_or(true);

//     if !needs_escaping {
//         format!(r#""{name}""#).into()
//     } else {
//         name.clone()
//     }
// }

// fn comment() {
//     // TODO: Different JSDoc modes

//     // TODO: Regular comments
//     // TODO: Deprecated

//     // TODO: When enabled: arguments, result types
// }

// /// Iterate with separate and error handling
// fn iter_with_sep<T>(
//     s: &mut String,
//     i: impl IntoIterator<Item = T>,
//     mut item: impl FnMut(&mut String, T) -> Result<(), Error>,
//     sep: &'static str,
// ) -> Result<(), Error> {
//     for (i, e) in i.into_iter().enumerate() {
//         if i != 0 {
//             s.push_str(sep);
//         }
//         (item)(s, e)?;
//     }
//     Ok(())
// }

// A smaller helper until this is stablised into the Rust standard library.
fn intersperse<T: Clone>(iter: impl Iterator<Item = T>, sep: T) -> impl Iterator<Item = T> {
    iter.enumerate().flat_map(move |(i, item)| {
        if i == 0 {
            vec![item]
        } else {
            vec![sep.clone(), item]
        }
    })
}
