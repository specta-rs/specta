use std::{borrow::Cow, collections::BTreeMap};

use specta::{
    Format, Types,
    datatype::{
        DataType, Enum, Field, Fields, Generic, NamedDataType, NamedReferenceType, Primitive,
        Reference, Struct, Variant,
    },
};

use crate::{
    Error, Kotlin, Layout, NamingConvention, Serialization, UnknownType, reserved_names::RESERVED,
};

pub(crate) enum Selection<'a> {
    All,
    One(&'a NamedDataType),
    RawOnly,
}

pub(crate) fn render_file(
    kotlin: &Kotlin,
    types: &Types,
    format: Option<&dyn Format>,
    selection: Selection<'_>,
) -> Result<String, Error> {
    let mut out = String::new();
    if !kotlin.header.is_empty() {
        out.push_str(kotlin.header.trim_end());
        out.push_str("\n\n");
    }
    if kotlin.serialization == Serialization::Kotlinx {
        out.push_str("@file:OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)\n\n");
    }
    if let Some(package) = &kotlin.package {
        out.push_str("package ");
        out.push_str(&package_name(package)?);
        out.push_str("\n\n");
    }
    if kotlin.serialization == Serialization::Kotlinx {
        out.push_str("import kotlinx.serialization.EncodeDefault\n");
        out.push_str("import kotlinx.serialization.SerialName\n");
        out.push_str("import kotlinx.serialization.Serializable\n\n");
    }

    match selection {
        Selection::All => {
            let mut names = BTreeMap::new();
            for ndt in types.into_sorted_iter().filter(|ndt| ndt.ty.is_some()) {
                let name = named_type_identifier(kotlin, ndt, &ndt.name)?;
                if names.insert(name.clone(), ndt.name.as_ref()).is_some() {
                    return Err(Error::DuplicateTypeName { name });
                }
                render_named(&mut out, kotlin, format, types, ndt)?;
                out.push_str("\n\n");
            }
            render_raw(&mut out, kotlin);
        }
        Selection::One(ndt) => render_named(&mut out, kotlin, format, types, ndt)?,
        Selection::RawOnly => render_raw(&mut out, kotlin),
    }

    Ok(out.trim_end().to_owned() + "\n")
}

fn render_raw(out: &mut String, kotlin: &Kotlin) {
    for (index, raw) in kotlin.raw().iter().enumerate() {
        if index != 0 || !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(raw.trim());
        out.push('\n');
    }
}

pub(crate) fn filename(kotlin: &Kotlin, name: &str) -> Result<String, Error> {
    let converted = convert_name(kotlin.naming, name, NameKind::Type);
    validate_identifier(&converted, name)?;
    Ok(converted)
}

