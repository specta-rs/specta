//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are for advanced usecases, you should generally use [crate::Typescript] or
//! [crate::JSDoc] in end-user applications.

use std::{borrow::Cow, cell::RefCell, fmt::Write as _, iter};

use specta::{
    ResolvedTypes, Types,
    datatype::{
        DataType, Deprecated, Enum, Fields, Generic, GenericReference, List, Map, NamedDataType,
        NamedReference, OpaqueReference, Primitive, Reference, Tuple, Variant,
    },
};

use crate::{
    Branded, BrandedTypeExporter, Error, Exporter, Layout,
    legacy::{
        ExportContext, deprecated_details, escape_jsdoc_text, escape_typescript_string_literal,
        is_identifier, js_doc,
    },
    map_keys, opaque,
};

/// Generate a group of `export Type = ...` Typescript string for a specific [`NamedDataType`].
///
/// This method leaves the following up to the implementer:
///  - Ensuring all referenced types are exported
///  - Handling multiple type with overlapping names
///  - Transforming the type for your serialization format (Eg. Serde)
///
/// We recommend passing in your types in bulk instead of doing individual calls as it leaves formatting to us and also allows us to merge the JSDoc types into a single large comment.
///
pub fn export<'a>(
    exporter: &dyn AsRef<Exporter>,
    types: &ResolvedTypes,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut s = String::new();
    export_internal(&mut s, exporter.as_ref(), types.as_types(), ndts, indent)?;
    Ok(s)
}

pub(crate) fn export_internal<'a>(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<(), Error> {
    if exporter.jsdoc {
        let mut ndts = ndts.peekable();
        if ndts.peek().is_none() {
            return Ok(());
        }

        s.push_str(indent);
        s.push_str("/**\n");

        for (index, ndt) in ndts.enumerate() {
            if index != 0 {
                s.push_str(indent);
                s.push_str("\t*\n");
            }

            append_typedef_body(s, exporter, types, ndt, indent)?;
        }

        s.push_str(indent);
        s.push_str("\t*/\n");
        return Ok(());
    }

    for (index, ndt) in ndts.enumerate() {
        if index != 0 {
            s.push('\n');
        }

        export_single_internal(s, exporter, types, ndt, indent)?;
    }

    Ok(())
}

