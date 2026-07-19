//! Primitives provide building blocks for Specta-based libraries.

use std::{borrow::Cow, collections::HashSet, fmt::Write as _};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Fields, GenericReference, List, Map, NamedDataType, NamedReference,
        NamedReferenceType, OpaqueReference, Primitive, Reference, Struct, Tuple,
    },
};
use specta_typescript::{Layout as TypescriptLayout, Typescript};

use crate::{Error, Layout, Valibot, map_keys, opaque, reserved_names::RESERVED_TYPE_NAMES};

pub(crate) type TypeRenderStack = Vec<(Cow<'static, str>, Cow<'static, str>)>;

const STRICT_OBJECT_MARKER: &str = "specta:strict_object";
const OPTIONAL_FLATTEN_UNION_MARKER: &str = "specta_serde:optional_flatten_union";

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
///
/// The generated code expects Valibot to be bound to `v` and
/// [`crate::runtime_helpers`] to be emitted once in the containing module.
pub fn export<'a>(
    exporter: &dyn AsRef<Valibot>,
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
    exporter: &Valibot,
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
    exporter: &Valibot,
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
        let alias_module_path = (exporter.layout == Layout::Files)
            .then(crate::references::current_module_path)
            .flatten()
            .unwrap_or_else(|| ndt.module_path.to_string());
        let mut type_alias = if exporter.layout == Layout::Files {
            specta_typescript::with_module_path(&alias_module_path, render_type_alias)
        } else {
            render_type_alias()
        }
        .map_err(|source| {
            Error::framework("failed to render the inferred TypeScript type", source)
        })?;
        if exporter.layout == Layout::Files {
            let current_alias = if alias_module_path.is_empty() {
                "$root".to_string()
            } else {
                alias_module_path.split("::").collect::<Vec<_>>().join("$")
            };
            type_alias = replace_typescript_code(&type_alias, &format!("{current_alias}."), "");
            let sanitized_current_alias = crate::valibot::module_alias(&alias_module_path);
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
                .filter(|path| !path.is_empty() && *path != alias_module_path)
                .collect::<std::collections::BTreeSet<_>>()
            {
                let raw = module_path.split("::").collect::<Vec<_>>().join("$");
                let sanitized = crate::valibot::module_alias(module_path);
                if raw != sanitized {
                    type_alias = replace_typescript_code(
                        &type_alias,
                        &format!("{raw}."),
                        &format!("{sanitized}."),
                    );
                }
            }
        } else if exporter.layout == Layout::Namespaces {
            let mut module_paths = types
                .into_unsorted_iter()
                .map(|ndt| ndt.module_path.as_ref())
                .filter(|path| !path.is_empty())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            module_paths.sort_by_key(|path| std::cmp::Reverse(path.split("::").count()));
            for module_path in module_paths {
                let raw = module_path.split("::").collect::<Vec<_>>().join(".");
                let sanitized = crate::valibot::namespace_module_path(module_path);
                if raw != sanitized {
                    type_alias = replace_typescript_code(
                        &type_alias,
                        &format!("$s$.{raw}."),
                        &format!("$s$.{sanitized}."),
                    );
                }
            }
        }
        let alias_prefix = format!("{indent}export type ");
        if type_alias
            .lines()
            .any(|line| line.starts_with(&alias_prefix) && line.ends_with(" = "))
        {
            let had_trailing_newline = type_alias.ends_with('\n');
            type_alias = type_alias
                .lines()
                .map(|line| {
                    if line.starts_with(&alias_prefix) && line.ends_with(" = ") {
                        &line[..line.len() - 1]
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if had_trailing_newline {
                type_alias.push('\n');
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
                "{indent}export const {schema_name}: v.GenericSchema<{base_name}> = {schema_expr};"
            )?;
            return Ok(());
        }

        let map_key_generics = named_map_key_parameters(ndt, types);
        let mut generic_params = Vec::with_capacity(ndt.generics.len());
        let mut fn_params = Vec::with_capacity(ndt.generics.len());
        let mut overload_params = Vec::with_capacity(ndt.generics.len());
        let mut first_default = None;
        for (index, generic) in ndt.generics.iter().enumerate() {
            let name = generic.name.as_ref();
            validate_type_name(name, format!("{name_path}.<generic {name}>"))?;

            let mut generic_group = vec![format!("{name} extends v.GenericSchema")];
            let mut fn_group = Vec::new();
            let mut overload_group = vec![format!("{name}: {name}")];
            if let Some(default) = &generic.default {
                first_default.get_or_insert(index);
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
                fn_group.push(format!("{name}: v.GenericSchema = {default_schema}"));
                if map_key_generics.contains(name) {
                    let key_name = map_key_generic_name(name);
                    map_keys::validate_map_key(
                        default,
                        types,
                        format!("{name_path}.<generic {name} default key>"),
                    )?;
                    let mut default_key_schema = String::new();
                    map_key_datatype(
                        &mut default_key_schema,
                        exporter,
                        types,
                        default,
                        vec![
                            ndt.name.clone(),
                            format!("<generic {name} default key>").into(),
                        ],
                        &[],
                        type_render_stack,
                    )?;
                    generic_group.push(format!(
                        "{key_name} extends v.GenericSchema<string, string>"
                    ));
                    overload_group.push(format!("{key_name}: {key_name}"));
                    fn_group.push(format!(
                        "{key_name}: v.GenericSchema<string, string> = {default_key_schema}"
                    ));
                }
            } else {
                fn_group.push(format!("{name}: {name}"));
                if map_key_generics.contains(name) {
                    let key_name = map_key_generic_name(name);
                    generic_group.push(format!(
                        "{key_name} extends v.GenericSchema<string, string>"
                    ));
                    overload_group.push(format!("{key_name}: {key_name}"));
                    fn_group.push(format!("{key_name}: {key_name}"));
                }
            }
            generic_params.push(generic_group);
            fn_params.push(fn_group);
            overload_params.push(overload_group);
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
                let generics = generic_params[..argument_count]
                    .iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let params = overload_params[..argument_count]
                    .iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                let input_type_arguments = ndt.generics[..argument_count]
                    .iter()
                    .map(|generic| format!("v.InferInput<{}>", generic.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let output_type_arguments = ndt.generics[..argument_count]
                    .iter()
                    .map(|generic| format!("v.InferOutput<{}>", generic.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let apply_type_arguments = |type_arguments: String| {
                    if type_arguments.is_empty() {
                        base_name.to_string()
                    } else {
                        format!("{base_name}<{type_arguments}>")
                    }
                };
                let input_type = apply_type_arguments(input_type_arguments);
                let output_type = apply_type_arguments(output_type_arguments);
                let generics = if generics.is_empty() {
                    String::new()
                } else {
                    format!("<{generics}>")
                };
                writeln!(
                    s,
                    "{indent}export function {schema_name}{generics}({params}): v.GenericSchema<{input_type}, {output_type}>;"
                )?;
            }
            writeln!(
                s,
                "{indent}export function {schema_name}({}): v.GenericSchema<any> {{\n{indent}\treturn {schema_expr};\n{indent}}}",
                fn_params
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        } else {
            let generic_params = generic_params
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");
            let fn_params = fn_params
                .into_iter()
                .flatten()
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

fn strict_tuple_with_trailing_defaults(items: &[String], required_len: usize) -> String {
    let variants = (required_len..=items.len())
        .map(|len| format!("v.strictTuple([{}])", items[..len].join(", ")))
        .collect::<Vec<_>>();

    match variants.as_slice() {
        [variant] => variant.clone(),
        variants => format!("v.union([{}])", variants.join(", ")),
    }
}

fn resolved_typescript_map_key_alias(
    reference: &NamedReference,
    types: &Types,
    visiting: &mut HashSet<NamedReference>,
) -> Option<DataType> {
    if matches!(reference.inner, NamedReferenceType::Recursive(_))
        || !visiting.insert(reference.clone())
    {
        return None;
    }

    let result = (|| {
        let mut resolved = named_reference_ty(types, reference).ok()?.clone();
        substitute_generics(&mut resolved, named_reference_generics(reference).ok()?);

        loop {
            match &resolved {
                DataType::Struct(strct) => {
                    let Fields::Unnamed(fields) = &strct.fields else {
                        break;
                    };
                    let mut live = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                    let Some(inner) = live.next() else {
                        break;
                    };
                    if live.next().is_some() {
                        break;
                    }
                    resolved = inner.clone();
                }
                DataType::Reference(Reference::Named(inner)) => {
                    resolved = resolved_typescript_map_key_alias(inner, types, visiting)?;
                }
                _ => break,
            }
        }

        Some(resolved)
    })();
    visiting.remove(reference);
    result
}

fn typescript_alias_datatype(dt: &mut DataType, map_key: bool, types: &Types) {
    match dt {
        DataType::Primitive(_) => {}
        DataType::Generic(_) if map_key => {
            *dt = DataType::Reference(specta_typescript::define("string"));
        }
        DataType::Generic(_) => {}
        DataType::List(list) => typescript_alias_datatype(&mut list.ty, map_key, types),
        DataType::Map(map) => {
            if contains_valibot_define(map.key_ty(), types, &[], &mut Vec::new()) {
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
        DataType::Reference(Reference::Named(reference)) => {
            if map_key
                && let Some(mut resolved) =
                    resolved_typescript_map_key_alias(reference, types, &mut HashSet::new())
            {
                typescript_alias_datatype(&mut resolved, true, types);
                *dt = resolved;
                return;
            }

            match &mut reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    typescript_alias_datatype(dt, map_key, types)
                }
                NamedReferenceType::Reference { generics, .. } => generics
                    .iter_mut()
                    .for_each(|(_, dt)| typescript_alias_datatype(dt, map_key, types)),
                NamedReferenceType::Recursive(_) => {}
            }
        }
        DataType::Reference(Reference::Opaque(reference)) => {
            let ty = if reference.downcast_ref::<opaque::Any>().is_some() {
                Some("any")
            } else if reference.downcast_ref::<opaque::Never>().is_some() {
                Some("never")
            } else if reference.downcast_ref::<opaque::Unknown>().is_some() {
                Some("unknown")
            } else if reference.downcast_ref::<opaque::Define>().is_some() {
                // A raw Valibot expression carries no corresponding TypeScript type metadata.
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

fn contains_valibot_define(
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
                    && contains_valibot_define(dt, types, generics, stack)
            }),
        DataType::List(list) => contains_valibot_define(&list.ty, types, generics, stack),
        DataType::Map(map) => {
            contains_valibot_define(map.key_ty(), types, generics, stack)
                || contains_valibot_define(map.value_ty(), types, generics, stack)
        }
        DataType::Nullable(inner) => contains_valibot_define(inner, types, generics, stack),
        DataType::Struct(strct) => {
            fields_contain_valibot_define(&strct.fields, types, generics, stack)
        }
        DataType::Enum(enm) => enm.variants.iter().any(|(_, variant)| {
            fields_contain_valibot_define(&variant.fields, types, generics, stack)
        }),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|dt| contains_valibot_define(dt, types, generics, stack)),
        DataType::Intersection(types_) => types_
            .iter()
            .any(|dt| contains_valibot_define(dt, types, generics, stack)),
        DataType::Reference(Reference::Opaque(reference)) => {
            reference.downcast_ref::<opaque::Define>().is_some()
        }
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                contains_valibot_define(dt, types, generics, stack)
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
                let contains = contains_valibot_define(ty, types, &reference_generics, stack);
                stack.pop();
                contains
            }
            NamedReferenceType::Recursive(_) => false,
        },
    }
}

fn map_key_generic_name(generic: &str) -> String {
    format!("$key${generic}")
}

fn generic_map_key_parameters(dt: &DataType, types: &Types) -> HashSet<String> {
    fn collect_fields(
        fields: &Fields,
        types: &Types,
        map_key: bool,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
        result: &mut HashSet<String>,
    ) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => fields
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .for_each(|dt| collect(dt, types, map_key, generics, stack, result)),
            Fields::Named(fields) => fields
                .fields
                .iter()
                .filter_map(|(_, field)| field.ty.as_ref())
                .for_each(|dt| collect(dt, types, map_key, generics, stack, result)),
        }
    }

    fn collect(
        dt: &DataType,
        types: &Types,
        map_key: bool,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
        result: &mut HashSet<String>,
    ) {
        match dt {
            DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
            DataType::Generic(generic) => {
                if let Some((_, replacement)) =
                    generics.iter().find(|(candidate, _)| candidate == generic)
                    && !matches!(replacement, DataType::Generic(candidate) if candidate == generic)
                {
                    collect(replacement, types, map_key, generics, stack, result);
                } else if map_key {
                    result.insert(generic.name().to_string());
                }
            }
            DataType::List(list) => collect(&list.ty, types, map_key, generics, stack, result),
            DataType::Map(map) => {
                collect(map.key_ty(), types, true, generics, stack, result);
                collect(map.value_ty(), types, false, generics, stack, result);
            }
            DataType::Nullable(inner) => collect(inner, types, map_key, generics, stack, result),
            DataType::Struct(strct) => {
                collect_fields(&strct.fields, types, map_key, generics, stack, result)
            }
            DataType::Enum(enm) => enm.variants.iter().for_each(|(_, variant)| {
                collect_fields(&variant.fields, types, map_key, generics, stack, result)
            }),
            DataType::Tuple(tuple) => tuple
                .elements
                .iter()
                .for_each(|dt| collect(dt, types, map_key, generics, stack, result)),
            DataType::Intersection(types_) => types_
                .iter()
                .for_each(|dt| collect(dt, types, map_key, generics, stack, result)),
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    collect(dt, types, map_key, generics, stack, result)
                }
                NamedReferenceType::Reference { .. } => {
                    if stack.contains(reference) {
                        return;
                    }
                    let Some(ty) = types.get(reference).and_then(|ndt| ndt.ty.as_ref()) else {
                        return;
                    };
                    let Ok(reference_generics) = resolved_reference_generics(reference, generics)
                    else {
                        return;
                    };
                    stack.push(reference.clone());
                    collect(ty, types, map_key, &reference_generics, stack, result);
                    stack.pop();
                }
                NamedReferenceType::Recursive(_) => {}
            },
        }
    }

    let mut result = HashSet::new();
    collect(dt, types, false, &[], &mut Vec::new(), &mut result);
    result
}

fn named_map_key_parameters(ndt: &NamedDataType, types: &Types) -> HashSet<String> {
    fn collect_generic_references(dt: &DataType, result: &mut HashSet<String>) {
        match dt {
            DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
            DataType::Generic(generic) => {
                result.insert(generic.name().to_string());
            }
            DataType::List(list) => collect_generic_references(&list.ty, result),
            DataType::Map(map) => {
                collect_generic_references(map.key_ty(), result);
                collect_generic_references(map.value_ty(), result);
            }
            DataType::Nullable(inner) => collect_generic_references(inner, result),
            DataType::Struct(strct) => collect_field_generic_references(&strct.fields, result),
            DataType::Enum(enm) => enm
                .variants
                .iter()
                .for_each(|(_, variant)| collect_field_generic_references(&variant.fields, result)),
            DataType::Tuple(tuple) => tuple
                .elements
                .iter()
                .for_each(|dt| collect_generic_references(dt, result)),
            DataType::Intersection(types) => types
                .iter()
                .for_each(|dt| collect_generic_references(dt, result)),
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => collect_generic_references(dt, result),
                NamedReferenceType::Reference { generics, .. } => generics
                    .iter()
                    .for_each(|(_, dt)| collect_generic_references(dt, result)),
                NamedReferenceType::Recursive(_) => {}
            },
        }
    }

    fn collect_field_generic_references(fields: &Fields, result: &mut HashSet<String>) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => fields
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .for_each(|dt| collect_generic_references(dt, result)),
            Fields::Named(fields) => fields
                .fields
                .iter()
                .filter_map(|(_, field)| field.ty.as_ref())
                .for_each(|dt| collect_generic_references(dt, result)),
        }
    }

    let mut result = ndt
        .ty
        .as_ref()
        .map(|ty| generic_map_key_parameters(ty, types))
        .unwrap_or_default();
    loop {
        let previous_len = result.len();
        for generic in ndt.generics.iter() {
            if result.contains(generic.name.as_ref())
                && let Some(default) = &generic.default
            {
                collect_generic_references(default, &mut result);
            }
        }
        if result.len() == previous_len {
            break;
        }
    }
    result
}

