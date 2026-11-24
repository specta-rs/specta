//! Building blocks for rendering Effect Schema expressions.

use std::{borrow::Cow, collections::HashSet, fmt::Write as _};

use specta::{
    Types,
    datatype::{
        DataType, Enum, Field, Fields, GenericReference, List, Map, NamedDataType, NamedReference,
        NamedReferenceType, OpaqueReference, Primitive, Reference, Struct, Tuple,
    },
};

use crate::{Effect, Error, Layout, map_keys, opaque, reserved_names::RESERVED_TYPE_NAMES};

type Generics = [(GenericReference, DataType)];
type RenderStack = Vec<(Cow<'static, str>, Cow<'static, str>)>;

/// Render named types as Effect Schema declarations.
pub fn export<'a>(
    exporter: &dyn AsRef<Effect>,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
) -> Result<String, Error> {
    let mut out = String::new();
    export_internal(
        &mut out,
        exporter.as_ref(),
        types,
        ndts,
        indent,
        &mut Vec::new(),
    )?;
    Ok(out)
}

pub(crate) fn export_internal<'a>(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    ndts: impl Iterator<Item = &'a NamedDataType>,
    indent: &str,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    for (index, ndt) in ndts.enumerate() {
        if index != 0 {
            out.push('\n');
        }
        export_single(out, exporter, types, ndt, indent, stack)?;
    }
    Ok(())
}

/// Render a datatype as an Effect Schema expression.
///
/// Ordinary named references are preserved and suspended so forward and
/// recursive references are safe at runtime.
pub fn inline(exporter: &dyn AsRef<Effect>, types: &Types, dt: &DataType) -> Result<String, Error> {
    let mut out = String::new();
    datatype(
        &mut out,
        exporter.as_ref(),
        types,
        dt,
        Vec::new(),
        &[],
        false,
        &mut Vec::new(),
    )?;
    Ok(out)
}

/// Render a reference as an Effect Schema expression.
pub fn reference(
    exporter: &dyn AsRef<Effect>,
    types: &Types,
    reference: &Reference,
) -> Result<String, Error> {
    let mut out = String::new();
    reference_dt(
        &mut out,
        exporter.as_ref(),
        types,
        reference,
        Vec::new(),
        &[],
        &mut Vec::new(),
    )?;
    Ok(out)
}

fn export_single(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    ndt: &NamedDataType,
    indent: &str,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    let Some(ty) = ndt.ty.as_ref() else {
        return Ok(());
    };

    let base_name = exported_type_name(exporter, ndt);
    validate_type_name(&base_name, rust_type_path(ndt))?;
    let schema_name = format!("{base_name}Schema");
    let key_generics = key_generics_for(ndt, types);
    stack.push((ndt.module_path.clone(), ndt.name.clone()));
    let result = (|| {
        let mut body = String::new();
        datatype(
            &mut body,
            exporter,
            types,
            ty,
            vec![ndt.name.clone()],
            &[],
            false,
            stack,
        )?;

        if ndt.generics.is_empty() {
            writeln!(
                out,
                "{indent}export const {schema_name} = {};",
                indent_continuations(&body, indent)
            )?;
            return Ok(());
        }

        let mut type_parameters = Vec::new();
        let mut value_parameters = Vec::new();
        let mut scope = Vec::new();

        for generic in ndt.generics.iter() {
            let name = generic.name.as_ref();
            validate_type_name(name, format!("{}<{name}>", rust_type_path(ndt)))?;

            let mut declaration = format!("{name} extends Schema.Constraint");
            let mut parameter = format!("{name}: {name}");
            if let Some(default) = &generic.default {
                let default_type = schema_type(default, exporter, types, &scope)?;
                declaration.push_str(" = ");
                declaration.push_str(&default_type);

                let mut default_schema = String::new();
                datatype(
                    &mut default_schema,
                    exporter,
                    types,
                    default,
                    vec![ndt.name.clone(), format!("<generic {name} default>").into()],
                    &scope,
                    false,
                    stack,
                )?;
                parameter.push_str(" = ");
                parameter.push_str(&default_schema);
                parameter.push_str(" as unknown as ");
                parameter.push_str(name);
            }
            type_parameters.push(declaration);
            value_parameters.push(parameter);

            if key_generics.contains(name) {
                let key_type = map_key_generic_name(name);
                let key_value = map_key_generic_value(name);
                let mut key_declaration = format!("{key_type} extends Schema.Record.Key");
                let mut key_parameter = format!("{key_value}: {key_type}");
                if let Some(default) = &generic.default {
                    key_declaration.push_str(" = ");
                    key_declaration.push_str(&map_key_schema_type(default, types, &scope)?);
                    let mut default_key = String::new();
                    map_key_datatype(
                        &mut default_key,
                        types,
                        default,
                        vec![
                            ndt.name.clone(),
                            format!("<generic {name} default key>").into(),
                        ],
                        &scope,
                    )?;
                    key_parameter.push_str(" = ");
                    key_parameter.push_str(&default_key);
                    key_parameter.push_str(" as unknown as ");
                    key_parameter.push_str(&key_type);
                }
                type_parameters.push(key_declaration);
                value_parameters.push(key_parameter);
            }

            scope.push((generic.reference(), DataType::Generic(generic.reference())));
        }

        writeln!(
            out,
            "{indent}export const {schema_name} = <{}>({}) => {};",
            type_parameters.join(", "),
            value_parameters.join(", "),
            indent_continuations(&body, indent)
        )?;
        Ok(())
    })();
    stack.pop();
    result
}

fn indent_continuations<'a>(value: &'a str, indent: &str) -> Cow<'a, str> {
    if indent.is_empty() || !value.contains('\n') {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.replace('\n', &format!("\n{indent}")))
    }
}

fn datatype(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    force_inline_reference: bool,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(primitive) => out.push_str(primitive_dt(primitive, location)?),
        DataType::Generic(generic) => generic_dt(out, generic),
        DataType::List(list) => list_dt(out, exporter, types, list, location, generics, stack)?,
        DataType::Map(map) => map_dt(out, exporter, types, map, location, generics, stack, &[])?,
        DataType::Nullable(inner) => {
            let mut value = String::new();
            datatype(
                &mut value,
                exporter,
                types,
                inner,
                location,
                generics,
                force_inline_reference,
                stack,
            )?;
            write!(out, "Schema.NullOr({value})")?;
        }
        DataType::Struct(strct) => {
            struct_dt(out, exporter, types, strct, location, generics, stack)?
        }
        DataType::Enum(enm) => enum_dt(out, exporter, types, enm, location, generics, stack)?,
        DataType::Tuple(tuple) => tuple_dt(out, exporter, types, tuple, location, generics, stack)?,
        DataType::Intersection(parts) => {
            intersection_dt(out, exporter, types, parts, location, generics, stack)?
        }
        DataType::Reference(reference) if force_inline_reference => match reference {
            Reference::Named(named) => {
                let ty = named_reference_ty(types, named, &location)?;
                let reference_generics = resolved_reference_generics(named, generics)?;
                datatype(
                    out,
                    exporter,
                    types,
                    ty,
                    location,
                    &reference_generics,
                    false,
                    stack,
                )?;
            }
            Reference::Opaque(_) => {
                reference_dt(out, exporter, types, reference, location, generics, stack)?
            }
        },
        DataType::Reference(reference) => {
            reference_dt(out, exporter, types, reference, location, generics, stack)?
        }
    }
    Ok(())
}