fn export_single_internal(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    ndt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    if exporter.jsdoc {
        let mut typedef = String::new();
        typedef_internal(&mut typedef, exporter, types, ndt)?;
        for line in typedef.lines() {
            s.push_str(indent);
            s.push_str(line);
            s.push('\n');
        }
        return Ok(());
    }

    let generics = (!ndt.generics().is_empty())
        .then(|| {
            iter::once("<")
                .chain(intersperse(
                    ndt.generics().iter().map(|g| g.name.as_ref()),
                    ", ",
                ))
                .chain(iter::once(">"))
        })
        .into_iter()
        .flatten();

    // TODO: Modernise this
    let name = crate::legacy::sanitise_type_name(
        crate::legacy::ExportContext {
            cfg: exporter,
            path: vec![],
        },
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

    let mut comments = String::new();
    js_doc(&mut comments, ndt.docs(), ndt.deprecated(), !exporter.jsdoc);
    if !comments.is_empty() {
        for line in comments.lines() {
            s.push_str(indent);
            s.push_str(line);
            s.push('\n');
        }
    }

    s.push_str(indent);
    s.push_str("export type ");
    s.push_str(&name);
    for part in generics {
        s.push_str(part);
    }
    s.push_str(" = ");

    let _generic_scope = push_generic_scope(ndt.generics());
    datatype(
        s,
        exporter,
        types,
        ndt.ty(),
        vec![ndt.name().clone()],
        Some(ndt.name()),
        indent,
        Default::default(),
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
    types: &ResolvedTypes,
    dt: &DataType,
) -> Result<String, Error> {
    let mut s = String::new();
    inline_datatype(
        &mut s,
        exporter.as_ref(),
        types.as_types(),
        dt,
        vec![],
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
    types: &Types,
    dt: &NamedDataType,
) -> Result<(), Error> {
    s.push_str("/**\n");
    append_typedef_body(s, exporter, types, dt, "")?;

    s.push_str("\t*/");

    Ok(())
}

fn append_jsdoc_properties(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    match dt.ty() {
        DataType::Struct(strct) => match strct.fields() {
            Fields::Unit => {}
            Fields::Unnamed(unnamed) => {
                for (idx, field) in unnamed.fields().iter().enumerate() {
                    let Some(ty) = field.ty() else {
                        continue;
                    };

                    let mut ty_str = String::new();
                    let datatype_prefix = format!("{indent}\t*\t");
                    datatype(
                        &mut ty_str,
                        exporter,
                        types,
                        ty,
                        vec![dt.name().clone(), idx.to_string().into()],
                        Some(dt.name()),
                        &datatype_prefix,
                        Default::default(),
                    )?;

                    push_jsdoc_property(
                        s,
                        &ty_str,
                        &idx.to_string(),
                        field.optional(),
                        field.docs(),
                        field.deprecated(),
                        indent,
                    );
                }
            }
            Fields::Named(named) => {
                for (name, field) in named.fields() {
                    let Some(ty) = field.ty() else {
                        continue;
                    };

                    let mut ty_str = String::new();
                    let datatype_prefix = format!("{indent}\t*\t");
                    datatype(
                        &mut ty_str,
                        exporter,
                        types,
                        ty,
                        vec![dt.name().clone(), name.clone()],
                        Some(dt.name()),
                        &datatype_prefix,
                        Default::default(),
                    )?;

                    push_jsdoc_property(
                        s,
                        &ty_str,
                        name,
                        field.optional(),
                        field.docs(),
                        field.deprecated(),
                        indent,
                    );
                }
            }
        },
        DataType::Enum(enm) => {
            for (variant_name, variant) in enm.variants().iter().filter(|(_, v)| !v.skip()) {
                let mut one_variant_enum = enm.clone();
                one_variant_enum
                    .variants_mut()
                    .retain(|(name, _)| name == variant_name);

                let mut variant_ty = String::new();
                crate::legacy::enum_datatype(
                    ExportContext {
                        cfg: exporter,
                        path: vec![],
                    },
                    &one_variant_enum,
                    types,
                    &mut variant_ty,
                    "",
                    &[],
                )?;

                push_jsdoc_property(
                    s,
                    &variant_ty,
                    variant_name,
                    false,
                    variant.docs(),
                    variant.deprecated(),
                    indent,
                );
            }
        }
        _ => {}
    }

    Ok(())
}

fn push_jsdoc_property(
    s: &mut String,
    ty: &str,
    name: &str,
    optional: bool,
    docs: &str,
    deprecated: Option<&Deprecated>,
    indent: &str,
) {
    s.push_str(indent);
    s.push_str("\t* @property {");
    push_jsdoc_type(s, ty, indent);
    s.push_str("} ");
    s.push_str(&jsdoc_property_name(name, optional));

    if let Some(description) = jsdoc_description(docs, deprecated) {
        s.push_str(" - ");
        s.push_str(&description);
    }

    s.push('\n');
}

fn push_jsdoc_type(s: &mut String, ty: &str, indent: &str) {
    let mut lines = ty.lines();
    if let Some(first_line) = lines.next() {
        s.push_str(first_line);
    }

    for line in lines {
        s.push('\n');

        if line
            .strip_prefix(indent)
            .is_some_and(|rest| rest.starts_with("\t*"))
        {
            s.push_str(line);
        } else {
            s.push_str(indent);
            s.push_str("\t* ");
            s.push_str(line);
        }
    }
}

fn jsdoc_property_name(name: &str, optional: bool) -> String {
    let name = if is_identifier(name) {
        name.to_string()
    } else {
        format!("\"{}\"", escape_typescript_string_literal(name))
    };

    if optional { format!("[{name}]") } else { name }
}

fn append_typedef_body(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    let generics = (!dt.generics().is_empty())
        .then(|| {
            iter::once("<")
                .chain(intersperse(
                    dt.generics().iter().map(|g| g.name.as_ref()),
                    ", ",
                ))
                .chain(iter::once(">"))
        })
        .into_iter()
        .flatten();

    let name = dt.name();
    let type_name = iter::empty()
        .chain([name.as_ref()])
        .chain(generics)
        .collect::<String>();

    let mut typedef_ty = String::new();
    let datatype_prefix = format!("{indent}\t*\t");
    let _generic_scope = push_generic_scope(dt.generics());
    datatype(
        &mut typedef_ty,
        exporter,
        types,
        dt.ty(),
        vec![dt.name().clone()],
        Some(dt.name()),
        &datatype_prefix,
        Default::default(),
    )?;

    if !dt.docs().is_empty() {
        for line in dt.docs().lines() {
            s.push_str(indent);
            s.push_str("\t* ");
            s.push_str(&escape_jsdoc_text(line));
            s.push('\n');
        }
        s.push_str(indent);
        s.push_str("\t*\n");
    }

    if let Some(deprecated) = dt.deprecated() {
        s.push_str(indent);
        s.push_str("\t* @deprecated");
        if let Some(details) = deprecated_details(deprecated) {
            s.push(' ');
            s.push_str(&details);
        }
        s.push('\n');
    }

    s.push_str(indent);
    s.push_str("\t* @typedef {");
    push_jsdoc_type(s, &typedef_ty, indent);
    s.push_str("} ");
    s.push_str(&type_name);
    s.push('\n');

    append_jsdoc_properties(s, exporter, types, dt, indent)?;

    Ok(())
}

fn jsdoc_description(docs: &str, deprecated: Option<&Deprecated>) -> Option<String> {
    let docs = docs
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| escape_jsdoc_text(line).into_owned())
        .collect::<Vec<_>>()
        .join(" ");

    let deprecated = deprecated.map(|deprecated| {
        let mut value = String::from("@deprecated");
        if let Some(details) = deprecated_details(deprecated) {
            value.push(' ');
            value.push_str(&escape_jsdoc_text(&details));
        }
        value
    });

    match (docs.is_empty(), deprecated) {
        (true, None) => None,
        (true, Some(deprecated)) => Some(deprecated),
        (false, None) => Some(docs),
        (false, Some(deprecated)) => Some(format!("{docs} {deprecated}")),
    }
}

/// Generate an Typescript string to refer to a specific [`DataType`].
///
/// For primitives this will include the literal type but for named type it will contain a reference.
///
/// See [`export`] for the list of things to consider when using this.
pub fn reference(
    exporter: &dyn AsRef<Exporter>,
    types: &ResolvedTypes,
    r: &Reference,
) -> Result<String, Error> {
    let mut s = String::new();
    reference_dt(
        &mut s,
        exporter.as_ref(),
        types.as_types(),
        r,
        vec![],
        "",
        &[],
    )?;
    Ok(s)
}

pub(crate) fn datatype_with_inline_attr(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    inline: bool,
) -> Result<(), Error> {
    if inline {
        return shallow_inline_datatype(
            s,
            exporter,
            types,
            dt,
            location,
            parent_name,
            prefix,
            generics,
        );
    }

    datatype(
        s,
        exporter,
        types,
        dt,
        location,
        parent_name,
        prefix,
        generics,
    )
}

fn merged_generics(
    parent: &[(GenericReference, DataType)],
    child: &[(GenericReference, DataType)],
) -> Vec<(GenericReference, DataType)> {
    let unshadowed_parent = parent
        .iter()
        .filter(|(parent_generic, _)| {
            !child
                .iter()
                .any(|(child_generic, _)| child_generic == parent_generic)
        })
        .cloned();

    child
        .iter()
        .map(|(generic, dt)| (generic.clone(), resolve_generics_in_datatype(dt, parent)))
        .chain(unshadowed_parent)
        .collect()
}

thread_local! {
    static INLINE_REFERENCE_STACK: RefCell<Vec<(Cow<'static, str>, Cow<'static, str>, Vec<(GenericReference, DataType)>)>> = const { RefCell::new(Vec::new()) };
    static RESOLVING_GENERICS: RefCell<Vec<GenericReference>> = const { RefCell::new(Vec::new()) };
    static GENERIC_NAME_STACK: RefCell<Vec<Vec<Generic>>> = const { RefCell::new(Vec::new()) };
}

struct GenericScopeGuard;

impl Drop for GenericScopeGuard {
    fn drop(&mut self) {
        GENERIC_NAME_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

fn push_generic_scope(generics: &[Generic]) -> GenericScopeGuard {
    GENERIC_NAME_STACK.with(|stack| {
        stack.borrow_mut().push(generics.to_vec());
    });
    GenericScopeGuard
}

fn resolve_generic_name(generic: &GenericReference) -> Option<Cow<'static, str>> {
    GENERIC_NAME_STACK.with(|stack| {
        stack.borrow().iter().rev().find_map(|scope| {
            scope
                .iter()
                .find(|candidate| candidate.reference() == *generic)
                .map(|generic| generic.name.clone())
        })
    })
}

fn write_generic_reference(s: &mut String, generic: &GenericReference) -> Result<(), Error> {
    let generic_name = resolve_generic_name(generic)
        .ok_or_else(|| Error::unresolved_generic_reference(format!("{generic:?}")))?;
    s.push_str(generic_name.as_ref());
    Ok(())
}

fn shallow_inline_datatype(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::List(list) => {
            let mut inner = String::new();
            shallow_inline_datatype(
                &mut inner,
                exporter,
                types,
                list.ty(),
                location,
                parent_name,
                prefix,
                generics,
            )?;

            let inner = if (inner.contains(' ') && !inner.ends_with('}'))
                || (inner.contains(' ') && (inner.contains('&') || inner.contains('|')))
            {
                format!("({inner})")
            } else {
                inner
            };

            if let Some(length) = list.length() {
                s.push('[');
                for i in 0..length {
                    if i != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&inner);
                }
                s.push(']');
            } else {
                write!(s, "{inner}[]")?;
            }
        }
        DataType::Map(map) => {
            let path = map_key_path(&location);
            map_keys::validate_map_key(map.key_ty(), types, generics, format!("{path}.<map_key>"))?;
            let rendered_key =
                map_key_render_type(resolve_generics_in_datatype(map.key_ty(), generics));

            fn is_exhaustive(dt: &DataType, types: &Types) -> bool {
                match dt {
                    DataType::Enum(e) => {
                        e.variants().iter().filter(|(_, v)| !v.skip()).count() == 0
                    }
                    DataType::Reference(Reference::Named(r)) => r
                        .get(types)
                        .is_some_and(|ndt| is_exhaustive(ndt.ty(), types)),
                    DataType::Reference(Reference::Opaque(_)) => false,
                    _ => true,
                }
            }

            let exhaustive = is_exhaustive(&rendered_key, types);
            if !exhaustive {
                s.push_str("Partial<");
            }

            s.push_str("{ [key in ");
            shallow_inline_datatype(
                s,
                exporter,
                types,
                &rendered_key,
                location.clone(),
                parent_name,
                prefix,
                generics,
            )?;
            s.push_str("]: ");
            shallow_inline_datatype(
                s,
                exporter,
                types,
                map.value_ty(),
                location,
                parent_name,
                prefix,
                generics,
            )?;
            s.push_str(" }");

            if !exhaustive {
                s.push('>');
            }
        }
        DataType::Nullable(dt) => {
            let mut inner = String::new();
            shallow_inline_datatype(
                &mut inner,
                exporter,
                types,
                dt,
                location,
                parent_name,
                prefix,
                generics,
            )?;

            s.push_str(&inner);
            if inner != "null" && !inner.ends_with(" | null") {
                s.push_str(" | null");
            }
        }
        DataType::Struct(st) => {
            crate::legacy::struct_datatype(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                },
                parent_name,
                st,
                types,
                s,
                prefix,
                generics,
            )?;
        }
        DataType::Enum(enm) => {
            crate::legacy::enum_datatype(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                },
                enm,
                types,
                s,
                prefix,
                generics,
            )?;
        }
        DataType::Tuple(tuple) => match tuple.elements() {
            [] => s.push_str("null"),
            elements => {
                s.push('[');
                for (idx, dt) in elements.iter().enumerate() {
                    if idx != 0 {
                        s.push_str(", ");
                    }
                    shallow_inline_datatype(
                        s,
                        exporter,
                        types,
                        dt,
                        location.clone(),
                        parent_name,
                        prefix,
                        generics,
                    )?;
                }
                s.push(']');
            }
        },
        DataType::Reference(r) => match r {
            Reference::Named(r) => {
                let ndt = r
                    .get(types)
                    .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;
                let inline_key = (
                    ndt.module_path().clone(),
                    ndt.name().clone(),
                    r.generics().to_vec(),
                );
                let already_inlining = INLINE_REFERENCE_STACK
                    .with(|stack| stack.borrow().iter().any(|key| key == &inline_key));

                if already_inlining {
                    return reference_named_dt(s, exporter, types, r, location, prefix, generics);
                }

                INLINE_REFERENCE_STACK.with(|stack| stack.borrow_mut().push(inline_key));
                let combined_generics = merged_generics(generics, r.generics());
                let resolved = resolve_generics_in_datatype(ndt.ty(), &combined_generics);
                let result = shallow_inline_datatype(
                    s,
                    exporter,
                    types,
                    &resolved,
                    location,
                    parent_name,
                    prefix,
                    &combined_generics,
                );
                INLINE_REFERENCE_STACK.with(|stack| {
                    stack.borrow_mut().pop();
                });

                result
            }
            Reference::Generic(g) => {
                if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                    if matches!(resolved_dt, DataType::Reference(Reference::Generic(inner)) if inner == g)
                    {
                        write_generic_reference(s, g)?;
                    } else {
                        let already_resolving = RESOLVING_GENERICS
                            .with(|stack| stack.borrow().iter().any(|seen| seen == g));
                        if already_resolving {
                            write_generic_reference(s, g)?;
                        } else {
                            RESOLVING_GENERICS.with(|stack| stack.borrow_mut().push(g.clone()));
                            let result = shallow_inline_datatype(
                                s,
                                exporter,
                                types,
                                resolved_dt,
                                location,
                                parent_name,
                                prefix,
                                generics,
                            );
                            RESOLVING_GENERICS.with(|stack| {
                                stack.borrow_mut().pop();
                            });
                            result?;
                        }
                    }
                } else {
                    write_generic_reference(s, g)?;
                }
                Ok(())
            }
            Reference::Opaque(_) => reference_dt(s, exporter, types, r, location, prefix, generics),
        }?,
    }

    Ok(())
}

