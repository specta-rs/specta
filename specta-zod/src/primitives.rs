//! Primitives provide building blocks for Specta-based libraries.

use std::{borrow::Cow, fmt::Write as _};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Fields, GenericReference, List, Map, NamedDataType, NamedReference,
        NamedReferenceType, OpaqueReference, Primitive, Reference, Struct, Tuple,
    },
};
use specta_typescript::{Layout as TypescriptLayout, Typescript};

use crate::{Error, Layout, Zod, opaque, reserved_names::RESERVED_TYPE_NAMES};

pub(crate) type TypeRenderStack = Vec<(Cow<'static, str>, Cow<'static, str>)>;

fn named_reference_generics(r: &NamedReference) -> Result<&[(GenericReference, DataType)], Error> {
    match &r.inner {
        NamedReferenceType::Reference { generics, .. } => Ok(generics),
        NamedReferenceType::Inline { .. } => Ok(&[]),
        NamedReferenceType::Recursive(_) => Err(Error::dangling_named_reference(format!(
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
        NamedReferenceType::Recursive(_) => Err(Error::dangling_named_reference(format!(
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
    validate_type_name(&base_name, name_path.clone())?;
    let schema_name = format!("{base_name}Schema");

    let Some(ty) = &ndt.ty else {
        return Ok(());
    };

    type_render_stack.push((ndt.module_path.clone(), ndt.name.clone()));

    let result = (|| {
        let typescript = Typescript::default().layout(match exporter.layout {
            Layout::Namespaces => TypescriptLayout::Namespaces,
            Layout::FlatFile => TypescriptLayout::FlatFile,
            Layout::ModulePrefixedName => TypescriptLayout::ModulePrefixedName,
            Layout::Files => TypescriptLayout::Files,
        });
        let mut alias_ndt = ndt.clone();
        alias_ndt.generics.to_mut().iter_mut().for_each(|generic| {
            if let Some(default) = &mut generic.default {
                typescript_alias_datatype(default, false, types);
            }
        });
        if let Some(ty) = &mut alias_ndt.ty {
            typescript_alias_datatype(ty, false, types);
        }
        let render_type_alias = || {
            specta_typescript::primitives::export(
                &typescript,
                types,
                std::iter::once(&alias_ndt),
                indent,
            )
        };
        let mut type_alias = if exporter.layout == Layout::Files {
            specta_typescript::with_module_path(&ndt.module_path, render_type_alias)
        } else {
            render_type_alias()
        }
        .map_err(|source| {
            Error::framework("failed to render the inferred TypeScript type", source)
        })?;
        if exporter.layout == Layout::Files {
            let current_alias = if ndt.module_path.is_empty() {
                "$root".to_string()
            } else {
                ndt.module_path.split("::").collect::<Vec<_>>().join("$")
            };
            type_alias = replace_typescript_code(&type_alias, &format!("{current_alias}."), "");
            let sanitized_current_alias = crate::zod::module_alias(&ndt.module_path);
            if current_alias != sanitized_current_alias {
                type_alias = replace_typescript_code(
                    &type_alias,
                    &format!("{sanitized_current_alias}."),
                    "",
                );
            }
            for module_path in types
                .into_unsorted_iter()
                .map(|ndt| ndt.module_path.as_ref())
                .filter(|path| !path.is_empty() && *path != ndt.module_path.as_ref())
                .collect::<std::collections::BTreeSet<_>>()
            {
                let raw = module_path.split("::").collect::<Vec<_>>().join("$");
                let sanitized = crate::zod::module_alias(module_path);
                if raw != sanitized {
                    type_alias = replace_typescript_code(
                        &type_alias,
                        &format!("{raw}."),
                        &format!("{sanitized}."),
                    );
                }
            }
        } else if exporter.layout == Layout::Namespaces {
            let module_paths = types
                .into_unsorted_iter()
                .map(|ndt| ndt.module_path.as_ref())
                .filter(|path| !path.is_empty())
                .collect::<std::collections::BTreeSet<_>>();
            for module_path in module_paths {
                let raw = module_path.split("::").collect::<Vec<_>>().join(".");
                let sanitized = crate::zod::namespace_module_path(module_path);
                if raw != sanitized {
                    type_alias = replace_typescript_code(
                        &type_alias,
                        &format!("$s$.{raw}."),
                        &format!("$s$.{sanitized}."),
                    );
                }
            }
        }
        s.push_str(&type_alias);

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
            let schema_expr = indent_continuations(&schema_expr, indent);

            writeln!(
                s,
                "{indent}export const {schema_name}: z.ZodType<{base_name}> = {schema_expr};"
            )?;
            return Ok(());
        }

        let mut generic_params = Vec::with_capacity(ndt.generics.len());
        let mut fn_params = Vec::with_capacity(ndt.generics.len());
        let mut first_default = None;
        for generic in ndt.generics.iter() {
            let name = generic.name.as_ref();
            validate_type_name(name, format!("{name_path}.<generic {name}>"))?;

            if let Some(default) = &generic.default {
                first_default.get_or_insert(fn_params.len());
                let mut default_schema = String::new();
                datatype(
                    &mut default_schema,
                    exporter,
                    types,
                    default,
                    vec![ndt.name.clone(), format!("<generic {name} default>").into()],
                    &[],
                    false,
                    type_render_stack,
                )?;
                generic_params.push(format!("{name} extends z.ZodType"));
                fn_params.push(format!("{name}: z.ZodType = {default_schema}"));
            } else {
                generic_params.push(format!("{name} extends z.ZodType"));
                fn_params.push(format!("{name}: z.ZodType"));
            }
        }

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
        let schema_expr = indent_continuations(&schema_expr, indent);

        if let Some(first_default) = first_default {
            for argument_count in first_default..=ndt.generics.len() {
                let generics = generic_params[..argument_count].join(", ");
                let params = ndt.generics[..argument_count]
                    .iter()
                    .map(|generic| {
                        let name = generic.name.as_ref();
                        format!("{name}: {name}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let type_arguments = ndt.generics[..argument_count]
                    .iter()
                    .map(|generic| format!("z.output<{}>", generic.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let type_arguments = if type_arguments.is_empty() {
                    String::new()
                } else {
                    format!("<{type_arguments}>")
                };
                let generics = if generics.is_empty() {
                    String::new()
                } else {
                    format!("<{generics}>")
                };
                writeln!(
                    s,
                    "{indent}export function {schema_name}{generics}({params}): z.ZodType<{base_name}{type_arguments}>;"
                )?;
            }
            writeln!(
                s,
                "{indent}export function {schema_name}({}): z.ZodType<any> {{\n{indent}\treturn {schema_expr};\n{indent}}}",
                fn_params.join(", ")
            )?;
        } else {
            let generic_params = generic_params.join(", ");
            let fn_params = ndt
                .generics
                .iter()
                .map(|generic| {
                    let name = generic.name.as_ref();
                    format!("{name}: {name}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(
                s,
                "{indent}export const {schema_name} = <{generic_params}>({fn_params}) => {schema_expr};"
            )?;
        }

        Ok(())
    })();

    type_render_stack.pop();
    result
}

fn indent_continuations<'a>(value: &'a str, indent: &str) -> Cow<'a, str> {
    if indent.is_empty() || !value.contains('\n') {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.replace('\n', &format!("\n{indent}")))
    }
}

fn typescript_alias_datatype(dt: &mut DataType, map_key: bool, types: &Types) {
    match dt {
        DataType::Primitive(_) | DataType::Generic(_) => {}
        DataType::List(list) => typescript_alias_datatype(&mut list.ty, map_key, types),
        DataType::Map(map) => {
            if contains_zod_define(map.key_ty(), types, &[], &mut Vec::new()) {
                *map.key_ty_mut() = DataType::Reference(specta_typescript::define("string"));
            } else {
                typescript_alias_datatype(map.key_ty_mut(), true, types);
            }
            typescript_alias_datatype(map.value_ty_mut(), false, types);
        }
        DataType::Nullable(inner) => typescript_alias_datatype(inner, map_key, types),
        DataType::Struct(strct) => typescript_alias_fields(&mut strct.fields, types),
        DataType::Enum(enm) => enm
            .variants
            .iter_mut()
            .for_each(|(_, variant)| typescript_alias_fields(&mut variant.fields, types)),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter_mut()
            .for_each(|dt| typescript_alias_datatype(dt, map_key, types)),
        DataType::Intersection(types_) => types_
            .iter_mut()
            .for_each(|dt| typescript_alias_datatype(dt, map_key, types)),
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => typescript_alias_datatype(dt, map_key, types),
            NamedReferenceType::Reference { generics, .. } => generics
                .iter_mut()
                .for_each(|(_, dt)| typescript_alias_datatype(dt, map_key, types)),
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Reference(Reference::Opaque(reference)) => {
            let ty = if reference.downcast_ref::<opaque::Any>().is_some() {
                Some("any")
            } else if reference.downcast_ref::<opaque::Never>().is_some() {
                Some("never")
            } else if reference.downcast_ref::<opaque::Unknown>().is_some() {
                Some("unknown")
            } else if reference.downcast_ref::<opaque::Define>().is_some() {
                // A raw Zod expression carries no corresponding TypeScript type metadata.
                // Map keys must still be valid TypeScript property keys, and JSON object
                // keys are strings regardless of the schema used to validate them.
                Some(if map_key { "string" } else { "unknown" })
            } else {
                None
            };
            if let Some(ty) = ty {
                *dt = DataType::Reference(specta_typescript::define(ty));
            }
        }
    }
}

fn typescript_alias_fields(fields: &mut Fields, types: &Types) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|field| field.ty.as_mut())
            .for_each(|dt| typescript_alias_datatype(dt, false, types)),
        Fields::Named(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|(_, field)| field.ty.as_mut())
            .for_each(|dt| typescript_alias_datatype(dt, false, types)),
    }
}

fn contains_zod_define(
    dt: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    stack: &mut Vec<NamedReference>,
) -> bool {
    match dt {
        DataType::Primitive(_) => false,
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .is_some_and(|(_, dt)| {
                !matches!(dt, DataType::Generic(candidate) if candidate == generic)
                    && contains_zod_define(dt, types, generics, stack)
            }),
        DataType::List(list) => contains_zod_define(&list.ty, types, generics, stack),
        DataType::Map(map) => {
            contains_zod_define(map.key_ty(), types, generics, stack)
                || contains_zod_define(map.value_ty(), types, generics, stack)
        }
        DataType::Nullable(inner) => contains_zod_define(inner, types, generics, stack),
        DataType::Struct(strct) => fields_contain_zod_define(&strct.fields, types, generics, stack),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| fields_contain_zod_define(&variant.fields, types, generics, stack)),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|dt| contains_zod_define(dt, types, generics, stack)),
        DataType::Intersection(types_) => types_
            .iter()
            .any(|dt| contains_zod_define(dt, types, generics, stack)),
        DataType::Reference(Reference::Opaque(reference)) => {
            reference.downcast_ref::<opaque::Define>().is_some()
        }
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                contains_zod_define(dt, types, generics, stack)
            }
            NamedReferenceType::Reference { .. } => {
                if stack.contains(reference) {
                    return false;
                }
                let Some(ty) = types.get(reference).and_then(|ndt| ndt.ty.as_ref()) else {
                    return false;
                };
                let Ok(reference_generics) = resolved_reference_generics(reference, generics)
                else {
                    return false;
                };
                stack.push(reference.clone());
                let contains = contains_zod_define(ty, types, &reference_generics, stack);
                stack.pop();
                contains
            }
            NamedReferenceType::Recursive(_) => false,
        },
    }
}

fn fields_contain_zod_define(
    fields: &Fields,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    stack: &mut Vec<NamedReference>,
) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) => fields.fields.iter().any(|field| {
            field
                .ty
                .as_ref()
                .is_some_and(|dt| contains_zod_define(dt, types, generics, stack))
        }),
        Fields::Named(fields) => fields.fields.iter().any(|(_, field)| {
            field
                .ty
                .as_ref()
                .is_some_and(|dt| contains_zod_define(dt, types, generics, stack))
        }),
    }
}

