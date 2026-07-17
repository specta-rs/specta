//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are for advanced usecases, you should generally use [crate::Typescript] or
//! [crate::JSDoc] in end-user applications.

use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap, HashSet, hash_map::Entry},
};

use specta::{
    Format, Types,
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, GenericDefinition, GenericReference, List, Map,
        NamedDataType, NamedReference, NamedReferenceType, OpaqueReference, Primitive, Reference,
        Struct, Tuple, Variant,
    },
};

use crate::{
    Branded, BrandedTypeExporter, Error, Exporter, Layout, map_keys, opaque,
    reserved_names::RESERVED_TYPE_NAMES,
};

const STRING: &str = "string";
const NULL: &str = "null";
const NEVER: &str = "never";
const FIELD_ALIAS_UNION_MARKER: &str = "specta_serde:deferred_alias_union";
const FIELD_ALIAS_EXCLUSION_MARKER: &str = "specta_serde:alias_exclusion";

fn path_string(location: &[Cow<'static, str>]) -> String {
    location.join(".")
}

fn rust_type_path(ndt: &NamedDataType) -> Cow<'static, str> {
    if ndt.module_path.is_empty() {
        ndt.name.clone()
    } else {
        Cow::Owned(format!("{}::{}", ndt.module_path, ndt.name))
    }
}

fn module_prefixed_type_name(ndt: &NamedDataType) -> String {
    let mut name = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
    if !name.is_empty() {
        name.push('_');
    }
    name.push_str(&ndt.name);
    name
}

fn exported_type_name<'a>(exporter: &Exporter, ndt: &'a NamedDataType) -> Cow<'a, str> {
    match exporter.layout {
        Layout::ModulePrefixedName => Cow::Owned(module_prefixed_type_name(ndt)),
        _ => ndt.name.clone(),
    }
}

fn referenced_type_name<'a>(exporter: &Exporter, ndt: &'a NamedDataType) -> Cow<'a, str> {
    match exporter.layout {
        Layout::ModulePrefixedName => Cow::Owned(module_prefixed_type_name(ndt)),
        Layout::Namespaces => {
            if ndt.module_path.is_empty() {
                ndt.name.clone()
            } else {
                let mut path =
                    ndt.module_path
                        .split("::")
                        .fold("$s$.".to_string(), |mut s, segment| {
                            s.push_str(segment);
                            s.push('.');
                            s
                        });
                path.push_str(&ndt.name);
                Cow::Owned(path)
            }
        }
        Layout::Files => {
            let current_module_path = crate::references::current_module_path().unwrap_or_default();

            if ndt.module_path == current_module_path {
                ndt.name.clone()
            } else {
                let mut path = crate::exporter::module_alias(&ndt.module_path);
                path.push('.');
                path.push_str(&ndt.name);
                Cow::Owned(path)
            }
        }
        _ => ndt.name.clone(),
    }
}

fn inner_comments(
    deprecated: Option<&Deprecated>,
    docs: &str,
    other: String,
    start_with_newline: bool,
    prefix: &str,
) -> String {
    let mut comments = String::new();
    js_doc(&mut comments, docs, deprecated);
    if comments.is_empty() {
        return other;
    }

    let mut out = String::new();
    if start_with_newline {
        out.push('\n');
    }

    for line in comments.lines() {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }

    out.push_str(&other);
    out
}

pub(crate) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

pub(crate) fn escape_typescript_string_literal(value: &str) -> Cow<'_, str> {
    if !value.chars().any(|ch| {
        ch == '"' || ch == '\\' || ch == '\u{2028}' || ch == '\u{2029}' || ch.is_control()
    }) {
        return Cow::Borrowed(value);
    }

    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str(r#"\""#),
            '\\' => escaped.push_str(r#"\\"#),
            '\n' => escaped.push_str(r#"\n"#),
            '\r' => escaped.push_str(r#"\r"#),
            '\t' => escaped.push_str(r#"\t"#),
            '\u{2028}' => escaped.push_str(r#"\u2028"#),
            '\u{2029}' => escaped.push_str(r#"\u2029"#),
            ch if ch.is_control() => push_unicode_escape(&mut escaped, ch),
            _ => escaped.push(ch),
        }
    }

    Cow::Owned(escaped)
}

fn push_unicode_escape(s: &mut String, ch: char) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let value = ch as u32;

    s.push_str(r#"\u"#);
    s.push(HEX[((value >> 12) & 0xF) as usize] as char);
    s.push(HEX[((value >> 8) & 0xF) as usize] as char);
    s.push(HEX[((value >> 4) & 0xF) as usize] as char);
    s.push(HEX[(value & 0xF) as usize] as char);
}

fn sanitise_key<'a>(field_name: Cow<'static, str>, force_string: bool) -> Cow<'a, str> {
    if force_string || !is_identifier(&field_name) {
        format!(r#""{}""#, escape_typescript_string_literal(&field_name)).into()
    } else {
        field_name
    }
}

fn sanitise_type_name(location: &[Cow<'static, str>], ident: &str) -> Result<String, Error> {
    let path = path_string(location);

    if ident.is_empty() {
        return Err(Error::empty_name(path));
    }

    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(Error::forbidden_name(path, name));
    }

    if let Some(first_char) = ident.chars().next()
        && !first_char.is_alphabetic()
        && first_char != '_'
    {
        return Err(Error::invalid_name(path, ident.to_string()));
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        return Err(Error::invalid_name(path, ident.to_string()));
    }

    Ok(ident.to_string())
}

pub(crate) fn js_doc(s: &mut String, docs: &str, deprecated: Option<&Deprecated>) {
    if docs.is_empty() && deprecated.is_none() {
        return;
    }

    if deprecated.is_none() {
        let mut lines = docs.lines();
        if let (Some(line), None) = (lines.next(), lines.next()) {
            s.push_str("/** ");
            s.push_str(&escape_jsdoc_text(line));
            s.push_str(" */\n");
            return;
        }
    }

    s.push_str("/**\n");
    if !docs.is_empty() {
        for line in docs.lines() {
            s.push_str(" * ");
            s.push_str(&escape_jsdoc_text(line));
            s.push('\n');
        }
    }

    if let Some(typ) = deprecated {
        s.push_str(" * @deprecated");
        if let Some(details) = deprecated_details(typ) {
            s.push(' ');
            s.push_str(&details);
        }
        s.push('\n');
    }

    s.push_str(" */\n");
}

pub(crate) fn escape_jsdoc_text(text: &str) -> Cow<'_, str> {
    if text.contains("*/") {
        Cow::Owned(text.replace("*/", "*\\/"))
    } else {
        Cow::Borrowed(text)
    }
}

pub(crate) fn deprecated_details(typ: &Deprecated) -> Option<String> {
    typ.note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(str::to_string)
}

/// Generate a group of `export Type = ...` Typescript string for a specific [`NamedDataType`].
///
/// This method leaves the following up to the implementer:
///  - Ensuring all referenced types are exported
///  - Handling multiple type with overlapping names
///  - Transforming the type for your serialization format (Eg. Serde)
///
/// We recommend passing in your types in bulk instead of doing individual calls as it leaves formatting to us and also allows us to merge the JSDoc types into a single large comment.
///
/// If you are using a custom format such as `serde::format` with the high-level exporter,
/// these primitive helpers do not apply that mapping automatically. Standalone primitive usage
/// should map both the full [`Types`] graph and any top-level [`DataType`] values with matching
/// helpers first.
///
pub fn export<'a>(
    exporter: &dyn AsRef<Exporter>,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut s = String::new();
    export_internal(&mut s, exporter.as_ref(), None, types, ndts, indent)?;
    Ok(s)
}

pub(crate) fn export_internal<'a>(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<(), Error> {
    let ndts = ndts.filter(|ndt| ndt.ty.is_some());

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

            append_typedef_body(s, exporter, format, types, ndt, indent)?;
        }

        s.push_str(indent);
        s.push_str("\t*/\n");
        return Ok(());
    }

    for (index, ndt) in ndts.enumerate() {
        if index != 0 {
            s.push('\n');
        }

        export_single_internal(s, exporter, format, types, ndt, indent)?;
    }

    Ok(())
}

fn export_single_internal(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    ndt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    if exporter.jsdoc {
        let mut typedef = String::new();
        typedef_internal(&mut typedef, exporter, format, types, ndt)?;
        for line in typedef.lines() {
            s.push_str(indent);
            s.push_str(line);
            s.push('\n');
        }
        return Ok(());
    }

    let raw_name = exported_type_name(exporter, ndt);
    let name = sanitise_type_name(&[rust_type_path(ndt)], &raw_name)
        .map_err(|err| err.with_named_datatype(ndt))?;

    let mut comments = String::new();
    js_doc(&mut comments, &ndt.docs, ndt.deprecated.as_ref());
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
    write_generic_parameters(s, exporter, types, &[rust_type_path(ndt)], &ndt.generics)?;
    s.push_str(" = ");

    let body = resolve_named_export_body(types, ndt)?;
    datatype(
        s,
        exporter,
        format,
        types,
        &body,
        vec![rust_type_path(ndt)],
        Some(ndt.name.as_ref()),
        indent,
        Default::default(),
    )
    .map_err(|err| err.with_named_datatype(ndt))?;
    s.push_str(";\n");

    Ok(())
}