fn fields_contain_valibot_define(
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
                .is_some_and(|dt| contains_valibot_define(dt, types, generics, stack))
        }),
        Fields::Named(fields) => fields.fields.iter().any(|(_, field)| {
            field
                .ty
                .as_ref()
                .is_some_and(|dt| contains_valibot_define(dt, types, generics, stack))
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

/// Generate an inline Valibot expression for a [`DataType`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically. Map both the full [`Types`] graph and any top-level
/// [`DataType`] values before calling this helper.
///
/// The generated expression expects Valibot to be bound to `v` and
/// [`crate::runtime_helpers`] to be emitted once in the containing module.
pub fn inline(
    exporter: &dyn AsRef<Valibot>,
    types: &Types,
    dt: &DataType,
) -> Result<String, Error> {
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

/// Generate a Valibot expression for a [`Reference`].
///
/// If you are using a custom format such as `specta_serde::Format`, this helper does not apply
/// datatype mapping automatically.
///
/// The generated expression expects Valibot to be bound to `v` and
/// [`crate::runtime_helpers`] to be emitted once in the containing module.
pub fn reference(
    exporter: &dyn AsRef<Valibot>,
    types: &Types,
    r: &Reference,
) -> Result<String, Error> {
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
    exporter: &Valibot,
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
    exporter: &Valibot,
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
            write!(s, "v.nullable({inner})")?;
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
            &[],
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
                s.push_str("v.unknown()");
                return Ok(());
            }
            let allowed_object_keys = intersection
                .iter()
                .filter(|ty| match ty {
                    DataType::Enum(_) => false,
                    DataType::Reference(Reference::Named(named))
                        if !matches!(&named.inner, NamedReferenceType::Recursive(_)) =>
                    {
                        !matches!(named_reference_ty(types, named), Ok(DataType::Enum(_)))
                    }
                    _ => true,
                })
                .filter_map(|ty| object_field_keys(ty, types, generics, &mut HashSet::new()))
                .flatten()
                .collect::<HashSet<_>>();
            let allowed_object_keys = allowed_object_keys
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let mut parts = Vec::with_capacity(intersection.len());
            for ty in intersection {
                let mut part = String::new();
                if let DataType::Enum(enm) = ty {
                    enum_dt(
                        &mut part,
                        exporter,
                        types,
                        enm,
                        location.clone(),
                        generics,
                        type_render_stack,
                        &allowed_object_keys,
                    )?;
                } else if let DataType::Reference(Reference::Named(named)) = ty
                    && !matches!(&named.inner, NamedReferenceType::Recursive(_))
                    && let DataType::Enum(enm) = named_reference_ty(types, named)?
                {
                    let reference_generics = named_reference_generics(named)?;
                    let mut resolved = DataType::Enum(enm.clone());
                    substitute_generics(&mut resolved, reference_generics);
                    let DataType::Enum(resolved) = resolved else {
                        unreachable!("enum generic substitution preserves the datatype kind")
                    };
                    enum_dt(
                        &mut part,
                        exporter,
                        types,
                        &resolved,
                        location.clone(),
                        generics,
                        type_render_stack,
                        &allowed_object_keys,
                    )?;
                } else {
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
                }
                parts.push(part);
            }
            write!(s, "$spectaIntersect([{}])", parts.join(", "))?;
        }
    }

    Ok(())
}

