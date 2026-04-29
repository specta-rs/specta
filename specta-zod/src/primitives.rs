//! Primitives provide building blocks for Specta-based libraries.

use std::{borrow::Cow, fmt::Write as _};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Fields, GenericReference, List, Map, NamedDataType, NamedReference,
        NamedReferenceType, OpaqueReference, Primitive, Reference, Struct, Tuple,
    },
};

use crate::{
    BigIntExportBehavior, Error, Layout, Zod, opaque, reserved_names::RESERVED_TYPE_NAMES,
};

pub(crate) type TypeRenderStack = Vec<(Cow<'static, str>, Cow<'static, str>)>;

fn named_reference_generics(r: &NamedReference) -> Result<&[(GenericReference, DataType)], Error> {
    match &r.inner {
        NamedReferenceType::Reference { generics, .. } => Ok(generics),
        NamedReferenceType::Inline { .. } => Ok(&[]),
        NamedReferenceType::Recursive => Err(Error::dangling_named_reference(format!(
            "recursive inline named reference {r:?}"
        ))),
    }
}

fn named_reference_ty<'a>(types: &'a Types, r: &'a NamedReference) -> Result<&'a DataType, Error> {
    match &r.inner {
        NamedReferenceType::Reference { .. } => types
            .get(r)
            .and_then(|ndt| ndt.ty.as_ref())
            .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}"))),
        NamedReferenceType::Inline { dt, .. } => Ok(dt),
        NamedReferenceType::Recursive => Err(Error::dangling_named_reference(format!(
            "recursive inline named reference {r:?}"
        ))),
    }
}

/// Generate a group of `export const XSchema = ...` declarations for named types.
pub fn export<'a>(
    exporter: &dyn AsRef<Zod>,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut s = String::new();
    let mut type_render_stack = TypeRenderStack::new();
    export_internal(
        &mut s,
        exporter.as_ref(),
        types,
        ndts,
        indent,
        &mut type_render_stack,
    )?;
    Ok(s)
}

pub(crate) fn export_internal<'a>(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    for (index, ndt) in ndts.enumerate() {
        if index != 0 {
            s.push('\n');
        }
        export_single_internal(s, exporter, types, ndt, indent, type_render_stack)?;
    }

    Ok(())
}

fn export_single_internal(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    ndt: &NamedDataType,
    indent: &str,
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let base_name = exported_type_name(exporter, ndt);
    let name_path = if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    };
    validate_type_name(&base_name, name_path)?;
    let schema_name = format!("{base_name}Schema");

    let Some(ty) = &ndt.ty else {
        return Ok(());
    };

    type_render_stack.push((ndt.module_path.clone(), ndt.name.clone()));

    let result = (|| {
        if ndt.generics.is_empty() {
            let mut schema_expr = String::new();
            datatype(
                &mut schema_expr,
                exporter,
                types,
                ty,
                vec![ndt.name.clone()],
                &[],
                false,
                type_render_stack,
            )?;

            writeln!(s, "{indent}export const {schema_name} = {schema_expr};")?;
            writeln!(
                s,
                "{indent}export type {base_name} = z.infer<typeof {schema_name}>;"
            )?;
            return Ok(());
        }

        let generic_names = ndt
            .generics
            .iter()
            .map(|generic| generic.name.as_ref().to_string())
            .collect::<Vec<_>>();

        let generic_params = generic_names
            .iter()
            .map(|name| format!("{name} extends z.ZodTypeAny"))
            .collect::<Vec<_>>()
            .join(", ");

        let fn_params = generic_names.join(", ");

        let mut schema_expr = String::new();
        datatype(
            &mut schema_expr,
            exporter,
            types,
            ty,
            vec![ndt.name.clone()],
            &[],
            false,
            type_render_stack,
        )?;

        writeln!(
            s,
            "{indent}export const {schema_name} = <{generic_params}>({fn_params}) => {schema_expr};"
        )?;

        let alias_params = generic_names.join(", ");
        let infer_args = generic_names
            .iter()
            .map(|name| format!("z.ZodType<{name}>"))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(
            s,
            "{indent}export type {base_name}<{alias_params}> = z.infer<ReturnType<typeof {schema_name}<{infer_args}>>>;"
        )?;

        Ok(())
    })();

    type_render_stack.pop();
    result
}