/// Generate an anonymous Typescript string for a specific [`DataType`].
///
/// This method leaves all the same things as the [`export`] method up to the user.
///
/// Note that calling this method with a tagged struct or enum may cause the tag to not be exported.
/// The type should be wrapped in a [`NamedDataType`] to provide a proper name.
///
/// You are responsible for apply Serde or other format mapping to the top-level datatype in the
/// same way as the [`Types`] graph before calling this helper.
///
pub fn inline(
    exporter: &dyn AsRef<Exporter>,
    types: &Types,
    dt: &DataType,
) -> Result<String, Error> {
    let mut s = String::new();
    inline_datatype(
        &mut s,
        exporter.as_ref(),
        None,
        types,
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
    format: Option<&dyn Format>,
    types: &Types,
    dt: &NamedDataType,
) -> Result<(), Error> {
    s.push_str("/**\n");
    append_typedef_body(s, exporter, format, types, dt, "")?;

    s.push_str("\t*/");

    Ok(())
}

fn append_jsdoc_properties(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    dt_name: &str,
    dt: &DataType,
    indent: &str,
) -> Result<(), Error> {
    match dt {
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit => {}
            Fields::Unnamed(unnamed) => {
                for (idx, field) in unnamed.fields.iter().enumerate() {
                    let Some(ty) = field.ty.as_ref() else {
                        continue;
                    };

                    let mut ty_str = String::new();
                    let datatype_prefix = format!("{indent}\t*\t");
                    datatype(
                        &mut ty_str,
                        exporter,
                        format,
                        types,
                        ty,
                        vec![Cow::Owned(dt_name.to_owned()), idx.to_string().into()],
                        Some(dt_name),
                        &datatype_prefix,
                        Default::default(),
                    )?;

                    push_jsdoc_property(
                        s,
                        &ty_str,
                        &idx.to_string(),
                        field.optional,
                        &field.docs,
                        field.deprecated.as_ref(),
                        indent,
                    );
                }
            }
            Fields::Named(named) => {
                for (name, field) in &named.fields {
                    let Some(ty) = field.ty.as_ref() else {
                        continue;
                    };

                    let mut ty_str = String::new();
                    let datatype_prefix = format!("{indent}\t*\t");
                    datatype(
                        &mut ty_str,
                        exporter,
                        format,
                        types,
                        ty,
                        vec![Cow::Owned(dt_name.to_owned()), name.clone()],
                        Some(dt_name),
                        &datatype_prefix,
                        Default::default(),
                    )?;

                    push_jsdoc_property(
                        s,
                        &ty_str,
                        name,
                        field.optional,
                        &field.docs,
                        field.deprecated.as_ref(),
                        indent,
                    );
                }
            }
        },
        DataType::Enum(enm) => {
            for (variant_name, variant) in enm.variants.iter().filter(|(_, v)| !v.skip) {
                let mut one_variant_enum = enm.clone();
                one_variant_enum
                    .variants
                    .retain(|(name, _)| name == variant_name);

                let mut variant_ty = String::new();
                enum_dt(
                    &mut variant_ty,
                    exporter,
                    types,
                    &one_variant_enum,
                    vec![Cow::Owned(dt_name.to_owned())],
                    "",
                    &[],
                )?;

                push_jsdoc_property(
                    s,
                    &variant_ty,
                    variant_name,
                    false,
                    &variant.docs,
                    variant.deprecated.as_ref(),
                    indent,
                );
            }
        }
        DataType::Intersection(types_) => {
            for ty in types_ {
                append_jsdoc_properties(s, exporter, format, types, dt_name, ty, indent)?;
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
    format: Option<&dyn Format>,
    types: &Types,
    dt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    let name = &dt.name;
    let mut type_name = String::from(name.as_ref());
    write_generic_parameters(
        &mut type_name,
        exporter,
        types,
        &[rust_type_path(dt)],
        &dt.generics,
    )?;

    let mut typedef_ty = String::new();
    let datatype_prefix = format!("{indent}\t*\t");
    let body = resolve_named_export_body(types, dt)?;
    datatype(
        &mut typedef_ty,
        exporter,
        format,
        types,
        &body,
        vec![rust_type_path(dt)],
        Some(dt.name.as_ref()),
        &datatype_prefix,
        Default::default(),
    )
    .map_err(|err| err.with_named_datatype(dt))?;

    if !dt.docs.is_empty() {
        for line in dt.docs.lines() {
            s.push_str(indent);
            s.push_str("\t* ");
            s.push_str(&escape_jsdoc_text(line));
            s.push('\n');
        }
        s.push_str(indent);
        s.push_str("\t*\n");
    }

    if let Some(deprecated) = dt.deprecated.as_ref() {
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

    if let Some(ty) = &dt.ty {
        let dt_path = rust_type_path(dt);
        append_jsdoc_properties(s, exporter, format, types, dt_path.as_ref(), ty, indent)?;
    }

    Ok(())
}

fn write_generic_parameters(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    parent_location: &[Cow<'static, str>],
    generics: &[GenericDefinition],
) -> Result<(), Error> {
    if generics.is_empty() {
        return Ok(());
    }

    s.push('<');
    for (index, generic) in generics.iter().enumerate() {
        if index != 0 {
            s.push_str(", ");
        }

        s.push_str(generic.name.as_ref());

        if let Some(default) = &generic.default {
            let mut rendered_default = String::new();
            let mut default_location = parent_location.to_vec();
            default_location.push(format!("<generic {} default>", generic.name).into());
            shallow_inline_datatype(
                &mut rendered_default,
                exporter,
                None,
                types,
                default,
                default_location,
                None,
                "",
                Default::default(),
            )?;
            s.push_str(" = ");
            s.push_str(&rendered_default);
        }
    }
    s.push('>');

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

/// Generate a Typescript string for a specific [`DataType`] while preserving ordinary named
/// references recursively.
///
/// Unlike [`inline`], named types are rendered as references even when nested within an anonymous
/// composite type. References explicitly marked as inline are still expanded.
///
/// See [`export`] for the list of things to consider when using this.
pub fn reference(
    exporter: &dyn AsRef<Exporter>,
    types: &Types,
    dt: &DataType,
) -> Result<String, Error> {
    let mut s = String::new();
    datatype(
        &mut s,
        exporter.as_ref(),
        None,
        types,
        dt,
        vec![],
        None,
        "",
        &[],
    )?;
    Ok(s)
}

pub(crate) fn datatype_with_inline_attr(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    shallow_inline: bool,
) -> Result<(), Error> {
    if shallow_inline {
        let inline_path = path_string(&location);
        return shallow_inline_datatype(
            s,
            exporter,
            format,
            types,
            dt,
            location,
            parent_name,
            prefix,
            generics,
        )
        .map_err(|err| err.with_inline_trace(inline_named_datatype(types, dt), inline_path));
    }

    datatype(
        s,
        exporter,
        format,
        types,
        dt,
        location,
        parent_name,
        prefix,
        generics,
    )
}

fn write_generic_reference(s: &mut String, generic: &GenericReference) {
    s.push_str(generic.name());
}

fn scoped_reference_generics(
    parent_generics: &[(GenericReference, DataType)],
    reference_generics: &[(GenericReference, DataType)],
) -> Vec<(GenericReference, DataType)> {
    parent_generics
        .iter()
        .filter(|(parent_generic, _)| {
            !reference_generics
                .iter()
                .any(|(child_generic, _)| child_generic == parent_generic)
        })
        .cloned()
        .collect()
}

fn named_reference_generics(r: &NamedReference) -> Result<&[(GenericReference, DataType)], Error> {
    match &r.inner {
        NamedReferenceType::Reference { generics, .. } => Ok(generics),
        NamedReferenceType::Inline { .. } => Ok(&[]),
        NamedReferenceType::Recursive(_) => Ok(&[]),
    }
}

fn named_reference_ty<'a>(
    types: &'a Types,
    r: &'a NamedReference,
    location: &[Cow<'static, str>],
) -> Result<&'a DataType, Error> {
    let path = path_string(location);
    match &r.inner {
        NamedReferenceType::Reference { .. } => types
            .get(r)
            .and_then(|ndt| ndt.ty.as_ref())
            .ok_or_else(|| Error::dangling_named_reference(path, format!("{r:?}"))),
        NamedReferenceType::Inline { dt, .. } => Ok(dt),
        NamedReferenceType::Recursive(cycle) => Err(Error::infinite_recursive_inline_type(
            path,
            format!("{r:?}"),
            cycle.clone(),
        )),
    }
}

fn inline_named_datatype<'a>(types: &'a Types, dt: &DataType) -> Option<&'a NamedDataType> {
    match dt {
        DataType::Reference(Reference::Named(r)) => types.get(r),
        _ => None,
    }
}

fn resolve_scoped_generic_default(
    default: &DataType,
    scoped_generics: &[(GenericReference, DataType)],
) -> DataType {
    match default {
        DataType::Generic(default) => scoped_generics
            .iter()
            .find_map(|(reference, dt)| (reference == default).then_some(dt.clone()))
            .unwrap_or_else(|| DataType::Generic(default.clone())),
        default => default.clone(),
    }
}

fn resolved_reference_generics(
    ndt: &specta::datatype::NamedDataType,
    r: &NamedReference,
    parent_generics: &[(GenericReference, DataType)],
) -> Option<(Vec<DataType>, bool, Vec<(GenericReference, DataType)>)> {
    let reference_generics = named_reference_generics(r).ok()?;
    let mut scoped_generics = scoped_reference_generics(parent_generics, reference_generics);
    let mut all_default = true;
    let mut rendered_generics = Vec::with_capacity(ndt.generics.len());

    for generic in ndt.generics.iter() {
        let explicit = reference_generics
            .iter()
            .find(|(reference, _)| *reference == generic.reference())
            .map(|(_, dt)| dt.clone());

        let resolved_default = generic
            .default
            .as_ref()
            .map(|default| resolve_scoped_generic_default(default, &scoped_generics));

        let resolved = explicit.or_else(|| resolved_default.clone()).or_else(|| {
            Some(DataType::Reference(Reference::opaque(
                crate::opaque::Unknown,
            )))
        });

        let resolved = resolved?;
        all_default &= resolved_default
            .as_ref()
            .is_some_and(|default| default == &resolved);
        scoped_generics.push((generic.reference(), resolved.clone()));
        rendered_generics.push(resolved);
    }

    Some((rendered_generics, all_default, scoped_generics))
}

#[derive(Clone, Copy)]
struct RenderCtx<'a> {
    exporter: &'a Exporter,
    format: Option<&'a dyn Format>,
    types: &'a Types,
    parent_name: Option<&'a str>,
    prefix: &'a str,
    generics: &'a [(GenericReference, DataType)],
}

#[derive(Clone, Copy)]
enum RenderMode {
    Normal,
    ShallowInline,
}

impl RenderMode {
    fn render(
        self,
        s: &mut String,
        ctx: RenderCtx<'_>,
        dt: &DataType,
        location: Vec<Cow<'static, str>>,
    ) -> Result<(), Error> {
        match self {
            Self::Normal => datatype(
                s,
                ctx.exporter,
                ctx.format,
                ctx.types,
                dt,
                location,
                ctx.parent_name,
                ctx.prefix,
                ctx.generics,
            ),
            Self::ShallowInline => shallow_inline_datatype(
                s,
                ctx.exporter,
                ctx.format,
                ctx.types,
                dt,
                location,
                ctx.parent_name,
                ctx.prefix,
                ctx.generics,
            ),
        }
    }

    fn render_intersection_part(
        self,
        s: &mut String,
        ctx: RenderCtx<'_>,
        dt: &DataType,
        location: Vec<Cow<'static, str>>,
    ) -> Result<(), Error> {
        match (self, dt) {
            (Self::ShallowInline, DataType::Reference(r)) => reference_dt(
                s,
                ctx.exporter,
                ctx.format,
                ctx.types,
                r,
                location,
                ctx.prefix,
                ctx.generics,
            ),
            _ => self.render(s, ctx, dt, location),
        }
    }
}

fn render_datatype(
    s: &mut String,
    ctx: RenderCtx<'_>,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    mode: RenderMode,
) -> Result<(), Error> {
    match (mode, dt) {
        (_, DataType::Primitive(p)) => s.push_str(primitive_dt(p, location)?),
        (_, DataType::Generic(g)) => write_generic_reference(s, g),
        (RenderMode::Normal, DataType::List(list)) => {
            list_dt(s, ctx.exporter, ctx.types, list, location, ctx.generics)?;
        }
        (RenderMode::ShallowInline, DataType::List(list)) => {
            let mut inner = String::new();
            render_datatype(&mut inner, ctx, &list.ty, location, mode)?;
            push_list(s, &inner, list.length);
        }
        (RenderMode::Normal, DataType::Map(map)) => {
            map_dt(
                s,
                ctx.exporter,
                ctx.format,
                ctx.types,
                map,
                location,
                ctx.generics,
            )?;
        }
        (RenderMode::ShallowInline, DataType::Map(map)) => render_map(
            s,
            ctx.exporter,
            ctx.format,
            ctx.types,
            map,
            location,
            ctx.parent_name,
            ctx.prefix,
            ctx.generics,
            mode,
        )?,
        (_, DataType::Nullable(inner)) => {
            let mut rendered = String::new();
            let child_ctx = RenderCtx { prefix: "", ..ctx };
            render_datatype(&mut rendered, child_ctx, inner, location, mode)?;
            push_nullable(s, &rendered);
        }
        (_, DataType::Struct(st)) => struct_dt(
            s,
            ctx.exporter,
            ctx.format,
            ctx.types,
            st,
            location,
            ctx.parent_name,
            ctx.prefix,
            ctx.generics,
        )?,
        (_, DataType::Enum(enm)) => enum_dt(
            s,
            ctx.exporter,
            ctx.types,
            enm,
            location,
            ctx.prefix,
            ctx.generics,
        )?,
        (RenderMode::Normal, DataType::Tuple(tuple)) => {
            tuple_dt(s, ctx.exporter, ctx.types, tuple, location, ctx.generics)?;
        }
        (RenderMode::ShallowInline, DataType::Tuple(tuple)) => match tuple.elements.as_slice() {
            [] => s.push_str(NULL),
            elements => {
                s.push('[');
                for (idx, dt) in elements.iter().enumerate() {
                    if idx != 0 {
                        s.push_str(", ");
                    }
                    render_datatype(s, ctx, dt, location.clone(), mode)?;
                }
                s.push(']');
            }
        },
        (RenderMode::Normal, DataType::Intersection(parts)) => {
            for (idx, ty) in parts.iter().enumerate() {
                if idx != 0 {
                    s.push_str(" & ");
                }

                let needs_parentheses = parts.len() > 1 && intersection_part_is_union(ty);
                if needs_parentheses {
                    s.push('(');
                }
                render_datatype(s, ctx, ty, location.clone(), mode)?;
                if needs_parentheses {
                    s.push(')');
                }
            }
        }
        (RenderMode::ShallowInline, DataType::Intersection(parts)) => intersection_dt(
            s,
            ctx.exporter,
            ctx.format,
            ctx.types,
            parts,
            location,
            ctx.parent_name,
            ctx.prefix,
            ctx.generics,
            mode,
        )?,
        (RenderMode::Normal, DataType::Reference(r)) => reference_dt(
            s,
            ctx.exporter,
            ctx.format,
            ctx.types,
            r,
            location,
            ctx.prefix,
            ctx.generics,
        )?,
        (RenderMode::ShallowInline, DataType::Reference(r)) => match r {
            Reference::Named(r) => {
                let ty = named_reference_ty(ctx.types, r, &location)?;
                let reference_generics = named_reference_generics(r)?;
                let child_ctx = RenderCtx {
                    generics: reference_generics,
                    ..ctx
                };
                let inline_path = path_string(&location);
                render_datatype(s, child_ctx, ty, location, mode)
                    .map_err(|err| err.with_inline_trace(ctx.types.get(r), inline_path))?;
            }
            Reference::Opaque(_) => reference_dt(
                s,
                ctx.exporter,
                ctx.format,
                ctx.types,
                r,
                location,
                ctx.prefix,
                ctx.generics,
            )?,
        },
    }

    Ok(())
}

fn needs_array_parens(ty: &str) -> bool {
    ty.contains(' ') && (!ty.ends_with('}') || ty.contains('&') || ty.contains('|'))
}

fn push_list(s: &mut String, ty: &str, length: Option<usize>) {
    let ty = if needs_array_parens(ty) {
        Cow::Owned(format!("({ty})"))
    } else {
        Cow::Borrowed(ty)
    };

    if let Some(length) = length {
        s.push('[');
        for i in 0..length {
            if i != 0 {
                s.push_str(", ");
            }
            s.push_str(&ty);
        }
        s.push(']');
    } else {
        s.push_str(&ty);
        s.push_str("[]");
    }
}

fn intersection_part_is_union(ty: &DataType) -> bool {
    match ty {
        DataType::Nullable(_) => true,
        DataType::Enum(enm) => enm.variants.len() > 1,
        DataType::Intersection(parts) if parts.len() == 1 => intersection_part_is_union(&parts[0]),
        _ => false,
    }
}

fn push_nullable(s: &mut String, inner: &str) {
    s.push_str(inner);
    if inner != NULL && !inner.ends_with(" | null") {
        s.push_str(" | null");
    }
}

fn is_exhaustive_map_key(
    dt: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
) -> bool {
    match dt {
        DataType::Enum(e) => e.variants.iter().filter(|(_, v)| !v.skip).count() == 0,
        DataType::Reference(Reference::Named(r)) => named_reference_ty(types, r, &[])
            .and_then(|ty| {
                named_reference_generics(r).map(|reference_generics| {
                    let substitution = generics
                        .iter()
                        .map(|(generic, dt)| (generic.name().clone(), dt.clone()))
                        .collect::<GenericSubst>();
                    let reference_generics = reference_generics
                        .iter()
                        .map(|(generic, dt)| {
                            let mut dt = dt.clone();
                            let _ = substitute_generics(&mut dt, &substitution);
                            (generic.clone(), dt)
                        })
                        .collect::<Vec<_>>();
                    is_exhaustive_map_key(ty, types, &reference_generics)
                })
            })
            .unwrap_or(false),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unnamed(fields) => {
                let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                fields
                    .next()
                    .is_none_or(|dt| is_exhaustive_map_key(dt, types, generics))
                    && fields.next().is_none()
            }
            _ => true,
        },
        DataType::Reference(Reference::Opaque(_)) => false,
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .is_none_or(|(_, dt)| {
                matches!(dt, DataType::Generic(candidate) if candidate == generic)
                    || is_exhaustive_map_key(dt, types, generics)
            }),
        _ => true,
    }
}

fn render_map(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    map: &Map,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    value_mode: RenderMode,
) -> Result<(), Error> {
    let path = map_key_path(&location);
    map_keys::validate_map_key(map.key_ty(), types, format!("{path}.<map_key>"))?;

    let rendered_key = map_key_render_type(map.key_ty().clone());
    let exhaustive = is_exhaustive_map_key(&rendered_key, types, generics);

    // Use `{ [key in K]: V }` instead of `Record<K, V>` to avoid circular reference issues.
    if !exhaustive {
        s.push_str("Partial<");
    }

    s.push_str("{ [key in ");
    map_key_datatype(
        s,
        exporter,
        format,
        types,
        &rendered_key,
        location.clone(),
        parent_name,
        prefix,
        generics,
    )?;
    s.push_str("]: ");
    value_mode.render(
        s,
        RenderCtx {
            exporter,
            format,
            types,
            parent_name,
            prefix,
            generics,
        },
        map.value_ty(),
        location,
    )?;
    s.push_str(" }");

    if !exhaustive {
        s.push('>');
    }

    Ok(())
}

fn shallow_inline_datatype(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    render_datatype(
        s,
        RenderCtx {
            exporter,
            format,
            types,
            parent_name,
            prefix,
            generics,
        },
        dt,
        location,
        RenderMode::ShallowInline,
    )
}

fn intersection_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    parts: &[DataType],
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    mode: RenderMode,
) -> Result<(), Error> {
    let mut rendered = Vec::with_capacity(parts.len());
    for part in parts {
        let mut out = String::new();
        mode.render_intersection_part(
            &mut out,
            RenderCtx {
                exporter,
                format,
                types,
                parent_name,
                prefix,
                generics,
            },
            part,
            location.clone(),
        )?;
        rendered.push(format!("({out})"));
    }

    s.push_str(&rendered.join(" & "));
    Ok(())
}