fn replace_typescript_code(input: &str, from: &str, to: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        SingleQuote,
        DoubleQuote,
        Template,
        LineComment,
        BlockComment,
    }

    let bytes = input.as_bytes();
    let mut state = State::Code;
    let mut index = 0;
    let mut copied_to = 0;
    let mut output = String::with_capacity(input.len());

    while index < bytes.len() {
        if matches!(state, State::Code)
            && input.is_char_boundary(index)
            && input[index..].starts_with(from)
            && input[..index]
                .chars()
                .next_back()
                .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')))
        {
            output.push_str(&input[copied_to..index]);
            output.push_str(to);
            index += from.len();
            copied_to = index;
            continue;
        }

        let byte = bytes[index];
        match state {
            State::Code => match (byte, bytes.get(index + 1).copied()) {
                (b'/', Some(b'/')) => {
                    state = State::LineComment;
                    index += 1;
                }
                (b'/', Some(b'*')) => {
                    state = State::BlockComment;
                    index += 1;
                }
                (b'\'', _) => state = State::SingleQuote,
                (b'"', _) => state = State::DoubleQuote,
                (b'`', _) => state = State::Template,
                _ => {}
            },
            State::SingleQuote | State::DoubleQuote | State::Template => {
                if byte == b'\\' {
                    index += 1;
                } else if matches!(state, State::SingleQuote) && byte == b'\''
                    || matches!(state, State::DoubleQuote) && byte == b'"'
                    || matches!(state, State::Template) && byte == b'`'
                {
                    state = State::Code;
                }
            }
            State::LineComment if byte == b'\n' => state = State::Code,
            State::BlockComment if byte == b'*' && bytes.get(index + 1) == Some(&b'/') => {
                state = State::Code;
                index += 1;
            }
            State::LineComment | State::BlockComment => {}
        }
        index += 1;
    }

    output.push_str(&input[copied_to..]);
    output
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
        DataType::Primitive(p) => s.push_str(primitive_dt(p, location)?),
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
            if intersection.is_empty() {
                s.push_str("z.unknown()");
                return Ok(());
            }
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

