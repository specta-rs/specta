//! Low-level helpers for generating Go declarations and anonymous types.
//!
//! These helpers do not apply a [`specta::Format`]. Prefer [`crate::Go`] unless
//! the caller has already mapped both the type graph and top-level datatype.

use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
};

use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Enum, Fields, Generic, NamedDataType, NamedReferenceType, Primitive,
        Reference, Struct,
    },
};

use crate::{Error, Go};

#[derive(Default)]
struct Context {
    imports: BTreeSet<&'static str>,
}

/// Generates declarations for a collection of named datatypes.
pub fn export<'a>(
    exporter: &Go,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
) -> Result<String, Error> {
    let mut ctx = Context::default();
    let mut declarations = Vec::new();
    let mut names = BTreeSet::new();
    let ndts = ndts.filter(|ndt| ndt.ty.is_some()).collect::<Vec<_>>();

    for ndt in &ndts {
        let name = exported_name(&ndt.name, &rust_type_path(ndt))?;
        if !names.insert(name.clone()) {
            return Err(Error::DuplicateName {
                path: "package scope".into(),
                name,
            });
        }
        if ndt.generics.is_empty()
            && let Some(DataType::Enum(enm)) = &ndt.ty
            && let Some(variants) = string_enum_variants(enm)
        {
            for (index, (variant, _)) in variants.into_iter().enumerate() {
                let constant = format!(
                    "{name}{}",
                    enum_constant_suffix(variant, index, &rust_type_path(ndt))
                );
                if !names.insert(constant.clone()) {
                    return Err(Error::DuplicateName {
                        path: "package scope".into(),
                        name: constant,
                    });
                }
            }
        }
    }
    for ndt in ndts {
        declarations.push(named_datatype(exporter, types, ndt, &mut ctx)?);
    }

    let mut out = String::new();
    if !ctx.imports.is_empty() {
        if ctx.imports.len() == 1 {
            out.push_str("import \"");
            out.push_str(ctx.imports.iter().next().expect("non-empty import set"));
            out.push_str("\"\n\n");
        } else {
            out.push_str("import (\n");
            for import in ctx.imports {
                out.push_str("\t\"");
                out.push_str(import);
                out.push_str("\"\n");
            }
            out.push_str(")\n\n");
        }
    }
    out.push_str(&declarations.join("\n"));
    Ok(out)
}

/// Generates an anonymous Go type for a datatype.
pub fn inline(exporter: &Go, types: &Types, dt: &DataType) -> Result<String, Error> {
    let mut ctx = Context::default();
    render_datatype(exporter, types, dt, &[], &mut Vec::new(), false, &mut ctx)
}

/// Generates a Go type for a Specta reference.
pub fn reference(exporter: &Go, types: &Types, r: &Reference) -> Result<String, Error> {
    inline(exporter, types, &DataType::Reference(r.clone()))
}

pub(crate) fn file_name(ndt: &NamedDataType) -> Result<String, Error> {
    let name = exported_name(&ndt.name, &rust_type_path(ndt))?;
    Ok(format!("{}.go", to_snake_case(&name)))
}

fn named_datatype(
    exporter: &Go,
    types: &Types,
    ndt: &NamedDataType,
    ctx: &mut Context,
) -> Result<String, Error> {
    let name = exported_name(&ndt.name, &rust_type_path(ndt))?;
    let Some(ty) = &ndt.ty else {
        return Ok(String::new());
    };
    let generics = ndt
        .generics
        .iter()
        .map(|generic| (generic.reference(), generic.name.clone()))
        .collect::<Vec<_>>();
    let path = vec![rust_type_path(ndt)];

    if is_bare_generic_newtype(ty) {
        return Err(Error::UnsupportedType {
            path: rust_type_path(ndt),
            reason: "Go does not permit a type parameter as the underlying type of a defined type"
                .into(),
        });
    }

    let mut out = String::new();
    write_doc_comment(
        &mut out,
        "",
        Some(&name),
        &ndt.docs,
        ndt.deprecated.as_ref(),
    );

    if ndt.generics.is_empty()
        && let DataType::Enum(enm) = ty
        && string_enum_variants(enm).is_some()
    {
        render_string_enum(&mut out, &name, ndt, enm)?;
        return Ok(out);
    }

    let rendered = render_datatype(
        exporter,
        types,
        ty,
        &generics,
        &mut path.clone(),
        false,
        ctx,
    )?;
    out.push_str("type ");
    out.push_str(&name);
    if is_method_backed_newtype(types, ty) {
        if !generics.is_empty() {
            return Err(Error::UnsupportedType {
                path: rust_type_path(ndt),
                reason: "Go cannot preserve JSON methods through a generic type alias".into(),
            });
        }
        out.push_str(" = ");
    } else {
        write_generic_definitions(&mut out, &generics, &path)?;
        out.push(' ');
    }
    out.push_str(&rendered);
    out.push('\n');
    Ok(out)
}