// Render an anonymous type while expanding core-provided inline references.
fn inline_datatype(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
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
        return Err(Error::inline_recursion_limit_exceeded(path_string(
            &location,
        )));
    }

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::Generic(g) => write_generic_reference(s, g),
        DataType::List(l) => {
            let mut dt_str = String::new();
            datatype(
                &mut dt_str,
                exporter,
                format,
                types,
                &l.ty,
                location.clone(),
                parent_name,
                prefix,
                generics,
            )?;
            push_list(s, &dt_str, l.length);
        }
        DataType::Map(m) => map_dt(s, exporter, format, types, m, location, generics)?,
        DataType::Nullable(def) => {
            let mut inner = String::new();
            inline_datatype(
                &mut inner,
                exporter,
                format,
                types,
                def,
                location,
                parent_name,
                "",
                depth + 1,
                generics,
            )?;

            push_nullable(s, &inner);
        }
        DataType::Struct(st) => {
            if !generics.is_empty() {
                inline_struct_with_generics(
                    s,
                    exporter,
                    format,
                    types,
                    st,
                    location,
                    parent_name,
                    prefix,
                    depth,
                    generics,
                )?;
            } else {
                struct_dt(
                    s,
                    exporter,
                    format,
                    types,
                    st,
                    location,
                    parent_name,
                    prefix,
                    generics,
                )?;
            }
        }
        DataType::Enum(e) => enum_dt(s, exporter, types, e, location, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, location, generics)?,
        DataType::Intersection(types_) => intersection_dt(
            s,
            exporter,
            format,
            types,
            types_,
            location,
            parent_name,
            prefix,
            generics,
            RenderMode::Normal,
        )?,
        DataType::Reference(r) => {
            if let Reference::Named(r) = r
                && let Ok(ty) = named_reference_ty(types, r, &location)
            {
                let reference_generics = named_reference_generics(r)?;
                let inline_path = path_string(&location);
                inline_datatype(
                    s,
                    exporter,
                    format,
                    types,
                    ty,
                    location,
                    parent_name,
                    prefix,
                    depth + 1,
                    reference_generics,
                )
                .map_err(|err| err.with_inline_trace(types.get(r), inline_path))?;
            } else {
                reference_dt(s, exporter, format, types, r, location, prefix, generics)?;
            }
        }
    }

    Ok(())
}

