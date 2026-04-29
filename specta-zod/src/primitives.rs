//! Primitives provide building blocks for Specta-based libraries.

use std::{borrow::Cow, cell::RefCell, fmt::Write as _};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Fields, Generic, GenericReference, List, Map, NamedDataType,
        NamedReference, OpaqueReference, Primitive, Reference, Struct, Tuple,
    },
};

use crate::{
    BigIntExportBehavior, Error, Layout, Zod, opaque, reserved_names::RESERVED_TYPE_NAMES,
};

thread_local! {
    static INLINE_REFERENCE_STACK: RefCell<Vec<NamedReference>> = const { RefCell::new(Vec::new()) };
    static TYPE_RENDER_STACK: RefCell<Vec<(Cow<'static, str>, Cow<'static, str>)>> = const { RefCell::new(Vec::new()) };
    static GENERIC_NAME_STACK: RefCell<Vec<Vec<Generic>>> = const { RefCell::new(Vec::new()) };
}

struct TypeRenderGuard;

impl Drop for TypeRenderGuard {
    fn drop(&mut self) {
        TYPE_RENDER_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

struct GenericScopeGuard;

impl Drop for GenericScopeGuard {
    fn drop(&mut self) {
        GENERIC_NAME_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

fn push_type_render_stack(
    module_path: Cow<'static, str>,
    name: Cow<'static, str>,
) -> TypeRenderGuard {
    TYPE_RENDER_STACK.with(|stack| {
        stack.borrow_mut().push((module_path, name));
    });
    TypeRenderGuard
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
                .map(|generic| generic.name().clone())
        })
    })
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

    child.iter().cloned().chain(unshadowed_parent).collect()
}

/// Generate a group of `export const XSchema = ...` declarations for named types.
pub fn export<'a>(
    exporter: &dyn AsRef<Zod>,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut s = String::new();
    export_internal(&mut s, exporter.as_ref(), types, ndts, indent)?;
    Ok(s)
}

pub(crate) fn export_internal<'a>(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<(), Error> {
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
    exporter: &Zod,
    types: &Types,
    ndt: &NamedDataType,
    indent: &str,
) -> Result<(), Error> {
    let base_name = exported_type_name(exporter, ndt);
    let name_path = if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    };
    validate_type_name(&base_name, name_path)?;
    let schema_name = format!("{base_name}Schema");

    let _guard = push_type_render_stack(ndt.module_path.clone(), ndt.name.clone());
    let generic_scope = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();
    let _generic_scope = push_generic_scope(&generic_scope);

    let Some(ty) = &ndt.ty else {
        return Ok(());
    };

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
}

/// Generate an inline Zod expression for a [`DataType`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically. Map both the full [`Types`] graph and any top-level
/// [`DataType`] values before calling this helper.
pub fn inline(exporter: &dyn AsRef<Zod>, types: &Types, dt: &DataType) -> Result<String, Error> {
    let mut s = String::new();
    datatype(&mut s, exporter.as_ref(), types, dt, vec![], &[], false)?;
    Ok(s)
}

/// Generate a Zod expression for a [`Reference`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically.
pub fn reference(exporter: &dyn AsRef<Zod>, types: &Types, r: &Reference) -> Result<String, Error> {
    let mut s = String::new();
    reference_dt(&mut s, exporter.as_ref(), types, r, vec![], &[])?;
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
) -> Result<(), Error> {
    datatype(s, exporter, types, dt, location, generics, inline)
}