fn is_method_backed_newtype(types: &Types, dt: &DataType) -> bool {
    let DataType::Struct(strct) = dt else {
        return false;
    };
    let Fields::Unnamed(fields) = &strct.fields else {
        return false;
    };
    let [field] = fields.fields.as_slice() else {
        return false;
    };
    field
        .ty
        .as_ref()
        .is_some_and(|dt| datatype_relies_on_json_methods(types, dt, &mut BTreeSet::new()))
}

fn datatype_relies_on_json_methods(
    types: &Types,
    dt: &DataType,
    visited: &mut BTreeSet<String>,
) -> bool {
    match dt {
        DataType::Primitive(Primitive::i128 | Primitive::u128) => true,
        DataType::Nullable(inner) => datatype_relies_on_json_methods(types, inner, visited),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unnamed(fields) => matches!(
                fields.fields.as_slice(),
                [field] if field.ty.as_ref().is_some_and(|dt| {
                    datatype_relies_on_json_methods(types, dt, visited)
                })
            ),
            Fields::Unit | Fields::Named(_) => false,
        },
        DataType::Reference(Reference::Opaque(opaque)) => {
            let name = opaque.type_name();
            name.ends_with("SystemTime") || name.contains("DateTime")
        }
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                datatype_relies_on_json_methods(types, dt, visited)
            }
            NamedReferenceType::Reference {
                generics: arguments,
                ..
            } => {
                let Some(ndt) = types.get(reference) else {
                    return false;
                };
                let type_path = rust_type_path(ndt);
                if !visited.insert(type_path.clone()) {
                    return false;
                }
                let result = ndt.ty.as_ref().is_some_and(|ty| {
                    let mut ty = ty.clone();
                    substitute_generics(&mut ty, &resolve_reference_arguments(ndt, arguments));
                    datatype_relies_on_json_methods(types, &ty, visited)
                });
                visited.remove(&type_path);
                result
            }
            NamedReferenceType::Recursive(_) => false,
        },
        DataType::List(_)
        | DataType::Map(_)
        | DataType::Tuple(_)
        | DataType::Enum(_)
        | DataType::Intersection(_)
        | DataType::Generic(_)
        | DataType::Primitive(_) => false,
    }
}

fn render_string_enum(
    out: &mut String,
    name: &str,
    ndt: &NamedDataType,
    enm: &Enum,
) -> Result<(), Error> {
    out.push_str("type ");
    out.push_str(name);
    out.push_str(" string\n\nconst (\n");
    let mut names = BTreeSet::new();
    let mut rows = Vec::new();
    for (index, (variant, metadata)) in string_enum_variants(enm)
        .expect("checked string enum")
        .into_iter()
        .enumerate()
    {
        let variant_name = enum_constant_suffix(variant, index, &rust_type_path(ndt));
        let constant = format!("{name}{variant_name}");
        if !names.insert(constant.clone()) {
            return Err(Error::DuplicateName {
                path: rust_type_path(ndt),
                name: constant,
            });
        }
        let mut comments = String::new();
        write_doc_comment(
            &mut comments,
            "\t",
            Some(&constant),
            &metadata.docs,
            metadata.deprecated.as_ref(),
        );
        if !comments.is_empty() {
            push_aligned_rows(out, &mut rows);
            out.push_str(&comments);
        }
        rows.push((
            constant,
            name.to_string(),
            format!("= \"{}\"", escape_go_string(variant)),
        ));
    }
    push_aligned_rows(out, &mut rows);
    out.push_str(")\n");
    Ok(())
}