fn primitive_dt(p: &Primitive, location: Vec<Cow<'static, str>>) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 => "z.int().min(-128).max(127)",
        i16 => "z.int().min(-32768).max(32767)",
        i32 => "z.int().min(-2147483648).max(2147483647)",
        u8 => "z.int().min(0).max(255)",
        u16 => "z.int().min(0).max(65535)",
        u32 => "z.int().min(0).max(4294967295)",
        // JSON serializers encode non-finite floats as `null`.
        f16 | f32 | f64 => "z.number().nullable()",
        usize | isize | i64 | u64 | i128 | u128 | f128 => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        Primitive::bool => "z.boolean()",
        str => "z.string()",
        char => "z.string().refine((value) => [...value].length === 1)",
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
    map_key_datatype(
        &mut key,
        exporter,
        types,
        m.key_ty(),
        child_location(&location, "<key>"),
        generics,
        type_render_stack,
    )?;
    let mut value = String::new();
    datatype(
        &mut value,
        exporter,
        types,
        m.value_ty(),
        child_location(&location, "<value>"),
        generics,
        false,
        type_render_stack,
    )?;

    let constructor = if map_key_is_finite(m.key_ty(), types, generics) {
        "z.partialRecord"
    } else {
        "z.record"
    };
    write!(s, "{constructor}({key}, {value})")?;
    Ok(())
}