/// Generate an inline Zod expression for a [`DataType`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically. Map both the full [`Types`] graph and any top-level
/// [`DataType`] values before calling this helper.
pub fn inline(exporter: &dyn AsRef<Zod>, types: &Types, dt: &DataType) -> Result<String, Error> {
    let mut s = String::new();
    let mut type_render_stack = TypeRenderStack::new();
    datatype(
        &mut s,
        exporter.as_ref(),
        types,
        dt,
        vec![],
        &[],
        false,
        &mut type_render_stack,
    )?;
    Ok(s)
}

/// Generate a Zod expression for a [`Reference`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically.
pub fn reference(exporter: &dyn AsRef<Zod>, types: &Types, r: &Reference) -> Result<String, Error> {
    let mut s = String::new();
    let mut type_render_stack = TypeRenderStack::new();
    reference_dt(
        &mut s,
        exporter.as_ref(),
        types,
        r,
        vec![],
        &[],
        &mut type_render_stack,
    )?;
    Ok(s)
}

pub(crate) fn datatype_with_inline_attr(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    inline: bool,
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    datatype(
        s,
        exporter,
        types,
        dt,
        location,
        generics,
        inline,
        type_render_stack,
    )
}

fn datatype(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    force_inline_ref: bool,
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(&exporter.bigint, p, location)?),
        DataType::List(l) => list_dt(s, exporter, types, l, location, generics, type_render_stack)?,
        DataType::Map(m) => map_dt(s, exporter, types, m, location, generics, type_render_stack)?,
        DataType::Nullable(def) => {
            let mut inner = String::new();
            datatype(
                &mut inner,
                exporter,
                types,
                def,
                location,
                generics,
                force_inline_ref,
                type_render_stack,
            )?;
            write!(s, "{inner}.nullable()")?;
        }
        DataType::Struct(st) => struct_dt(
            s,
            exporter,
            types,
            st,
            location,
            generics,
            type_render_stack,
        )?,
        DataType::Enum(enm) => enum_dt(
            s,
            exporter,
            types,
            enm,
            location,
            generics,
            type_render_stack,
        )?,
        DataType::Tuple(tuple) => tuple_dt(
            s,
            exporter,
            types,
            tuple,
            location,
            generics,
            type_render_stack,
        )?,
        DataType::Reference(r) => {
            if force_inline_ref {
                match r {
                    Reference::Named(named) => {
                        let ty = named_reference_ty(types, named)?;
                        let reference_generics = named_reference_generics(named)?;
                        datatype(
                            s,
                            exporter,
                            types,
                            ty,
                            location,
                            reference_generics,
                            false,
                            type_render_stack,
                        )?;
                    }
                    _ => {
                        reference_dt(s, exporter, types, r, location, generics, type_render_stack)?
                    }
                }
            } else {
                reference_dt(s, exporter, types, r, location, generics, type_render_stack)?;
            }
        }
        DataType::Generic(g) => generic_dt(s, g),
        DataType::Intersection(intersection) => {
            let mut parts = Vec::with_capacity(intersection.len());
            for ty in intersection {
                let mut part = String::new();
                datatype(
                    &mut part,
                    exporter,
                    types,
                    ty,
                    location.clone(),
                    generics,
                    false,
                    type_render_stack,
                )?;
                parts.push(part);
            }
            s.push_str(&parts.join(".and("));
            for _ in 1..parts.len() {
                s.push(')');
            }
        }
    }

    Ok(())
}