fn render_datatype(
    exporter: &Go,
    types: &Types,
    dt: &DataType,
    generics: &[(Generic, Cow<'static, str>)],
    path: &mut Vec<String>,
    direct_field: bool,
    ctx: &mut Context,
) -> Result<String, Error> {
    Ok(match dt {
        DataType::Primitive(primitive) => primitive_type(primitive, ctx),
        DataType::Nullable(inner) => {
            let inner = render_datatype(exporter, types, inner, generics, path, false, ctx)?;
            if can_be_nil(inner.as_str()) {
                inner
            } else {
                format!("*{inner}")
            }
        }
        DataType::List(list) => format!(
            "[]{}",
            render_datatype(exporter, types, &list.ty, generics, path, false, ctx)?
        ),
        DataType::Map(map) => {
            let key = render_map_key(exporter, types, map.key_ty(), generics, path, ctx)?;
            let value =
                render_datatype(exporter, types, map.value_ty(), generics, path, false, ctx)?;
            format!("map[{key}]{value}")
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => "[0]any".into(),
            elements => {
                let mut discarded_ctx = Context::default();
                for (index, element) in elements.iter().enumerate() {
                    path.push(index.to_string());
                    let _ = render_datatype(
                        exporter,
                        types,
                        element,
                        generics,
                        path,
                        false,
                        &mut discarded_ctx,
                    )?;
                    path.pop();
                }
                format!("[{}]any", elements.len())
            }
        },
        DataType::Struct(strct) => render_struct(exporter, types, strct, generics, path, ctx)?,
        DataType::Enum(enm) if string_enum_variants(enm).is_some() => "string".into(),
        // Go has no structural union type. `any` is the only type that accepts
        // every externally, internally, adjacently, and untagged Serde shape.
        DataType::Enum(_) => "any".into(),
        DataType::Intersection(elements) => {
            render_intersection(exporter, types, elements, generics, path, ctx)?
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .map(|(_, name)| exported_name(name, &path.join(".")))
            .transpose()?
            .unwrap_or_else(|| "any".into()),
        DataType::Reference(reference) => render_reference(
            exporter,
            types,
            reference,
            generics,
            path,
            direct_field,
            ctx,
        )?,
    })
}

fn render_struct(
    exporter: &Go,
    types: &Types,
    strct: &Struct,
    generics: &[(Generic, Cow<'static, str>)],
    path: &mut Vec<String>,
    ctx: &mut Context,
) -> Result<String, Error> {
    match &strct.fields {
        Fields::Unit => Ok("any".into()),
        Fields::Unnamed(fields) => {
            let live = fields
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .collect::<Vec<_>>();
            if fields.fields.len() == 1 {
                match live.as_slice() {
                    [ty] => render_datatype(exporter, types, ty, generics, path, true, ctx),
                    [] => Ok("any".into()),
                    _ => unreachable!("one source field has at most one live field"),
                }
            } else {
                let mut discarded_ctx = Context::default();
                for (index, ty) in live.iter().enumerate() {
                    path.push(index.to_string());
                    let _ = render_datatype(
                        exporter,
                        types,
                        ty,
                        generics,
                        path,
                        false,
                        &mut discarded_ctx,
                    )?;
                    path.pop();
                }
                Ok(format!("[{}]any", live.len()))
            }
        }
        Fields::Named(fields) => {
            let mut out = String::from("struct {\n");
            let mut names = BTreeSet::new();
            let mut rows = Vec::new();
            for (index, (json_name, field)) in fields.fields.iter().enumerate() {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };
                if !valid_json_tag_name(json_name) {
                    return Err(Error::UnsupportedType {
                        path: path.join("."),
                        reason: format!(
                            "encoding/json cannot represent the field name {json_name:?} in a struct tag"
                        ),
                    });
                }
                let field_name = field_name(json_name, index, &path.join("."))?;
                if !names.insert(field_name.clone()) {
                    return Err(Error::DuplicateName {
                        path: path.join("."),
                        name: field_name,
                    });
                }
                let mut comments = String::new();
                write_doc_comment(
                    &mut comments,
                    "\t",
                    Some(&field_name),
                    &field.docs,
                    field.deprecated.as_ref(),
                );
                path.push(json_name.to_string());
                let mut field_type =
                    render_datatype(exporter, types, ty, generics, path, true, ctx)?;
                path.pop();
                if field.optional
                    && (!can_be_nil(&field_type)
                        || field_type.starts_with("[]")
                        || field_type.starts_with("map["))
                {
                    field_type.insert(0, '*');
                }
                if !comments.is_empty() {
                    push_aligned_rows(&mut out, &mut rows);
                    out.push_str(&comments);
                }
                rows.push((
                    field_name,
                    field_type,
                    struct_tag(json_name, field.optional),
                ));
            }
            push_aligned_rows(&mut out, &mut rows);
            out.push('}');
            Ok(out)
        }
    }
}

fn push_aligned_rows(out: &mut String, rows: &mut Vec<(String, String, String)>) {
    let name_width = rows
        .iter()
        .map(|(name, _, _)| name.chars().count())
        .max()
        .unwrap_or(0);
    let type_width = rows
        .iter()
        .map(|(_, ty, _)| ty.chars().count())
        .max()
        .unwrap_or(0);
    for (name, ty, trailing) in rows.drain(..) {
        let name_len = name.chars().count();
        let type_len = ty.chars().count();
        out.push('\t');
        out.push_str(&name);
        out.extend(std::iter::repeat_n(' ', name_width - name_len + 1));
        out.push_str(&ty);
        out.extend(std::iter::repeat_n(' ', type_width - type_len + 1));
        out.push_str(&trailing);
        out.push('\n');
    }
}

fn render_intersection(
    exporter: &Go,
    types: &Types,
    elements: &[DataType],
    generics: &[(Generic, Cow<'static, str>)],
    path: &mut Vec<String>,
    ctx: &mut Context,
) -> Result<String, Error> {
    let mut fields = Vec::new();
    let mut visited = BTreeSet::new();
    for element in elements {
        if !collect_intersection_fields(types, element, &mut fields, &mut visited) {
            return Ok("any".into());
        }
    }
    if fields.is_empty() {
        return Ok("any".into());
    }
    let mut combined = match Struct::named().build() {
        DataType::Struct(strct) => strct,
        _ => unreachable!("Struct::named builds a struct datatype"),
    };
    let Fields::Named(combined_fields) = &mut combined.fields else {
        unreachable!("constructed a named struct")
    };
    combined_fields.fields = fields;
    render_struct(exporter, types, &combined, generics, path, ctx)
}

fn collect_intersection_fields(
    types: &Types,
    dt: &DataType,
    output: &mut Vec<(Cow<'static, str>, specta::datatype::Field)>,
    visited: &mut BTreeSet<String>,
) -> bool {
    match dt {
        DataType::Struct(strct) => match &strct.fields {
            Fields::Named(fields) => {
                output.extend(fields.fields.clone());
                true
            }
            Fields::Unit | Fields::Unnamed(_) => false,
        },
        DataType::Intersection(elements) => elements
            .iter()
            .all(|element| collect_intersection_fields(types, element, output, visited)),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                collect_intersection_fields(types, dt, output, visited)
            }
            NamedReferenceType::Reference {
                generics: arguments,
                ..
            } => {
                let Some(ndt) = types.get(reference) else {
                    return false;
                };
                let type_path = rust_type_path(ndt);
                if !visited.insert(type_path.clone()) {
                    return false;
                }
                let substitutions = resolve_reference_arguments(ndt, arguments);
                let result = ndt.ty.as_ref().is_some_and(|ty| {
                    let mut ty = ty.clone();
                    substitute_generics(&mut ty, &substitutions);
                    collect_intersection_fields(types, &ty, output, visited)
                });
                visited.remove(&type_path);
                result
            }
            NamedReferenceType::Recursive(_) => false,
        },
        _ => false,
    }
}

fn substitute_generics(dt: &mut DataType, substitutions: &HashMap<Generic, DataType>) {
    if let DataType::Generic(generic) = dt
        && let Some(replacement) = substitutions.get(generic)
    {
        *dt = replacement.clone();
        return;
    }
    match dt {
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => {}
        DataType::List(list) => substitute_generics(&mut list.ty, substitutions),
        DataType::Map(map) => {
            let mut key = map.key_ty().clone();
            let mut value = map.value_ty().clone();
            substitute_generics(&mut key, substitutions);
            substitute_generics(&mut value, substitutions);
            map.set_key_ty(key);
            map.set_value_ty(value);
        }
        DataType::Nullable(inner) => substitute_generics(inner, substitutions),
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, substitutions),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_field_generics(&mut variant.fields, substitutions);
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                substitute_generics(element, substitutions);
            }
        }
        DataType::Intersection(elements) => {
            for element in elements {
                substitute_generics(element, substitutions);
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, substitutions),
            NamedReferenceType::Reference { generics, .. } => {
                for (_, generic) in generics {
                    substitute_generics(generic, substitutions);
                }
            }
            NamedReferenceType::Recursive(_) => {}
        },
    }
}

fn substitute_field_generics(fields: &mut Fields, substitutions: &HashMap<Generic, DataType>) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(ty) = field.ty.as_mut() {
                    substitute_generics(ty, substitutions);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(ty) = field.ty.as_mut() {
                    substitute_generics(ty, substitutions);
                }
            }
        }
    }
}