fn inline_struct_with_generics(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    st: &Struct,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    depth: usize,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(_) => struct_dt(
            s,
            exporter,
            format,
            types,
            st,
            location,
            parent_name,
            prefix,
            generics,
        )?,
        Fields::Named(named) => {
            s.push('{');
            let mut has_field = false;

            for (key, field) in &named.fields {
                let Some(field_ty) = field.ty.as_ref() else {
                    continue;
                };

                has_field = true;
                s.push('\n');
                s.push_str(prefix);
                s.push('\t');
                s.push_str(&sanitise_key(key.clone(), false));
                if field.optional {
                    s.push('?');
                }
                s.push_str(": ");
                inline_datatype(
                    s,
                    exporter,
                    format,
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
    }

    Ok(())
}

pub(crate) fn datatype(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    render_datatype(
        s,
        RenderCtx {
            exporter,
            format,
            types,
            parent_name,
            prefix,
            generics,
        },
        dt,
        location,
        RenderMode::Normal,
    )
}

fn primitive_dt(p: &Primitive, location: Vec<Cow<'static, str>>) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 => "number",
        // `null` comes from `NaN`, `Infinity` and `-Infinity`. Is done by JS APIs and Serde JSON.
        f16 | f32 | f64 /* this looks wrong but `f64` is the direct equivalent of `number` */ => "number | null",
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
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let mut dt = String::new();
    datatype(
        &mut dt, exporter, None, types, &l.ty, location, None, "", generics,
    )?;
    push_list(s, &dt, l.length);

    Ok(())
}

fn map_key_datatype(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    key_ty: &DataType,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match key_ty {
        DataType::Reference(r) => {
            reference_dt(s, exporter, format, types, r, location, prefix, generics)
        }
        key_ty => shallow_inline_datatype(
            s,
            exporter,
            format,
            types,
            key_ty,
            location,
            parent_name,
            prefix,
            generics,
        ),
    }
}

fn map_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    m: &Map,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    render_map(
        s,
        exporter,
        format,
        types,
        m,
        location,
        None,
        "",
        generics,
        RenderMode::Normal,
    )
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
    let mut bool_enum = Enum::default();
    bool_enum
        .variants
        .push((Cow::Borrowed("true"), Variant::unit()));
    bool_enum
        .variants
        .push((Cow::Borrowed("false"), Variant::unit()));
    DataType::Enum(bool_enum)
}

fn unnamed_fields_datatype(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    fields: &[(&Field, &DataType)],
    declared_len: usize,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    force_inline: bool,
) -> Result<(), Error> {
    match fields {
        // Only a genuine newtype (declared arity 1) renders as its bare
        // inner type. A declared-multi-field tuple reduced to one live
        // element by skips stays a sequence on the wire, so it keeps the
        // array form below.
        [(field, ty)] if declared_len == 1 => {
            let mut v = String::new();
            datatype_with_inline_attr(
                &mut v,
                exporter,
                format,
                types,
                ty,
                location,
                parent_name,
                "",
                generics,
                force_inline,
            )?;
            s.push_str(&inner_comments(
                field.deprecated.as_ref(),
                &field.docs,
                v,
                true,
                prefix,
            ));
        }
        fields => {
            let optional_from = trailing_optional_run(fields.iter().map(|(field, _)| *field));

            s.push('[');
            for (i, (field, ty)) in fields.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }

                let mut v = String::new();
                let mut field_location = location.clone();
                field_location.push(i.to_string().into());
                datatype_with_inline_attr(
                    &mut v,
                    exporter,
                    format,
                    types,
                    ty,
                    field_location,
                    parent_name,
                    "",
                    generics,
                    force_inline,
                )?;
                if i >= optional_from {
                    if optional_element_needs_parens(ty) {
                        v = format!("({v})");
                    }
                    v.push('?');
                }
                s.push_str(&inner_comments(
                    field.deprecated.as_ref(),
                    &field.docs,
                    v,
                    true,
                    prefix,
                ));
            }
            s.push(']');
        }
    }

    Ok(())
}

/// The start index of the maximal all-optional TRAILING run of tuple
/// elements, or `fields.len()` when the last element isn't optional.
///
/// TypeScript only allows optional tuple elements at the end, so a
/// non-trailing `optional` flag is ignored (rendered required, the safe
/// wider-input direction). serde can't produce one anyway: `#[serde(default)]`
/// on an unnamed field requires every later field to be defaulted too, so
/// optional elements always form a suffix.
fn trailing_optional_run<'a>(fields: impl Iterator<Item = &'a Field>) -> usize {
    let mut run_start = 0;
    for (idx, field) in fields.enumerate() {
        if !field.optional {
            run_start = idx + 1;
        }
    }
    run_start
}

/// Whether an optional tuple element's type must be parenthesized before the
/// trailing `?` marker: `[number, (number | null)?]` is valid TypeScript
/// while `[number, number | null?]` is not, and likewise for intersections.
fn optional_element_needs_parens(ty: &DataType) -> bool {
    match ty {
        DataType::Nullable(_) | DataType::Enum(_) | DataType::Intersection(_) => true,
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => optional_element_needs_parens(dt),
            _ => false,
        },
        // A lone live unnamed field renders as its bare inner type.
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => unnamed
                .fields
                .first()
                .and_then(|field| field.ty.as_ref())
                .is_some_and(optional_element_needs_parens),
            _ => false,
        },
        _ => false,
    }
}

fn struct_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    st: &Struct,
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            s,
            exporter,
            format,
            types,
            &unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>(),
            unnamed.fields.len(),
            location,
            parent_name,
            prefix,
            generics,
            false,
        )?,
        Fields::Named(named) => {
            let fields = named
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                .collect::<Vec<_>>();

            if fields.is_empty() {
                s.push_str("Record<string, never>");
                return Ok(());
            }

            let mut unflattened_fields = Vec::with_capacity(fields.len());
            for (key, (field, ty)) in fields {
                let field_prefix = format!("{prefix}\t");
                let mut other = String::new();
                let mut field_location = location.clone();
                field_location.push(key.clone());
                object_field_to_ts(
                    &mut other,
                    exporter,
                    format,
                    types,
                    key.clone(),
                    (field, ty),
                    field_location,
                    parent_name,
                    generics,
                    &field_prefix,
                    false,
                    None,
                )?;

                unflattened_fields.push(inner_comments(
                    field.deprecated.as_ref(),
                    &field.docs,
                    other,
                    false,
                    &field_prefix,
                ));
            }

            s.push('{');
            for field in unflattened_fields {
                s.push('\n');
                s.push_str(&field);
                s.push(',');
            }
            s.push('\n');
            s.push_str(prefix);
            s.push('}');
        }
    }

    Ok(())
}

fn object_field_to_ts(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    key: Cow<'static, str>,
    (field, ty): (&Field, &DataType),
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    generics: &[(GenericReference, DataType)],
    prefix: &str,
    force_inline: bool,
    ty_override: Option<&str>,
) -> Result<(), Error> {
    let field_name_safe = sanitise_key(key, false);
    let key = if field.optional {
        format!("{field_name_safe}?").into()
    } else {
        field_name_safe
    };

    let value = match ty_override {
        Some(ty_override) => ty_override.to_string(),
        None => {
            let mut value = String::new();
            datatype_with_inline_attr(
                &mut value,
                exporter,
                format,
                types,
                ty,
                location,
                parent_name,
                prefix,
                generics,
                force_inline,
            )?;
            value
        }
    };

    s.push_str(prefix);
    s.push_str(&key);
    s.push_str(": ");
    s.push_str(&value);

    Ok(())
}