fn map_key_is_finite(
    dt: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
) -> bool {
    match dt {
        DataType::Primitive(Primitive::bool) => true,
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .all(|(_, variant)| match &variant.fields {
                Fields::Unit => true,
                Fields::Unnamed(fields) => {
                    let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                    fields
                        .next()
                        .is_some_and(|dt| map_key_is_finite(dt, types, generics))
                        && fields.next().is_none()
                }
                Fields::Named(_) => false,
            }),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unnamed(fields) => {
                let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                fields
                    .next()
                    .is_some_and(|dt| map_key_is_finite(dt, types, generics))
                    && fields.next().is_none()
            }
            _ => false,
        },
        DataType::Reference(Reference::Named(reference)) => types
            .get(reference)
            .and_then(|ndt| ndt.ty.as_ref())
            .is_some_and(|dt| {
                resolved_reference_generics(reference, generics)
                    .is_ok_and(|generics| map_key_is_finite(dt, types, &generics))
            }),
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .is_some_and(|(_, dt)| {
                !matches!(dt, DataType::Generic(candidate) if candidate == generic)
                    && map_key_is_finite(dt, types, generics)
            }),
        _ => false,
    }
}

fn map_key_datatype(
    s: &mut String,
    exporter: &Zod,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(Primitive::str) => s.push_str("z.string()"),
        DataType::Primitive(Primitive::char) => {
            s.push_str("z.string().refine((value) => [...value].length === 1)")
        }
        DataType::Primitive(Primitive::bool) => s.push_str(r#"z.enum(["true", "false"])"#),
        DataType::Primitive(Primitive::i8) => s.push_str(
            r"z.string().regex(/^-?\d+$/).refine((value) => Number(value) >= -128 && Number(value) <= 127)",
        ),
        DataType::Primitive(Primitive::i16) => s.push_str(
            r"z.string().regex(/^-?\d+$/).refine((value) => Number(value) >= -32768 && Number(value) <= 32767)",
        ),
        DataType::Primitive(Primitive::i32) => s.push_str(
            r"z.string().regex(/^-?\d+$/).refine((value) => Number(value) >= -2147483648 && Number(value) <= 2147483647)",
        ),
        DataType::Primitive(Primitive::isize | Primitive::usize) => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        DataType::Primitive(Primitive::u8) => s.push_str(
            r"z.string().regex(/^\d+$/).refine((value) => Number(value) <= 255)",
        ),
        DataType::Primitive(Primitive::u16) => s.push_str(
            r"z.string().regex(/^\d+$/).refine((value) => Number(value) <= 65535)",
        ),
        DataType::Primitive(Primitive::u32) => s.push_str(
            r"z.string().regex(/^\d+$/).refine((value) => Number(value) <= 4294967295)",
        ),
        DataType::Primitive(Primitive::f16 | Primitive::f32 | Primitive::f64) => {
            s.push_str(
                r"z.string().regex(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/).refine((value) => Number.isFinite(Number(value)))",
            )
        }
        DataType::Struct(strct) => {
            let Fields::Unnamed(fields) = &strct.fields else {
                return datatype(
                    s,
                    exporter,
                    types,
                    dt,
                    location,
                    generics,
                    false,
                    type_render_stack,
                );
            };
            let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
            let Some(field) = fields.next() else {
                return datatype(
                    s,
                    exporter,
                    types,
                    dt,
                    location,
                    generics,
                    false,
                    type_render_stack,
                );
            };
            if fields.next().is_some() {
                return datatype(
                    s,
                    exporter,
                    types,
                    dt,
                    location,
                    generics,
                    false,
                    type_render_stack,
                );
            }
            map_key_datatype(
                s,
                exporter,
                types,
                field,
                location,
                generics,
                type_render_stack,
            )?;
        }
        DataType::Enum(enm) => {
            let mut variants = Vec::new();
            for (name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                let mut rendered = String::new();
                match &variant.fields {
                    Fields::Unit => write!(rendered, "z.literal(\"{}\")", escape_string(name))?,
                    Fields::Unnamed(fields) => {
                        let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                        let Some(field) = fields.next() else {
                            continue;
                        };
                        if fields.next().is_some() {
                            return datatype(
                                s,
                                exporter,
                                types,
                                dt,
                                location,
                                generics,
                                false,
                                type_render_stack,
                            );
                        }
                        map_key_datatype(
                            &mut rendered,
                            exporter,
                            types,
                            field,
                            child_location(&location, name.to_string()),
                            generics,
                            type_render_stack,
                        )?;
                    }
                    Fields::Named(_) => {
                        return datatype(
                            s,
                            exporter,
                            types,
                            dt,
                            location,
                            generics,
                            false,
                            type_render_stack,
                        );
                    }
                }
                variants.push(rendered);
            }
            match variants.as_slice() {
                [] => s.push_str("z.never()"),
                [variant] => s.push_str(variant),
                variants => write!(s, "z.union([{}])", variants.join(", "))?,
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            crate::references::track_nr(reference);
            let ty = named_reference_ty(types, reference)?;
            let reference_generics = resolved_reference_generics(reference, generics)?;
            map_key_datatype(
                s,
                exporter,
                types,
                ty,
                location,
                &reference_generics,
                type_render_stack,
            )?;
        }
        DataType::Generic(generic) => {
            if let Some((_, dt)) = generics.iter().find(|(candidate, _)| candidate == generic)
                && !matches!(dt, DataType::Generic(candidate) if candidate == generic)
            {
                map_key_datatype(
                    s,
                    exporter,
                    types,
                    dt,
                    location,
                    generics,
                    type_render_stack,
                )?;
            } else {
                generic_dt(s, generic);
            }
        }
        _ => datatype(
            s,
            exporter,
            types,
            dt,
            location,
            generics,
            false,
            type_render_stack,
        )?,
    }
    Ok(())
}

fn resolved_reference_generics(
    reference: &NamedReference,
    outer_generics: &[(GenericReference, DataType)],
) -> Result<Vec<(GenericReference, DataType)>, Error> {
    named_reference_generics(reference).map(|generics| {
        generics
            .iter()
            .map(|(generic, dt)| {
                let mut dt = dt.clone();
                substitute_generics(&mut dt, outer_generics);
                (generic.clone(), dt)
            })
            .collect()
    })
}

fn substitute_generics(dt: &mut DataType, generics: &[(GenericReference, DataType)]) {
    match dt {
        DataType::Generic(generic) => {
            if let Some((_, replacement)) =
                generics.iter().find(|(candidate, _)| candidate == generic)
                && !matches!(replacement, DataType::Generic(candidate) if candidate == generic)
            {
                *dt = replacement.clone();
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, generics),
        DataType::Enum(enm) => enm
            .variants
            .iter_mut()
            .for_each(|(_, variant)| substitute_field_generics(&mut variant.fields, generics)),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter_mut()
            .for_each(|dt| substitute_generics(dt, generics)),
        DataType::Intersection(types) => types
            .iter_mut()
            .for_each(|dt| substitute_generics(dt, generics)),
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, generics),
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => reference_generics
                .iter_mut()
                .for_each(|(_, dt)| substitute_generics(dt, generics)),
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(fields: &mut Fields, generics: &[(GenericReference, DataType)]) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|field| field.ty.as_mut())
            .for_each(|dt| substitute_generics(dt, generics)),
        Fields::Named(fields) => fields
            .fields
            .iter_mut()
            .filter_map(|(_, field)| field.ty.as_mut())
            .for_each(|dt| substitute_generics(dt, generics)),
    }
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
                    child_location(&location, i.to_string()),
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
                    // serde accepts sequences truncated anywhere inside the
                    // trailing run of defaulted (`optional`) elements; zod
                    // expresses that with trailing `.optional()` elements.
                    let mut optional_from = 0;
                    for (i, (field, _)) in fields.iter().enumerate() {
                        if !field.optional {
                            optional_from = i + 1;
                        }
                    }

                    s.push_str("z.tuple([");
                    for (i, (_field, ty)) in fields.iter().enumerate() {
                        if i != 0 {
                            s.push_str(", ");
                        }
                        datatype_with_inline_attr(
                            s,
                            exporter,
                            types,
                            ty,
                            child_location(&location, i.to_string()),
                            generics,
                            false,
                            type_render_stack,
                        )?;
                        if i >= optional_from {
                            s.push_str(".optional()");
                        }
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
                s.push_str("z.object({})");
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
                    child_location(&location, name.to_string()),
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
            let strict_object = matches!(&variant.fields, Fields::Named(named) if
                named.fields.iter().filter(|(_, field)| field.ty.is_some()).count() == 1
                    && named.fields.iter().any(|(field_name, field)|
                        field.ty.is_some() && field_name.as_ref() == name.as_ref()));
            enum_variant_dt(
                exporter,
                types,
                name.as_ref(),
                variant,
                strict_object,
                child_location(&location, name.to_string()),
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
    strict_object: bool,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<Option<String>, Error> {
    match &variant.fields {
        Fields::Unit => Ok(Some(format!("z.literal(\"{}\")", escape_string(name)))),
        Fields::Named(named) => {
            if named.fields.iter().all(|(_, field)| field.ty.is_none()) {
                return Ok(Some("z.strictObject({})".to_string()));
            }

            let mut schema = if strict_object {
                String::from("z.strictObject({")
            } else {
                String::from("z.object({")
            };
            let mut has_field = false;

            for (field_name, field) in &named.fields {
                let Some(ty) = field.ty.as_ref() else {
                    continue;
                };

                has_field = true;
                let mut value = String::new();
                datatype_with_inline_attr(
                    &mut value,
                    exporter,
                    types,
                    ty,
                    child_location(&location, field_name.to_string()),
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

            Ok(Some(schema))
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
                    let mut optional_from = 0;
                    for (i, (field, _)) in fields.iter().enumerate() {
                        if !field.optional {
                            optional_from = i + 1;
                        }
                    }

                    let mut out = String::from("z.tuple([");
                    for (i, (_field, ty)) in fields.iter().enumerate() {
                        if i != 0 {
                            out.push_str(", ");
                        }
                        let mut item = String::new();
                        datatype_with_inline_attr(
                            &mut item,
                            exporter,
                            types,
                            ty,
                            child_location(&location, i.to_string()),
                            generics,
                            false,
                            type_render_stack,
                        )?;
                        out.push_str(&item);
                        if i >= optional_from {
                            out.push_str(".optional()");
                        }
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
        Layout::Namespaces => {
            let mut name = String::from("$s$");
            for segment in crate::zod::namespace_module_path(&ndt.module_path)
                .split('.')
                .filter(|segment| !segment.is_empty())
            {
                name.push('.');
                name.push_str(segment);
            }
            name.push('.');
            name.push_str(&ndt.name);
            name.push_str("Schema");
            name
        }
        Layout::FlatFile => format!("{}Schema", ndt.name),
        Layout::ModulePrefixedName => {
            let mut name = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            name.push('_');
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

    let mut reference_expr = schema_name;
    let reference_generics = match &r.inner {
        NamedReferenceType::Recursive(_) => &[],
        _ => named_reference_generics(r)?,
    };
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

    // Declarations are sorted, so this reference may target a schema whose
    // initializer has not run yet. Laziness also handles recursive references.
    write!(s, "z.lazy(() => {reference_expr})")?;

    Ok(())
}

pub(crate) fn exported_type_name(exporter: &Zod, ndt: &NamedDataType) -> Cow<'static, str> {
    match exporter.layout {
        Layout::Namespaces | Layout::FlatFile | Layout::Files => ndt.name.clone(),
        Layout::ModulePrefixedName => {
            let mut s = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            s.push('_');
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

    if !(first.is_alphabetic() || first == '_') {
        return Err(Error::invalid_name(path, name.to_string()));
    }
    if chars.any(|ch| !(ch.is_alphanumeric() || ch == '_')) {
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

fn child_location(
    location: &[Cow<'static, str>],
    child: impl Into<Cow<'static, str>>,
) -> Vec<Cow<'static, str>> {
    let mut location = location.to_vec();
    location.push(child.into());
    location
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

#[cfg(test)]
mod tests {
    use super::replace_typescript_code;

    #[test]
    fn typescript_alias_replacement_respects_tokens_and_literals() {
        assert_eq!(
            replace_typescript_code(
                r#"foo.Type | myfoo.Type | "foo.Type" /* foo.Type */"#,
                "foo.",
                "",
            ),
            r#"Type | myfoo.Type | "foo.Type" /* foo.Type */"#
        );
    }
}