fn render_reference(
    exporter: &Go,
    types: &Types,
    reference: &Reference,
    generics: &[(Generic, Cow<'static, str>)],
    path: &mut Vec<String>,
    direct_field: bool,
    ctx: &mut Context,
) -> Result<String, Error> {
    match reference {
        Reference::Named(reference) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                render_datatype(exporter, types, dt, generics, path, direct_field, ctx)
            }
            NamedReferenceType::Recursive(cycle) => Err(Error::RecursiveInline {
                path: path.join("."),
                cycle: cycle.clone(),
            }),
            NamedReferenceType::Reference {
                generics: arguments,
                ..
            } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference {
                        path: path.join("."),
                        reference: format!("{reference:?}"),
                    })?;
                let mut out = exported_name(&ndt.name, &rust_type_path(ndt))?;
                let mut rendered_arguments = Vec::new();
                let resolved_arguments = resolve_reference_arguments(ndt, arguments);
                for definition in ndt.generics.iter() {
                    rendered_arguments.push(
                        match resolved_arguments.get(&definition.reference()) {
                            Some(argument) => render_datatype(
                                exporter, types, argument, generics, path, false, ctx,
                            )?,
                            None => "any".into(),
                        },
                    );
                }
                if !rendered_arguments.is_empty() {
                    out.push('[');
                    out.push_str(&rendered_arguments.join(", "));
                    out.push(']');
                }
                if direct_field && reference_requires_pointer(types, ndt, arguments, path) {
                    out.insert(0, '*');
                }
                Ok(out)
            }
        },
        Reference::Opaque(opaque) => {
            let name = opaque.type_name();
            Ok(
                if name.ends_with("SystemTime") || name.contains("DateTime") {
                    ctx.imports.insert("time");
                    "time.Time".into()
                } else if name.ends_with("Duration") {
                    ctx.imports.insert("time");
                    "time.Duration".into()
                } else {
                    "any".into()
                },
            )
        }
    }
}