struct EnumVariantOutput {
    value: String,
    strict_keys: Option<BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct DiscriminatorAnalysis {
    key: String,
    known_literals: Vec<String>,
    fallback_variant_idx: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct VariantTypeOverride<'a> {
    key: &'a str,
    ty: &'a str,
}

#[derive(Debug, Clone)]
enum DiscriminatorValue {
    StringLiteral(String),
    String,
}

fn analyze_discriminator(
    variants: &[&(Cow<'static, str>, Variant)],
) -> Option<DiscriminatorAnalysis> {
    if variants.iter().any(|(name, _)| name.is_empty()) {
        return None;
    }

    let mut key = None::<String>;
    let mut known_literals = BTreeSet::new();
    let mut fallback_variant_idx = None;

    for (idx, (_, variant)) in variants.iter().enumerate() {
        let (variant_key, value) = variant_discriminator(variant)?;

        if let Some(expected) = &key {
            if expected != &variant_key {
                return None;
            }
        } else {
            key = Some(variant_key.clone());
        }

        match value {
            DiscriminatorValue::StringLiteral(value) => {
                known_literals.insert(value);
            }
            DiscriminatorValue::String => {
                if fallback_variant_idx.replace(idx).is_some() {
                    return None;
                }
            }
        }
    }

    if known_literals.is_empty() {
        return None;
    }

    Some(DiscriminatorAnalysis {
        key: key?,
        known_literals: known_literals.into_iter().collect(),
        fallback_variant_idx,
    })
}

fn variant_discriminator(variant: &Variant) -> Option<(String, DiscriminatorValue)> {
    let Fields::Named(named) = &variant.fields else {
        return None;
    };

    let (name, field) = named.fields.iter().find(|(_, field)| !field.optional)?;
    let ty = field.ty.as_ref()?;

    if matches!(ty, DataType::Primitive(Primitive::str)) {
        return Some((name.to_string(), DiscriminatorValue::String));
    }

    string_literal_datatype_value(ty)
        .map(|value| (name.to_string(), DiscriminatorValue::StringLiteral(value)))
}

fn string_literal_datatype_value(ty: &DataType) -> Option<String> {
    let DataType::Enum(enm) = ty else {
        return None;
    };

    let mut variants = enm.variants.iter();
    let (name, variant) = variants.next()?;
    if variants.next().is_some() || !matches!(&variant.fields, Fields::Unit) {
        return None;
    }

    Some(name.to_string())
}

fn exclude_known_literals_type(literals: &[String]) -> Option<String> {
    if literals.is_empty() {
        return None;
    }

    let known = literals
        .iter()
        .map(|value| format!("\"{}\"", escape_typescript_string_literal(value.as_str())))
        .collect::<Vec<_>>()
        .join(" | ");

    Some(format!("Exclude<string, {known}>"))
}

fn fallback_discriminator_override(
    discriminator: Option<&DiscriminatorAnalysis>,
) -> Option<(usize, &str, String)> {
    let discriminator = discriminator?;
    discriminator.fallback_variant_idx.and_then(|idx| {
        exclude_known_literals_type(&discriminator.known_literals)
            .map(|ty| (idx, discriminator.key.as_str(), ty))
    })
}

fn untagged_strict_keys(variant: &Variant) -> Option<BTreeSet<String>> {
    match &variant.fields {
        Fields::Named(obj) => Some(
            obj.fields
                .iter()
                .filter_map(|(name, field)| {
                    field
                        .ty
                        .as_ref()
                        .map(|_| sanitise_key(name.clone(), false).to_string())
                })
                .collect(),
        ),
        _ => None,
    }
}

fn has_anonymous_variant(variants: &[&(Cow<'static, str>, Variant)]) -> bool {
    variants.iter().any(|(name, _)| name.is_empty())
}

fn active_variants(e: &Enum) -> Vec<&(Cow<'static, str>, Variant)> {
    e.variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect()
}

fn strictify_enum_variants(variants: &mut [EnumVariantOutput]) {
    let strict_key_universe = variants
        .iter()
        .filter_map(|variant| variant.strict_keys.as_ref())
        .flat_map(|keys| keys.iter().cloned())
        .collect::<BTreeSet<_>>();

    if strict_key_universe.len() < 2 {
        return;
    }

    for variant in variants {
        let Some(keys) = variant.strict_keys.as_ref() else {
            continue;
        };

        let missing_keys = strict_key_universe
            .iter()
            .filter(|key| !keys.contains(*key))
            .map(|key| format!("{key}?: {NEVER}"))
            .collect::<Vec<_>>();

        if !missing_keys.is_empty() {
            variant.value = format!("({}) & {{ {} }}", variant.value, missing_keys.join("; "));
        }
    }
}

fn push_union(s: &mut String, variants: Vec<String>) {
    let mut seen = BTreeSet::new();
    let variants = variants
        .into_iter()
        .filter(|variant| seen.insert(variant.clone()))
        .collect::<Vec<_>>();

    if variants.is_empty() {
        s.push_str(NEVER);
    } else {
        s.push_str(&variants.join(" | "));
    }
}

fn enum_variant_datatype(
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    name: Cow<'static, str>,
    variant: &Variant,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    ty_override: Option<VariantTypeOverride<'_>>,
) -> Result<Option<String>, Error> {
    match &variant.fields {
        Fields::Unit if name.is_empty() => Err(Error::unsupported_anonymous_enum_variant(
            path_string(&location),
            "unit",
        )),
        Fields::Unit => Ok(Some(sanitise_key(name, true).to_string())),
        Fields::Named(_) if name.is_empty() => Err(Error::unsupported_anonymous_enum_variant(
            path_string(&location),
            "named-field",
        )),
        Fields::Named(obj) => {
            let mut regular_fields = Vec::new();
            for (field_name, field) in &obj.fields {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };

                let mut other = String::new();
                let mut field_location = location.clone();
                if field_location
                    .last()
                    .is_some_and(|location| location == field_name)
                {
                    if !matches!(ty, DataType::Struct(_)) {
                        field_location.push("0".into());
                    }
                } else {
                    field_location.push(field_name.clone());
                }
                object_field_to_ts(
                    &mut other,
                    exporter,
                    format,
                    types,
                    field_name.clone(),
                    (field, ty),
                    field_location,
                    None,
                    generics,
                    "",
                    false,
                    ty_override
                        .as_ref()
                        .filter(|override_ty| override_ty.key == field_name.as_ref())
                        .map(|override_ty| override_ty.ty),
                )?;

                regular_fields.push(inner_comments(
                    field.deprecated.as_ref(),
                    &field.docs,
                    other,
                    true,
                    prefix,
                ));
            }

            Ok(Some(if regular_fields.is_empty() {
                format!("Record<{STRING}, {NEVER}>")
            } else {
                format!("{{ {} }}", regular_fields.join("; "))
            }))
        }
        Fields::Unnamed(obj) => {
            let live_fields = obj
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();
            // A newtype payload renders bare, with no tuple position for an
            // optional marker to attach to.
            let is_newtype = obj.fields.len() == 1;
            let optional_from = if is_newtype {
                live_fields.len()
            } else {
                trailing_optional_run(live_fields.iter().map(|(field, _)| *field))
            };

            let fields = live_fields
                .iter()
                .enumerate()
                .map(|(idx, (_, ty))| {
                    let mut out = String::new();
                    let mut field_location = location.clone();
                    field_location.push(idx.to_string().into());
                    datatype_with_inline_attr(
                        &mut out,
                        exporter,
                        format,
                        types,
                        ty,
                        field_location,
                        None,
                        "",
                        generics,
                        false,
                    )?;
                    if idx >= optional_from {
                        if optional_element_needs_parens(ty) {
                            out = format!("({out})");
                        }
                        out.push('?');
                    }
                    Ok(out)
                })
                .collect::<Result<Vec<_>, Error>>()?;

            Ok(match &fields[..] {
                [] if obj.fields.is_empty() => Some("[]".to_string()),
                [] => None,
                [field] if is_newtype => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

/// Identity used to detect Serde `#[serde(untagged)]` alias cycles among
/// named exports (see [`collapse_untagged_alias_cycle`]). Two named types
/// are treated as "the same" if they share a Rust path, mirroring the
/// identity `rust_type_path` already uses for name-collision detection
/// elsewhere in this module.
///
/// Generic arguments are deliberately ignored: TypeScript's alias-cycle
/// check (TS2456) is declaration-level, so `GenP<T>` referencing `GenQ<T>`
/// referencing `GenP<U>` back is a cycle no matter how the parameters are
/// instantiated along the way.
type AliasId = (Cow<'static, str>, Cow<'static, str>);

fn alias_id(ndt: &NamedDataType) -> AliasId {
    (ndt.module_path.clone(), ndt.name.clone())
}

/// Substitution from a cycle member's generic parameter names to the
/// datatypes they are instantiated with along some path from the exported
/// root type: either the root's own parameters (`GenQ<U>` in a cycle with
/// `GenP<T>` says `U` where `GenP`'s body must say `T`) or concrete types
/// (`Root -> Gen<String> -> Root` instantiates `Gen`'s `T` at `string`).
/// Applying it to a member's variants lets them merge into the root's body
/// without dangling parameters.
type GenericSubst = HashMap<Cow<'static, str>, DataType>;

/// An alias-transparent hop from one named export to another: a position
/// whose TypeScript rendering is the *bare* referenced name (possibly
/// `| null`), with no `{}`/`[]`/tuple wrapper deferring alias resolution
/// around it.
///
/// This is exactly the shape that makes TypeScript's alias resolution
/// eager: `type X = Y` requires resolving `Y` immediately, so a cycle
/// through only this shape of hop is what produces the illegal
/// `type X = X | ...` (TS2456), whereas a cycle through `{ field: X }`,
/// `X[]`, or `[X]` does not, because object/array/tuple types defer
/// resolution.
struct TransparentHop<'t, 'r> {
    target: &'t NamedDataType,
    /// The reference itself, kept so the collapse can resolve the hop's
    /// generic arguments (including omitted ones) with the same
    /// [`resolved_reference_generics`] logic ordinary rendering uses.
    reference: &'r NamedReference,
    /// Whether the hop passes through [`DataType::Nullable`], rendering as
    /// `Target | null`. A union member is still resolved eagerly, so this is
    /// just as much a cycle - but dropping such a back edge must keep the
    /// `null` branch, because an `Option` chain bottoms out at `None`.
    nullable: bool,
}

/// Returns the alias-transparent hop `dt` renders as, if any: a bare
/// [`Reference::Named`], optionally wrapped in [`DataType::Nullable`]
/// (which renders as `| null` - still an eagerly-resolved position).
///
/// The hop's target borrows from `types` while the reference borrows from
/// `dt`, so a hop derived from a temporary (substituted) datatype still
/// yields a long-lived target.
fn transparent_reference<'t, 'r>(
    types: &'t Types,
    dt: &'r DataType,
) -> Option<TransparentHop<'t, 'r>> {
    match dt {
        DataType::Reference(Reference::Named(r)) => match &r.inner {
            NamedReferenceType::Reference { .. } => Some(TransparentHop {
                target: types.get(r)?,
                reference: r,
                nullable: false,
            }),
            _ => None,
        },
        DataType::Nullable(inner) => Some(TransparentHop {
            nullable: true,
            ..transparent_reference(types, inner)?
        }),
        _ => None,
    }
}

/// If `variant`'s rendering is exactly a transparent hop - the shape a
/// `#[serde(untagged)]` newtype variant takes, since serde adds no wire
/// structure around it - returns that hop.
///
/// The variant must have exactly one field *total*, mirroring the
/// `obj.fields.len() == 1` bare-rendering rule in [`enum_variant_datatype`]:
/// a skipped extra field makes the renderer emit a `[T]` one-tuple
/// (matching serde's one-element seq), which is a deferred position and
/// therefore not transparent.
fn transparent_variant_hop<'t, 'r>(
    types: &'t Types,
    variant: &'r Variant,
) -> Option<TransparentHop<'t, 'r>> {
    let Fields::Unnamed(unnamed) = &variant.fields else {
        return None;
    };
    let [field] = unnamed.fields.as_slice() else {
        return None;
    };
    transparent_reference(types, field.ty.as_ref()?)
}

/// Returns the alias-transparent *position* of a named export whose own
/// body renders as a single bare datatype - a plain alias body (e.g. what
/// `#[serde(transparent)]` rewrites to) or a newtype tuple struct, both
/// exported as `export type W = <position>;`. Such exports continue an
/// alias cycle without contributing union members of their own.
///
/// Unlike enum variants, [`struct_dt`] pre-filters skipped fields, so a
/// struct renders bare whenever exactly one *live* field remains.
fn transparent_export_position(ndt: &NamedDataType) -> Option<&DataType> {
    match ndt.ty.as_ref()? {
        DataType::Struct(s) => {
            let Fields::Unnamed(unnamed) = &s.fields else {
                return None;
            };
            let mut live = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            let ty = live.next()?;
            if live.next().is_some() {
                return None;
            }
            Some(ty)
        }
        // An enum body contributes hops through its variants instead.
        DataType::Enum(_) => None,
        dt => Some(dt),
    }
}

/// The transparent hop of a passthrough export's body, if any (see
/// [`transparent_export_position`]).
fn transparent_export_hop<'t, 'r>(
    types: &'t Types,
    ndt: &'r NamedDataType,
) -> Option<TransparentHop<'t, 'r>> {
    transparent_reference(types, transparent_export_position(ndt)?)
}

/// A named export participating in the alias-transparent reference graph.
struct AliasNode<'a> {
    ndt: &'a NamedDataType,
    /// Every distinct generic instantiation this node is reached with from
    /// the root, in deterministic discovery order. A node can have several -
    /// e.g. `Gen<T>` reached both as itself (the root, identity) and as
    /// `Gen<String>` through the cycle - and its variants merge into the
    /// root's body once per instantiation.
    substs: Vec<GenericSubst>,
    /// Targets of this node's outgoing alias-transparent hops, used to
    /// compute cycle membership. The hops themselves are re-derived (and
    /// re-classified post-substitution) during the merge walk.
    edges: Vec<AliasId>,
}

/// Divergence guards for [`collapse_untagged_alias_cycle`]'s instantiation
/// walk (see [`enqueue_cycle_instantiation`]). The instantiation set is
/// infinite only when the cycle grows its own type arguments
/// (`Gen<T> -> Gen<Vec<T>>`) - a shape serde itself cannot serialize (it
/// would need infinite monomorphization) but dynamically built [`Types`]
/// can express - so two generous caps turn divergence into an error
/// instead of a hang: one on *distinct* instantiations per node (breadth -
/// many small instantiations), and a much smaller one on the structural
/// size of a single instantiation (growth - it stops both linearly-growing
/// arguments after a few hundred trips and exponentially-growing ones like
/// `Gen<T> -> Gen<(T, T)>` long before they exhaust memory; no realistic
/// single type argument is 256 nodes deep/wide).
const MAX_SUBSTS_PER_NODE: usize = 1024;
const MAX_SUBST_WEIGHT: usize = 256;

/// Adds a (cycle member, instantiation) pair to the collapse's merge work
/// list unless that exact pair was already processed, enforcing the
/// divergence guards above. Only genuinely new pairs enter the list, which
/// is what makes the walk a terminating fixed-point iteration.
fn enqueue_cycle_instantiation(
    nodes: &mut HashMap<AliasId, AliasNode<'_>>,
    work: &mut Vec<(AliasId, GenericSubst)>,
    target_id: AliasId,
    subst: GenericSubst,
) -> Result<(), Cow<'static, str>> {
    let node = nodes
        .get_mut(&target_id)
        .expect("cycle members were discovered in phase 1");
    if node.substs.contains(&subst) {
        return Ok(());
    }
    if node.substs.len() >= MAX_SUBSTS_PER_NODE {
        return Err(Cow::Borrowed(
            "the cycle's generic parameter instantiations do not converge (a single type is instantiated with an unbounded number of distinct arguments)",
        ));
    }
    if subst.values().map(datatype_weight).sum::<usize>() > MAX_SUBST_WEIGHT {
        return Err(Cow::Borrowed(
            "the cycle's generic parameter instantiations do not converge (a type argument keeps growing with every trip around the cycle)",
        ));
    }
    node.substs.push(subst.clone());
    work.push((target_id, subst));
    Ok(())
}

fn node_enum(ndt: &NamedDataType) -> Option<&Enum> {
    match ndt.ty.as_ref() {
        Some(DataType::Enum(e)) => Some(e),
        _ => None,
    }
}

/// The *effective* transparent hop of a position under an instantiation:
/// where its rendering ends up once generic parameters are taken into
/// account, which is how TypeScript's eager alias resolution sees it.
struct EffectiveHop<'t> {
    target: &'t NamedDataType,
    /// The full substitution the hop applies to its target, resolved via
    /// [`edge_substitution`].
    target_subst: GenericSubst,
    nullable: bool,
}

/// Computes the effective transparent hop of an enum variant under `subst`,
/// along with the substituted variant to merge if the hop is not taken:
///
/// - A *raw* hop's target is fixed regardless of substitution; its
///   arguments resolve under `subst`.
/// - Otherwise the *substituted* form can reveal a hop that only exists
///   under this instantiation - `B(T)` with `T = Root` (or
///   `T = Gen<String>`). A revealed reference's arguments are already in
///   the root's scope, so they resolve under `identity`.
fn effective_variant_hop<'t>(
    types: &'t Types,
    variant: &Variant,
    subst: &GenericSubst,
    identity: &GenericSubst,
) -> Result<(Variant, Option<EffectiveHop<'t>>), Cow<'static, str>> {
    let raw_hop = transparent_variant_hop(types, variant);

    let mut variant = variant.clone();
    substitute_fields_generics(&mut variant.fields, subst)?;

    if let Some(hop) = raw_hop {
        let target_subst = edge_substitution(subst, hop.target, hop.reference)?;
        let hop = EffectiveHop {
            target: hop.target,
            target_subst,
            nullable: hop.nullable,
        };
        return Ok((variant, Some(hop)));
    }

    let hop = match transparent_variant_hop(types, &variant) {
        Some(hop) => Some(EffectiveHop {
            target: hop.target,
            target_subst: edge_substitution(identity, hop.target, hop.reference)?,
            nullable: hop.nullable,
        }),
        None => None,
    };
    Ok((variant, hop))
}

/// The passthrough-export analogue of [`effective_variant_hop`]: the
/// effective hop of the export's body position under `subst`, along with
/// the substituted position itself (a passthrough member whose effective
/// body is *not* a cycle hop contributes it as a terminal).
fn effective_export_hop<'t>(
    types: &'t Types,
    ndt: &NamedDataType,
    subst: &GenericSubst,
    identity: &GenericSubst,
) -> Result<Option<(DataType, Option<EffectiveHop<'t>>)>, Cow<'static, str>> {
    let Some(position) = transparent_export_position(ndt) else {
        return Ok(None);
    };
    let raw_hop = transparent_reference(types, position);

    let mut position = position.clone();
    substitute_generics(&mut position, subst)?;

    if let Some(hop) = raw_hop {
        let target_subst = edge_substitution(subst, hop.target, hop.reference)?;
        let hop = EffectiveHop {
            target: hop.target,
            target_subst,
            nullable: hop.nullable,
        };
        return Ok(Some((position, Some(hop))));
    }

    let hop = match transparent_reference(types, &position) {
        Some(hop) => Some(EffectiveHop {
            target: hop.target,
            target_subst: edge_substitution(identity, hop.target, hop.reference)?,
            nullable: hop.nullable,
        }),
        None => None,
    };
    Ok(Some((position, hop)))
}

/// Resolves the full substitution an in-cycle hop applies to its target: one
/// entry per *declared* parameter of the target, filled exactly the way
/// ordinary reference rendering fills them via
/// [`resolved_reference_generics`] - the explicit argument if present,
/// otherwise the parameter's declared default, otherwise `unknown` - so the
/// collapse and the renderer can never disagree about omitted parameters.
/// Each resolved value is then rewritten from the source node's scope into
/// the root's via `source_subst` (extended with the target's earlier
/// parameters, which declared defaults may reference).
fn edge_substitution(
    source_subst: &GenericSubst,
    target: &NamedDataType,
    reference: &NamedReference,
) -> Result<GenericSubst, Cow<'static, str>> {
    let (resolved, _, _) = resolved_reference_generics(target, reference, &[]).ok_or(
        Cow::Borrowed("the cycle contains a reference whose generic arguments cannot be resolved"),
    )?;

    let mut scope = source_subst.clone();
    let mut target_subst = GenericSubst::with_capacity(resolved.len());
    for (generic, mut value) in target.generics.iter().zip(resolved) {
        substitute_generics(&mut value, &scope)?;
        scope.insert(generic.name.clone(), value.clone());
        target_subst.insert(generic.name.clone(), value);
    }
    Ok(target_subst)
}

/// Whether a substitution maps every parameter to itself, i.e. applying it
/// is a no-op. The root's own variants merge under the identity, and only
/// non-identity substitutions can invalidate payloads the walk can't see
/// into (unknown opaque references).
fn subst_is_identity(subst: &GenericSubst) -> bool {
    subst
        .iter()
        .all(|(name, dt)| matches!(dt, DataType::Generic(g) if g.name() == name))
}

/// Rewrites every generic parameter reference in `dt` (part of a cycle
/// member's variant being merged into the exported root type) to the
/// datatype the member is instantiated with, per `subst`. Errors abort the
/// collapse and fail the export - by this point the cycle is known to
/// render as illegal TypeScript (TS2456), so an honest error beats silently
/// emitting either that or a dangling parameter name.
fn substitute_generics(dt: &mut DataType, subst: &GenericSubst) -> Result<(), Cow<'static, str>> {
    match dt {
        DataType::Generic(g) => match subst.get(g.name()) {
            Some(replacement) => {
                *dt = replacement.clone();
                Ok(())
            }
            None => Err(Cow::Owned(format!(
                "the cycle references generic parameter `{}` with no instantiation for it",
                g.name()
            ))),
        },
        DataType::Primitive(_) => Ok(()),
        DataType::List(list) => substitute_generics(&mut list.ty, subst),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), subst)?;
            substitute_generics(map.value_ty_mut(), subst)
        }
        DataType::Struct(s) => substitute_fields_generics(&mut s.fields, subst),
        DataType::Enum(e) => e
            .variants
            .iter_mut()
            .try_for_each(|(_, variant)| substitute_fields_generics(&mut variant.fields, subst)),
        DataType::Tuple(t) => t
            .elements
            .iter_mut()
            .try_for_each(|ty| substitute_generics(ty, subst)),
        DataType::Nullable(inner) => substitute_generics(inner, subst),
        DataType::Intersection(parts) => parts
            .iter_mut()
            .try_for_each(|ty| substitute_generics(ty, subst)),
        DataType::Reference(Reference::Named(r)) => match &mut r.inner {
            NamedReferenceType::Reference { generics, .. } => generics
                .iter_mut()
                .try_for_each(|(_, dt)| substitute_generics(dt, subst)),
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, subst),
            NamedReferenceType::Recursive(_) => Ok(()),
        },
        DataType::Reference(Reference::Opaque(opaque_ref)) => {
            if let Some(branded) = opaque_ref.downcast_ref::<Branded>() {
                // A branded payload can embed generic parameters
                // (`branded!(struct Id<T>(T))` renders as `T & { ... }`),
                // so it has to be rewritten like any other datatype.
                let mut ty = branded.ty().clone();
                let brand = branded.brand().clone();
                substitute_generics(&mut ty, subst)?;
                *dt = DataType::Reference(Reference::opaque(Branded::new(brand, ty)));
                Ok(())
            } else if subst_is_identity(subst) {
                // Identity substitutions are no-ops: nothing is being
                // rewritten, so no payload can be invalidated.
                Ok(())
            } else if let Some(def) = opaque_ref.downcast_ref::<opaque::Define>() {
                // `define(...)` is raw TypeScript text: it can *name* the
                // member's generic parameters but cannot be structurally
                // rewritten (and textually rewriting raw TS would be
                // fragile). Allow it only when a conservative
                // word-boundary scan shows it mentions no parameter this
                // substitution actually rewrites.
                match subst.iter().find(|(name, replacement)| {
                    !matches!(replacement, DataType::Generic(g) if g.name() == *name)
                        && raw_ts_mentions_identifier(&def.0, name)
                }) {
                    Some((name, _)) => Err(Cow::Owned(format!(
                        "the cycle contains raw TypeScript (`define(...)`) that mentions generic parameter `{name}`, which cannot be rewritten"
                    ))),
                    None => Ok(()),
                }
            } else if opaque_ref.downcast_ref::<opaque::Any>().is_some()
                || opaque_ref.downcast_ref::<opaque::Unknown>().is_some()
                || opaque_ref.downcast_ref::<opaque::Never>().is_some()
                || opaque_ref.downcast_ref::<opaque::Number>().is_some()
                || opaque_ref.downcast_ref::<opaque::BigInt>().is_some()
            {
                // This crate's remaining opaque payloads can't embed a
                // datatype or mention a parameter at all.
                Ok(())
            } else {
                // An opaque reference from elsewhere might embed generic
                // parameters this walk can't see, let alone rewrite.
                Err(Cow::Owned(format!(
                    "the cycle contains an opaque reference (`{}`) whose payload cannot be checked for generic parameters",
                    opaque_ref.type_name()
                )))
            }
        }
    }
}