fn primitive_dt(p: &Primitive, location: Vec<Cow<'static, str>>) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match p {
        i8 => "v.pipe(v.number(), v.integer(), v.minValue(-128), v.maxValue(127))",
        i16 => "v.pipe(v.number(), v.integer(), v.minValue(-32768), v.maxValue(32767))",
        i32 => "v.pipe(v.number(), v.integer(), v.minValue(-2147483648), v.maxValue(2147483647))",
        u8 => "v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255))",
        u16 => "v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(65535))",
        u32 => "v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(4294967295))",
        // JSON serializers encode non-finite floats as `null`.
        f16 => "v.nullable(v.pipe(v.number(), v.finite(), v.minValue(-65504), v.maxValue(65504)))",
        f32 => {
            "v.nullable(v.pipe(v.number(), v.finite(), v.minValue(-3.4028235e38), v.maxValue(3.4028235e38)))"
        }
        f64 => "v.nullable(v.pipe(v.number(), v.finite()))",
        usize | isize | i64 | u64 | i128 | u128 | f128 => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        Primitive::bool => "v.boolean()",
        str => "v.pipe(v.string(), v.check((value) => !/[\\uD800-\\uDFFF]/u.test(value)))",
        char => {
            "v.pipe(v.string(), v.check((value) => [...value].length === 1 && (value.length !== 1 || value.charCodeAt(0) < 0xd800 || value.charCodeAt(0) > 0xdfff)))"
        }
    })
}