fn primitive_dt(
    primitive: &Primitive,
    location: Vec<Cow<'static, str>>,
) -> Result<&'static str, Error> {
    use Primitive::*;

    Ok(match primitive {
        i8 | i16 | i32 | u8 | u16 | u32 => "Schema.Int",
        f16 | f32 | f64 => "Schema.NullOr(Schema.Finite)",
        usize | isize | i64 | u64 | i128 | u128 | f128 => {
            return Err(Error::bigint_forbidden(location.join(".")));
        }
        Primitive::bool => "Schema.Boolean",
        str => {
            r#"Schema.String.pipe(Schema.check(Schema.makeFilter((value: string) => !/[\uD800-\uDFFF]/u.test(value))))"#
        }
        char => {
            r#"Schema.String.pipe(Schema.check(Schema.makeFilter((value: string) => [...value].length === 1 && !/[\uD800-\uDFFF]/u.test(value))))"#
        }
    })
}

fn generic_dt(out: &mut String, generic: &GenericReference) {
    out.push_str(generic.name());
}

fn list_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    list: &List,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    let mut element = String::new();
    datatype(
        &mut element,
        exporter,
        types,
        &list.ty,
        child_location(&location, "<item>"),
        generics,
        false,
        stack,
    )?;

    if let Some(length) = list.length {
        let elements = std::iter::repeat_n(element, length).collect::<Vec<_>>();
        write!(out, "Schema.Tuple([{}])", elements.join(", "))?;
    } else {
        write!(out, "Schema.Array({element})")?;
    }
    Ok(())
}

fn map_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    map: &Map,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
    excluded_keys: &[String],
) -> Result<(), Error> {
    let mut resolved_key = map.key_ty().clone();
    substitute_generics(&mut resolved_key, generics);
    map_keys::validate_map_key(
        &resolved_key,
        types,
        child_location(&location, "<key>").join("."),
    )?;

    let mut key = String::new();
    map_key_datatype(
        &mut key,
        types,
        map.key_ty(),
        child_location(&location, "<key>"),
        generics,
    )?;
    if !excluded_keys.is_empty() {
        key = exclude_record_keys(&key, excluded_keys);
    }

    let mut value = String::new();
    datatype(
        &mut value,
        exporter,
        types,
        map.value_ty(),
        child_location(&location, "<value>"),
        generics,
        false,
        stack,
    )?;
    write!(out, "Schema.Record({key}, {value})")?;
    Ok(())
}

