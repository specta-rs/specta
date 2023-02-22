mod comments;
mod context;
mod error;
mod export_config;

pub use comments::*;
pub use context::*;
pub use error::*;
pub use export_config::*;

use crate::*;

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: NamedType>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    let mut type_name = TypeDefs::default();
    let result = export_datatype(
        conf,
        &T::definition_named_data_type(DefOpts {
            parent_inline: false,
            type_map: &mut type_name,
        })?,
    );

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_name).into_iter().next() {
        return Err(TsExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    let mut type_name = TypeDefs::default();
    let result = datatype(
        conf,
        &T::inline(
            DefOpts {
                parent_inline: false,
                type_map: &mut type_name,
            },
            &[],
        )?,
    );

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_name).into_iter().next() {
        return Err(TsExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `export Name = { demo: string; }`
pub fn export_datatype(
    conf: &ExportConfiguration,
    typ: &NamedDataType,
) -> Result<String, TsExportError> {
    // TODO: Duplicate type name detection?

    export_datatype_inner(ExportContext { conf, path: vec![] }, typ)
}

fn export_datatype_inner(
    ctx: ExportContext,
    NamedDataType {
        name,
        comments,
        item,
        ..
    }: &NamedDataType,
) -> Result<String, TsExportError> {
    let ctx = ctx.with(PathItem::Type(name));
    if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == *name) {
        return Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ctx.export_path(),
            name,
        ));
    }

    let inline_ts = datatype_inner(
        ctx.clone(),
        &match item {
            NamedDataTypeItem::Object(obj) => DataType::Object(obj.clone()),
            NamedDataTypeItem::Tuple(tuple) => DataType::Tuple(tuple.clone()),
            NamedDataTypeItem::Enum(enum_) => DataType::Enum(enum_.clone()),
        },
    )?;

    let generics = match item {
        // Named struct
        NamedDataTypeItem::Object(ObjectType {
            generics, fields, ..
        }) => match fields.len() {
            0 => Some(generics),
            _ => (!generics.is_empty()).then_some(generics),
        },
        // Enum
        NamedDataTypeItem::Enum(e) => {
            let generics = e.generics();
            (!generics.is_empty()).then_some(generics)
        }
        // Struct with unnamed fields
        NamedDataTypeItem::Tuple(TupleType { generics, .. }) => {
            (!generics.is_empty()).then_some(generics)
        }
    };

    let generics = generics
        .map(|generics| format!("<{}>", generics.to_vec().join(", ")))
        .unwrap_or_default();

    let comments = ctx
        .conf
        .comment_exporter
        .map(|v| v(comments))
        .unwrap_or_default();
    Ok(format!(
        "{comments}export type {name}{generics} = {inline_ts}"
    ))
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfiguration, typ: &DataType) -> Result<String, TsExportError> {
    // TODO: Duplicate type name detection?

    datatype_inner(ExportContext { conf, path: vec![] }, typ)
}

fn datatype_inner(ctx: ExportContext, typ: &DataType) -> Result<String, TsExportError> {
    Ok(match &typ {
        DataType::Any => "any".into(),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str()));
            match p {
                primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => "number".into(),
                primitive_def!(usize isize i64 u64 i128 u128) => match ctx.conf.bigint {
                    BigIntExportBehavior::String => "string".into(),
                    BigIntExportBehavior::Number => "number".into(),
                    BigIntExportBehavior::BigInt => "BigInt".into(),
                    BigIntExportBehavior::Fail => {
                        return Err(TsExportError::BigIntForbidden(ctx.export_path()))
                    }
                    BigIntExportBehavior::FailWithReason(reason) => {
                        return Err(TsExportError::Other(ctx.export_path(), reason.to_owned()))
                    }
                },
                primitive_def!(String char) => "string".into(),
                primitive_def!(bool) => "boolean".into(),
            }
        }
        DataType::Literal(literal) => literal.to_ts(),
        DataType::Nullable(def) => format!("{} | null", datatype_inner(ctx, def)?),
        DataType::Record(def) => {
            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key: {}]: {} }}",
                datatype_inner(ctx.clone(), &def.0)?,
                datatype_inner(ctx, &def.1)?
            )
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => format!("{}[]", datatype_inner(ctx, def)?),
        DataType::Named(NamedDataType {
            name,
            item: NamedDataTypeItem::Tuple(TupleType { fields, .. }),
            ..
        }) => tuple_datatype(ctx.with(PathItem::Type(name)), fields)?,
        DataType::Tuple(TupleType { fields, .. }) => tuple_datatype(ctx, fields)?,
        DataType::Named(NamedDataType {
            name,
            item: NamedDataTypeItem::Object(item),
            ..
        }) => object_datatype(ctx.with(PathItem::Type(name)), Some(name), item)?,
        DataType::Object(item) => object_datatype(ctx, None, item)?,
        DataType::Named(NamedDataType {
            name,
            item: NamedDataTypeItem::Enum(item),
            ..
        }) => enum_datatype(ctx.with(PathItem::Type(name)), Some(name), item)?,
        DataType::Enum(item) => enum_datatype(ctx, None, item)?,
        DataType::Reference(DataTypeReference { name, generics, .. }) => match &generics[..] {
            [] => name.to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|v| datatype_inner(ctx.with(PathItem::Type(name)), v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                format!("{name}<{generics}>")
            }
        },
        DataType::Generic(GenericType(ident)) => ident.to_string(),
    })
}

fn tuple_datatype(ctx: ExportContext, fields: &[DataType]) -> Result<String, TsExportError> {
    match fields {
        [] => Ok("null".to_string()),
        [ty] => datatype_inner(ctx, ty),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| datatype_inner(ctx.clone(), v))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
    }
}