fn resolve_reference_arguments(
    ndt: &NamedDataType,
    arguments: &[(Generic, DataType)],
) -> HashMap<Generic, DataType> {
    let mut resolved = HashMap::new();
    for definition in ndt.generics.iter() {
        let reference = definition.reference();
        let argument = arguments
            .iter()
            .find(|(generic, _)| generic == &reference)
            .map(|(_, ty)| ty.clone())
            .or_else(|| {
                definition.default.clone().map(|mut ty| {
                    substitute_generics(&mut ty, &resolved);
                    ty
                })
            });
        if let Some(argument) = argument {
            resolved.insert(reference, argument);
        }
    }
    resolved
}

fn render_map_key(
    exporter: &Go,
    types: &Types,
    dt: &DataType,
    generics: &[(Generic, Cow<'static, str>)],
    path: &mut Vec<String>,
    ctx: &mut Context,
) -> Result<String, Error> {
    let valid = match dt {
        DataType::Primitive(
            Primitive::str
            | Primitive::char
            | Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::i64
            | Primitive::isize
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32
            | Primitive::u64
            | Primitive::usize,
        ) => true,
        DataType::Generic(_) => false,
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => map_key_is_valid(types, dt),
            NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => {
                types.get(reference).is_some_and(|ndt| {
                    ndt.generics.is_empty()
                        && ndt
                            .ty
                            .as_ref()
                            .is_some_and(|dt| map_key_is_valid(types, dt))
                })
            }
        },
        _ => false,
    };
    if !valid {
        return Err(Error::InvalidMapKey {
            path: path.join("."),
            reason: format!("{dt:?} is not a string, integer, or named scalar"),
        });
    }
    render_datatype(exporter, types, dt, generics, path, false, ctx)
}

fn map_key_is_valid(types: &Types, dt: &DataType) -> bool {
    map_key_is_valid_inner(types, dt, &mut BTreeSet::new())
}

fn map_key_is_valid_inner(types: &Types, dt: &DataType, visited: &mut BTreeSet<String>) -> bool {
    match dt {
        DataType::Primitive(
            Primitive::str
            | Primitive::char
            | Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::i64
            | Primitive::isize
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32
            | Primitive::u64
            | Primitive::usize,
        ) => true,
        DataType::Generic(_) => false,
        DataType::Enum(enm) => string_enum_variants(enm).is_some(),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unnamed(fields) => matches!(
                fields.fields.as_slice(),
                [field] if field
                    .ty
                    .as_ref()
                    .is_some_and(|dt| map_key_is_valid_inner(types, dt, visited))
            ),
            Fields::Unit | Fields::Named(_) => false,
        },
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => map_key_is_valid_inner(types, dt, visited),
            NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => {
                types.get(reference).is_some_and(|ndt| {
                    ndt.generics.is_empty()
                        && visited.insert(rust_type_path(ndt))
                        && ndt
                            .ty
                            .as_ref()
                            .is_some_and(|dt| map_key_is_valid_inner(types, dt, visited))
                })
            }
        },
        _ => false,
    }
}