fn render_named(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    ndt: &NamedDataType,
) -> Result<(), Error> {
    let Some(original) = &ndt.ty else {
        return Ok(());
    };
    let ty = map_datatype(format, types, original)?;
    render_kdoc(out, "", &ndt.docs);
    render_deprecated(out, "", ndt.deprecated.as_ref());

    let name = named_type_identifier(kotlin, ndt, &ndt.name)?;
    let generics = ndt
        .generics
        .iter()
        .map(|generic| identifier(&generic.name, &ndt.name))
        .collect::<Result<Vec<_>, _>>()?;
    ensure_unique(ndt.name.as_ref(), generics.iter().map(String::as_str))?;
    let generic_scope = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();

    match &ty {
        DataType::Struct(strct) => render_struct(
            out,
            kotlin,
            format,
            types,
            &name,
            &generics,
            &generic_scope,
            strct,
            &ndt.name,
        ),
        DataType::Enum(enm) => render_enum(
            out,
            kotlin,
            format,
            types,
            &name,
            &generics,
            &generic_scope,
            enm,
            &ndt.name,
        ),
        _ => {
            out.push_str("public typealias ");
            out.push_str(&name);
            out.push_str(&generic_declaration(&generics));
            out.push_str(" = ");
            out.push_str(&datatype(
                kotlin,
                format,
                types,
                &ty,
                &generic_scope,
                &ndt.name,
            )?);
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_struct(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    name: &str,
    generics: &[String],
    generic_scope: &[Generic],
    strct: &Struct,
    path: &str,
) -> Result<(), Error> {
    if kotlin.serialization == Serialization::Kotlinx {
        match &strct.fields {
            Fields::Unit => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx object serialization does not preserve unit-struct null encoding",
                });
            }
            Fields::Unnamed(fields)
                if fields.fields.len() != 1 || fields.fields[0].ty.is_none() =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx data classes do not preserve tuple-struct array encoding",
                });
            }
            Fields::Unnamed(_) if kotlin.mutable_properties => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "mutable Kotlinx newtypes cannot preserve scalar encoding",
                });
            }
            Fields::Unnamed(_) | Fields::Named(_) => {}
        }
    }
    annotation(out, "", kotlin, "Serializable", None);
    match &strct.fields {
        Fields::Unit => {
            if generics.is_empty() {
                out.push_str("public data object ");
                out.push_str(name);
            } else {
                out.push_str("public class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
            }
        }
        Fields::Named(fields) if fields.fields.iter().all(|(_, field)| field.ty.is_none()) => {
            out.push_str("public class ");
            out.push_str(name);
            out.push_str(&generic_declaration(generics));
        }
        Fields::Named(fields) => {
            ensure_unique(
                path,
                fields
                    .fields
                    .iter()
                    .filter(|(_, field)| field.ty.is_some())
                    .map(|(name, _)| {
                        safe_member_name(
                            &convert_name(kotlin.naming, name, NameKind::Property),
                            "field",
                        )
                    }),
            )?;
            out.push_str("public data class ");
            out.push_str(name);
            out.push_str(&generic_declaration(generics));
            out.push_str("(\n");
            let fields = fields
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            for (index, (field_name, field)) in fields.iter().enumerate() {
                render_property(
                    out,
                    kotlin,
                    format,
                    types,
                    generic_scope,
                    field_name,
                    field,
                    &format!("{path}.{field_name}"),
                    1,
                )?;
                if index + 1 != fields.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push(')');
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.len() == 1 && !kotlin.mutable_properties {
                out.push_str("@JvmInline\npublic value class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(public ");
                out.push_str(property_keyword(kotlin));
                out.push_str(" value: ");
                let (index, field) = fields[0];
                out.push_str(&field_datatype(
                    kotlin,
                    format,
                    types,
                    generic_scope,
                    field,
                    &format!("{path}.{index}"),
                )?);
                if field.optional {
                    out.push_str(" = null");
                }
                out.push(')');
            } else if fields.is_empty() {
                out.push_str("public class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
            } else {
                out.push_str("public data class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                for (position, (index, field)) in fields.iter().enumerate() {
                    let field_name = format!("field{index}");
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        &field_name,
                        field,
                        &format!("{path}.{index}"),
                        1,
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push(')');
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_enum(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    name: &str,
    generics: &[String],
    generic_scope: &[Generic],
    enm: &Enum,
    path: &str,
) -> Result<(), Error> {
    let variants = enm
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();
    let serde_rewritten = enm
        .attributes
        .contains_key("specta_serde:enum_repr_rewritten");
    ensure_unique(
        path,
        variants.iter().map(|(name, _)| {
            safe_member_name(&convert_variant_name(kotlin.naming, name), "Variant")
        }),
    )?;
    let unit_only = generics.is_empty()
        && variants.iter().all(|(name, variant)| {
            is_unit_fields(&normalized_variant_fields(
                name,
                &variant.fields,
                serde_rewritten,
            ))
        });

    if kotlin.serialization == Serialization::Kotlinx {
        return Err(Error::UnsupportedType {
            path: path.into(),
            reason: "standard Kotlinx enum and polymorphic encodings cannot preserve every Serde enum representation",
        });
    }

    annotation(out, "", kotlin, "Serializable", None);
    if unit_only {
        out.push_str("public enum class ");
        out.push_str(name);
        out.push_str(" {\n");
        for (index, (variant_name, variant)) in variants.iter().enumerate() {
            let indent = kotlin.indentation(1);
            render_kdoc(out, &indent, &variant.docs);
            render_deprecated(out, &indent, variant.deprecated.as_ref());
            let converted = safe_member_name(
                &convert_variant_name(kotlin.naming, variant_name),
                "Variant",
            );
            if converted != variant_name.as_ref() {
                annotation(out, &indent, kotlin, "SerialName", Some(variant_name));
            }
            out.push_str(&indent);
            out.push_str(&identifier(&converted, &format!("{path}.{variant_name}"))?);
            if index + 1 != variants.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push('}');
        return Ok(());
    }

    out.push_str("public sealed interface ");
    out.push_str(name);
    out.push_str(&generic_declaration(generics));
    out.push_str(" {\n");
    for (variant_name, variant) in variants {
        render_variant(
            out,
            kotlin,
            format,
            types,
            name,
            generics,
            generic_scope,
            variant_name,
            variant,
            path,
            serde_rewritten,
        )?;
        out.push('\n');
    }
    out.push('}');
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_variant(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    parent: &str,
    generics: &[String],
    generic_scope: &[Generic],
    original_name: &str,
    variant: &Variant,
    path: &str,
    serde_rewritten: bool,
) -> Result<(), Error> {
    let fields = normalized_variant_fields(original_name, &variant.fields, serde_rewritten);
    let indent = kotlin.indentation(1);
    render_kdoc(out, &indent, &variant.docs);
    render_deprecated(out, &indent, variant.deprecated.as_ref());
    annotation(out, &indent, kotlin, "Serializable", None);
    let converted = safe_member_name(
        &convert_variant_name(kotlin.naming, original_name),
        "Variant",
    );
    if converted != original_name {
        annotation(out, &indent, kotlin, "SerialName", Some(original_name));
    }
    let variant_name = identifier(&converted, &format!("{path}.{original_name}"))?;
    out.push_str(&indent);
    let parent_type = format!("{parent}{}", generic_usage(generics));

    match &fields {
        Fields::Unit => {
            if generics.is_empty() {
                out.push_str("public data object ");
                out.push_str(&variant_name);
            } else {
                out.push_str("public class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str(" : ");
                out.push_str(&parent_type);
                return Ok(());
            }
            out.push_str(" : ");
            out.push_str(&parent_type);
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.is_empty() {
                out.push_str("public data object ");
                out.push_str(&variant_name);
                out.push_str(" : ");
                out.push_str(&parent_type);
            } else {
                out.push_str("public data class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                for (position, (index, field)) in fields.iter().enumerate() {
                    let property = if fields.len() == 1 {
                        "value".to_owned()
                    } else {
                        format!("field{index}")
                    };
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        &property,
                        field,
                        &format!("{path}.{original_name}.{index}"),
                        2,
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&indent);
                out.push_str(") : ");
                out.push_str(&parent_type);
            }
        }
        Fields::Named(fields) => {
            ensure_unique(
                &format!("{path}.{original_name}"),
                fields
                    .fields
                    .iter()
                    .filter(|(_, field)| field.ty.is_some())
                    .map(|(name, _)| {
                        safe_member_name(
                            &convert_name(kotlin.naming, name, NameKind::Property),
                            "field",
                        )
                    }),
            )?;
            let fields = fields
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.is_empty() {
                out.push_str("public data object ");
                out.push_str(&variant_name);
                out.push_str(" : ");
                out.push_str(&parent_type);
            } else {
                out.push_str("public data class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                for (position, (field_name, field)) in fields.iter().enumerate() {
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        field_name,
                        field,
                        &format!("{path}.{original_name}.{field_name}"),
                        2,
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&indent);
                out.push_str(") : ");
                out.push_str(&parent_type);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_property(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    generic_scope: &[Generic],
    original_name: &str,
    field: &Field,
    path: &str,
    depth: usize,
) -> Result<(), Error> {
    let indent = kotlin.indentation(depth);
    let rendered = field_datatype(kotlin, format, types, generic_scope, field, path)?;
    render_kdoc(out, &indent, &field.docs);
    render_deprecated(out, &indent, field.deprecated.as_ref());
    let converted = safe_member_name(
        &convert_name(kotlin.naming, original_name, NameKind::Property),
        "field",
    );
    if converted != original_name {
        annotation(out, &indent, kotlin, "SerialName", Some(original_name));
    }
    if kotlin.serialization == Serialization::Kotlinx && rendered.ends_with('?') && !field.optional
    {
        annotation(out, &indent, kotlin, "EncodeDefault", None);
    }
    out.push_str(&indent);
    out.push_str("public ");
    out.push_str(property_keyword(kotlin));
    out.push(' ');
    out.push_str(&identifier(&converted, path)?);
    out.push_str(": ");
    out.push_str(&rendered);
    if field.optional || (kotlin.serialization == Serialization::Kotlinx && rendered.ends_with('?'))
    {
        out.push_str(" = null");
    }
    Ok(())
}

fn field_datatype(
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    generic_scope: &[Generic],
    field: &Field,
    path: &str,
) -> Result<String, Error> {
    let ty = field.ty.as_ref().ok_or_else(|| Error::UnsupportedType {
        path: path.to_owned(),
        reason: "skipped field cannot be rendered",
    })?;
    let mut rendered = datatype(kotlin, format, types, ty, generic_scope, path)?;
    if kotlin.serialization == Serialization::Kotlinx && field.optional && !rendered.ends_with('?')
    {
        return Err(Error::UnsupportedType {
            path: path.into(),
            reason: "optional non-null fields require a wire-specific Kotlin default",
        });
    }
    if field.optional && !rendered.ends_with('?') {
        rendered.push('?');
    }
    Ok(rendered)
}

fn datatype(
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    generic_scope: &[Generic],
    path: &str,
) -> Result<String, Error> {
    let dt = map_datatype(format, types, dt)?;
    Ok(match &dt {
        DataType::Primitive(Primitive::i128 | Primitive::u128 | Primitive::f128)
            if kotlin.serialization == Serialization::Kotlinx =>
        {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "wide number has no built-in Kotlinx serializer",
            });
        }
        DataType::Primitive(primitive) => primitive_name(primitive).to_owned(),
        DataType::List(list) => format!(
            "List<{}>",
            datatype(kotlin, format, types, &list.ty, generic_scope, path)?
        ),
        DataType::Map(map) => format!(
            "Map<{}, {}>",
            datatype(kotlin, format, types, map.key_ty(), generic_scope, path)?,
            datatype(kotlin, format, types, map.value_ty(), generic_scope, path)?
        ),
        DataType::Nullable(inner) => {
            let inner = datatype(kotlin, format, types, inner, generic_scope, path)?;
            if inner.ends_with('?') {
                inner
            } else {
                format!("{inner}?")
            }
        }
        DataType::Tuple(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "Kotlin Pair/Triple serialization does not preserve tuple array encoding",
            });
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => "Unit".to_owned(),
            [single] => datatype(kotlin, format, types, single, generic_scope, path)?,
            [first, second] => format!(
                "Pair<{}, {}>",
                datatype(kotlin, format, types, first, generic_scope, path)?,
                datatype(kotlin, format, types, second, generic_scope, path)?
            ),
            [first, second, third] => format!(
                "Triple<{}, {}, {}>",
                datatype(kotlin, format, types, first, generic_scope, path)?,
                datatype(kotlin, format, types, second, generic_scope, path)?,
                datatype(kotlin, format, types, third, generic_scope, path)?
            ),
            _ => "List<Any?>".to_owned(),
        },
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
                let name = named_type_identifier(kotlin, ndt, path)?;
                let generics = generics
                    .iter()
                    .map(|(_, generic)| {
                        datatype(kotlin, format, types, generic, generic_scope, path)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                format!("{name}{}", generic_usage(&generics))
            }
            NamedReferenceType::Inline { dt, .. } => {
                datatype(kotlin, format, types, dt, generic_scope, path)?
            }
            NamedReferenceType::Recursive(_) => {
                return Err(Error::RecursiveInlineType { path: path.into() });
            }
        },
        DataType::Reference(Reference::Opaque(reference)) => match kotlin.unknown_types {
            UnknownType::Any if kotlin.serialization == Serialization::None => "Any?".to_owned(),
            UnknownType::Any | UnknownType::Error => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: reference.type_name(),
                });
            }
        },
        DataType::Generic(generic) => generic_scope
            .iter()
            .find(|candidate| *candidate == generic)
            .map(|generic| identifier(generic.name(), path))
            .transpose()?
            .ok_or_else(|| Error::UnsupportedType {
                path: path.into(),
                reason: "generic reference is not declared by the containing type",
            })?,
        // Kotlin has no structural type expressions. Preserve the useful outer shape for
        // inline containers while falling back to `Any?` for their heterogeneous values.
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx object serialization does not preserve unit-struct null encoding",
                });
            }
            Fields::Unit => "Unit".to_owned(),
            Fields::Unnamed(fields)
                if kotlin.serialization == Serialization::Kotlinx
                    && (fields.fields.len() != 1 || fields.fields[0].ty.is_none()) =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx data classes do not preserve tuple-struct array encoding",
                });
            }
            Fields::Unnamed(_)
                if kotlin.serialization == Serialization::Kotlinx && kotlin.mutable_properties =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "mutable Kotlinx newtypes cannot preserve scalar encoding",
                });
            }
            Fields::Unnamed(fields) if fields.fields.len() == 1 => fields.fields[0]
                .ty
                .as_ref()
                .map(|ty| datatype(kotlin, format, types, ty, generic_scope, path))
                .transpose()?
                .unwrap_or_else(|| "Unit".to_owned()),
            Fields::Unnamed(_) if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "heterogeneous inline tuple has no inferred Kotlinx serializer",
                });
            }
            Fields::Unnamed(_) => "List<Any?>".to_owned(),
            Fields::Named(_) if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "anonymous structural type has no inferred Kotlinx serializer",
                });
            }
            Fields::Named(_) => "Map<String, Any?>".to_owned(),
        },
        DataType::Enum(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "structural union has no inferred Kotlinx serializer",
            });
        }
        DataType::Enum(enm)
            if enm
                .variants
                .iter()
                .all(|(_, variant)| is_unit_fields(&variant.fields)) =>
        {
            "String".to_owned()
        }
        DataType::Intersection(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "structural union has no inferred Kotlinx serializer",
            });
        }
        DataType::Enum(_) | DataType::Intersection(_) => "Any?".to_owned(),
    })
}