/// Conservative check for whether raw TypeScript text (a `define(...)`
/// payload) mentions `name` as a standalone identifier - a word-boundary
/// match, so the parameter `U` is found in `ReadonlyArray<U>` but not in
/// `Unit`. Any hit means the raw text may depend on a generic parameter a
/// cycle merge would rewrite, which is impossible for raw text, so the
/// caller must reject the merge. Non-ASCII bytes are treated as identifier
/// characters, erring toward a match (and therefore rejection) around
/// exotic identifiers.
fn raw_ts_mentions_identifier(raw: &str, name: &str) -> bool {
    fn is_ident_byte(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || !b.is_ascii()
    }

    if name.is_empty() {
        return false;
    }
    let bytes = raw.as_bytes();
    let mut search_start = 0;
    while let Some(pos) = raw[search_start..].find(name) {
        let start = search_start + pos;
        let end = start + name.len();
        if (start == 0 || !is_ident_byte(bytes[start - 1]))
            && (end == raw.len() || !is_ident_byte(bytes[end]))
        {
            return true;
        }
        // A later occurrence overlapping this one starts inside `name`, so
        // its preceding character is part of `name` - an identifier - and
        // would fail the boundary check anyway; skipping past `end` is safe
        // and keeps the cursor on a UTF-8 character boundary.
        search_start = end;
    }
    false
}