fn resolve_generics_in_datatype(
    dt: &DataType,
    generics: &[(GenericReference, DataType)],
) -> DataType {
    fn resolve(
        dt: &DataType,
        generics: &[(GenericReference, DataType)],
        visiting: &mut Vec<GenericReference>,
    ) -> DataType {
        match dt {
            DataType::Primitive(_) => dt.clone(),
            DataType::List(l) => {
                let mut out = l.clone();
                out.set_ty(resolve(l.ty(), generics, visiting));
                DataType::List(out)
            }
            DataType::Map(m) => {
                let mut out = m.clone();
                out.set_key_ty(resolve(m.key_ty(), generics, visiting));
                out.set_value_ty(resolve(m.value_ty(), generics, visiting));
                DataType::Map(out)
            }
            DataType::Nullable(def) => {
                DataType::Nullable(Box::new(resolve(def, generics, visiting)))
            }
            DataType::Struct(st) => {
                let mut out = st.clone();
                match out.fields_mut() {
                    specta::datatype::Fields::Unit => {}
                    specta::datatype::Fields::Unnamed(unnamed) => {
                        for field in unnamed.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve(ty, generics, visiting);
                            }
                        }
                    }
                    specta::datatype::Fields::Named(named) => {
                        for (_, field) in named.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve(ty, generics, visiting);
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
                                    *ty = resolve(ty, generics, visiting);
                                }
                            }
                        }
                        specta::datatype::Fields::Named(named) => {
                            for (_, field) in named.fields_mut() {
                                if let Some(ty) = field.ty_mut() {
                                    *ty = resolve(ty, generics, visiting);
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
                    *element = resolve(element, generics, visiting);
                }
                DataType::Tuple(out)
            }
            DataType::Reference(Reference::Generic(g)) => {
                if visiting.iter().any(|seen| seen == g) {
                    return dt.clone();
                }

                if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                    if matches!(resolved_dt, DataType::Reference(Reference::Generic(inner)) if inner == g)
                    {
                        dt.clone()
                    } else {
                        visiting.push(g.clone());
                        let out = resolve(resolved_dt, generics, visiting);
                        visiting.pop();
                        out
                    }
                } else {
                    dt.clone()
                }
            }
            DataType::Reference(_) => dt.clone(),
        }
    }

    resolve(dt, generics, &mut Vec::new())
}