fn primitive_name(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::i8 => "Byte",
        Primitive::i16 => "Short",
        Primitive::i32 => "Int",
        Primitive::i64 | Primitive::isize => "Long",
        Primitive::i128 => "java.math.BigInteger",
        Primitive::u8 => "UByte",
        Primitive::u16 => "UShort",
        Primitive::u32 => "UInt",
        Primitive::u64 | Primitive::usize => "ULong",
        Primitive::u128 => "java.math.BigInteger",
        Primitive::f16 | Primitive::f32 => "Float",
        Primitive::f64 => "Double",
        Primitive::f128 => "java.math.BigDecimal",
        Primitive::bool => "Boolean",
        Primitive::char => "Char",
        Primitive::str => "String",
    }
}

fn map_datatype(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
) -> Result<DataType, Error> {
    let Some(format) = format else {
        return Ok(dt.clone());
    };
    format
        .map_type(types, dt)
        .map(Cow::into_owned)
        .map_err(|source| Error::format("datatype formatter failed", source))
}

fn is_unit_fields(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) => fields.fields.iter().all(|field| field.ty.is_none()),
        Fields::Named(fields) => fields.fields.iter().all(|(_, field)| field.ty.is_none()),
    }
}

fn normalized_variant_fields(variant_name: &str, fields: &Fields, serde_rewritten: bool) -> Fields {
    if !serde_rewritten {
        return fields.clone();
    }
    let payload = match fields {
        Fields::Unnamed(fields) if fields.fields.len() == 1 => fields.fields[0]
            .ty
            .as_ref()
            .map(|ty| (None, &fields.fields[0], ty)),
        Fields::Named(fields) if fields.fields.len() == 1 => {
            let (name, field) = &fields.fields[0];
            field.ty.as_ref().map(|ty| (Some(name.as_ref()), field, ty))
        }
        _ => None,
    };

    let Some((field_name, field, payload)) = payload else {
        return fields.clone();
    };
    if literal_enum_value(payload).is_some() {
        return Fields::Unit;
    }
    if field_name.is_some_and(|name| name.eq_ignore_ascii_case(variant_name)) {
        if let DataType::Struct(strct) = payload {
            return strct.fields.clone();
        }
        let mut builder = Struct::unnamed();
        builder.field_mut(field.clone());
        let DataType::Struct(strct) = builder.build() else {
            unreachable!("StructBuilder always creates a struct")
        };
        return strct.fields;
    }
    fields.clone()
}