fn primitive_dt(
    b: &BigIntExportBehavior,
    p: &Primitive,
    location: Vec<Cow<'static, str>>,
) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 | f16 | f32 | f64 | f128 => "z.number()",
        usize | isize | i64 | u64 | i128 | u128 => match b {
            BigIntExportBehavior::String => "z.string()",
            BigIntExportBehavior::Number => "z.number()",
            BigIntExportBehavior::BigInt => "z.bigint()",
            BigIntExportBehavior::Fail => return Err(Error::bigint_forbidden(location.join("."))),
        },
        Primitive::bool => "z.boolean()",
        str | char => "z.string()",
    })
}

fn list_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    l: &List,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let mut dt = String::new();
    datatype(
        &mut dt,
        exporter,
        types,
        &l.ty,
        location,
        generics,
        false,
        type_render_stack,
    )?;

    if let Some(length) = l.length {
        s.push_str("z.tuple([");
        for n in 0..length {
            if n != 0 {
                s.push_str(", ");
            }
            s.push_str(&dt);
        }
        s.push_str("])");
    } else {
        write!(s, "z.array({dt})")?;
    }

    Ok(())
}

fn map_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    m: &Map,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let mut key = String::new();
    datatype(
        &mut key,
        exporter,
        types,
        m.key_ty(),
        location.clone(),
        generics,
        false,
        type_render_stack,
    )?;
    let mut value = String::new();
    datatype(
        &mut value,
        exporter,
        types,
        m.value_ty(),
        location,
        generics,
        false,
        type_render_stack,
    )?;

    write!(s, "z.record({key}, {value})")?;
    Ok(())
}

fn tuple_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    t: &Tuple,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match t.elements.as_slice() {
        [] => s.push_str("z.null()"),
        elements => {
            s.push_str("z.tuple([");
            for (i, dt) in elements.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }
                datatype(
                    s,
                    exporter,
                    types,
                    dt,
                    location.clone(),
                    generics,
                    false,
                    type_render_stack,
                )?;
            }
            s.push_str("])");
        }
    }

    Ok(())
}

fn struct_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    st: &Struct,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => s.push_str("z.null()"),
        Fields::Unnamed(unnamed) => {
            let fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();

            match fields.as_slice() {
                [] => s.push_str("z.tuple([])"),
                [(field, ty)] if unnamed.fields.len() == 1 => {
                    datatype_with_inline_attr(
                        s,
                        exporter,
                        types,
                        ty,
                        location,
                        generics,
                        false,
                        type_render_stack,
                    )?;
                }
                fields => {
                    s.push_str("z.tuple([");
                    for (i, (field, ty)) in fields.iter().enumerate() {
                        if i != 0 {
                            s.push_str(", ");
                        }
                        datatype_with_inline_attr(
                            s,
                            exporter,
                            types,
                            ty,
                            location.clone(),
                            generics,
                            false,
                            type_render_stack,
                        )?;
                    }
                    s.push_str("])");
                }
            }
        }
        Fields::Named(named) => {
            let all_fields = named
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
                .collect::<Vec<_>>();

            if all_fields.is_empty() {
                s.push_str("z.object({}).strict()");
                return Ok(());
            }

            let non_flattened = all_fields.iter().collect::<Vec<_>>();

            let mut schema = String::from("z.object({");
            for (name, field, ty) in &non_flattened {
                let key = sanitise_key(name.as_ref());
                write!(schema, "\n\t{key}: ")?;
                let mut value = String::new();
                datatype_with_inline_attr(
                    &mut value,
                    exporter,
                    types,
                    ty,
                    location.clone(),
                    generics,
                    false,
                    type_render_stack,
                )?;
                if field.optional {
                    write!(schema, "{value}.optional(),")?;
                } else {
                    write!(schema, "{value},")?;
                }
            }
            if !non_flattened.is_empty() {
                schema.push('\n');
            }
            schema.push_str("})");

            s.push_str(&schema);
        }
    }

    Ok(())
}

