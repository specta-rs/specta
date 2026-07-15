//! Low-level helpers for rendering individual Specta datatypes as Python.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashSet},
};

use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, Generic, GenericDefinition, NamedDataType,
        NamedReference, NamedReferenceType, OpaqueReference, Primitive, Reference, Struct,
    },
};

use crate::{Error, Layout, Python, opaque, reserved_names::RESERVED_NAMES};

type Location = Vec<Cow<'static, str>>;

#[derive(Clone, Copy)]
pub(crate) struct RenderContext<'a> {
    pub exporter: &'a Python,
    pub types: &'a Types,
    pub current_module: &'a str,
}

/// Renders top-level Python declarations for the supplied named datatypes.
///
/// This low-level function assumes the type graph has already been transformed by any desired
/// [`specta::Format`]. Most applications should use [`Python::export`](crate::Python::export).
pub fn export<'a>(
    exporter: &Python,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut out = String::new();
    for ndt in ndts {
        let declaration = export_named(
            RenderContext {
                exporter,
                types,
                current_module: "",
            },
            ndt,
            indent,
        )?;
        if declaration.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&declaration);
    }
    Ok(out)
}

/// Renders a datatype inline as a Python type expression.
pub fn inline(exporter: &Python, types: &Types, datatype: &DataType) -> Result<String, Error> {
    datatype_to_python(
        RenderContext {
            exporter,
            types,
            current_module: "",
        },
        datatype,
        Vec::new(),
        &[],
    )
}

/// Renders a Specta reference as a Python type expression.
pub fn reference(exporter: &Python, types: &Types, reference: &Reference) -> Result<String, Error> {
    datatype_to_python(
        RenderContext {
            exporter,
            types,
            current_module: "",
        },
        &DataType::Reference(reference.clone()),
        Vec::new(),
        &[],
    )
}

pub(crate) fn export_named(
    ctx: RenderContext<'_>,
    ndt: &NamedDataType,
    indent: &str,
) -> Result<String, Error> {
    export_named_inner(ctx, ndt, indent).map_err(|error| error.with_named_datatype(ndt))
}

fn export_named_inner(
    ctx: RenderContext<'_>,
    ndt: &NamedDataType,
    indent: &str,
) -> Result<String, Error> {
    let Some(ty) = ndt.ty.as_ref() else {
        return Ok(String::new());
    };
    let name = exported_type_name(ctx.exporter, ndt);
    validate_identifier(&name, &rust_type_path(ndt))?;

    let mut out = String::new();
    comments(&mut out, indent, &ndt.docs, ndt.deprecated.as_ref());

    let location = vec![rust_type_path(ndt)];
    let generics = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();

    if let DataType::Struct(Struct {
        fields: Fields::Named(fields),
        ..
    }) = ty
        && fields
            .fields
            .iter()
            .filter(|(_, field)| field.ty.is_some())
            .all(|(field_name, _)| {
                is_identifier(field_name)
                    && !is_reserved(field_name)
                    && !field_name.starts_with("__")
                    && field_name.as_ref() == normalized_identifier(field_name)
            })
    {
        out.push_str(indent);
        out.push_str("class ");
        out.push_str(&name);
        write_generic_parameters(&mut out, ctx, &ndt.generics, &location)?;
        out.push_str("(_specta_typing.TypedDict):\n");

        let mut has_fields = false;
        for (field_name, field) in &fields.fields {
            let Some(ty) = field.ty.as_ref() else {
                continue;
            };
            has_fields = true;
            let field_indent = format!("{indent}    ");
            comments(
                &mut out,
                &field_indent,
                &field.docs,
                field.deprecated.as_ref(),
            );
            out.push_str(&field_indent);
            out.push_str(field_name);
            out.push_str(": ");
            let mut field_location = location.clone();
            field_location.push(field_name.clone());
            let rendered = datatype_to_python(ctx, ty, field_location, &generics)?;
            if field.optional {
                out.push_str("_specta_typing.NotRequired[");
                out.push_str(&rendered);
                out.push(']');
            } else {
                out.push_str(&rendered);
            }
            out.push('\n');
        }
        if !has_fields {
            out.push_str(indent);
            out.push_str("    pass\n");
        }
        return Ok(out);
    }

    member_comments(&mut out, indent, ty);
    out.push_str(indent);
    out.push_str("type ");
    out.push_str(&name);
    write_generic_parameters(&mut out, ctx, &ndt.generics, &location)?;
    out.push_str(" = ");
    out.push_str(&datatype_to_python(ctx, ty, location, &generics)?);
    out.push('\n');
    Ok(out)
}