/// Structural size of a datatype, used as a divergence guard for cycle
/// instantiations (see [`collapse_untagged_alias_cycle`] phase 3): a type
/// argument that grows on every trip around a cycle grows in weight, so
/// capping the weight bounds the walk even for exponential growth.
fn datatype_weight(dt: &DataType) -> usize {
    fn fields_weight(fields: &Fields) -> usize {
        match fields {
            Fields::Unit => 1,
            Fields::Unnamed(unnamed) => unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .map(datatype_weight)
                .sum(),
            Fields::Named(named) => named
                .fields
                .iter()
                .filter_map(|(_, field)| field.ty.as_ref())
                .map(datatype_weight)
                .sum(),
        }
    }

    1_usize.saturating_add(match dt {
        DataType::Primitive(_) | DataType::Generic(_) => 0,
        DataType::List(list) => datatype_weight(&list.ty),
        DataType::Map(map) => {
            datatype_weight(map.key_ty()).saturating_add(datatype_weight(map.value_ty()))
        }
        DataType::Struct(s) => fields_weight(&s.fields),
        DataType::Enum(e) => e
            .variants
            .iter()
            .map(|(_, variant)| fields_weight(&variant.fields))
            .sum(),
        DataType::Tuple(t) => t.elements.iter().map(datatype_weight).sum(),
        DataType::Nullable(inner) => datatype_weight(inner),
        DataType::Intersection(parts) => parts.iter().map(datatype_weight).sum(),
        DataType::Reference(Reference::Named(r)) => match &r.inner {
            NamedReferenceType::Reference { generics, .. } => {
                generics.iter().map(|(_, dt)| datatype_weight(dt)).sum()
            }
            NamedReferenceType::Inline { dt, .. } => datatype_weight(dt),
            NamedReferenceType::Recursive(_) => 0,
        },
        DataType::Reference(Reference::Opaque(opaque_ref)) => opaque_ref
            .downcast_ref::<Branded>()
            .map(|branded| datatype_weight(branded.ty()))
            .unwrap_or(0),
    })
}

fn substitute_fields_generics(
    fields: &mut Fields,
    subst: &GenericSubst,
) -> Result<(), Cow<'static, str>> {
    match fields {
        Fields::Unit => Ok(()),
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .iter_mut()
            .filter_map(|field| field.ty.as_mut())
            .try_for_each(|ty| substitute_generics(ty, subst)),
        Fields::Named(named) => named
            .fields
            .iter_mut()
            .filter_map(|(_, field)| field.ty.as_mut())
            .try_for_each(|ty| substitute_generics(ty, subst)),
    }
}

/// Detects whether `root` is part of an alias-transparent reference cycle -
/// i.e. whether naively rendering its variants would produce a TypeScript
/// alias that circularly references itself, directly or through other
/// untagged enums or passthrough exports (`type Rec = Rec | ...`, or
/// `type RecA = RecB | ...; type RecB = RecA | ...`).
///
/// Every transparent hop serializes as exactly its inner value, so the
/// wire-level value set of an enum in such a cycle is precisely the union
/// of the *non-cyclic* branches reachable from anywhere in the cycle (plus
/// `null` if any cyclic branch passes through an `Option`): any finite
/// value must eventually bottom out at one of them, since the recursive
/// branches never add wire structure of their own. Returns the merged,
/// cycle-free variant list to render in `root`'s place, or `Ok(None)` if
/// `root` isn't part of such a cycle - in which case its rendering must be
/// left completely untouched.
///
/// Returns an error (the human-readable reason) when a cycle exists but
/// cannot be collapsed. The naive rendering of such a cycle is illegal
/// TypeScript (TS2456), so failing the export loudly beats emitting it.
///
/// Regression test for https://github.com/specta-rs/specta/pull/517#discussion_r3584346217:
/// exporting `#[serde(untagged)] enum Rec { A(Box<Rec>), B(Map) }` used to
/// emit `export type Rec = Rec | Map;`, which `tsc` rejects.
fn collapse_untagged_alias_cycle(
    types: &Types,
    root: &NamedDataType,
) -> Result<Option<Enum>, Cow<'static, str>> {
    // Only enum exports assemble a union that can be collapsed. Passthrough
    // exports inside a cycle (see `transparent_export_hop`) keep their bare
    // alias body, which becomes legal once every enum in the cycle is
    // collapsed and the chain of aliases stops looping.
    if node_enum(root).is_none() {
        return Ok(None);
    }
    let root_id = alias_id(root);

    let identity: GenericSubst = root
        .generics
        .iter()
        .map(|g| (g.name.clone(), DataType::Generic(g.reference())))
        .collect();

    // Phase 1: discover the *effective* alias-transparent reference graph
    // reachable from `root`. Nodes are explored once per (node,
    // instantiation) pair (same dedup and divergence guards as the merge
    // walk), and hops are derived from each node's substituted form, so an
    // edge carried entirely by a generic argument - `Gen<Root>` where
    // `Gen<T>`'s only transparent branch is the bare parameter `T` - is
    // discovered exactly like a raw reference edge. TypeScript's alias
    // cycle check is declaration-level, so edges are recorded per node
    // (not per instantiation) for the cycle test in phase 2.
    //
    // A substitution failure here (e.g. an opaque payload the walk refuses
    // to touch) must not fail an export that may not be cyclic at all: the
    // raw hop's target is still recorded (its identity doesn't depend on
    // the substitution) and explored under its own identity, which keeps
    // raw reachability complete. If such a node does end up in a cycle,
    // the merge walk hits the same substitution error and fails honestly.
    let mut nodes: HashMap<AliasId, AliasNode<'_>> = HashMap::new();
    nodes.insert(
        root_id.clone(),
        AliasNode {
            ndt: root,
            substs: vec![identity.clone()],
            edges: Vec::new(),
        },
    );

    let mut work: Vec<(AliasId, GenericSubst)> = vec![(root_id.clone(), identity.clone())];
    let mut cursor = 0;
    while cursor < work.len() {
        let (id, subst) = work[cursor].clone();
        cursor += 1;

        let ndt = nodes[&id].ndt;
        let mut hops: Vec<(&NamedDataType, Option<GenericSubst>)> = Vec::new();
        match node_enum(ndt) {
            Some(enm) => {
                for (_, variant) in &enm.variants {
                    if variant.skip {
                        continue;
                    }
                    match effective_variant_hop(types, variant, &subst, &identity) {
                        Ok((_, Some(hop))) => hops.push((hop.target, Some(hop.target_subst))),
                        Ok((_, None)) => {}
                        Err(_) => {
                            if let Some(hop) = transparent_variant_hop(types, variant) {
                                hops.push((hop.target, None));
                            }
                        }
                    }
                }
            }
            None => match effective_export_hop(types, ndt, &subst, &identity) {
                Ok(Some((_, Some(hop)))) => hops.push((hop.target, Some(hop.target_subst))),
                Ok(_) => {}
                Err(_) => {
                    if let Some(hop) = transparent_export_hop(types, ndt) {
                        hops.push((hop.target, None));
                    }
                }
            },
        }

        for (target, target_subst) in hops {
            let target_id = alias_id(target);
            if let Entry::Vacant(entry) = nodes.entry(target_id.clone()) {
                entry.insert(AliasNode {
                    ndt: target,
                    substs: Vec::new(),
                    edges: Vec::new(),
                });
            }

            let source = nodes
                .get_mut(&id)
                .expect("node was inserted before being visited");
            if !source.edges.contains(&target_id) {
                source.edges.push(target_id.clone());
            }

            // Fall back to the target's own identity when the hop's
            // instantiation couldn't be resolved, so its raw hops are
            // still explored.
            let target_subst = match target_subst {
                Some(target_subst) => target_subst,
                None => target
                    .generics
                    .iter()
                    .map(|g| (g.name.clone(), DataType::Generic(g.reference())))
                    .collect(),
            };
            enqueue_cycle_instantiation(&mut nodes, &mut work, target_id, target_subst)?;
        }
    }

    // Phase 2: `root`'s strongly-connected component. Every discovered node
    // is reachable from `root`, so the members of `root`'s cycle are exactly
    // the nodes that can reach `root` back - found by walking the reversed
    // edges from `root`. `root` itself shows up iff some cycle through it
    // exists at all.
    let mut reverse: HashMap<&AliasId, Vec<&AliasId>> = HashMap::new();
    for (id, node) in &nodes {
        for target in &node.edges {
            reverse.entry(target).or_default().push(id);
        }
    }
    let mut in_cycle: HashSet<AliasId> = HashSet::new();
    let mut frontier = vec![&root_id];
    while let Some(id) = frontier.pop() {
        for &source in reverse.get(id).into_iter().flatten() {
            if in_cycle.insert(source.clone()) {
                frontier.push(source);
            }
        }
    }
    if !in_cycle.contains(&root_id) {
        return Ok(None);
    }
    drop(frontier);
    drop(reverse);

    // Phase 3: propagate generic instantiations around the cycle to a fixed
    // point, starting from the root's identity, merging each (member,
    // instantiation) pair's non-back-edge branches as it is processed. A
    // member can be reached with several distinct instantiations (`Gen<T>`
    // as the root *and* as `Gen<String>` through the cycle); each one
    // contributes a copy of its branches.
    //
    // Back edges are classified on each branch's *effective* hop (see
    // `effective_variant_hop`): the raw hop when the branch is one, or the
    // hop its substituted form reveals (`B(T)` with `T = Root`, or
    // `T = Gen<String>` for a cycle member `Gen`). A back edge is dropped -
    // it adds no wire structure of its own, except a `Nullable` hop whose
    // `None` case is a real `null` wire value to keep - and its
    // instantiation is fed through this same work list, so its terminals
    // still merge. Discovery errors were tolerated in phase 1; here the
    // cycle is confirmed, so substitution failures fail the export
    // honestly.
    //
    // Termination: only genuinely new (node, substitution) pairs enter the
    // work list (see `enqueue_cycle_instantiation`), so the walk stops
    // exactly when the instantiation set stops growing - a finite set of
    // any size converges. Branches are merged in work-list order (`root`'s
    // own branches under the identity first), so the output is
    // deterministic; `push_union` dedupes identical renderings.
    for node in nodes.values_mut() {
        node.substs.clear();
    }
    let mut work: Vec<(AliasId, GenericSubst)> = vec![(root_id.clone(), identity.clone())];
    nodes
        .get_mut(&root_id)
        .expect("root node always exists")
        .substs
        .push(identity.clone());

    let mut variants = Vec::new();
    let mut needs_null = false;
    let mut cursor = 0;
    while cursor < work.len() {
        let (id, subst) = work[cursor].clone();
        cursor += 1;

        let ndt = nodes[&id].ndt;
        let Some(enm) = node_enum(ndt) else {
            // Passthrough member: its effective body is usually a back edge
            // into the cycle (contributing nothing but a possible `null`
            // remnant), but under some instantiations it can be a hop out
            // of the cycle or no hop at all (`W<T>` reached at
            // `T = string`), in which case the substituted body is a
            // terminal to keep.
            let Some((position, hop)) = effective_export_hop(types, ndt, &subst, &identity)? else {
                continue;
            };
            match hop {
                Some(hop) if in_cycle.contains(&alias_id(hop.target)) => {
                    needs_null |= hop.nullable;
                    enqueue_cycle_instantiation(
                        &mut nodes,
                        &mut work,
                        alias_id(hop.target),
                        hop.target_subst,
                    )?;
                }
                _ => variants.push((
                    Cow::Borrowed("passthrough"),
                    Variant::unnamed().field(Field::new(position)).build(),
                )),
            }
            continue;
        };

        for (name, variant) in &enm.variants {
            if variant.skip {
                continue;
            }

            let (variant, hop) = effective_variant_hop(types, variant, &subst, &identity)?;
            if let Some(hop) = hop
                && in_cycle.contains(&alias_id(hop.target))
            {
                needs_null |= hop.nullable;
                enqueue_cycle_instantiation(
                    &mut nodes,
                    &mut work,
                    alias_id(hop.target),
                    hop.target_subst,
                )?;
                continue;
            }

            variants.push((name.clone(), variant));
        }
    }

    if needs_null {
        // The same `null`-rendering empty-tuple shape `specta-serde` uses
        // for untagged unit variants.
        variants.push((
            Cow::Borrowed("null"),
            Variant::unnamed()
                .field(Field::new(DataType::Tuple(Tuple::new(vec![]))))
                .build(),
        ));
    }

    let mut collapsed = Enum::default();
    collapsed.variants = variants;
    Ok(Some(collapsed))
}