fn enum_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    e: &Enum,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let variants = e
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(name, variant)| {
            enum_variant_dt(
                exporter,
                types,
                name.as_ref(),
                variant,
                location.clone(),
                generics,
                type_render_stack,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut variants = variants.into_iter().flatten().collect::<Vec<_>>();

    if variants.is_empty() {
        s.push_str("z.never()");
        return Ok(());
    }

    variants.sort();
    variants.dedup();

    if variants.len() == 1 {
        s.push_str(&variants[0]);
    } else {
        write!(s, "z.union([{}])", variants.join(", "))?;
    }

    Ok(())
}

fn enum_variant_dt(
    exporter: &Zod,
    types: &Types,
    name: &str,
    variant: &specta::datatype::Variant,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<Option<String>, Error> {
    match &variant.fields {
        Fields::Unit => Ok(Some(format!("z.literal(\"{}\")", escape_string(name)))),
        Fields::Named(named) => {
            if named.fields.iter().all(|(_, field)| field.ty.is_none()) {
                return Ok(Some("z.object({}).strict()".to_string()));
            }

            let mut schema = String::from("z.object({");
            let mut has_field = false;
            let mut flattened_sections = Vec::new();

            for (field_name, field) in &named.fields {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };

                if false {
                    let mut value = String::new();
                    datatype_with_inline_attr(
                        &mut value,
                        exporter,
                        types,
                        ty,
                        location.clone(),
                        generics,
                        false,
                        type_render_stack,
                    )?;
                    flattened_sections.push(value);
                    continue;
                }

                has_field = true;
                let mut value = String::new();
                datatype_with_inline_attr(
                    &mut value,
                    exporter,
                    types,
                    ty,
                    location.clone(),
                    generics,
                    false,
                    type_render_stack,
                )?;

                let key = sanitise_key(field_name);
                if field.optional {
                    write!(schema, "\n\t{key}: {value}.optional(),")?;
                } else {
                    write!(schema, "\n\t{key}: {value},")?;
                }
            }

            if has_field {
                schema.push('\n');
            }
            schema.push_str("})");

            if flattened_sections.is_empty() {
                return Ok(Some(schema));
            }

            let mut sections = vec![schema];
            sections.extend(flattened_sections);
            let mut out = String::new();
            out.push_str(&sections.join(".and("));
            for _ in 1..sections.len() {
                out.push(')');
            }
            Ok(Some(out))
        }
        Fields::Unnamed(unnamed) => {
            let fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();

            Ok(match fields.as_slice() {
                [] => {
                    if unnamed.fields.is_empty() {
                        Some("z.tuple([])".to_string())
                    } else {
                        None
                    }
                }
                [(field, ty)] if unnamed.fields.len() == 1 => {
                    let mut out = String::new();
                    datatype_with_inline_attr(
                        &mut out,
                        exporter,
                        types,
                        ty,
                        location,
                        generics,
                        false,
                        type_render_stack,
                    )?;
                    Some(out)
                }
                fields => {
                    let mut out = String::from("z.tuple([");
                    for (i, (field, ty)) in fields.iter().enumerate() {
                        if i != 0 {
                            out.push_str(", ");
                        }
                        let mut item = String::new();
                        datatype_with_inline_attr(
                            &mut item,
                            exporter,
                            types,
                            ty,
                            location.clone(),
                            generics,
                            false,
                            type_render_stack,
                        )?;
                        out.push_str(&item);
                    }
                    out.push_str("])");
                    Some(out)
                }
            })
        }
    }
}

fn reference_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    r: &Reference,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match r {
        Reference::Named(r) => {
            reference_named_dt(s, exporter, types, r, location, generics, type_render_stack)
        }
        Reference::Opaque(r) => reference_opaque_dt(s, r),
    }
}

fn generic_dt(s: &mut String, g: &GenericReference) {
    s.push_str(g.name());
}