fn list_dt(
    s: &mut String,
    exporter: &Valibot,
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
        s.push_str("v.strictTuple([");
        for n in 0..length {
            if n != 0 {
                s.push_str(", ");
            }
            s.push_str(&dt);
        }
        s.push_str("])");
    } else {
        write!(s, "v.array({dt})")?;
    }

    Ok(())
}

fn map_dt(
    s: &mut String,
    exporter: &Valibot,
    types: &Types,
    m: &Map,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    let mut resolved_key = m.key_ty().clone();
    substitute_generics(&mut resolved_key, generics);
    map_keys::validate_map_key(
        &resolved_key,
        types,
        child_location(&location, "<key>").join("."),
    )?;

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

    write!(s, "$spectaRecord({key}, {value})")?;
    Ok(())
}

fn map_key_datatype(
    s: &mut String,
    exporter: &Valibot,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(Primitive::str) => s.push_str(
            "v.pipe(v.string(), v.check((value) => !/[\\uD800-\\uDFFF]/u.test(value)))",
        ),
        DataType::Primitive(Primitive::char) => {
            s.push_str(
                "v.pipe(v.string(), v.check((value) => [...value].length === 1 && (value.length !== 1 || value.charCodeAt(0) < 0xd800 || value.charCodeAt(0) > 0xdfff)))",
            )
        }
        DataType::Primitive(Primitive::bool) => s.push_str(r#"v.picklist(["true", "false"])"#),
        DataType::Primitive(Primitive::i8) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?\d+$/), v.check((value) => Number(value) >= -128 && Number(value) <= 127))",
        ),
        DataType::Primitive(Primitive::i16) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?\d+$/), v.check((value) => Number(value) >= -32768 && Number(value) <= 32767))",
        ),
        DataType::Primitive(Primitive::i32) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?\d+$/), v.check((value) => Number(value) >= -2147483648 && Number(value) <= 2147483647))",
        ),
        DataType::Primitive(Primitive::isize | Primitive::usize) => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        DataType::Primitive(Primitive::u8) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^\d+$/), v.check((value) => Number(value) <= 255))",
        ),
        DataType::Primitive(Primitive::u16) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^\d+$/), v.check((value) => Number(value) <= 65535))",
        ),
        DataType::Primitive(Primitive::u32) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^\d+$/), v.check((value) => Number(value) <= 4294967295))",
        ),
        DataType::Primitive(Primitive::f16) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/), v.check((value) => Number.isFinite(Number(value)) && Math.abs(Number(value)) <= 65504))",
        ),
        DataType::Primitive(Primitive::f32) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/), v.check((value) => Number.isFinite(Number(value)) && Math.abs(Number(value)) <= 3.4028235e38))",
        ),
        DataType::Primitive(Primitive::f64) => s.push_str(
            r"v.pipe(v.string(), v.regex(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$/), v.check((value) => Number.isFinite(Number(value))))",
        ),
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
                    Fields::Unit => write!(rendered, "v.literal(\"{}\")", escape_string(name))?,
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
                [] => s.push_str("v.never()"),
                [variant] => s.push_str(variant),
                variants => write!(s, "v.union([{}])", variants.join(", "))?,
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
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
                s.push_str(&map_key_generic_name(generic.name()));
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
    exporter: &Valibot,
    types: &Types,
    t: &Tuple,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match t.elements.as_slice() {
        [] => s.push_str("v.null()"),
        elements => {
            s.push_str("v.strictTuple([");
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
    exporter: &Valibot,
    types: &Types,
    st: &Struct,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => s.push_str("v.null()"),
        Fields::Unnamed(unnamed) => {
            let fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();

            match fields.as_slice() {
                [] => s.push_str("v.strictTuple([])"),
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
                    // trailing run of defaulted (`optional`) elements. Exact
                    // tuple alternatives preserve the input's actual length.
                    let mut optional_from = 0;
                    for (i, (field, _)) in fields.iter().enumerate() {
                        if !field.optional {
                            optional_from = i + 1;
                        }
                    }

                    let mut items = Vec::with_capacity(fields.len());
                    for (i, (_field, ty)) in fields.iter().enumerate() {
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
                        items.push(item);
                    }
                    s.push_str(&strict_tuple_with_trailing_defaults(&items, optional_from));
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
                s.push_str("$spectaObject({})");
                return Ok(());
            }

            let non_flattened = all_fields.iter().collect::<Vec<_>>();

            let mut schema = String::from("$spectaObject({");
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
                    write!(schema, "v.optional({value}),")?;
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

fn object_field_keys(
    dt: &DataType,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    visiting: &mut HashSet<NamedReference>,
) -> Option<HashSet<String>> {
    match dt {
        DataType::Struct(strct) => match &strct.fields {
            Fields::Named(named) => Some(
                named
                    .fields
                    .iter()
                    .filter(|(_, field)| field.ty.is_some())
                    .map(|(name, _)| name.to_string())
                    .collect(),
            ),
            Fields::Unnamed(fields) => {
                let mut live = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                let inner = live.next()?;
                if live.next().is_some() {
                    return None;
                }
                object_field_keys(inner, types, generics, visiting)
            }
            Fields::Unit => Some(HashSet::new()),
        },
        DataType::Intersection(parts) => {
            let mut keys = HashSet::new();
            for part in parts {
                keys.extend(object_field_keys(part, types, generics, visiting)?);
            }
            Some(keys)
        }
        DataType::Enum(enm) => {
            let mut keys = HashSet::new();
            for (_, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                keys.extend(object_field_keys_for_fields(
                    &variant.fields,
                    types,
                    generics,
                    visiting,
                )?);
            }
            Some(keys)
        }
        DataType::Reference(Reference::Named(reference)) => {
            if matches!(reference.inner, NamedReferenceType::Recursive(_))
                || !visiting.insert(reference.clone())
            {
                return None;
            }
            let result = (|| {
                let ty = named_reference_ty(types, reference).ok()?;
                let reference_generics = resolved_reference_generics(reference, generics).ok()?;
                object_field_keys(ty, types, &reference_generics, visiting)
            })();
            visiting.remove(reference);
            result
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .and_then(|(_, ty)| {
                (!matches!(ty, DataType::Generic(candidate) if candidate == generic))
                    .then(|| object_field_keys(ty, types, generics, visiting))
                    .flatten()
            }),
        _ => None,
    }
}

fn object_field_keys_for_fields(
    fields: &Fields,
    types: &Types,
    generics: &[(GenericReference, DataType)],
    visiting: &mut HashSet<NamedReference>,
) -> Option<HashSet<String>> {
    match fields {
        Fields::Named(named) => Some(
            named
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .map(|(name, _)| name.to_string())
                .collect(),
        ),
        Fields::Unnamed(unnamed) => {
            let mut live = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            let inner = live.next()?;
            if live.next().is_some() {
                return None;
            }
            object_field_keys(inner, types, generics, visiting)
        }
        Fields::Unit => Some(HashSet::new()),
    }
}

fn enum_dt(
    s: &mut String,
    exporter: &Valibot,
    types: &Types,
    e: &Enum,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
    allowed_object_keys: &[&str],
) -> Result<(), Error> {
    let optional_flatten_union = e.attributes.contains_key(OPTIONAL_FLATTEN_UNION_MARKER);
    let entries = e
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(name, variant)| {
            let keys = optional_flatten_union.then(|| {
                object_field_keys_for_fields(&variant.fields, types, generics, &mut HashSet::new())
            });
            (name, variant, keys.flatten())
        })
        .collect::<Vec<_>>();
    let all_optional_flatten_keys = entries
        .iter()
        .filter_map(|(_, _, keys)| keys.as_ref())
        .flat_map(|keys| keys.iter().cloned())
        .collect::<HashSet<_>>();

    let variants = entries
        .into_iter()
        .map(|(name, variant, keys)| -> Result<Option<String>, Error> {
            let strict_object = variant.attributes.contains_key(STRICT_OBJECT_MARKER);
            let rendered = enum_variant_dt(
                exporter,
                types,
                name.as_ref(),
                variant,
                strict_object,
                child_location(&location, name.to_string()),
                generics,
                type_render_stack,
                allowed_object_keys,
            )?;
            let Some(mut rendered) = rendered else {
                return Ok(None);
            };

            if let Some(keys) = keys {
                let mut missing = all_optional_flatten_keys
                    .difference(&keys)
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                missing.sort_unstable();
                if !missing.is_empty() {
                    let mut exclusions = String::from("$spectaObject({");
                    for key in missing {
                        write!(
                            exclusions,
                            "\n\t{}: v.optional(v.never()),",
                            sanitise_key(key)
                        )?;
                    }
                    exclusions.push_str("\n})");
                    rendered = format!("$spectaIntersect([{rendered}, {exclusions}])");
                }
            }

            Ok(Some(rendered))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let variants = variants.into_iter().flatten().collect::<Vec<_>>();

    if variants.is_empty() {
        s.push_str("v.never()");
        return Ok(());
    }

    let mut unique_variants = Vec::with_capacity(variants.len());
    for variant in variants {
        if !unique_variants.contains(&variant) {
            unique_variants.push(variant);
        }
    }
    let variants = unique_variants;

    if variants.len() == 1 {
        s.push_str(&variants[0]);
    } else {
        write!(s, "v.union([{}])", variants.join(", "))?;
    }

    Ok(())
}

fn enum_variant_dt(
    exporter: &Valibot,
    types: &Types,
    name: &str,
    variant: &specta::datatype::Variant,
    strict_object: bool,
    location: Vec<Cow<'static, str>>,
    generics: &[(GenericReference, DataType)],
    type_render_stack: &mut TypeRenderStack,
    allowed_object_keys: &[&str],
) -> Result<Option<String>, Error> {
    match &variant.fields {
        Fields::Unit => Ok(Some(format!("v.literal(\"{}\")", escape_string(name)))),
        Fields::Named(named) => {
            let mut schema = String::from("$spectaObject({");
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
                    write!(schema, "\n\t{key}: v.optional({value}),")?;
                } else {
                    write!(schema, "\n\t{key}: {value},")?;
                }
            }

            if strict_object {
                for field_name in allowed_object_keys {
                    if named
                        .fields
                        .iter()
                        .any(|(name, field)| field.ty.is_some() && name.as_ref() == *field_name)
                    {
                        continue;
                    }
                    has_field = true;
                    let key = sanitise_key(field_name);
                    write!(schema, "\n\t{key}: v.optional(v.unknown()),")?;
                }
            }

            if has_field {
                schema.push('\n');
            }
            schema.push_str(if strict_object { "}, true)" } else { "})" });

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
                        Some("v.strictTuple([])".to_string())
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

                    let mut items = Vec::with_capacity(fields.len());
                    for (i, (_field, ty)) in fields.iter().enumerate() {
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
                        items.push(item);
                    }
                    Some(strict_tuple_with_trailing_defaults(&items, optional_from))
                }
            })
        }
    }
}