fn member_comments(out: &mut String, indent: &str, datatype: &DataType) {
    fn field_comment(out: &mut String, indent: &str, label: &str, field: &Field) {
        for line in field.docs.lines() {
            out.push_str(indent);
            out.push_str("# ");
            out.push_str(label);
            out.push_str(": ");
            out.push_str(line);
            out.push('\n');
        }
        if let Some(deprecated) = &field.deprecated {
            out.push_str(indent);
            out.push_str("# ");
            out.push_str(label);
            out.push_str(" is deprecated");
            if let Some(note) = deprecated
                .note
                .as_deref()
                .map(str::trim)
                .filter(|note| !note.is_empty())
            {
                out.push_str(": ");
                out.push_str(note);
            }
            out.push('\n');
        }
    }

    fn fields(out: &mut String, indent: &str, fields: &Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for (index, field) in fields.fields.iter().enumerate() {
                    field_comment(out, indent, &format!("Field {index}"), field);
                }
            }
            Fields::Named(fields) => {
                for (name, field) in &fields.fields {
                    field_comment(out, indent, &format!("Field {name}"), field);
                }
            }
        }
    }

    match datatype {
        DataType::Struct(strct) => fields(out, indent, &strct.fields),
        DataType::Enum(enm) => {
            for (name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                for line in variant.docs.lines() {
                    out.push_str(indent);
                    out.push_str("# Variant ");
                    out.push_str(name);
                    out.push_str(": ");
                    out.push_str(line);
                    out.push('\n');
                }
                if let Some(deprecated) = &variant.deprecated {
                    out.push_str(indent);
                    out.push_str("# Variant ");
                    out.push_str(name);
                    out.push_str(" is deprecated");
                    if let Some(note) = deprecated
                        .note
                        .as_deref()
                        .map(str::trim)
                        .filter(|note| !note.is_empty())
                    {
                        out.push_str(": ");
                        out.push_str(note);
                    }
                    out.push('\n');
                }
                fields(out, indent, &variant.fields);
            }
        }
        _ => {}
    }
}

fn write_generic_parameters(
    out: &mut String,
    ctx: RenderContext<'_>,
    generics: &[GenericDefinition],
    location: &Location,
) -> Result<(), Error> {
    if generics.is_empty() {
        return Ok(());
    }
    out.push('[');
    for (index, generic) in generics.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        validate_identifier(&generic.name, "generic parameter")?;
        out.push_str(&generic.name);
        if let Some(default) = &generic.default {
            let scoped_generics = generics[..index]
                .iter()
                .map(GenericDefinition::reference)
                .collect::<Vec<_>>();
            let mut default_location = location.clone();
            default_location.push(format!("<generic {} default>", generic.name).into());
            out.push_str(" = ");
            out.push_str(&datatype_to_python(
                ctx,
                default,
                default_location,
                &scoped_generics,
            )?);
        }
    }
    out.push(']');
    Ok(())
}

