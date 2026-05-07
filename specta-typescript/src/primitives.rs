//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are for advanced usecases, you should generally use [crate::Typescript] or
//! [crate::JSDoc] in end-user applications.

use std::{borrow::Cow, collections::BTreeSet};

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

fn path_string(location: &[Cow<'static, str>]) -> String {
    location.join(".")
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

    let raw_name = match exporter.layout {
        Layout::ModulePrefixedName => {
            let mut s = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            s.push('_');
            s.push_str(&ndt.name);
            Cow::Owned(s)
        }
        _ => ndt.name.clone(),
    };
    let name = sanitise_type_name(&[], &raw_name)?;

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
    write_generic_parameters(s, exporter, types, &ndt.generics)?;
    s.push_str(" = ");

    datatype(
        s,
        exporter,
        format,
        types,
        ndt.ty.as_ref().expect("named datatype must have a body"),
        vec![ndt.name.clone()],
        Some(ndt.name.as_ref()),
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
                enum_dt(&mut variant_ty, exporter, types, &one_variant_enum, "", &[])?;

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
    write_generic_parameters(&mut type_name, exporter, types, &dt.generics)?;

    let mut typedef_ty = String::new();
    let datatype_prefix = format!("{indent}\t*\t");
    datatype(
        &mut typedef_ty,
        exporter,
        format,
        types,
        dt.ty.as_ref().expect("named datatype must have a body"),
        vec![dt.name.clone()],
        Some(dt.name.as_ref()),
        &datatype_prefix,
        Default::default(),
    )?;

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
        append_jsdoc_properties(s, exporter, format, types, dt.name.as_ref(), ty, indent)?;
    }

    Ok(())
}

fn write_generic_parameters(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
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
            shallow_inline_datatype(
                &mut rendered_default,
                exporter,
                None,
                types,
                default,
                Vec::new(),
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

/// Generate an Typescript string to refer to a specific [`DataType`].
///
/// For primitives this will include the literal type but for named type it will contain a reference.
///
/// See [`export`] for the list of things to consider when using this.
pub fn reference(
    exporter: &dyn AsRef<Exporter>,
    types: &Types,
    r: &Reference,
) -> Result<String, Error> {
    let mut s = String::new();
    datatype(
        &mut s,
        exporter.as_ref(),
        None,
        types,
        &DataType::Reference(r.clone()),
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
    inline: bool,
) -> Result<(), Error> {
    if inline {
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
        );
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
        NamedReferenceType::Recursive => Ok(&[]),
    }
}

fn named_reference_ty<'a>(types: &'a Types, r: &'a NamedReference) -> Result<&'a DataType, Error> {
    match &r.inner {
        NamedReferenceType::Reference { .. } => types
            .get(r)
            .and_then(|ndt| ndt.ty.as_ref())
            .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}"))),
        NamedReferenceType::Inline { dt, .. } => Ok(dt),
        NamedReferenceType::Recursive => types
            .get(r)
            .and_then(|ndt| ndt.ty.as_ref())
            .ok_or_else(|| Error::infinite_recursive_inline_type(format!("{r:?}"))),
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

fn push_nullable(s: &mut String, inner: &str) {
    s.push_str(inner);
    if inner != NULL && !inner.ends_with(" | null") {
        s.push_str(" | null");
    }
}

fn is_exhaustive_map_key(dt: &DataType, types: &Types) -> bool {
    match dt {
        DataType::Enum(e) => e.variants.iter().filter(|(_, v)| !v.skip).count() == 0,
        DataType::Reference(Reference::Named(r)) => named_reference_ty(types, r)
            .map(|ty| is_exhaustive_map_key(ty, types))
            .unwrap_or(false),
        DataType::Reference(Reference::Opaque(_)) => false,
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
    let exhaustive = is_exhaustive_map_key(&rendered_key, types);

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
    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::Generic(g) => write_generic_reference(s, g),
        DataType::List(list) => {
            let mut inner = String::new();
            shallow_inline_datatype(
                &mut inner,
                exporter,
                format,
                types,
                &list.ty,
                location,
                parent_name,
                prefix,
                generics,
            )?;
            push_list(s, &inner, list.length);
        }
        DataType::Map(map) => render_map(
            s,
            exporter,
            format,
            types,
            map,
            location,
            parent_name,
            prefix,
            generics,
            RenderMode::ShallowInline,
        )?,
        DataType::Nullable(dt) => {
            let mut inner = String::new();
            shallow_inline_datatype(
                &mut inner,
                exporter,
                format,
                types,
                dt,
                location,
                parent_name,
                prefix,
                generics,
            )?;
            push_nullable(s, &inner);
        }
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
            RenderMode::ShallowInline,
        )?,
        DataType::Struct(st) => {
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
        DataType::Enum(enm) => {
            enum_dt(s, exporter, types, enm, prefix, generics)?;
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
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
                        format,
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
                let ty = named_reference_ty(types, r)?;
                let reference_generics = named_reference_generics(r)?;
                shallow_inline_datatype(
                    s,
                    exporter,
                    format,
                    types,
                    ty,
                    location,
                    parent_name,
                    prefix,
                    reference_generics,
                )
            }
            Reference::Opaque(_) => {
                reference_dt(s, exporter, format, types, r, location, prefix, generics)
            }
        }?,
    }

    Ok(())
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

// Internal function to handle inlining without cloning DataType nodes
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
        return Err(Error::invalid_name(
            location.join("."),
            "Type recursion limit exceeded during inline expansion",
        ));
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
                match &st.fields {
                    Fields::Unit => s.push_str(NULL),
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
                }
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
        DataType::Enum(e) => enum_dt(s, exporter, types, e, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, generics)?,
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
                && let Ok(ty) = named_reference_ty(types, r)
            {
                let reference_generics = named_reference_generics(r)?;
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
                )?;
            } else {
                reference_dt(s, exporter, format, types, r, location, prefix, generics)?;
            }
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
    // TODO: Validating the variant from `dt` can be flattened

    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
        DataType::List(l) => list_dt(s, exporter, types, l, generics)?,
        DataType::Map(m) => map_dt(s, exporter, format, types, m, location, generics)?,
        DataType::Nullable(def) => {
            let mut inner = String::new();
            datatype(
                &mut inner,
                exporter,
                format,
                types,
                def,
                location,
                parent_name,
                "",
                generics,
            )?;

            push_nullable(s, &inner);
        }
        DataType::Struct(st) => struct_dt(
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
        DataType::Enum(e) => enum_dt(s, exporter, types, e, prefix, generics)?,
        DataType::Tuple(t) => tuple_dt(s, exporter, types, t, generics)?,
        DataType::Intersection(types_) => {
            for (idx, ty) in types_.iter().enumerate() {
                if idx != 0 {
                    s.push_str(" & ");
                }
                datatype(
                    s,
                    exporter,
                    format,
                    types,
                    ty,
                    location.clone(),
                    parent_name,
                    prefix,
                    generics,
                )?;
            }
        }
        DataType::Generic(g) => write_generic_reference(s, g),
        DataType::Reference(r) => {
            reference_dt(s, exporter, format, types, r, location, prefix, generics)?
        }
    };

    Ok(())
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
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let mut dt = String::new();
    datatype(
        &mut dt,
        exporter,
        None,
        types,
        &l.ty,
        vec![],
        None,
        "",
        generics,
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
    location: Vec<Cow<'static, str>>,
    parent_name: Option<&str>,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    force_inline: bool,
) -> Result<(), Error> {
    match fields {
        [(field, ty)] => {
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
        Fields::Unit if name.is_empty() => Err(Error::invalid_name(
            path_string(&location),
            "anonymous unit enum variants cannot be exported to Typescript",
        )),
        Fields::Unit => Ok(Some(sanitise_key(name, true).to_string())),
        Fields::Named(_) if name.is_empty() => Err(Error::invalid_name(
            path_string(&location),
            "anonymous named-field enum variants cannot be exported to Typescript",
        )),
        Fields::Named(obj) => {
            let mut regular_fields = Vec::new();
            for (field_name, field) in &obj.fields {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };

                let mut other = String::new();
                let mut field_location = location.clone();
                field_location.push(field_name.clone());
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
            let fields = obj
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .enumerate()
                .map(|(idx, ty)| {
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
                    )
                    .map(|_| out)
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(match &fields[..] {
                [] if obj.fields.is_empty() => Some("[]".to_string()),
                [] => None,
                [field] if obj.fields.len() == 1 => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

fn enum_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    e: &Enum,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    if e.variants.is_empty() {
        s.push_str(NEVER);
        return Ok(());
    }

    let filtered_variants = active_variants(e);

    let discriminator = analyze_discriminator(&filtered_variants);
    let fallback_override = discriminator.as_ref().and_then(|discriminator| {
        discriminator.fallback_variant_idx.and_then(|idx| {
            exclude_known_literals_type(&discriminator.known_literals)
                .map(|ty| (idx, discriminator.key.as_str(), ty))
        })
    });

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

        let variant_location = vec![variant_name.clone()];
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

fn tuple_dt(
    s: &mut String,
    exporter: &Exporter,
    types: &Types,
    t: &Tuple,
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
                datatype(
                    s,
                    exporter,
                    None,
                    types,
                    dt,
                    vec![idx.to_string().into()],
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
                reference_named_dt(s, exporter, types, r, generics)
            }
            NamedReferenceType::Inline { dt, .. } => inline_datatype(
                s, exporter, format, types, dt, location, None, prefix, 0, generics,
            ),
            NamedReferenceType::Recursive => reference_named_dt(s, exporter, types, r, generics),
        },
        Reference::Opaque(r) => reference_opaque_dt(s, exporter, format, types, r),
    }
}

fn reference_opaque_dt(
    s: &mut String,
    exporter: &Exporter,
    format: Option<&dyn Format>,
    types: &Types,
    r: &OpaqueReference,
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

        // TODO: Build onto `s` instead of appending a separate string
        match def.ty() {
            DataType::Reference(r) => reference_dt(s, exporter, format, types, r, vec![], "", &[])?,
            ty => inline_datatype(s, exporter, format, types, ty, vec![], None, "", 0, &[])?,
        }
        s.push_str(r#" & { readonly __brand: ""#);
        s.push_str(&escape_typescript_string_literal(def.brand()));
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
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    let ndt = types
        .get(r)
        .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;
    // We check it's valid before tracking
    crate::references::track_nr(r);

    let name = match exporter.layout {
        Layout::ModulePrefixedName => {
            let mut s = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            s.push('_');
            s.push_str(&ndt.name);
            Cow::Owned(s)
        }
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
    };

    let (rendered_generics, omit_generics, scoped_generics) =
        resolved_reference_generics(ndt, r, generics)
            .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;

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