fn literal_enum_value(dt: &DataType) -> Option<&str> {
    let DataType::Enum(enm) = dt else {
        return None;
    };
    let [(name, variant)] = enm.variants.as_slice() else {
        return None;
    };
    is_unit_fields(&variant.fields).then_some(name)
}

fn property_keyword(kotlin: &Kotlin) -> &'static str {
    if kotlin.mutable_properties {
        "var"
    } else {
        "val"
    }
}

fn generic_declaration(generics: &[String]) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    }
}

fn generic_usage(generics: &[String]) -> String {
    generic_declaration(generics)
}

fn annotation(
    out: &mut String,
    indent: &str,
    kotlin: &Kotlin,
    annotation: &str,
    argument: Option<&str>,
) {
    if kotlin.serialization != Serialization::Kotlinx {
        return;
    }
    out.push_str(indent);
    out.push('@');
    out.push_str(annotation);
    if let Some(argument) = argument {
        out.push_str("(\"");
        out.push_str(&escape_string(argument));
        out.push_str("\")");
    }
    out.push('\n');
}

fn render_kdoc(out: &mut String, indent: &str, docs: &str) {
    if docs.trim().is_empty() {
        return;
    }
    out.push_str(indent);
    out.push_str("/**\n");
    for line in docs.lines() {
        out.push_str(indent);
        out.push_str(" * ");
        out.push_str(&line.trim_start().replace("*/", "* /"));
        out.push('\n');
    }
    out.push_str(indent);
    out.push_str(" */\n");
}

