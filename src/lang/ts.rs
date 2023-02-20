use thiserror::Error;

use crate::*;

/// Allows you to configure how Specta's Typescript exporter will deal with BigInt types (i64 u64 i128 u128).
#[derive(Default)]
pub enum BigIntExportBehavior {
    /// Export BigInt as a Typescript `string`
    /// WARNING: Specta takes no responsibility that the Rust number is encoded as a string.
    /// Make sure you instruct serde <https://github.com/serde-rs/json/issues/329#issuecomment-305608405> or your other serializer of this.
    String,
    /// Export BigInt as a Typescript `number`.
    /// WARNING: `JSON.parse` in JS will truncate your number resulting in data loss so ensure your deserializer supports bigint types.
    Number,
    /// Export BigInt as a Typescript `BigInt`.
    /// WARNING: Specta takes no responsibility that the Rust number is decoded into this type on the frontend.
    /// Ensure you deserializer is able to do this.
    BigInt,
    /// Abort the export with an error
    /// This is the default behavior because without integration from your serializer and deserializer we can't guarantee data loss won't occur.
    #[default]
    Fail,
    /// Same as `Self::Fail` but it allows a library to configure the message shown to the end user.
    #[doc(hidden)]
    FailWithReason(&'static str),
}

/// The signature for a function responsible for exporting Typescript comments.
pub type CommentFormatterFn = fn(&'static [&'static str]) -> String;

/// Built in formatters for exporting Rust doc comments into Typescript.
pub mod comments {
    use super::CommentFormatterFn;

    /// Export the Typescript comments as JS Doc comments. This means all JS Doc attributes will work.
    pub fn js_doc(comments: &'static [&'static str]) -> String {
        if comments.is_empty() {
            return "".to_owned();
        }

        let mut result = "/**\n".to_owned();
        for comment in comments {
            result.push_str(&format!(" * {comment}\n"));
        }
        result.push_str(" */\n");
        result
    }

    const _: CommentFormatterFn = js_doc;
}

/// allows you to control the behavior of the Typescript exporter
pub struct ExportConfiguration {
    /// control the bigint exporting behavior
    bigint: BigIntExportBehavior,
    /// control the style of exported comments
    comment_exporter: Option<CommentFormatterFn>,
    /// Configure whether or not to export types by default.
    /// This can be overridden on a type basis by using `#[specta(export)]`
    #[cfg(feature = "export")]
    pub(crate) export_by_default: Option<bool>,
}

impl ExportConfiguration {
    /// Construct a new `ExportConfiguration`
    pub fn new() -> Self {
        Default::default()
    }

    /// Configure the BigInt handling behaviour
    pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
        self.bigint = bigint;
        self
    }

    /// Configure a function which is responsible for styling the comments to be exported
    pub fn comment_style(mut self, exporter: Option<CommentFormatterFn>) -> Self {
        self.comment_exporter = exporter;
        self
    }

    /// Configure whether or not to export types by default.
    /// Note: This parameter only work if this configuration if passed into [crate::export::ts]
    #[cfg(feature = "export")]
    pub fn export_by_default(mut self, x: Option<bool>) -> Self {
        self.export_by_default = x;
        self
    }
}