fn datatype_to_python(
    ctx: RenderContext<'_>,
    datatype: &DataType,
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    match datatype {
        DataType::Primitive(primitive) => Ok(primitive_to_python(primitive).to_string()),
        DataType::List(list) => {
            let inner = datatype_to_python(ctx, &list.ty, location, generics)?;
            Ok(match list.length {
                Some(0) => "_specta_builtins.tuple[()]".to_string(),
                Some(length) => {
                    format!("_specta_builtins.tuple[{}]", vec![inner; length].join(", "))
                }
                None => format!("_specta_builtins.list[{inner}]"),
            })
        }
        DataType::Map(map) => {
            let key = datatype_to_python(ctx, map.key_ty(), location.clone(), generics)?;
            let value = datatype_to_python(ctx, map.value_ty(), location, generics)?;
            Ok(format!("_specta_builtins.dict[{key}, {value}]"))
        }
        DataType::Struct(strct) => struct_to_python(ctx, strct, location, generics),
        DataType::Enum(enm) => enum_to_python(ctx, enm, location, generics),
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => Ok("None".to_string()),
            elements => Ok(format!(
                "_specta_builtins.tuple[{}]",
                elements
                    .iter()
                    .enumerate()
                    .map(|(index, ty)| {
                        let mut location = location.clone();
                        location.push(index.to_string().into());
                        datatype_to_python(ctx, ty, location, generics)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
        },
        DataType::Nullable(inner) => Ok(join_union([
            datatype_to_python(ctx, inner, location, generics)?,
            "None".to_string(),
        ])),
        DataType::Intersection(parts) => intersection_to_python(ctx, parts, location, generics),
        DataType::Generic(generic) => {
            if !generics.iter().any(|candidate| candidate == generic) {
                return Err(Error::dangling_reference(
                    crate::error::display_path(&location),
                    format!("generic {}", generic.name()),
                ));
            }
            validate_identifier(generic.name(), &crate::error::display_path(&location))?;
            Ok(generic.name().to_string())
        }
        DataType::Reference(reference) => reference_to_python(ctx, reference, location, generics),
    }
}

fn primitive_to_python(primitive: &Primitive) -> &'static str {
    use Primitive::*;
    match primitive {
        i8 | i16 | i32 | i64 | i128 | isize | u8 | u16 | u32 | u64 | u128 | usize => {
            "_specta_builtins.int"
        }
        f16 | f32 | f64 | f128 => "_specta_builtins.float",
        bool => "_specta_builtins.bool",
        str | char => "_specta_builtins.str",
    }
}

fn struct_to_python(
    ctx: RenderContext<'_>,
    strct: &Struct,
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    match &strct.fields {
        Fields::Unit => Ok("None".to_string()),
        Fields::Unnamed(fields) => {
            unnamed_fields_to_python(ctx, &fields.fields, location, generics)
        }
        Fields::Named(fields) => typed_dict_expression(
            ctx,
            fields
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name.as_ref(), field, ty))),
            location,
            generics,
        ),
    }
}