fn is_bare_generic_newtype(dt: &DataType) -> bool {
    if matches!(dt, DataType::Generic(_)) {
        return true;
    }
    let DataType::Struct(strct) = dt else {
        return false;
    };
    let Fields::Unnamed(fields) = &strct.fields else {
        return false;
    };
    matches!(fields.fields.as_slice(), [field] if matches!(field.ty, Some(DataType::Generic(_))))
}

fn reference_requires_pointer(
    types: &Types,
    ndt: &NamedDataType,
    arguments: &[(Generic, DataType)],
    path: &[String],
) -> bool {
    let Some(root) = path.first() else {
        return false;
    };
    let target = rust_type_path(ndt);
    if target == *root {
        return true;
    }
    let substitutions = resolve_reference_arguments(ndt, arguments);
    let Some(mut target_ty) = ndt.ty.clone() else {
        return false;
    };
    substitute_generics(&mut target_ty, &substitutions);
    let mut visited = BTreeSet::from([target]);
    datatype_reaches(types, &target_ty, root, &mut visited)
}

fn datatype_reaches(
    types: &Types,
    dt: &DataType,
    target: &str,
    visited: &mut BTreeSet<String>,
) -> bool {
    match dt {
        DataType::Struct(strct) => fields_reach(types, &strct.fields, target, visited),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => datatype_reaches(types, dt, target, visited),
            NamedReferenceType::Reference { generics, .. } => {
                if generics
                    .iter()
                    .any(|(_, generic)| datatype_reaches(types, generic, target, visited))
                {
                    return true;
                }
                let Some(ndt) = types.get(reference) else {
                    return false;
                };
                let path = rust_type_path(ndt);
                if path == target {
                    return true;
                }
                if !visited.insert(path) {
                    return false;
                }
                let Some(mut ty) = ndt.ty.clone() else {
                    return false;
                };
                substitute_generics(&mut ty, &resolve_reference_arguments(ndt, generics));
                datatype_reaches(types, &ty, target, visited)
            }
            NamedReferenceType::Recursive(_) => true,
        },
        // These representations already introduce indirection in generated Go.
        DataType::List(_)
        | DataType::Map(_)
        | DataType::Nullable(_)
        | DataType::Tuple(_)
        | DataType::Enum(_)
        | DataType::Intersection(_)
        | DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => false,
    }
}