// Internal function to handle inlining without cloning DataType nodes
fn inline_datatype(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    depth: usize,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    // Prevent infinite recursion
    if depth == 25 {
        return Err(Error::invalid_name(
            location.join("."),
            "Type recursion limit exceeded during inline expansion",
        ));
    }

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::List(l) => {
            // Inline the list element type
            let mut dt_str = String::new();
            crate::legacy::datatype_inner(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                },
                l.ty(),
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
        DataType::Map(m) => map_dt(s, exporter, types, m, location, generics)?,
        DataType::Nullable(def) => {
            let mut inner = String::new();
            inline_datatype(
                &mut inner,
                exporter,
                types,
                def,
                location,
                parent_name,
                prefix,
                depth + 1,
                generics,
            )?;

            s.push_str(&inner);
            if inner != "null" && !inner.ends_with(" | null") {
                s.push_str(" | null");
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
        DataType::Enum(e) => enum_dt(s, exporter, types, e, location, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, location, generics)?,
        DataType::Reference(r) => {
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
                    parent_name,
                    prefix,
                    depth + 1,
                    &combined_generics,
                )?;
            } else {
                reference_dt(s, exporter, types, r, location, prefix, generics)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn datatype(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    // TODO: Validating the variant from `dt` can be flattened

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::List(l) => list_dt(s, exporter, types, l, location, generics)?,
        DataType::Map(m) => map_dt(s, exporter, types, m, location, generics)?,
        DataType::Nullable(def) => {
            // TODO: Replace legacy stuff
            let mut inner = String::new();
            crate::legacy::datatype_inner(
                crate::legacy::ExportContext {
                    cfg: exporter,
                    path: vec![],
                },
                def,
                types,
                &mut inner,
                generics,
            )?;

            s.push_str(&inner);
            if inner != "null" && !inner.ends_with(" | null") {
                s.push_str(" | null");
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
                },
                parent_name,
                st,
                types,
                s,
                prefix,
                generics,
            )?
        }
        DataType::Enum(e) => enum_dt(s, exporter, types, e, location, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, location, generics)?,
        DataType::Reference(r) => reference_dt(s, exporter, types, r, location, prefix, generics)?,
    };

    Ok(())
}

fn primitive_dt(p: &Primitive, location: Vec<Cow<'static, str>>) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 | f16 | f32 | f64 /* this looks wrong but `f64` is the direct equivalent of `number` */ => "number",
        usize | isize | i64 | u64 | i128 | u128 | f128 => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        Primitive::bool => "boolean",
        str | char => "string",
    })
}