fn unnamed_fields_to_python(
    ctx: RenderContext<'_>,
    fields: &[Field],
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    let live = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| field.ty.as_ref().map(|ty| (index, field, ty)))
        .collect::<Vec<_>>();
    if fields.len() == 1
        && let [(_, _, ty)] = live.as_slice()
    {
        return datatype_to_python(ctx, ty, location, generics);
    }
    if live.is_empty() {
        return Ok("_specta_builtins.tuple[()]".to_string());
    }

    let optional_start = live
        .iter()
        .rposition(|(_, field, _)| !field.optional)
        .map_or(0, |index| index + 1);
    let rendered = live
        .iter()
        .enumerate()
        .map(|(live_index, (source_index, _, ty))| {
            let mut location = location.clone();
            location.push(source_index.to_string().into());
            datatype_to_python(ctx, ty, location, generics).map(|ty| (live_index, ty))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Python has no optional tuple element syntax. A union of tuple lengths precisely models a
    // trailing run of fields defaulted by the serialization format.
    Ok((optional_start..=rendered.len())
        .map(|length| {
            if length == 0 {
                "_specta_builtins.tuple[()]".to_string()
            } else {
                format!(
                    "_specta_builtins.tuple[{}]",
                    rendered[..length]
                        .iter()
                        .map(|(_, ty)| ty.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" | "))
}

fn enum_to_python(
    ctx: RenderContext<'_>,
    enm: &Enum,
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    let mut variants = BTreeSet::new();
    for (name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
        let mut variant_location = location.clone();
        variant_location.push(name.clone());
        let rendered = match &variant.fields {
            Fields::Unit if name.is_empty() => {
                return Err(Error::invalid_name(
                    crate::error::display_path(&variant_location),
                    "",
                ));
            }
            Fields::Unit => format!("_specta_typing.Literal[{}]", python_string(name)),
            Fields::Unnamed(fields) => {
                unnamed_fields_to_python(ctx, &fields.fields, variant_location, generics)?
            }
            Fields::Named(fields) => typed_dict_expression(
                ctx,
                fields.fields.iter().filter_map(|(name, field)| {
                    field.ty.as_ref().map(|ty| (name.as_ref(), field, ty))
                }),
                variant_location,
                generics,
            )?,
        };
        variants.insert(rendered);
    }
    if variants.is_empty() {
        Ok("_specta_typing.Never".to_string())
    } else {
        Ok(join_union(variants))
    }
}

fn typed_dict_expression<'a>(
    ctx: RenderContext<'_>,
    fields: impl Iterator<Item = (&'a str, &'a Field, &'a DataType)>,
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    let name = anonymous_type_name(&location);
    let fields = fields
        .map(|(name, field, ty)| {
            let mut field_location = location.clone();
            field_location.push(Cow::Owned(name.to_string()));
            let ty = datatype_to_python(ctx, ty, field_location, generics)?;
            Ok(format!(
                "{}: {}",
                python_string(name),
                if field.optional {
                    format!("_specta_typing.NotRequired[{ty}]")
                } else {
                    ty
                }
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(format!(
        "_specta_typing.TypedDict({}, {{{}}})",
        python_string(&name),
        fields.join(", ")
    ))
}

fn intersection_to_python(
    ctx: RenderContext<'_>,
    parts: &[DataType],
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    if let [part] = parts {
        return datatype_to_python(ctx, part, location, generics);
    }

    let mut visiting = HashSet::new();
    let variants = match intersection_variants(ctx, parts, &location, &mut visiting) {
        Ok(variants) => variants,
        Err(_) => {
            // Python's typing syntax has no general intersection operator. Intersections of
            // records are distributed and merged above. For a record intersected with exactly
            // one collection type (for example an internally tagged map), the collection is the
            // narrowest representable Python supertype. Other mixed intersections fail loudly.
            let non_objects = parts
                .iter()
                .filter(|part| object_variants(ctx, part, &location, &mut HashSet::new()).is_err())
                .collect::<Vec<_>>();
            if let [part] = non_objects.as_slice() {
                return datatype_to_python(ctx, part, location, generics);
            }
            return Err(Error::unrepresentable_intersection(
                crate::error::display_path(&location),
            ));
        }
    };
    if variants.is_empty() {
        return Err(Error::unrepresentable_intersection(
            crate::error::display_path(&location),
        ));
    }

    variants
        .into_iter()
        .enumerate()
        .map(|(index, fields)| {
            typed_dict_expression(
                ctx,
                fields
                    .iter()
                    .map(|(name, (field, ty))| (name.as_str(), field, ty)),
                {
                    let mut location = location.clone();
                    location.push(format!("intersection{index}").into());
                    location
                },
                generics,
            )
        })
        .collect::<Result<BTreeSet<_>, _>>()
        .map(join_union)
}

fn join_union(types: impl IntoIterator<Item = String>) -> String {
    fn parts(ty: String) -> Vec<String> {
        let chars = ty.char_indices().collect::<Vec<_>>();
        let mut depth = 0_usize;
        let mut quote = None;
        let mut escaped = false;
        let mut start = 0_usize;
        let mut output = Vec::new();
        let mut index = 0_usize;
        while index < chars.len() {
            let (byte, character) = chars[index];
            if let Some(delimiter) = quote {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == delimiter {
                    quote = None;
                }
                index += 1;
                continue;
            }
            match character {
                '"' | '\'' => quote = Some(character),
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => depth = depth.saturating_sub(1),
                '|' if depth == 0 => {
                    output.push(ty[start..byte].trim().to_string());
                    start = byte + character.len_utf8();
                }
                _ => {}
            }
            index += 1;
        }
        output.push(ty[start..].trim().to_string());
        output
    }

    types
        .into_iter()
        .flat_map(parts)
        .filter(|part| !part.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" | ")
}

type ObjectFields = BTreeMap<String, (Field, DataType)>;

fn intersection_variants(
    ctx: RenderContext<'_>,
    parts: &[DataType],
    location: &Location,
    visiting: &mut HashSet<NamedReference>,
) -> Result<Vec<ObjectFields>, Error> {
    let mut product = vec![ObjectFields::new()];
    for part in parts {
        let alternatives = object_variants(ctx, part, location, visiting)?;
        if alternatives.is_empty() {
            return Ok(Vec::new());
        }
        let mut next = Vec::new();
        for existing in &product {
            for alternative in &alternatives {
                let mut merged = existing.clone();
                let mut incompatible = false;
                for (name, value) in alternative {
                    if let Some((existing_field, existing_ty)) = merged.get_mut(name) {
                        if existing_ty != &value.1 {
                            incompatible = true;
                            break;
                        }
                        existing_field.optional &= value.0.optional;
                    } else {
                        merged.insert(name.clone(), value.clone());
                    }
                }
                if !incompatible {
                    next.push(merged);
                }
            }
        }
        product = next;
    }
    Ok(product)
}

fn object_variants(
    ctx: RenderContext<'_>,
    datatype: &DataType,
    location: &Location,
    visiting: &mut HashSet<NamedReference>,
) -> Result<Vec<ObjectFields>, Error> {
    match datatype {
        DataType::Struct(Struct {
            fields: Fields::Named(fields),
            ..
        }) => Ok(vec![
            fields
                .fields
                .iter()
                .filter_map(|(name, field)| {
                    field
                        .ty
                        .as_ref()
                        .map(|ty| (name.to_string(), (field.clone(), ty.clone())))
                })
                .collect(),
        ]),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .map(|(_, variant)| match &variant.fields {
                Fields::Named(fields) => Ok(fields
                    .fields
                    .iter()
                    .filter_map(|(name, field)| {
                        field
                            .ty
                            .as_ref()
                            .map(|ty| (name.to_string(), (field.clone(), ty.clone())))
                    })
                    .collect()),
                _ => Err(Error::unrepresentable_intersection(
                    crate::error::display_path(location),
                )),
            })
            .collect(),
        DataType::Intersection(parts) => intersection_variants(ctx, parts, location, visiting),
        DataType::Reference(Reference::Named(reference)) => {
            if !visiting.insert(reference.clone()) {
                // A recursive flattened object contributes fields already seen earlier in the
                // expansion. Treating this edge as an empty object computes the finite least
                // fixed point while preserving the recursive named alias at its direct use site.
                return Ok(vec![ObjectFields::new()]);
            }
            let result = match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    object_variants(ctx, dt, location, visiting)
                }
                NamedReferenceType::Reference { generics, .. } => {
                    let ndt = ctx.types.get(reference).ok_or_else(|| {
                        Error::dangling_reference(
                            crate::error::display_path(location),
                            format!("{reference:?}"),
                        )
                    })?;
                    let mut ty = ndt.ty.clone().ok_or_else(|| {
                        Error::dangling_reference(
                            crate::error::display_path(location),
                            format!("{reference:?}"),
                        )
                    })?;
                    let mut resolved = Vec::with_capacity(ndt.generics.len());
                    for definition in ndt.generics.iter() {
                        let mut argument = generics
                            .iter()
                            .find(|(generic, _)| generic == &definition.reference())
                            .map(|(_, datatype)| datatype.clone())
                            .or_else(|| definition.default.clone())
                            .unwrap_or_else(|| {
                                DataType::Reference(Reference::opaque(opaque::Unknown))
                            });
                        substitute_generics(&mut argument, &resolved);
                        resolved.push((definition.reference(), argument));
                    }
                    substitute_generics(&mut ty, &resolved);
                    object_variants(ctx, &ty, location, visiting)
                }
                NamedReferenceType::Recursive(_) => Err(Error::unrepresentable_intersection(
                    crate::error::display_path(location),
                )),
            };
            visiting.remove(reference);
            result
        }
        _ => Err(Error::unrepresentable_intersection(
            crate::error::display_path(location),
        )),
    }
}

fn substitute_generics(datatype: &mut DataType, generics: &[(Generic, DataType)]) {
    match datatype {
        DataType::Generic(generic) => {
            if let Some((_, replacement)) = generics.iter().find(|(name, _)| name == generic) {
                *datatype = replacement.clone();
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Struct(strct) => substitute_fields(&mut strct.fields, generics),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_fields(&mut variant.fields, generics);
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                substitute_generics(element, generics);
            }
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Intersection(parts) => {
            for part in parts {
                substitute_generics(part, generics);
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } = &mut reference.inner
            {
                for (_, ty) in reference_generics {
                    substitute_generics(ty, generics);
                }
            }
        }
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_fields(fields: &mut Fields, generics: &[(Generic, DataType)]) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
    }
}

fn reference_to_python(
    ctx: RenderContext<'_>,
    reference: &Reference,
    location: Location,
    generics: &[Generic],
) -> Result<String, Error> {
    match reference {
        Reference::Named(reference) => match &reference.inner {
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => {
                let ndt = ctx.types.get(reference).ok_or_else(|| {
                    Error::dangling_reference(
                        crate::error::display_path(&location),
                        format!("{reference:?}"),
                    )
                })?;
                if ndt.ty.is_none() {
                    return Err(Error::dangling_reference(
                        crate::error::display_path(&location),
                        format!("{reference:?}"),
                    ));
                }
                let name = referenced_type_name(ctx, ndt);
                if ndt.generics.is_empty() {
                    return Ok(name);
                }
                let mut scoped = Vec::<(Generic, DataType)>::new();
                let mut all_default = true;
                let arguments = ndt
                    .generics
                    .iter()
                    .map(|definition| {
                        let explicit = reference_generics
                            .iter()
                            .find(|(generic, _)| generic == &definition.reference())
                            .map(|(_, datatype)| datatype.clone());
                        let default = definition.default.as_ref().map(|default| {
                            let mut default = default.clone();
                            substitute_generics(&mut default, &scoped);
                            default
                        });
                        let resolved = explicit.or_else(|| default.clone()).unwrap_or_else(|| {
                            DataType::Reference(Reference::opaque(opaque::Unknown))
                        });
                        all_default &= default.as_ref().is_some_and(|default| default == &resolved);
                        scoped.push((definition.reference(), resolved.clone()));
                        datatype_to_python(ctx, &resolved, location.clone(), generics)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if all_default {
                    Ok(name)
                } else {
                    Ok(format!("{name}[{}]", arguments.join(", ")))
                }
            }
            NamedReferenceType::Inline { dt, .. } => {
                let path = crate::error::display_path(&location);
                datatype_to_python(ctx, dt, location, generics)
                    .map_err(|error| error.with_inline_trace(ctx.types.get(reference), path))
            }
            NamedReferenceType::Recursive(cycle) => Err(Error::recursive_inline(
                crate::error::display_path(&location),
                cycle.clone(),
            )),
        },
        Reference::Opaque(reference) => opaque_to_python(reference, &location),
    }
}

fn opaque_to_python(reference: &OpaqueReference, location: &Location) -> Result<String, Error> {
    if let Some(define) = reference.downcast_ref::<opaque::Define>() {
        return Ok(define.0.to_string());
    }
    if reference.downcast_ref::<opaque::Any>().is_some()
        || reference.downcast_ref::<opaque::Unknown>().is_some()
    {
        return Ok("_specta_typing.Any".to_string());
    }
    if reference.downcast_ref::<opaque::Never>().is_some() {
        return Ok("_specta_typing.Never".to_string());
    }
    Err(Error::unsupported_opaque_reference(
        crate::error::display_path(location),
        reference.clone(),
    ))
}

pub(crate) fn exported_type_name(exporter: &Python, ndt: &NamedDataType) -> String {
    match exporter.layout {
        Layout::ModulePrefixedName => module_prefixed_name(ndt),
        _ => ndt.name.to_string(),
    }
}

fn referenced_type_name(ctx: RenderContext<'_>, ndt: &NamedDataType) -> String {
    match ctx.exporter.layout {
        Layout::ModulePrefixedName => module_prefixed_name(ndt),
        Layout::Namespaces if !ndt.module_path.is_empty() => {
            format!("{}.{}", ndt.module_path.replace("::", "."), ndt.name)
        }
        Layout::Files if ndt.module_path != ctx.current_module => {
            file_import_alias(&ndt.module_path, &ndt.name)
        }
        _ => ndt.name.to_string(),
    }
}

pub(crate) fn file_import_alias(module_path: &str, name: &str) -> String {
    let module = if module_path.is_empty() {
        "0_root".to_string()
    } else {
        module_path
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(normalized_identifier)
            .map(|segment| format!("{}_{}", segment.chars().count(), segment))
            .collect::<Vec<_>>()
            .join("_")
    };
    format!("_specta_import_{module}_{name}")
}

fn module_prefixed_name(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}_{}", ndt.module_path.replace("::", "_"), ndt.name)
    }
}

fn rust_type_path(ndt: &NamedDataType) -> Cow<'static, str> {
    if ndt.module_path.is_empty() {
        ndt.name.clone()
    } else {
        Cow::Owned(format!("{}::{}", ndt.module_path, ndt.name))
    }
}

fn comments(out: &mut String, indent: &str, docs: &str, deprecated: Option<&Deprecated>) {
    for line in docs.lines() {
        out.push_str(indent);
        out.push_str("# ");
        out.push_str(line);
        out.push('\n');
    }
    if let Some(deprecated) = deprecated {
        out.push_str(indent);
        out.push_str("# Deprecated");
        if let Some(note) = deprecated
            .note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            out.push_str(": ");
            out.push_str(note);
        }
        out.push('\n');
    }
}

pub(crate) fn is_identifier(name: &str) -> bool {
    let normalized = normalized_identifier(name);
    let mut chars = normalized.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || unicode_ident::is_xid_start(first))
        && chars.all(|character| character == '_' || unicode_ident::is_xid_continue(character))
}

pub(crate) fn normalized_identifier(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    name.nfkc().collect()
}

fn is_reserved(name: &str) -> bool {
    let normalized = normalized_identifier(name);
    RESERVED_NAMES.contains(&normalized.as_str())
}

pub(crate) fn validate_identifier(name: &str, path: &str) -> Result<(), Error> {
    let normalized = normalized_identifier(name);
    if normalized.starts_with("_specta_") {
        return Err(Error::invalid_name(path.to_string(), name));
    }
    if let Some(reserved) = RESERVED_NAMES
        .iter()
        .find(|reserved| **reserved == normalized)
    {
        return Err(Error::forbidden_name(path.to_string(), reserved));
    }
    if !is_identifier(name) {
        return Err(Error::invalid_name(path.to_string(), name));
    }
    Ok(())
}

fn anonymous_type_name(location: &Location) -> String {
    let mut name = location
        .iter()
        .flat_map(|part| part.chars())
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if name.is_empty() || name.starts_with(|character: char| character.is_numeric()) {
        name.insert(0, '_');
    }
    name
}

pub(crate) fn python_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04X}", character as u32);
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}