fn datatype(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    force_inline_ref: bool,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(p) => s.push_str(primitive_dt(&exporter.bigint, p, location)?),
        DataType::List(l) => list_dt(s, exporter, types, l, location, generics)?,
        DataType::Map(m) => map_dt(s, exporter, types, m, location, generics)?,
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
            )?;
            write!(s, "{inner}.nullable()")?;
        }
        DataType::Struct(st) => struct_dt(s, exporter, types, st, location, generics)?,
        DataType::Enum(enm) => enum_dt(s, exporter, types, enm, location, generics)?,
        DataType::Tuple(tuple) => tuple_dt(s, exporter, types, tuple, location, generics)?,
        DataType::Reference(r) => {
            if force_inline_ref {
                match r {
                    Reference::Named(named) => {
                        let ty = named
                            .ty(types)
                            .ok_or_else(|| Error::dangling_named_reference(format!("{named:?}")))?;
                        let combined_generics = merged_generics(generics, named.generics());
                        datatype(s, exporter, types, ty, location, &combined_generics, false)?;
                    }
                    _ => reference_dt(s, exporter, types, r, location, generics)?,
                }
            } else {
                reference_dt(s, exporter, types, r, location, generics)?;
            }
        }
        DataType::Generic(g) => generic_dt(s, exporter, types, g, location, generics)?,
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
) -> Result<(), Error> {
    let mut dt = String::new();
    datatype(&mut dt, exporter, types, &l.ty, location, generics, false)?;

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
) -> Result<(), Error> {
    match t.elements.as_slice() {
        [] => s.push_str("z.null()"),
        elements => {
            s.push_str("z.tuple([");
            for (i, dt) in elements.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }
                datatype(s, exporter, types, dt, location.clone(), generics, false)?;
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
                    datatype_with_inline_attr(s, exporter, types, ty, location, generics, false)?;
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
                        &mut out, exporter, types, ty, location, generics, false,
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
) -> Result<(), Error> {
    match r {
        Reference::Named(r) => reference_named_dt(s, exporter, types, r, location, generics),
        Reference::Opaque(r) => reference_opaque_dt(s, r),
    }
}

fn generic_dt(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    g: &GenericReference,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
) -> Result<(), Error> {
    if let Some((_, resolved_dt)) = generics.iter().find(|(ge, _)| ge == g) {
        if matches!(resolved_dt, DataType::Generic(inner) if inner == g) {
            let generic_name = resolve_generic_name(g)
                .ok_or_else(|| Error::unresolved_generic_reference(format!("{g:?}")))?;
            s.push_str(generic_name.as_ref());
        } else {
            datatype(s, exporter, types, resolved_dt, location, generics, false)?;
        }
    } else {
        let generic_name = resolve_generic_name(g)
            .ok_or_else(|| Error::unresolved_generic_reference(format!("{g:?}")))?;
        s.push_str(generic_name.as_ref());
    }

    Ok(())
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
) -> Result<(), Error> {
    let ndt = r
        .get(types)
        .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;

    let generic_scope = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();
    let _generic_scope = push_generic_scope(&generic_scope);

    if matches!(r.inner, specta::datatype::NamedReferenceType::Inline { .. }) {
        let ty = r
            .ty(types)
            .ok_or_else(|| Error::dangling_named_reference(format!("{r:?}")))?;
        let inline_key = r.clone();
        let already_inlining = INLINE_REFERENCE_STACK
            .with(|stack| stack.borrow().iter().any(|key| key == &inline_key));

        if !already_inlining {
            INLINE_REFERENCE_STACK.with(|stack| stack.borrow_mut().push(inline_key));
            let combined_generics = merged_generics(generics, r.generics());
            let result = datatype(s, exporter, types, ty, location, &combined_generics, false);
            INLINE_REFERENCE_STACK.with(|stack| {
                stack.borrow_mut().pop();
            });
            return result;
        }
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

    let should_lazy = TYPE_RENDER_STACK.with(|stack| {
        stack
            .borrow()
            .iter()
            .any(|(module, name)| module == &ndt.module_path && name == &ndt.name)
    });

    let mut reference_expr = schema_name;
    if !r.generics().is_empty() {
        let scoped_generics = generics
            .iter()
            .filter(|(parent_generic, _)| {
                !r.generics()
                    .iter()
                    .any(|(child_generic, _)| child_generic == parent_generic)
            })
            .cloned()
            .collect::<Vec<_>>();

        reference_expr.push('(');
        for (i, (_, v)) in r.generics().iter().enumerate() {
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