fn render_deprecated(
    out: &mut String,
    indent: &str,
    deprecated: Option<&specta::datatype::Deprecated>,
) {
    let Some(deprecated) = deprecated else {
        return;
    };
    let note = deprecated
        .note
        .as_deref()
        .unwrap_or("This declaration is deprecated");
    out.push_str(indent);
    out.push_str("@Deprecated(\"");
    out.push_str(&escape_string(note));
    out.push_str("\")\n");
}

#[derive(Clone, Copy)]
enum NameKind {
    Type,
    Property,
}

fn converted_identifier(
    kotlin: &Kotlin,
    name: &str,
    kind: NameKind,
    path: &str,
) -> Result<String, Error> {
    identifier(&convert_name(kotlin.naming, name, kind), path)
}

fn named_type_identifier(
    kotlin: &Kotlin,
    ndt: &NamedDataType,
    path: &str,
) -> Result<String, Error> {
    let name = if kotlin.layout == Layout::ModulePrefixedName && !ndt.module_path.is_empty() {
        let prefix = ndt
            .module_path
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(pascal_case)
            .collect::<String>();
        format!("{prefix}{}", ndt.name)
    } else {
        ndt.name.to_string()
    };
    converted_identifier(kotlin, &name, NameKind::Type, path)
}

fn convert_name(convention: NamingConvention, name: &str, kind: NameKind) -> String {
    match convention {
        NamingConvention::Preserve => name.to_owned(),
        NamingConvention::PascalCase if matches!(kind, NameKind::Type) => pascal_case(name),
        NamingConvention::PascalCase | NamingConvention::CamelCase => camel_case(name),
        NamingConvention::SnakeCase => snake_case(name),
    }
}