fn fields_reach(
    types: &Types,
    fields: &Fields,
    target: &str,
    visited: &mut BTreeSet<String>,
) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .any(|dt| datatype_reaches(types, dt, target, visited)),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(_, field)| field.ty.as_ref())
            .any(|dt| datatype_reaches(types, dt, target, visited)),
    }
}

fn primitive_type(primitive: &Primitive, ctx: &mut Context) -> String {
    match primitive {
        Primitive::i8 => "int8",
        Primitive::i16 => "int16",
        Primitive::i32 => "int32",
        Primitive::i64 | Primitive::isize => "int64",
        Primitive::u8 => "uint8",
        Primitive::u16 => "uint16",
        Primitive::u32 => "uint32",
        Primitive::u64 | Primitive::usize => "uint64",
        Primitive::f16 | Primitive::f32 => "float32",
        Primitive::f64 | Primitive::f128 => "float64",
        Primitive::bool => "bool",
        Primitive::str | Primitive::char => "string",
        Primitive::i128 | Primitive::u128 => {
            ctx.imports.insert("math/big");
            "*big.Int"
        }
    }
    .into()
}

fn write_generic_definitions(
    out: &mut String,
    generics: &[(Generic, Cow<'static, str>)],
    path: &[String],
) -> Result<(), Error> {
    if generics.is_empty() {
        return Ok(());
    }
    out.push('[');
    let mut names = BTreeSet::new();
    for (index, (_, name)) in generics.iter().enumerate() {
        if index != 0 {
            out.push_str(", ");
        }
        let name = exported_name(name, &path.join("."))?;
        if !names.insert(name.clone()) {
            return Err(Error::DuplicateName {
                path: path.join("."),
                name,
            });
        }
        out.push_str(&name);
        out.push_str(" any");
    }
    out.push(']');
    Ok(())
}

fn string_enum_variants(enm: &Enum) -> Option<Vec<(&str, &specta::datatype::Variant)>> {
    let variants = enm
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(name, variant)| {
            if !name.is_empty() && matches!(variant.fields, Fields::Unit) {
                return Some((name.as_ref(), variant));
            }
            let Fields::Unnamed(fields) = &variant.fields else {
                return None;
            };
            let [field] = fields.fields.as_slice() else {
                return None;
            };
            let DataType::Enum(literal) = field.ty.as_ref()? else {
                return None;
            };
            let [(literal, literal_variant)] = literal.variants.as_slice() else {
                return None;
            };
            matches!(literal_variant.fields, Fields::Unit).then_some((literal.as_ref(), variant))
        })
        .collect::<Option<Vec<_>>>()?;
    (!variants.is_empty()).then_some(variants)
}

fn can_be_nil(ty: &str) -> bool {
    ty == "any"
        || ty.starts_with('*')
        || ty.starts_with("[]")
        || ty.starts_with("map[")
        || ty.starts_with("func(")
        || ty.starts_with("chan ")
}

fn write_doc_comment(
    out: &mut String,
    indent: &str,
    subject: Option<&str>,
    docs: &str,
    deprecated: Option<&Deprecated>,
) {
    if !docs.is_empty() {
        for (index, line) in docs.lines().enumerate() {
            out.push_str(indent);
            out.push_str("// ");
            if index == 0
                && let Some(subject) = subject
                && !line.starts_with(subject)
            {
                out.push_str(subject);
                out.push_str(" — ");
            }
            out.push_str(line.trim());
            out.push('\n');
        }
    }
    if let Some(deprecated) = deprecated {
        out.push_str(indent);
        out.push_str("// Deprecated:");
        if let Some(note) = deprecated
            .note
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            out.push(' ');
            out.push_str(note);
        }
        if let Some(since) = deprecated
            .since
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            out.push_str(" (since ");
            out.push_str(since);
            out.push(')');
        }
        out.push('\n');
    }
}