fn reference_dt(
    s: &mut String,
    exporter: &Valibot,
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
        s.push_str("v.any()");
        return Ok(());
    }
    if r.downcast_ref::<opaque::Unknown>().is_some() {
        s.push_str("v.unknown()");
        return Ok(());
    }
    if r.downcast_ref::<opaque::Never>().is_some() {
        s.push_str("v.never()");
        return Ok(());
    }

    Err(Error::unsupported_opaque_reference(r.clone()))
}

fn reference_named_dt(
    s: &mut String,
    exporter: &Valibot,
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
            for segment in crate::valibot::namespace_module_path(&ndt.module_path)
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
        Layout::FlatFile | Layout::ModulePrefixedName => {
            format!("{}Schema", exported_type_name(exporter, ndt))
        }
        Layout::Files => {
            let current_module_path = crate::references::current_module_path().unwrap_or_default();
            let base = format!("{}Schema", ndt.name);
            if ndt.module_path == current_module_path {
                base
            } else {
                format!(
                    "{}.{}",
                    crate::valibot::module_alias(&ndt.module_path),
                    base
                )
            }
        }
    };

    let mut reference_expr = schema_name;
    let reference_generics = match &r.inner {
        NamedReferenceType::Recursive(recursive) => recursive.generics(),
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

        let map_key_generics = named_map_key_parameters(ndt, types);
        let mut schema_arguments = Vec::new();
        for (generic, v) in reference_generics {
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
            schema_arguments.push(generic_schema);
            if map_key_generics.contains(generic.name().as_ref()) {
                let mut resolved_key = v.clone();
                substitute_generics(&mut resolved_key, &scoped_generics);
                map_keys::validate_map_key(
                    &resolved_key,
                    types,
                    format!("{}.<generic {} key>", ndt.name, generic.name()),
                )?;
                let mut key_schema = String::new();
                map_key_datatype(
                    &mut key_schema,
                    exporter,
                    types,
                    v,
                    vec![],
                    &scoped_generics,
                    type_render_stack,
                )?;
                schema_arguments.push(key_schema);
            }
        }
        reference_expr.push('(');
        reference_expr.push_str(&schema_arguments.join(", "));
        reference_expr.push(')');
    }

    // Declarations are sorted, so this reference may target a schema whose
    // initializer has not run yet. Laziness also handles recursive references.
    write!(s, "v.lazy(() => {reference_expr})")?;

    Ok(())
}

pub(crate) fn exported_type_name(exporter: &Valibot, ndt: &NamedDataType) -> Cow<'static, str> {
    match exporter.layout {
        Layout::Namespaces | Layout::FlatFile | Layout::Files => ndt.name.clone(),
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

    if !(first.is_alphabetic() || first == '_') {
        return Err(Error::invalid_name(path, name.to_string()));
    }
    if chars.any(|ch| !(ch.is_alphanumeric() || ch == '_')) {
        return Err(Error::invalid_name(path, name.to_string()));
    }

    Ok(())
}

fn sanitise_key(field_name: &str) -> String {
    if field_name == "__proto__" {
        // `__proto__: value` is prototype-setter syntax in JavaScript object
        // literals, so it must be emitted as a computed own property.
        "[\"__proto__\"]".to_string()
    } else if is_identifier(field_name) {
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