/// Returns the [`DataType`] to render as `ndt`'s own top-level export body,
/// collapsing an untagged-enum alias cycle into valid TypeScript if `ndt` is
/// part of one (see [`collapse_untagged_alias_cycle`]). Borrows `ndt.ty`
/// unchanged in the (overwhelmingly common) non-cyclic case.
fn resolve_named_export_body<'a>(
    types: &Types,
    ndt: &'a NamedDataType,
) -> Result<Cow<'a, DataType>, Error> {
    let ty = ndt.ty.as_ref().expect("named datatype must have a body");
    match collapse_untagged_alias_cycle(types, ndt) {
        Ok(Some(collapsed)) => Ok(Cow::Owned(DataType::Enum(collapsed))),
        Ok(None) => Ok(Cow::Borrowed(ty)),
        Err(reason) => Err(Error::unrepresentable_alias_cycle(
            rust_type_path(ndt).into_owned(),
            reason,
        )
        .with_named_datatype(ndt)),
    }
}

fn enum_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    e: &Enum,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let is_finite_number = e.attributes.get_named_as("specta:finite_number") == Some(&true);

    if e.attributes.get_named_as(FIELD_ALIAS_UNION_MARKER) == Some(&true) {
        return alias_field_union_dt(s, exporter, types, e, location, prefix, generics);
    }

    if e.variants.is_empty() {
        s.push_str(NEVER);
        return Ok(());
    }

    let filtered_variants = active_variants(e);

    let discriminator = analyze_discriminator(&filtered_variants);
    let fallback_override = fallback_discriminator_override(discriminator.as_ref());

    let mut rendered_variants = Vec::with_capacity(filtered_variants.len());
    for (idx, (variant_name, variant)) in filtered_variants.iter().enumerate() {
        let variant_override = fallback_override
            .as_ref()
            .and_then(|(fallback_idx, key, ty)| {
                (*fallback_idx == idx).then_some(VariantTypeOverride {
                    key,
                    ty: ty.as_str(),
                })
            });

        let mut variant_location = location.clone();
        variant_location.push(variant_name.clone());
        let ts_values = enum_variant_datatype(
            exporter,
            None,
            types,
            variant_name.clone(),
            variant,
            variant_location,
            prefix,
            generics,
            variant_override,
        )?;

        rendered_variants.push(EnumVariantOutput {
            value: ts_values.unwrap_or_else(|| NEVER.to_string()),
            strict_keys: untagged_strict_keys(variant),
        });
    }

    // Rendering first preserves errors for unremapped BigInt-style variants.
    if is_finite_number {
        s.push_str("number");
        return Ok(());
    }

    if discriminator.is_none() && !has_anonymous_variant(&filtered_variants) {
        strictify_enum_variants(&mut rendered_variants);
    }

    let variants = filtered_variants
        .into_iter()
        .zip(rendered_variants)
        .map(|((_, variant), rendered)| {
            inner_comments(
                variant.deprecated.as_ref(),
                &variant.docs,
                rendered.value,
                true,
                prefix,
            )
        })
        .collect::<Vec<_>>();

    push_union(s, variants);

    Ok(())
}

/// Renders serde field aliases as a deferred mapped XOR instead of an
/// intersection of unions. The latter distributes into one union member for
/// every combination of aliased fields, which quickly reaches TS2590.
///
/// The mapped type retains the original semantics: each required field must
/// use exactly one accepted spelling, optional fields may be omitted, and two
/// spellings of the same field cannot be supplied together.
fn alias_field_union_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    e: &Enum,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let mut fields = Vec::with_capacity(e.variants.len());

    for (name, variant) in active_variants(e) {
        let mut variant = (*variant).clone();
        if let Fields::Unnamed(unnamed) = &mut variant.fields
            && let Some(DataType::Struct(strct)) = unnamed
                .fields
                .first_mut()
                .and_then(|field| field.ty.as_mut())
            && let Fields::Named(named) = &mut strct.fields
        {
            named
                .fields
                .retain(|(_, field)| !field.attributes.contains_key(FIELD_ALIAS_EXCLUSION_MARKER));
        }

        let mut variant_location = location.clone();
        variant_location.push(name.clone());
        let field = enum_variant_datatype(
            exporter,
            None,
            types,
            name.clone(),
            &variant,
            variant_location,
            prefix,
            generics,
            None,
        )?
        .unwrap_or_else(|| NEVER.to_string());
        fields.push(field);
    }

    let source = fields.join(" & ");
    s.push_str("((");
    s.push_str(&source);
    s.push_str(") extends infer T extends object ? { [K in keyof T]: { [P in keyof T as P extends K ? P : never]: T[P] } & { [P in keyof T as P extends K ? never : P]?: never } }[keyof T] & object : never)");

    Ok(())
}

fn tuple_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    t: &Tuple,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match t.elements.as_slice() {
        [] => s.push_str(NULL),
        elements => {
            s.push('[');
            for (idx, dt) in elements.iter().enumerate() {
                if idx != 0 {
                    s.push_str(", ");
                }
                let mut element_location = location.clone();
                element_location.push(idx.to_string().into());
                datatype(
                    s,
                    exporter,
                    None,
                    types,
                    dt,
                    element_location,
                    None,
                    "",
                    generics,
                )?;
            }
            s.push(']');
        }
    }

    Ok(())
}

fn reference_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    r: &Reference,
    location: Vec<Cow<'static, str>>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    match r {
        Reference::Named(r) => match &r.inner {
            NamedReferenceType::Reference { .. } => {
                reference_named_dt(s, exporter, types, r, location, generics)
            }
            NamedReferenceType::Inline { dt, .. } => {
                let inline_path = path_string(&location);
                inline_datatype(
                    s, exporter, format, types, dt, location, None, prefix, 0, generics,
                )
                .map_err(|err| err.with_inline_trace(types.get(r), inline_path))
            }
            NamedReferenceType::Recursive(cycle) => Err(Error::infinite_recursive_inline_type(
                path_string(&location),
                format!("{r:?}"),
                cycle.clone(),
            )),
        },
        Reference::Opaque(r) => reference_opaque_dt(s, exporter, format, types, r, location),
    }
}

fn reference_opaque_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    r: &OpaqueReference,
    location: Vec<Cow<'static, str>>,
) -> Result<(), Error> {
    if let Some(def) = r.downcast_ref::<opaque::Define>() {
        s.push_str(&def.0);
        return Ok(());
    }

    if r.downcast_ref::<opaque::Any>().is_some() {
        s.push_str("any");
        return Ok(());
    }

    if r.downcast_ref::<opaque::Unknown>().is_some() {
        s.push_str("unknown");
        return Ok(());
    }

    if r.downcast_ref::<opaque::Never>().is_some() {
        s.push_str("never");
        return Ok(());
    }

    if r.downcast_ref::<opaque::Number>().is_some() {
        s.push_str("number");
        return Ok(());
    }

    if r.downcast_ref::<opaque::BigInt>().is_some() {
        s.push_str("bigint");
        return Ok(());
    }

    if let Some(def) = r.downcast_ref::<Branded>() {
        if let Some(branded_type) = exporter
            .branded_type_impl
            .as_ref()
            .map(|builder| {
                (builder.0)(
                    BrandedTypeExporter {
                        exporter,
                        format,
                        types,
                    },
                    def,
                )
            })
            .transpose()?
        {
            s.push_str(branded_type.as_ref());
            return Ok(());
        }

        match def.ty() {
            DataType::Reference(r) => {
                reference_dt(s, exporter, format, types, r, location.clone(), "", &[])?
            }
            ty => inline_datatype(
                s,
                exporter,
                format,
                types,
                ty,
                location.clone(),
                None,
                "",
                0,
                &[],
            )?,
        }
        s.push_str(r#" & { readonly __brand: ""#);
        s.push_str(&escape_typescript_string_literal(def.brand()));
        s.push_str("\" }");
        return Ok(());
    }

    Err(Error::unsupported_opaque_reference(
        path_string(&location),
        r.clone(),
    ))
}

fn reference_named_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    r: &NamedReference,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let path = path_string(&location);
    let ndt = types
        .get(r)
        .ok_or_else(|| Error::dangling_named_reference(path.clone(), format!("{r:?}")))?;
    // We check it's valid before tracking
    crate::references::track_nr(r);

    let name = referenced_type_name(exporter, ndt);

    let (rendered_generics, omit_generics, scoped_generics) =
        resolved_reference_generics(ndt, r, generics)
            .ok_or_else(|| Error::dangling_named_reference(path, format!("{r:?}")))?;

    s.push_str(&name);
    if !omit_generics && !rendered_generics.is_empty() {
        s.push('<');

        for (i, dt) in rendered_generics.iter().enumerate() {
            if i != 0 {
                s.push_str(", ");
            }

            datatype(
                s,
                exporter,
                None,
                types,
                dt,
                vec![],
                None,
                "",
                &scoped_generics,
            )?;
        }

        s.push('>');
    }

    Ok(())
}