fn object_datatype(
    ctx: ExportContext,
    name: Option<&'static str>,
    ObjectType { fields, tag, .. }: &ObjectType,
) -> Result<String, TsExportError> {
    match &fields[..] {
        [] => Ok("null".to_string()),
        fields => {
            let mut field_sections = fields
                .iter()
                .filter(|f| f.flatten)
                .map(|field| {
                    datatype_inner(ctx.with(PathItem::Field(field.key)), &field.ty)
                        .map(|type_str| format!("({type_str})"))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut unflattened_fields = fields
                .iter()
                .filter(|f| !f.flatten)
                .map(|field| {
                    let ctx = ctx.with(PathItem::Field(field.key));
                    let field_name_safe =
                        sanitise_name(ctx.clone(), NamedLocation::Field, field.key)?;
                    let field_ts_str = datatype_inner(ctx, &field.ty);

                    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
                    let (key, result) = match field.optional {
                        true => (
                            format!("{field_name_safe}?"),
                            match &field.ty {
                                DataType::Nullable(_) => field_ts_str,
                                _ => field_ts_str.map(|v| format!("{v} | null")),
                            },
                        ),
                        false => (field_name_safe, field_ts_str),
                    };

                    result.map(|v| format!("{key}: {v}"))
                })
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(tag) = tag {
                unflattened_fields.push(format!(
                    "{tag}: \"{}\"",
                    name.ok_or_else(|| TsExportError::UnableToTagUnnamedType(ctx.export_path()))?
                ));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            Ok(field_sections.join(" & "))
        }
    }
}

fn enum_datatype(
    ctx: ExportContext,
    _ty_name: Option<&'static str>,
    e: &EnumType,
) -> Result<String, TsExportError> {
    if e.variants_len() == 0 {
        return Ok("never".to_string());
    }

    Ok(match e {
        EnumType::Tagged { variants, repr, .. } => variants
            .iter()
            .map(|(variant_name, variant)| {
                let ctx = ctx.with(PathItem::Variant(variant_name));
                let sanitised_name =
                    sanitise_name(ctx.clone(), NamedLocation::Variant, variant_name)?;

                Ok(match (repr, variant) {
                    (EnumRepr::Internal { tag }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
                        let typ = datatype_inner(ctx, &DataType::Tuple(tuple.clone()))?;

                        format!("({{ {tag}: \"{sanitised_name}\" }} & {typ})")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
                        let mut fields = vec![format!("{tag}: \"{sanitised_name}\"")];

                        fields.extend(
                            obj.fields
                                .iter()
                                .map(|v| object_field_to_ts(ctx.with(PathItem::Field(v.key)), v))
                                .collect::<Result<Vec<_>, _>>()?,
                        );

                        format!("{{ {} }}", fields.join("; "))
                    }
                    (EnumRepr::External, EnumVariant::Unit) => {
                        format!("\"{sanitised_name}\"")
                    }

                    (EnumRepr::External, v) => {
                        let ts_values = datatype_inner(ctx, &v.data_type())?;

                        format!("{{ {sanitised_name}: {ts_values} }}")
                    }
                    (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Adjacent { tag, content }, v) => {
                        let ts_values = datatype_inner(ctx, &v.data_type())?;

                        format!("{{ {tag}: \"{sanitised_name}\"; {content}: {ts_values} }}")
                    }
                })
            })
            .collect::<Result<Vec<_>, TsExportError>>()?
            .join(" | "),
        EnumType::Untagged { variants, .. } => variants
            .iter()
            .map(|variant| {
                Ok(match variant {
                    EnumVariant::Unit => "null".to_string(),
                    v => datatype_inner(ctx.clone(), &v.data_type())?,
                })
            })
            .collect::<Result<Vec<_>, TsExportError>>()?
            .join(" | "),
    })
}

impl LiteralType {
    fn to_ts(&self) -> String {
        match self {
            Self::i8(v) => v.to_string(),
            Self::i16(v) => v.to_string(),
            Self::i32(v) => v.to_string(),
            Self::u8(v) => v.to_string(),
            Self::u16(v) => v.to_string(),
            Self::u32(v) => v.to_string(),
            Self::f32(v) => v.to_string(),
            Self::f64(v) => v.to_string(),
            Self::bool(v) => v.to_string(),
            Self::String(v) => format!(r#""{v}""#),
            Self::None => "null".to_string(),
        }
    }
}

/// convert an object field into a Typescript string
fn object_field_to_ts(ctx: ExportContext, field: &ObjectField) -> Result<String, TsExportError> {
    let field_name_safe = sanitise_name(ctx.clone(), NamedLocation::Field, field.key)?;

    let (key, ty) = match field.optional {
        true => (
            format!("{field_name_safe}?"),
            match &field.ty {
                DataType::Nullable(ty) => ty,
                ty => ty,
            },
        ),
        false => (field_name_safe, &field.ty),
    };

    Ok(format!("{key}: {}", datatype_inner(ctx, ty)?))
}

/// sanitise a string to be a valid Typescript key
fn sanitise_name(
    ctx: ExportContext,
    loc: NamedLocation,
    field_name: &str,
) -> Result<String, TsExportError> {
    if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == field_name) {
        return Err(TsExportError::ForbiddenName(loc, ctx.export_path(), name));
    }

    let valid = field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && field_name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    Ok(if !valid {
        format!(r#""{field_name}""#)
    } else {
        field_name.to_string()
    })
}

// Taken from: https://github.com/microsoft/TypeScript/issues/2536#issuecomment-87194347
const RESERVED_WORDS: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "as",
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
    "any",
    "boolean",
    "constructor",
    "declare",
    "get",
    "module",
    "require",
    "number",
    "set",
    "string",
    "symbol",
    "type",
    "from",
    "of",
    "namespace",
    "async",
    "await",
];