fn reference_opaque_dt(s: &mut String, r: &OpaqueReference) -> Result<(), Error> {
    if let Some(def) = r.downcast_ref::<opaque::Define>() {
        s.push_str(&def.0);
        return Ok(());
    }
    if r.downcast_ref::<opaque::Any>().is_some() {
        s.push_str("z.any()");
        return Ok(());
    }
    if r.downcast_ref::<opaque::Unknown>().is_some() {
        s.push_str("z.unknown()");
        return Ok(());
    }
    if r.downcast_ref::<opaque::Never>().is_some() {
        s.push_str("z.never()");
        return Ok(());
    }

    Err(Error::unsupported_opaque_reference(r.clone()))
}

fn reference_named_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    r: &NamedReference,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let ndt = types
        .get(r)
        .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;

    if matches!(r.inner, NamedReferenceType::Inline { .. }) {
        let ty = named_reference_ty(types, r)?;
        let reference_generics = named_reference_generics(r)?;
        return datatype(
            s,
            exporter,
            types,
            ty,
            location,
            reference_generics,
            false,
            type_render_stack,
        );
    }

    crate::references::track_nr(r);

    let schema_name = match exporter.layout {
        Layout::FlatFile => format!("{}Schema", ndt.name),
        Layout::ModulePrefixedName => {
            let mut name = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            if !name.is_empty() {
                name.push('_');
            }
            name.push_str(&ndt.name);
            name.push_str("Schema");
            name
        }
        Layout::Files => {
            let current_module_path = crate::references::current_module_path().unwrap_or_default();
            let base = format!("{}Schema", ndt.name);
            if ndt.module_path == current_module_path {
                base
            } else {
                format!("{}.{}", crate::zod::module_alias(&ndt.module_path), base)
            }
        }
    };

    let should_lazy = type_render_stack
        .iter()
        .any(|(module, name)| module == &ndt.module_path && name == &ndt.name);

    let mut reference_expr = schema_name;
    let reference_generics = named_reference_generics(r)?;
    if !reference_generics.is_empty() {
        let scoped_generics = generics
            .iter()
            .filter(|(parent_generic, _)| {
                !reference_generics
                    .iter()
                    .any(|(child_generic, _)| child_generic == parent_generic)
            })
            .cloned()
            .collect::<Vec<_>>();

        reference_expr.push('(');
        for (i, (_, v)) in reference_generics.iter().enumerate() {
            if i != 0 {
                reference_expr.push_str(", ");
            }
            let mut generic_schema = String::new();
            datatype(
                &mut generic_schema,
                exporter,
                types,
                v,
                vec![],
                &scoped_generics,
                false,
                type_render_stack,
            )?;
            reference_expr.push_str(&generic_schema);
        }
        reference_expr.push(')');
    }

    if should_lazy {
        write!(s, "z.lazy(() => {reference_expr})")?;
    } else {
        s.push_str(&reference_expr);
    }

    Ok(())
}

fn exported_type_name(exporter: &Zod, ndt: &NamedDataType) -> Cow<'static, str> {
    match exporter.layout {
        Layout::FlatFile | Layout::Files => ndt.name.clone(),
        Layout::ModulePrefixedName => {
            let mut s = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            if !s.is_empty() {
                s.push('_');
            }
            s.push_str(&ndt.name);
            Cow::Owned(s)
        }
    }
}

fn validate_type_name(name: &str, path: String) -> Result<(), Error> {
    if RESERVED_TYPE_NAMES.contains(&name) {
        return Err(Error::forbidden_name(path, name.to_string()));
    }

    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(Error::invalid_name(path, name.to_string()));
    };

    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return Err(Error::invalid_name(path, name.to_string()));
    }
    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')) {
        return Err(Error::invalid_name(path, name.to_string()));
    }

    Ok(())
}

fn sanitise_key(field_name: &str) -> String {
    if is_identifier(field_name) {
        field_name.to_string()
    } else {
        format!("\"{}\"", escape_string(field_name))
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

fn escape_string(value: &str) -> Cow<'_, str> {
    if !value
        .chars()
        .any(|ch| ch == '"' || ch == '\\' || ch == '\n' || ch == '\r' || ch == '\t')
    {
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
            _ => escaped.push(ch),
        }
    }

    Cow::Owned(escaped)
}