impl Default for ExportConfiguration {
    fn default() -> Self {
        Self {
            bigint: Default::default(),
            comment_exporter: Some(comments::js_doc),
            #[cfg(feature = "export")]
            export_by_default: None,
        }
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum TsExportError {
    #[error("Failed to export type '{}' on field `{}`: {err}", .ty_name.unwrap_or_default(), .field_name.unwrap_or_default())]
    WithCtx {
        // TODO: Handle this better. Make `ty_name` non optional
        ty_name: Option<&'static str>,
        field_name: Option<&'static str>,
        err: Box<TsExportError>,
    },
    #[error("Your Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`")]
    BigIntForbidden,
    #[error("Cannot export anonymous type. Try wrapping the type in a tuple struct which has the `ToDataType` derive macro on it.")]
    AnonymousType, // TODO: Include metadata about the type, `IMPL_LOCATION`, `NAME`, etc
    #[error("Unable to export a tagged type which is unnamed")]
    UnableToTagUnnamedType,
    #[error("You have defined a type with the name '{0}' which is a reserved name by the Typescript exporter. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenTypeName(&'static str),
    #[error("You have defined a field '{1}' on type '{0}' which has a name that is reserved name by the Typescript exporter. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenFieldName(String, &'static str),
    #[error("Type cannot be exported: {0:?}")]
    CannotExport(DataType), // TODO: probs just have `SID` and `IMPL_LOCATION` in this error?
    #[error("Cannot export type due to an internal error. This likely is a bug in Specta itself and not your code: {0}")]
    InternalError(&'static str),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl PartialEq for TsExportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::WithCtx {
                    ty_name: l_ty_name,
                    field_name: l_field_name,
                    err: l_err,
                },
                Self::WithCtx {
                    ty_name: r_ty_name,
                    field_name: r_field_name,
                    err: r_err,
                },
            ) => l_ty_name == r_ty_name && l_field_name == r_field_name && l_err == r_err,
            (Self::ForbiddenTypeName(l0), Self::ForbiddenTypeName(r0)) => l0 == r0,
            (Self::ForbiddenFieldName(l0, l1), Self::ForbiddenFieldName(r0, r1)) => {
                l0 == r0 && l1 == r1
            }
            (Self::CannotExport(l0), Self::CannotExport(r0)) => l0 == r0,
            (Self::InternalError(l0), Self::InternalError(r0)) => l0 == r0,
            (Self::Io(l0), Self::Io(r0)) => l0.to_string() == r0.to_string(), // This is a bit hacky but it will be fine for usage in unit tests!
            (Self::Other(l0), Self::Other(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: Type>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    export_datatype(
        conf,
        &T::definition(DefOpts {
            parent_inline: true,
            type_map: &mut TypeDefs::default(),
        }),
    )
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    datatype(
        conf,
        &T::inline(
            DefOpts {
                parent_inline: true,
                type_map: &mut TypeDefs::default(),
            },
            &[],
        ),
    )
}

/// Convert a DataType to a TypeScript string with an export.
/// Eg. `export type Foo = { demo: string; };`
///
// TODO: Accept `DataTypeExt` or `DataType`. This is hard because we take it by reference
pub fn export_datatype(
    conf: &ExportConfiguration,
    typ: &DataType,
) -> Result<String, TsExportError> {
    let inline_ts = datatype(conf, &typ).map_err(|err| TsExportError::WithCtx {
        ty_name: Some("TODO"), // TODO: Fix this
        field_name: None,
        err: Box::new(err),
    })?;

    let (declaration, comments) = match &typ {
        // Named struct
        DataType::Object(CustomDataType::Named {
            name,
            comments,
            item: ObjectType {
                generics, fields, ..
            },
            ..
        }) => {
            if name.is_empty() {
                return Err(TsExportError::AnonymousType);
            } else if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == *name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            (
                match fields.len() {
                    0 => format!("type {name} = {inline_ts}"),
                    _ => {
                        let generics = match generics.len() {
                            0 => "".into(),
                            _ => format!("<{}>", generics.to_vec().join(", ")),
                        };

                        format!("type {name}{generics} = {inline_ts}")
                    }
                },
                *comments,
            )
        }
        // Enum
        DataType::Enum(CustomDataType::Named {
            name,
            comments,
            item: EnumType { generics, .. },
            ..
        }) => {
            if name.is_empty() {
                return Err(TsExportError::AnonymousType);
            } else if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == *name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            let generics = match generics.len() {
                0 => "".into(),
                _ => format!("<{}>", generics.to_vec().join(", ")),
            };

            (format!("type {name}{generics} = {inline_ts}"), *comments)
        }
        // Struct with unnamed fields
        DataType::Tuple(CustomDataType::Named {
            name,
            comments,
            item: TupleType { generics, .. },
            ..
        }) => {
            if let Some(name) = RESERVED_WORDS.iter().find(|v| *v == name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            let generics = match generics.len() {
                0 => "".into(),
                _ => format!("<{}>", generics.to_vec().join(", ")),
            };

            (format!("type {name}{generics} = {inline_ts}"), *comments)
        }
        _ => return Err(TsExportError::CannotExport(typ.clone())), // TODO: Can this be enforced at a type system level
    };

    let comments = conf
        .comment_exporter
        .map(|v| v(comments))
        .unwrap_or_default();
    Ok(format!("{comments}export {declaration}"))
}

/// Convert a DataType to a TypeScript string
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfiguration, typ: &DataType) -> Result<String, TsExportError> {
    Ok(match &typ {
        DataType::Any => "any".into(),
        primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => "number".into(),
        primitive_def!(usize isize i64 u64 i128 u128) => match conf.bigint {
            BigIntExportBehavior::String => "string".into(),
            BigIntExportBehavior::Number => "number".into(),
            BigIntExportBehavior::BigInt => "BigInt".into(),
            BigIntExportBehavior::Fail => return Err(TsExportError::BigIntForbidden),
            BigIntExportBehavior::FailWithReason(reason) => {
                return Err(TsExportError::Other(reason.to_owned()))
            }
        },
        primitive_def!(String char) => "string".into(),
        primitive_def!(bool) => "boolean".into(),
        DataType::Literal(literal) => literal.to_ts(),
        DataType::Nullable(def) => format!("{} | null", datatype(conf, def)?),
        DataType::Record(def) => {
            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key: {}]: {} }}",
                datatype(conf, &def.0)?,
                datatype(conf, &def.1)?
            )
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => format!("{}[]", datatype(conf, def)?),
        DataType::Tuple(CustomDataType::Named {
            item: TupleType { fields, .. },
            ..
        })
        | DataType::Tuple(CustomDataType::Anonymous(TupleType { fields, .. })) => match &fields[..]
        {
            [] => "null".to_string(),
            [ty] => datatype(conf, ty)?,
            tys => format!(
                "[{}]",
                tys.iter()
                    .map(|v| datatype(conf, v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ),
        },
        DataType::Object(CustomDataType::Named { name, item, .. }) => {
            object_datatype(conf, Some(name), item)?
        }
        DataType::Object(CustomDataType::Anonymous(item)) => object_datatype(conf, None, item)?,
        DataType::Enum(CustomDataType::Named { name, item, .. }) => {
            enum_datatype(conf, Some(name), item)?
        }
        DataType::Enum(CustomDataType::Anonymous(item)) => enum_datatype(conf, None, item)?,
        DataType::Reference(DataTypeReference { name, generics, .. }) => match &generics[..] {
            [] => name.to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|v| datatype(conf, v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                format!("{name}<{generics}>")
            }
        },
        DataType::Generic(GenericType(ident)) => ident.to_string(),
        DataType::Placeholder => {
            return Err(TsExportError::InternalError(
                "Attempted to export a placeholder!",
            ))
        }
    })
}

fn object_datatype(
    conf: &ExportConfiguration,
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
                    datatype(conf, &field.ty)
                        .map(|type_str| format!("({type_str})"))
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some(field.key),
                            err: Box::new(err),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut unflattened_fields = fields
                .iter()
                .filter(|f| !f.flatten)
                .map(|field| {
                    let field_name_safe = sanitise_name("TODO", field.key)?;
                    let field_ts_str = datatype(conf, &field.ty);

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

                    result
                        .map(|v| format!("{key}: {v}"))
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some(field.key),
                            err: Box::new(err),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(tag) = tag {
                unflattened_fields.push(format!(
                    "{tag}: \"{}\"",
                    name.ok_or(TsExportError::UnableToTagUnnamedType)?
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
    conf: &ExportConfiguration,
    _ty_name: Option<&'static str>,
    EnumType { repr, variants, .. }: &EnumType,
) -> Result<String, TsExportError> {
    Ok(match &variants[..] {
        [] => "never".to_string(),
        variants => variants
            .iter()
            .map(|(variant_name, variant)| {
                let sanitised_name = sanitise_name("TODO", variant_name)?;

                Ok(match (repr.clone(), variant) {
                    (EnumRepr::Internal { tag }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
                        let typ = datatype(
                            conf,
                            &DataType::Tuple(CustomDataType::Anonymous(tuple.clone())),
                        )
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some("TODO"),
                            err: Box::new(err),
                        })?;

                        format!("({{ {tag}: \"{sanitised_name}\" }} & {typ})")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
                        let mut fields = vec![format!("{tag}: \"{sanitised_name}\"")];

                        fields.extend(
                            obj.fields
                                .iter()
                                .map(|v| object_field_to_ts(conf, "TODO", v))
                                .collect::<Result<Vec<_>, _>>()?,
                        );

                        format!("{{ {} }}", fields.join("; "))
                    }
                    (EnumRepr::External, EnumVariant::Unit) => {
                        format!("\"{sanitised_name}\"")
                    }

                    (EnumRepr::External, v) => {
                        let ts_values = datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"), // TODO
                                err: Box::new(err),
                            }
                        })?;

                        format!("{{ {sanitised_name}: {ts_values} }}")
                    }
                    (EnumRepr::Untagged, EnumVariant::Unit) => "null".to_string(),
                    (EnumRepr::Untagged, v) => {
                        datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"), // TODO
                                err: Box::new(err),
                            }
                        })?
                    }
                    (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Adjacent { tag, content }, v) => {
                        let ts_values = datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"),
                                err: Box::new(err),
                            }
                        })?;

                        format!("{{ {tag}: \"{sanitised_name}\"; {content}: {ts_values} }}")
                    }
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
pub fn object_field_to_ts(
    conf: &ExportConfiguration,
    typ_name: &str,
    field: &ObjectField,
) -> Result<String, TsExportError> {
    let field_name_safe = sanitise_name(typ_name, field.key)?;

    let (key, ty) = match field.optional {
        true => (
            format!("{field_name_safe}?"),
            match &field.ty {
                DataType::Nullable(ty) => &ty,
                ty => &field.ty,
            },
        ),
        false => (field_name_safe, &field.ty),
    };

    Ok(format!("{key}: {}", datatype(conf, &ty)?))
}

/// sanitise a string to be a valid Typescript key
pub fn sanitise_name(type_name: &str, field_name: &str) -> Result<String, TsExportError> {
    if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == field_name) {
        return Err(TsExportError::ForbiddenFieldName(
            type_name.to_owned(),
            name,
        ));
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