fn list_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    l: &List,
    _location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    // TODO: This is the legacy stuff
    {
        let mut dt = String::new();
        crate::legacy::datatype_inner(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
            },
            l.ty(),
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
    types: &Types,
    m: &Map,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let path = map_key_path(&location);
    map_keys::validate_map_key(m.key_ty(), types, generics, format!("{path}.<map_key>"))?;

    {
        fn is_exhaustive(dt: &DataType, types: &Types) -> bool {
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

        let resolved_key = map_key_render_type(resolve_generics_in_datatype(m.key_ty(), generics));
        let is_exhaustive = is_exhaustive(&resolved_key, types);

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
            },
            &resolved_key,
            types,
            s,
            generics,
        )?;
        s.push_str("]: ");
        crate::legacy::datatype_inner(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
            },
            m.value_ty(),
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

fn map_key_path(location: &[Cow<'static, str>]) -> String {
    if location.is_empty() {
        return "HashMap".to_string();
    }

    location.join(".")
}

fn map_key_render_type(dt: DataType) -> DataType {
    if matches!(dt, DataType::Primitive(Primitive::bool)) {
        return bool_key_literal_datatype();
    }

    dt
}

fn bool_key_literal_datatype() -> DataType {
    let mut bool_enum = Enum::new();
    bool_enum
        .variants_mut()
        .push((Cow::Borrowed("true"), Variant::unit()));
    bool_enum
        .variants_mut()
        .push((Cow::Borrowed("false"), Variant::unit()));
    DataType::Enum(bool_enum)
}

fn enum_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    e: &Enum,
    _location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    // TODO: Drop legacy stuff
    {
        crate::legacy::enum_datatype(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
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
//     types: &Types,
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
//     types: &Types,
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
//     types: &Types,
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
    types: &Types,
    t: &Tuple,
    _location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    {
        s.push_str(&crate::legacy::tuple_datatype(
            crate::legacy::ExportContext {
                cfg: exporter,
                path: vec![],
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
    types: &Types,
    r: &Reference,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match r {
        Reference::Named(r) => {
            reference_named_dt(s, exporter, types, r, location, prefix, generics)
        }
        Reference::Generic(g) => {
            if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
                if matches!(resolved_dt, DataType::Reference(Reference::Generic(inner)) if inner == g)
                {
                    write_generic_reference(s, g)?;
                    Ok(())
                } else {
                    datatype(
                        s,
                        exporter,
                        types,
                        resolved_dt,
                        location,
                        None,
                        prefix,
                        generics,
                    )
                }
            } else {
                write_generic_reference(s, g)?;
                Ok(())
            }
        }
        Reference::Opaque(r) => reference_opaque_dt(s, exporter, types, r),
    }
}

fn reference_opaque_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
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
        let resolved_types = ResolvedTypes::from_resolved_types(types.clone());

        if let Some(branded_type) = exporter
            .branded_type_impl
            .as_ref()
            .map(|builder| {
                (builder.0)(
                    BrandedTypeExporter {
                        exporter,
                        types: &resolved_types,
                    },
                    def,
                )
            })
            .transpose()?
        {
            s.push_str(branded_type.as_ref());
            return Ok(());
        }

        // TODO: Build onto `s` instead of appending a separate string
        match def.ty() {
            DataType::Reference(r) => reference_dt(s, exporter, types, r, vec![], "", &[])?,
            ty => inline_datatype(s, exporter, types, ty, vec![], None, "", 0, &[])?,
        }
        s.push_str(r#" & { { readonly __brand: ""#);
        s.push_str(def.brand());
        s.push_str("\" }");
        return Ok(());
    }

    Err(Error::unsupported_opaque_reference(r.clone()))
}

fn reference_named_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    r: &NamedReference,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    // TODO: Legacy stuff
    {
        let ndt = r
            .get(types)
            .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;
        let _generic_scope = push_generic_scope(ndt.generics());

        // Check if this reference should be inlined
        if r.inline() {
            let inline_key = (
                ndt.module_path().clone(),
                ndt.name().clone(),
                r.generics().to_vec(),
            );
            let already_inlining = INLINE_REFERENCE_STACK
                .with(|stack| stack.borrow().iter().any(|key| key == &inline_key));

            if already_inlining {
                // Fall through and emit a named reference to break recursive inline expansions.
            } else {
                INLINE_REFERENCE_STACK.with(|stack| stack.borrow_mut().push(inline_key));
                let combined_generics = merged_generics(generics, r.generics());
                let resolved = resolve_generics_in_datatype(ndt.ty(), &combined_generics);
                let result = datatype(
                    s,
                    exporter,
                    types,
                    &resolved,
                    location,
                    None,
                    prefix,
                    &combined_generics,
                );
                INLINE_REFERENCE_STACK.with(|stack| {
                    stack.borrow_mut().pop();
                });
                return result;
            }
        }

        // We check it's valid before tracking
        crate::references::track_nr(r);

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
                let current_module_path =
                    crate::references::current_module_path().unwrap_or_default();

                if ndt.module_path() == &current_module_path {
                    ndt.name().clone()
                } else {
                    let mut path = crate::exporter::module_alias(ndt.module_path());
                    path.push('.');
                    path.push_str(ndt.name());
                    Cow::Owned(path)
                }
            }
            _ => ndt.name().clone(),
        };

        let scoped_generics = generics
            .iter()
            .filter(|(parent_generic, _)| {
                !r.generics()
                    .iter()
                    .any(|(child_generic, _)| child_generic == parent_generic)
            })
            .cloned()
            .collect::<Vec<_>>();

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
                    },
                    v,
                    types,
                    s,
                    &scoped_generics,
                )?;
            }

            s.push('>');
        }
    }

    //     let ndt = types
    //         .get(r.sid())
    //         // Should be impossible without a bug in Specta.
    //         .unwrap_or_else(|| panic!("Missing {:?} in `Types`", r.sid()));

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