fn exclude_record_keys(key_schema: &str, excluded_keys: &[String]) -> String {
    if excluded_keys.is_empty() || !key_schema.starts_with("Schema.String") {
        return key_schema.to_string();
    }

    let keys = excluded_keys
        .iter()
        .map(|key| format!("\"{}\"", escape_string_literal(key)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{key_schema}.pipe(Schema.check(Schema.makeFilter((key: PropertyKey) => ![{keys}].includes(String(key)) ? undefined : \"key is reserved\")))"
    )
}

fn map_key_datatype(
    out: &mut String,
    types: &Types,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
) -> Result<(), Error> {
    match dt {
        DataType::Primitive(primitive) => match primitive {
            Primitive::str | Primitive::char => out.push_str("Schema.String"),
            Primitive::bool => {
                out.push_str(&filtered_string_literals(&[
                    "true".to_string(),
                    "false".to_string(),
                ]));
            }
            Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32 => out.push_str("Schema.Int"),
            Primitive::f16 | Primitive::f32 | Primitive::f64 => out.push_str("Schema.Finite"),
            Primitive::usize
            | Primitive::isize
            | Primitive::i64
            | Primitive::u64
            | Primitive::i128
            | Primitive::u128
            | Primitive::f128 => return Err(Error::bigint_forbidden(location.join("."))),
        },
        DataType::Struct(strct) => {
            let Fields::Unnamed(fields) = &strct.fields else {
                return Err(Error::invalid_map_key(
                    location.join("."),
                    "struct keys must serialize as a newtype struct",
                ));
            };
            let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
            let Some(inner) = fields.next() else {
                return Err(Error::invalid_map_key(
                    location.join("."),
                    "newtype struct map keys must have exactly one field",
                ));
            };
            if fields.next().is_some() {
                return Err(Error::invalid_map_key(
                    location.join("."),
                    "newtype struct map keys must have exactly one field",
                ));
            }
            map_key_datatype(out, types, inner, location, generics)?;
        }
        DataType::Enum(enm) => {
            let untagged = enm.attributes.contains_key("serde:container:untagged");
            let mut literals = Vec::new();
            let mut all_unit = true;
            for (name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                match &variant.fields {
                    Fields::Unit
                        if !untagged
                            && !variant.attributes.contains_key("serde:variant:untagged") =>
                    {
                        literals.push(name.to_string());
                    }
                    Fields::Unit => {
                        return Err(Error::invalid_map_key(
                            location.join("."),
                            format!(
                                "untagged enum key variant '{name}' does not serialize as a string"
                            ),
                        ));
                    }
                    Fields::Unnamed(fields) => {
                        all_unit = false;
                        let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                        let Some(inner) = fields.next() else {
                            continue;
                        };
                        if fields.next().is_none()
                            && let Some(values) = string_literal_values(inner, types, generics)
                        {
                            literals.extend(values);
                        }
                    }
                    Fields::Named(_) => {
                        return Err(Error::invalid_map_key(
                            location.join("."),
                            "struct enum keys cannot be serialized as serde_json map keys",
                        ));
                    }
                }
            }
            if all_unit || !literals.is_empty() {
                out.push_str(&filtered_string_literals(&literals));
            } else {
                out.push_str("Schema.String");
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            let ty = named_reference_ty(types, reference, &location)?;
            let reference_generics = rendered_reference_generics(reference, generics, types)?;
            map_key_datatype(out, types, ty, location, &reference_generics)?;
        }
        DataType::Generic(generic) => {
            if let Some((_, replacement)) = generics
                .iter()
                .find(|(candidate, _)| candidate == generic)
                .filter(
                    |(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic),
                )
            {
                map_key_datatype(out, types, replacement, location, generics)?;
            } else {
                out.push_str(&map_key_generic_value(generic.name()));
            }
        }
        DataType::Reference(Reference::Opaque(reference))
            if reference.downcast_ref::<opaque::Define>().is_some() =>
        {
            let raw = reference
                .downcast_ref::<opaque::Define>()
                .map(|define| define.0.as_ref())
                .unwrap_or_default();
            write!(out, "({raw}) as Schema.Record.Key")?;
        }
        DataType::Tuple(_)
        | DataType::List(_)
        | DataType::Map(_)
        | DataType::Nullable(_)
        | DataType::Intersection(_)
        | DataType::Reference(Reference::Opaque(_)) => {
            return Err(Error::invalid_map_key(
                location.join("."),
                "collection, map, nullable, intersection, and opaque keys are not supported",
            ));
        }
    }
    Ok(())
}

fn filtered_string_literals(literals: &[String]) -> String {
    let values = literals
        .iter()
        .map(|value| format!("\"{}\"", escape_string_literal(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Schema.String.pipe(Schema.check(Schema.makeFilter((key: string) => [{values}].includes(key) ? undefined : \"invalid map key\")))"
    )
}

fn string_literal_values(dt: &DataType, types: &Types, generics: &Generics) -> Option<Vec<String>> {
    match dt {
        DataType::Enum(enm) => {
            let untagged = enm.attributes.contains_key("serde:container:untagged");
            enm.variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .map(|(name, variant)| {
                    if matches!(variant.fields, Fields::Unit)
                        && !untagged
                        && !variant.attributes.contains_key("serde:variant:untagged")
                    {
                        Some(name.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        }
        DataType::Struct(strct) => {
            let Fields::Unnamed(fields) = &strct.fields else {
                return None;
            };
            let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
            let inner = fields.next()?;
            fields
                .next()
                .is_none()
                .then(|| string_literal_values(inner, types, generics))
                .flatten()
        }
        DataType::Reference(Reference::Named(reference)) => {
            let ty = named_reference_ty(types, reference, &[]).ok()?;
            let reference_generics =
                rendered_reference_generics(reference, generics, types).ok()?;
            string_literal_values(ty, types, &reference_generics)
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .filter(|(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic))
            .and_then(|(_, ty)| string_literal_values(ty, types, generics)),
        _ => None,
    }
}

fn tuple_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    tuple: &Tuple,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    if tuple.elements.is_empty() {
        out.push_str("Schema.Null");
        return Ok(());
    }

    let mut elements = Vec::with_capacity(tuple.elements.len());
    for (index, element) in tuple.elements.iter().enumerate() {
        let mut rendered = String::new();
        datatype(
            &mut rendered,
            exporter,
            types,
            element,
            child_location(&location, index.to_string()),
            generics,
            false,
            stack,
        )?;
        elements.push(rendered);
    }
    write!(out, "Schema.Tuple([{}])", elements.join(", "))?;
    Ok(())
}

fn struct_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    strct: &Struct,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    match &strct.fields {
        Fields::Unit => out.push_str("Schema.Null"),
        Fields::Unnamed(unnamed) => {
            let declared_len = unnamed.fields.len();
            let fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();
            if declared_len == 1 && fields.len() == 1 {
                return datatype(
                    out,
                    exporter,
                    types,
                    fields[0].1,
                    location,
                    generics,
                    false,
                    stack,
                );
            }

            let required_len = fields
                .iter()
                .rposition(|(field, _)| !field.optional)
                .map_or(0, |index| index + 1);
            let mut items = Vec::with_capacity(fields.len());
            for (index, (_, ty)) in fields.iter().enumerate() {
                let mut rendered = String::new();
                datatype(
                    &mut rendered,
                    exporter,
                    types,
                    ty,
                    child_location(&location, index.to_string()),
                    generics,
                    false,
                    stack,
                )?;
                items.push(rendered);
            }
            out.push_str(&tuple_with_optional_suffix(&items, required_len));
        }
        Fields::Named(named) => {
            render_named_fields(
                out,
                exporter,
                types,
                &named.fields,
                location,
                generics,
                stack,
            )?;
        }
    }
    Ok(())
}

fn tuple_with_optional_suffix(items: &[String], required_len: usize) -> String {
    let variants = (required_len..=items.len())
        .map(|length| format!("Schema.Tuple([{}])", items[..length].join(", ")))
        .collect::<Vec<_>>();
    match variants.as_slice() {
        [variant] => variant.clone(),
        [] => "Schema.Null".to_string(),
        variants => format!("Schema.Union([{}])", variants.join(", ")),
    }
}

fn render_named_fields(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    fields: &[(Cow<'static, str>, Field)],
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    let mut rendered = Vec::new();
    for (name, field) in fields {
        let Some(ty) = field.ty.as_ref() else {
            continue;
        };
        let mut value = String::new();
        datatype(
            &mut value,
            exporter,
            types,
            ty,
            child_location(&location, name.to_string()),
            generics,
            false,
            stack,
        )?;
        if field.optional {
            value = format!("Schema.optionalKey({value})");
        }
        rendered.push(format!("{}: {value}", sanitise_key(name)));
    }

    if rendered.is_empty() {
        out.push_str("Schema.Struct({})");
    } else {
        write!(out, "Schema.Struct({{\n\t{}\n}})", rendered.join(",\n\t"))?;
    }
    Ok(())
}

fn enum_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    enm: &Enum,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    if enm.attributes.get_named_as("specta:finite_number") == Some(&true) {
        for (name, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
            enum_variant_dt(
                exporter,
                types,
                name,
                variant,
                child_location(&location, name.to_string()),
                generics,
                stack,
            )?;
        }
        out.push_str("Schema.Finite");
        return Ok(());
    }

    let entries = enm
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        out.push_str("Schema.Never");
        return Ok(());
    }

    let optional_flatten = enm
        .attributes
        .contains_key("specta_serde:optional_flatten_union");
    let entries = entries
        .into_iter()
        .map(|entry| {
            let fields = optional_flatten.then(|| {
                object_fields_for_fields(&entry.1.fields, types, generics, &mut HashSet::new())
            });
            let open = optional_flatten
                .then(|| fields_contain_map(&entry.1.fields, types, generics, &mut HashSet::new()));
            (entry, fields.flatten(), open.unwrap_or(false))
        })
        .collect::<Vec<_>>();
    let all_keys = entries
        .iter()
        .filter_map(|(_, fields, _)| fields.as_ref())
        .flatten()
        .map(|(name, _)| name.to_string())
        .collect::<HashSet<_>>();
    let has_open_branch = entries.iter().any(|(_, _, open)| *open);

    let mut variants = Vec::with_capacity(entries.len());
    for ((name, variant), fields, open) in entries {
        let variant_location = child_location(&location, name.to_string());
        let rendered = if let Some(mut fields) = fields {
            let present = fields
                .iter()
                .map(|(name, _)| name.to_string())
                .collect::<HashSet<_>>();
            let mut missing = all_keys.difference(&present).cloned().collect::<Vec<_>>();
            missing.sort();
            for key in missing {
                let mut field = Field::new(DataType::Reference(Reference::opaque(opaque::Never)));
                field.optional = true;
                fields.push((key.into(), field));
            }
            let mut rendered = String::new();
            if has_open_branch && !open {
                render_closed_object(
                    &mut rendered,
                    exporter,
                    types,
                    &fields,
                    variant_location,
                    generics,
                    stack,
                )?;
            } else {
                render_named_fields(
                    &mut rendered,
                    exporter,
                    types,
                    &fields,
                    variant_location,
                    generics,
                    stack,
                )?;
            }
            rendered
        } else {
            enum_variant_dt(
                exporter,
                types,
                name,
                variant,
                variant_location,
                generics,
                stack,
            )?
        };
        variants.push(rendered);
    }

    variants.dedup();
    match variants.as_slice() {
        [] => out.push_str("Schema.Never"),
        [variant] => out.push_str(variant),
        variants => write!(out, "Schema.Union([{}])", variants.join(", "))?,
    }
    Ok(())
}

fn enum_variant_dt(
    exporter: &Effect,
    types: &Types,
    name: &str,
    variant: &specta::datatype::Variant,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<String, Error> {
    match &variant.fields {
        Fields::Unit => Ok(format!(
            "Schema.Literal(\"{}\")",
            escape_string_literal(name)
        )),
        Fields::Named(named) => {
            let mut out = String::new();
            render_named_fields(
                &mut out,
                exporter,
                types,
                &named.fields,
                location,
                generics,
                stack,
            )?;
            Ok(out)
        }
        Fields::Unnamed(unnamed) => {
            let declared_len = unnamed.fields.len();
            let fields = unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>();
            if declared_len == 1 && fields.len() == 1 {
                let mut out = String::new();
                datatype(
                    &mut out,
                    exporter,
                    types,
                    fields[0].1,
                    location,
                    generics,
                    false,
                    stack,
                )?;
                return Ok(out);
            }

            let required_len = fields
                .iter()
                .rposition(|(field, _)| !field.optional)
                .map_or(0, |index| index + 1);
            let mut items = Vec::with_capacity(fields.len());
            for (index, (_, ty)) in fields.iter().enumerate() {
                let mut rendered = String::new();
                datatype(
                    &mut rendered,
                    exporter,
                    types,
                    ty,
                    child_location(&location, index.to_string()),
                    generics,
                    false,
                    stack,
                )?;
                items.push(rendered);
            }
            Ok(tuple_with_optional_suffix(&items, required_len))
        }
    }
}

fn object_fields_for_fields(
    fields: &Fields,
    types: &Types,
    generics: &Generics,
    visiting: &mut HashSet<Reference>,
) -> Option<Vec<(Cow<'static, str>, Field)>> {
    match fields {
        Fields::Unit => Some(Vec::new()),
        Fields::Named(named) => Some(
            named
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .cloned()
                .collect(),
        ),
        Fields::Unnamed(unnamed) => {
            let mut fields = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            let inner = fields.next()?;
            fields
                .next()
                .is_none()
                .then(|| object_fields(inner, types, generics, visiting))
                .flatten()
        }
    }
}

fn object_fields(
    dt: &DataType,
    types: &Types,
    generics: &Generics,
    visiting: &mut HashSet<Reference>,
) -> Option<Vec<(Cow<'static, str>, Field)>> {
    match dt {
        DataType::Struct(strct) => {
            object_fields_for_fields(&strct.fields, types, generics, visiting)
        }
        DataType::Intersection(parts) => {
            let mut fields = Vec::new();
            for part in parts {
                fields.extend(object_fields(part, types, generics, visiting)?);
            }
            Some(fields)
        }
        DataType::Reference(Reference::Named(reference)) => {
            let key = Reference::Named(reference.clone());
            if !visiting.insert(key.clone()) {
                return None;
            }
            let result = (|| {
                let ty = named_reference_ty(types, reference, &[]).ok()?;
                let reference_generics =
                    rendered_reference_generics(reference, generics, types).ok()?;
                object_fields(ty, types, &reference_generics, visiting)
            })();
            visiting.remove(&key);
            result
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .filter(|(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic))
            .and_then(|(_, ty)| object_fields(ty, types, generics, visiting)),
        _ => None,
    }
}

fn fields_contain_map(
    fields: &Fields,
    types: &Types,
    generics: &Generics,
    visiting: &mut HashSet<Reference>,
) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .any(|ty| datatype_contains_map(ty, types, generics, visiting)),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(_, field)| field.ty.as_ref())
            .any(|ty| datatype_contains_map(ty, types, generics, visiting)),
    }
}

fn datatype_contains_map(
    dt: &DataType,
    types: &Types,
    generics: &Generics,
    visiting: &mut HashSet<Reference>,
) -> bool {
    match dt {
        DataType::Map(_) => true,
        DataType::List(list) => datatype_contains_map(&list.ty, types, generics, visiting),
        DataType::Nullable(inner) => datatype_contains_map(inner, types, generics, visiting),
        DataType::Struct(strct) => fields_contain_map(&strct.fields, types, generics, visiting),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| fields_contain_map(&variant.fields, types, generics, visiting)),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|element| datatype_contains_map(element, types, generics, visiting)),
        DataType::Intersection(parts) => parts
            .iter()
            .any(|part| datatype_contains_map(part, types, generics, visiting)),
        DataType::Reference(Reference::Named(reference)) => {
            let generics_at_use = match &reference.inner {
                NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
                NamedReferenceType::Recursive(recursive) => recursive.generics(),
                NamedReferenceType::Inline { dt, .. } => {
                    return datatype_contains_map(dt, types, generics, visiting);
                }
            };
            if generics_at_use
                .iter()
                .any(|(_, ty)| datatype_contains_map(ty, types, generics, visiting))
            {
                return true;
            }
            let key = Reference::Named(reference.clone());
            if !visiting.insert(key.clone()) {
                return false;
            }
            let result = (|| {
                let ty = named_reference_ty(types, reference, &[]).ok()?;
                let reference_generics =
                    rendered_reference_generics(reference, generics, types).ok()?;
                Some(datatype_contains_map(
                    ty,
                    types,
                    &reference_generics,
                    visiting,
                ))
            })();
            visiting.remove(&key);
            result.unwrap_or(false)
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .filter(|(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic))
            .is_some_and(|(_, ty)| datatype_contains_map(ty, types, generics, visiting)),
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => false,
    }
}

fn render_closed_object(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    fields: &[(Cow<'static, str>, Field)],
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    let mut base = String::new();
    render_named_fields(
        &mut base,
        exporter,
        types,
        fields,
        location.clone(),
        generics,
        stack,
    )?;
    let fixed_keys = fields
        .iter()
        .filter(|(_, field)| field.ty.is_some())
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    let key = exclude_record_keys("Schema.String", &fixed_keys);
    write!(
        out,
        "Schema.StructWithRest({base}, [Schema.Record({key}, Schema.Never)])"
    )?;
    Ok(())
}

fn intersection_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    parts: &[DataType],
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    if parts.is_empty() {
        out.push_str("Schema.Unknown");
        return Ok(());
    }

    if parts.len() == 1 {
        return datatype(
            out, exporter, types, &parts[0], location, generics, false, stack,
        );
    }

    let mut fields = Vec::new();
    let mut records = Vec::new();
    let mut aliases = Vec::new();
    for part in parts {
        collect_object_part(
            part,
            types,
            generics,
            &mut HashSet::new(),
            &mut fields,
            &mut records,
            &mut aliases,
            &location,
        )?;
    }

    if aliases.is_empty() {
        return render_intersection_object(
            out, exporter, types, &fields, &records, location, generics, stack,
        );
    }

    let mut combinations = vec![fields];
    for branches in aliases {
        let mut next = Vec::new();
        for combination in &combinations {
            for branch in &branches {
                let mut merged = combination.clone();
                merged.extend(branch.iter().cloned());
                next.push(merged);
                if next.len() > 256 {
                    return Err(Error::alias_combination_limit(location.join("."), 256));
                }
            }
        }
        combinations = next;
    }

    let mut rendered = Vec::with_capacity(combinations.len());
    for fields in combinations {
        let mut schema = String::new();
        render_intersection_object(
            &mut schema,
            exporter,
            types,
            &fields,
            &records,
            location.clone(),
            generics,
            stack,
        )?;
        if !rendered.contains(&schema) {
            rendered.push(schema);
        }
    }
    match rendered.as_slice() {
        [] => out.push_str("Schema.Never"),
        [schema] => out.push_str(schema),
        schemas => write!(out, "Schema.Union([{}])", schemas.join(", "))?,
    }
    Ok(())
}

fn render_intersection_object(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    fields: &[(Cow<'static, str>, Field)],
    records: &[DataType],
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    if records.is_empty() {
        return render_named_fields(out, exporter, types, fields, location, generics, stack);
    }

    let mut base = String::new();
    render_named_fields(
        &mut base,
        exporter,
        types,
        fields,
        location.clone(),
        generics,
        stack,
    )?;
    let fixed_keys = fields
        .iter()
        .filter(|(_, field)| field.ty.is_some())
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();

    let mut rendered_records = Vec::with_capacity(records.len());
    for record in records {
        let DataType::Map(map) = record else {
            unreachable!("object collector only accepts maps as records");
        };
        let mut rendered = String::new();
        map_dt(
            &mut rendered,
            exporter,
            types,
            map,
            location.clone(),
            generics,
            stack,
            &fixed_keys,
        )?;
        rendered_records.push(rendered);
    }

    write!(
        out,
        "Schema.StructWithRest({base}, [{}])",
        rendered_records.join(", ")
    )?;
    Ok(())
}

fn collect_object_part(
    dt: &DataType,
    types: &Types,
    generics: &Generics,
    visiting: &mut HashSet<Reference>,
    fields: &mut Vec<(Cow<'static, str>, Field)>,
    records: &mut Vec<DataType>,
    aliases: &mut Vec<Vec<Vec<(Cow<'static, str>, Field)>>>,
    location: &[Cow<'static, str>],
) -> Result<(), Error> {
    match dt {
        DataType::Struct(strct) => match &strct.fields {
            Fields::Named(named) => {
                fields.extend(named.fields.iter().cloned());
                Ok(())
            }
            Fields::Unit => Ok(()),
            Fields::Unnamed(unnamed) => {
                let mut field_types = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
                let Some(inner) = field_types.next() else {
                    return Ok(());
                };
                if field_types.next().is_some() {
                    return Err(Error::unrepresentable_flattened_generic(location.join(".")));
                }
                collect_object_part(
                    inner, types, generics, visiting, fields, records, aliases, location,
                )
            }
        },
        DataType::Map(map) => {
            records.push(DataType::Map(map.clone()));
            Ok(())
        }
        DataType::Intersection(parts) => {
            for part in parts {
                collect_object_part(
                    part, types, generics, visiting, fields, records, aliases, location,
                )?;
            }
            Ok(())
        }
        DataType::Enum(enm) if enm.attributes.contains_key("specta_serde:alias_union") => {
            let mut branches = Vec::with_capacity(enm.variants.len());
            for (_, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                let Fields::Unnamed(fields) = &variant.fields else {
                    return Err(Error::unrepresentable_flattened_generic(location.join(".")));
                };
                let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                let Some(payload) = fields.next() else {
                    return Err(Error::unrepresentable_flattened_generic(location.join(".")));
                };
                if fields.next().is_some() {
                    return Err(Error::unrepresentable_flattened_generic(location.join(".")));
                }
                branches.push(
                    object_fields(payload, types, generics, &mut HashSet::new()).ok_or_else(
                        || Error::unrepresentable_flattened_generic(location.join(".")),
                    )?,
                );
            }
            aliases.push(branches);
            Ok(())
        }
        DataType::Reference(Reference::Named(reference)) => {
            let key = Reference::Named(reference.clone());
            if !visiting.insert(key.clone()) {
                return Err(Error::unrepresentable_flattened_generic(location.join(".")));
            }
            let result = (|| {
                let ty = named_reference_ty(types, reference, location)?;
                let reference_generics = rendered_reference_generics(reference, generics, types)?;
                collect_object_part(
                    ty,
                    types,
                    &reference_generics,
                    visiting,
                    fields,
                    records,
                    aliases,
                    location,
                )
            })();
            visiting.remove(&key);
            result
        }
        DataType::Generic(generic) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .filter(|(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic))
            .map_or_else(
                || Err(Error::unrepresentable_flattened_generic(location.join("."))),
                |(_, ty)| {
                    collect_object_part(
                        ty, types, generics, visiting, fields, records, aliases, location,
                    )
                },
            ),
        _ => Err(Error::unrepresentable_flattened_generic(location.join("."))),
    }
}

fn reference_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    reference: &Reference,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    match reference {
        Reference::Named(reference) => {
            reference_named_dt(out, exporter, types, reference, location, generics, stack)
        }
        Reference::Opaque(reference) => reference_opaque_dt(out, reference, location),
    }
}

fn reference_opaque_dt(
    out: &mut String,
    reference: &OpaqueReference,
    location: Vec<Cow<'static, str>>,
) -> Result<(), Error> {
    if let Some(define) = reference.downcast_ref::<opaque::Define>() {
        out.push_str(&define.0);
    } else if reference.downcast_ref::<opaque::Any>().is_some() {
        out.push_str("Schema.Any");
    } else if reference.downcast_ref::<opaque::Unknown>().is_some() {
        out.push_str("Schema.Unknown");
    } else if reference.downcast_ref::<opaque::Never>().is_some() {
        out.push_str("Schema.Never");
    } else if reference.downcast_ref::<opaque::Number>().is_some() {
        out.push_str("Schema.Number");
    } else if reference.downcast_ref::<opaque::BigInt>().is_some() {
        out.push_str("Schema.BigInt");
    } else {
        return Err(Error::unsupported_opaque_reference(reference.clone()));
    }
    let _ = location;
    Ok(())
}

fn reference_named_dt(
    out: &mut String,
    exporter: &Effect,
    types: &Types,
    reference: &NamedReference,
    location: Vec<Cow<'static, str>>,
    generics: &Generics,
    stack: &mut RenderStack,
) -> Result<(), Error> {
    if let NamedReferenceType::Inline { dt, .. } = &reference.inner {
        return datatype(out, exporter, types, dt, location, generics, false, stack);
    }

    let ndt = types.get(reference).ok_or_else(|| {
        Error::dangling_named_reference(format!("{reference:?} at {}", location.join(".")))
    })?;
    crate::references::track_nr(reference);

    let name = referenced_schema_name(exporter, ndt);
    let mut expression = name;
    let reference_generics = match &reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
        NamedReferenceType::Recursive(recursive) => recursive.generics(),
        NamedReferenceType::Inline { .. } => unreachable!(),
    };
    let key_generics = key_generics_for(ndt, types);
    let mut scope = generics.to_vec();
    let mut arguments = Vec::new();

    for generic in ndt.generics.iter() {
        let explicit = reference_generics
            .iter()
            .find(|(candidate, _)| candidate == &generic.reference())
            .map(|(_, ty)| ty.clone());
        let mut resolved = explicit
            .or_else(|| {
                generic
                    .default
                    .as_ref()
                    .map(|default| resolve_generic_default(default, &scope))
            })
            .unwrap_or_else(|| DataType::Reference(Reference::opaque(opaque::Unknown)));
        substitute_generics(&mut resolved, &scope);

        let mut argument = String::new();
        datatype(
            &mut argument,
            exporter,
            types,
            &resolved,
            Vec::new(),
            &scope,
            false,
            stack,
        )?;
        arguments.push(argument);

        if key_generics.contains(generic.name.as_ref()) {
            let mut key = String::new();
            map_key_datatype(&mut key, types, &resolved, Vec::new(), &scope)?;
            arguments.push(key);
        }
        scope.push((generic.reference(), resolved));
    }

    if !arguments.is_empty() {
        expression.push('(');
        expression.push_str(&arguments.join(", "));
        expression.push(')');
    }

    let recursive = stack.last().is_some_and(|(module, name)| {
        named_type_reaches(types, ndt, module, name, &mut HashSet::new())
    });
    if recursive {
        write!(
            out,
            "Schema.suspend((): Schema.Codec<unknown> => {expression})"
        )?;
    } else {
        write!(out, "Schema.suspend(() => {expression})")?;
    }
    Ok(())
}

fn named_type_reaches(
    types: &Types,
    from: &NamedDataType,
    target_module: &str,
    target_name: &str,
    visiting: &mut HashSet<Reference>,
) -> bool {
    from.ty
        .as_ref()
        .is_some_and(|ty| datatype_reaches(types, ty, target_module, target_name, visiting))
        || from.generics.iter().any(|generic| {
            generic.default.as_ref().is_some_and(|default| {
                datatype_reaches(types, default, target_module, target_name, visiting)
            })
        })
}

fn datatype_reaches(
    types: &Types,
    dt: &DataType,
    target_module: &str,
    target_name: &str,
    visiting: &mut HashSet<Reference>,
) -> bool {
    match dt {
        DataType::Reference(Reference::Named(reference)) => {
            let generics = match &reference.inner {
                NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
                NamedReferenceType::Recursive(recursive) => recursive.generics(),
                NamedReferenceType::Inline { .. } => &[],
            };
            if generics
                .iter()
                .any(|(_, ty)| datatype_reaches(types, ty, target_module, target_name, visiting))
            {
                return true;
            }
            if let NamedReferenceType::Inline { dt, .. } = &reference.inner {
                return datatype_reaches(types, dt, target_module, target_name, visiting);
            }
            let Some(ndt) = types.get(reference) else {
                return false;
            };
            if ndt.module_path == target_module && ndt.name == target_name {
                return true;
            }
            let key = Reference::Named(reference.clone());
            if !visiting.insert(key.clone()) {
                return false;
            }
            let reached = named_type_reaches(types, ndt, target_module, target_name, visiting);
            visiting.remove(&key);
            reached
        }
        DataType::List(list) => {
            datatype_reaches(types, &list.ty, target_module, target_name, visiting)
        }
        DataType::Map(map) => {
            datatype_reaches(types, map.key_ty(), target_module, target_name, visiting)
                || datatype_reaches(types, map.value_ty(), target_module, target_name, visiting)
        }
        DataType::Nullable(inner) => {
            datatype_reaches(types, inner, target_module, target_name, visiting)
        }
        DataType::Struct(strct) => {
            fields_reach(types, &strct.fields, target_module, target_name, visiting)
        }
        DataType::Enum(enm) => enm.variants.iter().any(|(_, variant)| {
            fields_reach(types, &variant.fields, target_module, target_name, visiting)
        }),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|element| datatype_reaches(types, element, target_module, target_name, visiting)),
        DataType::Intersection(parts) => parts
            .iter()
            .any(|part| datatype_reaches(types, part, target_module, target_name, visiting)),
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => false,
    }
}

fn fields_reach(
    types: &Types,
    fields: &Fields,
    target_module: &str,
    target_name: &str,
    visiting: &mut HashSet<Reference>,
) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter_map(|field| field.ty.as_ref())
            .any(|ty| datatype_reaches(types, ty, target_module, target_name, visiting)),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter_map(|(_, field)| field.ty.as_ref())
            .any(|ty| datatype_reaches(types, ty, target_module, target_name, visiting)),
    }
}

fn resolve_generic_default(default: &DataType, scope: &Generics) -> DataType {
    match default {
        DataType::Generic(generic) => scope
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .map(|(_, ty)| ty.clone())
            .unwrap_or_else(|| default.clone()),
        _ => default.clone(),
    }
}

fn named_reference_ty<'a>(
    types: &'a Types,
    reference: &'a NamedReference,
    location: &[Cow<'static, str>],
) -> Result<&'a DataType, Error> {
    match &reference.inner {
        NamedReferenceType::Reference { .. } => types
            .get(reference)
            .and_then(|ndt| ndt.ty.as_ref())
            .ok_or_else(|| Error::dangling_named_reference(format!("{reference:?}"))),
        NamedReferenceType::Inline { dt, .. } => Ok(dt),
        NamedReferenceType::Recursive(_) => Err(Error::dangling_named_reference(format!(
            "recursive inline named reference {reference:?} at {}",
            location.join(".")
        ))),
    }
}

fn rendered_reference_generics(
    reference: &NamedReference,
    outer_generics: &Generics,
    types: &Types,
) -> Result<Vec<(GenericReference, DataType)>, Error> {
    if matches!(reference.inner, NamedReferenceType::Inline { .. }) {
        Ok(outer_generics.to_vec())
    } else {
        instantiated_reference_generics(reference, outer_generics, types)
    }
}

fn instantiated_reference_generics(
    reference: &NamedReference,
    outer_generics: &Generics,
    types: &Types,
) -> Result<Vec<(GenericReference, DataType)>, Error> {
    if matches!(reference.inner, NamedReferenceType::Inline { .. }) {
        return Ok(Vec::new());
    }
    let ndt = types
        .get(reference)
        .ok_or_else(|| Error::dangling_named_reference(format!("{reference:?}")))?;
    let explicit = match &reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => &[],
    };
    let mut scope = outer_generics.to_vec();
    let mut result = Vec::with_capacity(ndt.generics.len());
    for generic in ndt.generics.iter() {
        let mut resolved = explicit
            .iter()
            .find(|(candidate, _)| candidate == &generic.reference())
            .map(|(_, ty)| ty.clone())
            .or_else(|| {
                generic
                    .default
                    .as_ref()
                    .map(|default| resolve_generic_default(default, &scope))
            })
            .unwrap_or_else(|| DataType::Reference(Reference::opaque(opaque::Unknown)));
        substitute_generics(&mut resolved, &scope);
        result.push((generic.reference(), resolved.clone()));
        scope.push((generic.reference(), resolved));
    }
    Ok(result)
}

fn resolved_reference_generics(
    reference: &NamedReference,
    outer_generics: &Generics,
) -> Result<Vec<(GenericReference, DataType)>, Error> {
    let generics = match &reference.inner {
        NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => &[],
    };
    Ok(generics
        .iter()
        .map(|(generic, ty)| {
            let mut ty = ty.clone();
            substitute_generics(&mut ty, outer_generics);
            (generic.clone(), ty)
        })
        .collect())
}

fn substitute_generics(dt: &mut DataType, generics: &Generics) {
    match dt {
        DataType::Generic(generic) => {
            if let Some((_, replacement)) = generics
                .iter()
                .find(|(candidate, _)| candidate == generic)
                .filter(
                    |(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic),
                )
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
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_field_generics(&mut variant.fields, generics);
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                substitute_generics(element, generics);
            }
        }
        DataType::Intersection(parts) => {
            for part in parts {
                substitute_generics(part, generics);
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, generics),
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => {
                for (_, ty) in reference_generics {
                    substitute_generics(ty, generics);
                }
            }
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(fields: &mut Fields, generics: &Generics) {
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

fn key_generics_for(ndt: &NamedDataType, types: &Types) -> HashSet<String> {
    let Some(ty) = ndt.ty.as_ref() else {
        return HashSet::new();
    };
    let mut result = generic_map_key_parameters(ty, types);
    loop {
        let previous_len = result.len();
        for generic in ndt.generics.iter() {
            let Some(default) = &generic.default else {
                continue;
            };
            result.extend(generic_map_key_parameters(default, types));
            if result.contains(generic.name.as_ref()) {
                collect_generic_names(default, &mut result);
            }
        }
        if result.len() == previous_len {
            return result;
        }
    }
}

fn collect_generic_names(dt: &DataType, result: &mut HashSet<String>) {
    match dt {
        DataType::Generic(generic) => {
            result.insert(generic.name().to_string());
        }
        DataType::List(list) => collect_generic_names(&list.ty, result),
        DataType::Map(map) => {
            collect_generic_names(map.key_ty(), result);
            collect_generic_names(map.value_ty(), result);
        }
        DataType::Nullable(inner) => collect_generic_names(inner, result),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for field in fields.fields.iter().filter_map(|field| field.ty.as_ref()) {
                    collect_generic_names(field, result);
                }
            }
            Fields::Named(fields) => {
                for (_, field) in &fields.fields {
                    if let Some(ty) = &field.ty {
                        collect_generic_names(ty, result);
                    }
                }
            }
        },
        DataType::Enum(enm) => {
            for (_, variant) in &enm.variants {
                match &variant.fields {
                    Fields::Unit => {}
                    Fields::Unnamed(fields) => {
                        for field in fields.fields.iter().filter_map(|field| field.ty.as_ref()) {
                            collect_generic_names(field, result);
                        }
                    }
                    Fields::Named(fields) => {
                        for (_, field) in &fields.fields {
                            if let Some(ty) = &field.ty {
                                collect_generic_names(ty, result);
                            }
                        }
                    }
                }
            }
        }
        DataType::Tuple(tuple) => {
            for element in &tuple.elements {
                collect_generic_names(element, result);
            }
        }
        DataType::Intersection(parts) => {
            for part in parts {
                collect_generic_names(part, result);
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => collect_generic_names(dt, result),
            NamedReferenceType::Reference { generics, .. } => {
                for (_, ty) in generics {
                    collect_generic_names(ty, result);
                }
            }
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn generic_map_key_parameters(dt: &DataType, types: &Types) -> HashSet<String> {
    fn collect_fields(
        fields: &Fields,
        types: &Types,
        in_key: bool,
        generics: &Generics,
        visiting: &mut HashSet<Reference>,
        result: &mut HashSet<String>,
    ) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for field in fields.fields.iter().filter_map(|field| field.ty.as_ref()) {
                    collect(field, types, in_key, generics, visiting, result);
                }
            }
            Fields::Named(fields) => {
                for (_, field) in &fields.fields {
                    if let Some(ty) = &field.ty {
                        collect(ty, types, in_key, generics, visiting, result);
                    }
                }
            }
        }
    }

    fn collect(
        dt: &DataType,
        types: &Types,
        in_key: bool,
        generics: &Generics,
        visiting: &mut HashSet<Reference>,
        result: &mut HashSet<String>,
    ) {
        match dt {
            DataType::Generic(generic) => {
                if let Some((_, replacement)) = generics
                    .iter()
                    .find(|(candidate, _)| candidate == generic)
                    .filter(|(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic))
                {
                    collect(replacement, types, in_key, generics, visiting, result);
                } else if in_key {
                    result.insert(generic.name().to_string());
                }
            }
            DataType::List(list) => collect(&list.ty, types, in_key, generics, visiting, result),
            DataType::Map(map) => {
                collect(map.key_ty(), types, true, generics, visiting, result);
                collect(map.value_ty(), types, false, generics, visiting, result);
            }
            DataType::Nullable(inner) => collect(inner, types, in_key, generics, visiting, result),
            DataType::Struct(strct) => {
                collect_fields(&strct.fields, types, in_key, generics, visiting, result)
            }
            DataType::Enum(enm) => {
                for (_, variant) in &enm.variants {
                    collect_fields(&variant.fields, types, in_key, generics, visiting, result);
                }
            }
            DataType::Tuple(tuple) => {
                for element in &tuple.elements {
                    collect(element, types, in_key, generics, visiting, result);
                }
            }
            DataType::Intersection(parts) => {
                for part in parts {
                    collect(part, types, in_key, generics, visiting, result);
                }
            }
            DataType::Reference(Reference::Named(reference)) => {
                let key = Reference::Named(reference.clone());
                if !visiting.insert(key.clone()) {
                    return;
                }
                let result = (|| {
                    let ty = named_reference_ty(types, reference, &[]).ok()?;
                    let reference_generics =
                        rendered_reference_generics(reference, generics, types).ok()?;
                    collect(ty, types, in_key, &reference_generics, visiting, result);
                    Some(())
                })();
                visiting.remove(&key);
                let _ = result;
            }
            DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
        }
    }

    let mut result = HashSet::new();
    collect(dt, types, false, &[], &mut HashSet::new(), &mut result);
    result
}

fn map_key_schema_type(dt: &DataType, types: &Types, generics: &Generics) -> Result<String, Error> {
    match dt {
        DataType::Primitive(Primitive::str | Primitive::char | Primitive::bool) => {
            Ok("typeof Schema.String".to_string())
        }
        DataType::Primitive(
            Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32,
        ) => Ok("typeof Schema.Int".to_string()),
        DataType::Primitive(Primitive::f16 | Primitive::f32 | Primitive::f64) => {
            Ok("typeof Schema.Finite".to_string())
        }
        DataType::Primitive(_) => Err(Error::bigint_forbidden("generic default key".to_string())),
        DataType::Struct(strct) => {
            let Fields::Unnamed(fields) = &strct.fields else {
                return Ok("Schema.Record.Key".to_string());
            };
            let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
            let Some(inner) = fields.next() else {
                return Ok("Schema.Record.Key".to_string());
            };
            if fields.next().is_some() {
                return Ok("Schema.Record.Key".to_string());
            }
            map_key_schema_type(inner, types, generics)
        }
        DataType::Enum(_) => Ok("typeof Schema.String".to_string()),
        DataType::Reference(Reference::Named(reference)) => {
            let ty = named_reference_ty(types, reference, &[])?;
            let reference_generics = rendered_reference_generics(reference, generics, types)?;
            map_key_schema_type(ty, types, &reference_generics)
        }
        DataType::Generic(generic) => {
            if let Some((_, replacement)) = generics
                .iter()
                .find(|(candidate, _)| candidate == generic)
                .filter(
                    |(_, ty)| !matches!(ty, DataType::Generic(candidate) if candidate == generic),
                )
            {
                map_key_schema_type(replacement, types, generics)
            } else {
                Ok(map_key_generic_name(generic.name()))
            }
        }
        _ => Ok("Schema.Record.Key".to_string()),
    }
}

fn schema_type(
    dt: &DataType,
    exporter: &Effect,
    types: &Types,
    generics: &Generics,
) -> Result<String, Error> {
    match dt {
        DataType::Primitive(primitive) => Ok(match primitive {
            Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32 => "typeof Schema.Int".to_string(),
            Primitive::f16 | Primitive::f32 | Primitive::f64 => {
                "Schema.NullOr<typeof Schema.Finite>".to_string()
            }
            Primitive::bool => "typeof Schema.Boolean".to_string(),
            Primitive::str | Primitive::char => "typeof Schema.String".to_string(),
            Primitive::usize
            | Primitive::isize
            | Primitive::i64
            | Primitive::u64
            | Primitive::i128
            | Primitive::u128
            | Primitive::f128 => {
                return Err(Error::bigint_forbidden("generic default".to_string()));
            }
        }),
        DataType::Generic(generic) => Ok(generic.name().to_string()),
        DataType::List(list) => Ok(format!(
            "Schema.$Array<{}>",
            schema_type(&list.ty, exporter, types, generics)?
        )),
        DataType::Map(map) => Ok(format!(
            "Schema.$Record<Schema.Record.Key, {}>",
            schema_type(map.value_ty(), exporter, types, generics)?
        )),
        DataType::Nullable(inner) => Ok(format!(
            "Schema.NullOr<{}>",
            schema_type(inner, exporter, types, generics)?
        )),
        DataType::Tuple(tuple) => {
            let elements = tuple
                .elements
                .iter()
                .map(|element| schema_type(element, exporter, types, generics))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(format!("Schema.Tuple<[{elements}]>"))
        }
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit => Ok("typeof Schema.Null".to_string()),
            Fields::Unnamed(fields) => {
                let mut field_types = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                let Some(inner) = field_types.next() else {
                    return Ok("Schema.Tuple<[]>".to_string());
                };
                if field_types.next().is_none() {
                    Ok(schema_type(inner, exporter, types, generics)?)
                } else {
                    Ok("Schema.Constraint".to_string())
                }
            }
            Fields::Named(named) => {
                let fields = named
                    .fields
                    .iter()
                    .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
                    .map(|(name, field, ty)| {
                        let ty = schema_type(ty, exporter, types, generics)?;
                        Ok(if field.optional {
                            format!("{}: Schema.optionalKey<{ty}>", sanitise_key(name))
                        } else {
                            format!("{}: {ty}", sanitise_key(name))
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?
                    .join("; ");
                Ok(format!("Schema.Struct<{{{fields}}}>"))
            }
        },
        DataType::Enum(enm) => {
            let variants = enm
                .variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .map(|(name, variant)| match &variant.fields {
                    Fields::Unit => Ok(format!(
                        "Schema.Literal<\"{}\">",
                        escape_string_literal(name)
                    )),
                    Fields::Named(named) => {
                        let fields = named
                            .fields
                            .iter()
                            .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, ty)))
                            .map(|(name, ty)| {
                                Ok(format!(
                                    "{}: {}",
                                    sanitise_key(name),
                                    schema_type(ty, exporter, types, generics)?
                                ))
                            })
                            .collect::<Result<Vec<_>, Error>>()?
                            .join("; ");
                        Ok(format!("Schema.Struct<{{{fields}}}>"))
                    }
                    Fields::Unnamed(fields) => {
                        let elements = fields
                            .fields
                            .iter()
                            .filter_map(|field| field.ty.as_ref())
                            .map(|ty| schema_type(ty, exporter, types, generics))
                            .collect::<Result<Vec<_>, _>>()?;
                        if elements.len() == 1 {
                            Ok(elements[0].clone())
                        } else {
                            Ok(format!("Schema.Tuple<[{}]>", elements.join(", ")))
                        }
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;
            if variants.is_empty() {
                Ok("typeof Schema.Never".to_string())
            } else if variants.len() == 1 {
                Ok(variants[0].clone())
            } else {
                Ok(format!("Schema.Union<[{}]>", variants.join(", ")))
            }
        }
        DataType::Intersection(_) => Ok("Schema.Constraint".to_string()),
        DataType::Reference(Reference::Named(reference)) => {
            if let NamedReferenceType::Inline { dt, .. } = &reference.inner {
                return schema_type(dt, exporter, types, generics);
            }
            let ndt = types
                .get(reference)
                .ok_or_else(|| Error::dangling_named_reference(format!("{reference:?}")))?;
            let name = referenced_schema_name(exporter, ndt);
            if ndt.generics.is_empty() {
                return Ok(format!("typeof {name}"));
            }
            let reference_generics = rendered_reference_generics(reference, generics, types)?;
            let key_generics = key_generics_for(ndt, types);
            let mut arguments = Vec::new();
            for (generic, ty) in &reference_generics {
                arguments.push(schema_type(ty, exporter, types, generics)?);
                if key_generics.contains(generic.name().as_ref()) {
                    arguments.push(map_key_schema_type(ty, types, generics)?);
                }
            }
            Ok(format!(
                "ReturnType<typeof {name}<{}>>",
                arguments.join(", ")
            ))
        }
        DataType::Reference(Reference::Opaque(reference)) => {
            if reference.downcast_ref::<opaque::Any>().is_some() {
                Ok("typeof Schema.Any".to_string())
            } else if reference.downcast_ref::<opaque::Unknown>().is_some() {
                Ok("typeof Schema.Unknown".to_string())
            } else if reference.downcast_ref::<opaque::Never>().is_some() {
                Ok("typeof Schema.Never".to_string())
            } else if reference.downcast_ref::<opaque::Number>().is_some() {
                Ok("typeof Schema.Number".to_string())
            } else if reference.downcast_ref::<opaque::BigInt>().is_some() {
                Ok("typeof Schema.BigInt".to_string())
            } else {
                Ok("Schema.Constraint".to_string())
            }
        }
    }
}

pub(crate) fn exported_type_name<'a>(exporter: &Effect, ndt: &'a NamedDataType) -> Cow<'a, str> {
    match exporter.layout {
        Layout::ModulePrefixedName => {
            let mut name = ndt.module_path.split("::").collect::<Vec<_>>().join("_");
            if !name.is_empty() {
                name.push('_');
            }
            name.push_str(&ndt.name);
            Cow::Owned(name)
        }
        _ => ndt.name.clone(),
    }
}

fn referenced_schema_name(exporter: &Effect, ndt: &NamedDataType) -> String {
    let base = exported_type_name(exporter, ndt);
    match exporter.layout {
        Layout::Namespaces => {
            let module_path = crate::effect::namespace_module_path(&ndt.module_path);
            if module_path.is_empty() {
                format!("$s$.{base}Schema")
            } else {
                format!("$s$.{module_path}.{base}Schema")
            }
        }
        Layout::Files => {
            let current = crate::references::current_module_path().unwrap_or_default();
            if ndt.module_path == current {
                format!("{base}Schema")
            } else {
                format!(
                    "{}.{}Schema",
                    crate::effect::module_alias(&ndt.module_path),
                    ndt.name
                )
            }
        }
        Layout::FlatFile | Layout::ModulePrefixedName => format!("{base}Schema"),
    }
}

fn validate_type_name(name: &str, path: String) -> Result<(), Error> {
    if RESERVED_TYPE_NAMES.contains(&name) {
        return Err(Error::forbidden_name(path, name.to_string()));
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(Error::invalid_name(path, String::new()));
    };
    if !(first.is_alphabetic() || first == '_') {
        return Err(Error::invalid_name(path, name.to_string()));
    }
    if chars.any(|ch| !(ch.is_alphanumeric() || ch == '_')) {
        return Err(Error::invalid_name(path, name.to_string()));
    }
    Ok(())
}

fn sanitise_key(name: &str) -> String {
    if is_identifier(name) {
        if name == "__proto__" {
            "[\"__proto__\"]".to_string()
        } else {
            name.to_string()
        }
    } else {
        format!("\"{}\"", escape_string_literal(name))
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

pub(crate) fn escape_string_literal(value: &str) -> Cow<'_, str> {
    if !value.chars().any(|ch| {
        ch == '\"' || ch == '\\' || ch == '\u{2028}' || ch == '\u{2029}' || ch.is_control()
    }) {
        return Cow::Borrowed(value);
    }

    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            ch if ch.is_control() => {
                use std::fmt::Write as _;
                write!(escaped, "\\u{:04X}", ch as u32).expect("writing to a string cannot fail");
            }
            ch => escaped.push(ch),
        }
    }
    Cow::Owned(escaped)
}

fn child_location(
    location: &[Cow<'static, str>],
    child: impl Into<Cow<'static, str>>,
) -> Vec<Cow<'static, str>> {
    let mut location = location.to_vec();
    location.push(child.into());
    location
}

fn rust_type_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn map_key_generic_name(generic: &str) -> String {
    ["$key$", generic].concat()
}

fn map_key_generic_value(generic: &str) -> String {
    ["$key$", &generic.to_lowercase()].concat()
}