fn rust_type_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn exported_name(name: &str, path: &str) -> Result<String, Error> {
    let mut out = String::with_capacity(name.len());
    let mut uppercase = true;
    let mut segment = String::new();
    let flush = |out: &mut String, segment: &mut String| {
        if segment.is_empty() {
            return;
        }
        let upper = segment.to_ascii_uppercase();
        if GO_INITIALISMS.contains(&upper.as_str()) {
            out.push_str(&upper);
        } else {
            let mut chars = segment.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.extend(chars);
            }
        }
        segment.clear();
    };

    for ch in name.chars() {
        if ch == '_' || ch == '-' || ch == ' ' || ch == '.' {
            flush(&mut out, &mut segment);
            uppercase = true;
        } else if ch.is_alphanumeric() {
            if uppercase {
                flush(&mut out, &mut segment);
                uppercase = false;
            }
            segment.push(ch);
        } else {
            return Err(Error::InvalidName {
                path: path.into(),
                name: name.into(),
            });
        }
    }
    flush(&mut out, &mut segment);

    if out.is_empty()
        || out
            .chars()
            .next()
            .is_some_and(|ch| ch.is_numeric() || !ch.is_uppercase())
        || crate::reserved_names::RESERVED_GO_NAMES.contains(&out.as_str())
    {
        return Err(Error::InvalidName {
            path: path.into(),
            name: name.into(),
        });
    }
    Ok(out)
}

fn field_name(name: &str, index: usize, path: &str) -> Result<String, Error> {
    if name.is_empty() {
        return Err(Error::InvalidName {
            path: path.into(),
            name: name.into(),
        });
    }
    Ok(exported_name(name, path).unwrap_or_else(|_| format!("Field{}", index + 1)))
}

fn enum_constant_suffix(value: &str, index: usize, path: &str) -> String {
    exported_name(value, path).unwrap_or_else(|_| format!("Value{}", index + 1))
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    let chars = name.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().copied().enumerate() {
        let previous = index.checked_sub(1).and_then(|index| chars.get(index));
        let next = chars.get(index + 1);
        if ch.is_uppercase()
            && previous.is_some()
            && (previous.is_some_and(|ch| ch.is_lowercase() || ch.is_numeric())
                || next.is_some_and(|ch| ch.is_lowercase()))
        {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

fn escape_go_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(escaped, "\\u{:04X}", ch as u32);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn struct_tag(name: &str, optional: bool) -> String {
    let mut tag = String::from("json:\"");
    tag.push_str(&escape_go_string(name));
    if optional {
        tag.push_str(",omitempty");
    } else if name == "-" {
        // A bare `json:"-"` means "ignore this field". The empty option
        // segment disambiguates the literal JSON key `-`.
        tag.push(',');
    }
    tag.push('"');
    format!("`{tag}`")
}

fn valid_json_tag_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|ch| {
            ch.is_alphabetic() || ch.is_ascii_digit() || "!#$%&()*+-./:;<=>?@[]^_{|}~ ".contains(ch)
        })
}

const GO_INITIALISMS: &[&str] = &[
    "ACL", "API", "ASCII", "CPU", "CSS", "DNS", "EOF", "GUID", "HTML", "HTTP", "HTTPS", "ID", "IP",
    "JSON", "QPS", "RAM", "RPC", "SLA", "SMTP", "SQL", "SSH", "TCP", "TLS", "TTL", "UDP", "UI",
    "UID", "UUID", "URI", "URL", "UTF8", "VM", "XML", "XMPP", "XSRF", "XSS",
];