fn convert_variant_name(convention: NamingConvention, name: &str) -> String {
    match convention {
        NamingConvention::Preserve | NamingConvention::PascalCase => pascal_case(name),
        NamingConvention::CamelCase => camel_case(name),
        NamingConvention::SnakeCase => snake_case(name),
    }
}

fn identifier(name: &str, path: &str) -> Result<String, Error> {
    validate_identifier(name, path)?;
    let plain = name
        .chars()
        .enumerate()
        .all(|(index, c)| c == '_' || c.is_alphanumeric() && (index != 0 || !c.is_numeric()));
    if plain && !RESERVED.contains(&name) {
        Ok(name.to_owned())
    } else {
        Ok(format!("`{name}`"))
    }
}

fn validate_identifier(name: &str, path: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.chars().all(|c| c == '_')
        || name.chars().any(|c| {
            matches!(
                c,
                '\0' | '`' | '\n' | '\r' | '.' | ';' | '/' | '\\' | '[' | ']' | ':' | '<' | '>'
            )
        })
    {
        return Err(Error::InvalidIdentifier {
            path: path.into(),
            name: name.into(),
        });
    }
    Ok(())
}

fn safe_member_name(name: &str, prefix: &str) -> String {
    if validate_identifier(name, "generated member").is_ok() {
        return name.to_owned();
    }

    // FNV-1a gives invalid wire names stable, collision-resistant source identifiers without
    // leaking filesystem separators or other forbidden characters into generated Kotlin.
    let hash = name
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{prefix}_{hash:x}")
}

fn ensure_unique(
    path: &str,
    names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    let mut seen = std::collections::BTreeSet::new();
    for name in names {
        let name = name.as_ref();
        if !seen.insert(name.to_owned()) {
            return Err(Error::DuplicateIdentifier {
                path: path.into(),
                name: name.into(),
            });
        }
    }
    Ok(())
}

fn package_name(package: &str) -> Result<String, Error> {
    package
        .split('.')
        .map(|part| identifier(part, "package"))
        .collect::<Result<Vec<_>, _>>()
        .map(|segments| segments.join("."))
}

fn pascal_case(name: &str) -> String {
    let camel = camel_case(name);
    let mut chars = camel.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase = false;
    for (index, c) in name.chars().enumerate() {
        if matches!(c, '_' | '-' | ' ') {
            uppercase = true;
        } else if uppercase {
            out.extend(c.to_uppercase());
            uppercase = false;
        } else if index == 0 {
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (index, c) in name.chars().enumerate() {
        if matches!(c, '-' | ' ') {
            if !out.ends_with('_') {
                out.push('_');
            }
        } else if c.is_uppercase() {
            if index != 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn escape_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
