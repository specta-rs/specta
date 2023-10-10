use std::borrow::Cow;

pub mod comments;
mod context;
mod error;
mod export_config;
mod formatter;
pub(crate) mod js_doc;
mod reserved_terms;

pub use context::*;
pub use error::*;
pub use export_config::*;
pub use formatter::*;
use reserved_terms::*;

use crate::{
    internal::{skip_fields, skip_fields_named, NonSkipField},
    *,
};

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, ExportError>;

pub(crate) type Output = Result<String>;

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export_ref<T: NamedType>(_: &T, conf: &ExportConfig) -> Output {
    export::<T>(conf)
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: NamedType>(conf: &ExportConfig) -> Output {
    let mut type_map = TypeMap::default();
    let named_data_type = T::definition_named_data_type(DefOpts {
        parent_inline: false,
        type_map: &mut type_map,
    });
    is_valid_ty(&named_data_type.inner, &type_map)?;
    let result = export_named_datatype(conf, &named_data_type, &type_map);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline_ref<T: Type>(_: &T, conf: &ExportConfig) -> Output {
    inline::<T>(conf)
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &ExportConfig) -> Output {
    let mut type_map = TypeMap::default();
    let ty = T::inline(
        DefOpts {
            parent_inline: false,
            type_map: &mut type_map,
        },
        &[],
    );
    is_valid_ty(&ty, &type_map)?;
    let result = datatype(conf, &ty, &type_map);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `export Name = { demo: string; }`
pub fn export_named_datatype(
    conf: &ExportConfig,
    typ: &NamedDataType,
    type_map: &TypeMap,
) -> Output {
    // TODO: Duplicate type name detection?

    is_valid_ty(&typ.inner, type_map)?;
    export_datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: true,
        },
        typ,
        type_map,
    )
}

fn inner_comments(
    ctx: ExportContext,
    deprecated: Option<&DeprecatedType>,
    docs: &Cow<'static, str>,
    other: String,
    start_with_newline: bool,
) -> String {
    if !ctx.is_export {
        return other;
    }

    let comments = ctx
        .cfg
        .comment_exporter
        .map(|v| v(CommentFormatterArgs { docs, deprecated }))
        .unwrap_or_default();

    let prefix = match start_with_newline && !comments.is_empty() {
        true => "\n",
        false => "",
    };

    format!("{prefix}{comments}{other}")
}

fn export_datatype_inner(
    ctx: ExportContext,
    typ @ NamedDataType {
        name,
        docs,
        deprecated,
        inner: item,
        ..
    }: &NamedDataType,
    type_map: &TypeMap,
) -> Output {
    let ctx = ctx.with(PathItem::Type(name.clone()));
    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let generics = item
        .generics()
        .filter(|generics| !generics.is_empty())
        .map(|generics| format!("<{}>", generics.join(", ")))
        .unwrap_or_default();

    let inline_ts = datatype_inner(ctx.clone(), &typ.inner, type_map)?;

    Ok(inner_comments(
        ctx,
        deprecated.as_ref(),
        docs,
        format!("export type {name}{generics} = {inline_ts}"),
        false,
    ))
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfig, typ: &DataType, type_map: &TypeMap) -> Output {
    // TODO: Duplicate type name detection?

    datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: false,
        },
        typ,
        type_map,
    )
}

pub(crate) fn datatype_inner(ctx: ExportContext, typ: &DataType, type_map: &TypeMap) -> Output {
    Ok(match &typ {
        DataType::Any => ANY.into(),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
            match p {
                primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => NUMBER.into(),
                primitive_def!(usize isize i64 u64 i128 u128) => match ctx.cfg.bigint {
                    BigIntExportBehavior::String => STRING.into(),
                    BigIntExportBehavior::Number => NUMBER.into(),
                    BigIntExportBehavior::BigInt => BIGINT.into(),
                    BigIntExportBehavior::Fail => {
                        return Err(ExportError::BigIntForbidden(ctx.export_path()))
                    }
                    BigIntExportBehavior::FailWithReason(reason) => {
                        return Err(ExportError::Other(ctx.export_path(), reason.to_owned()))
                    }
                },
                primitive_def!(String char) => STRING.into(),
                primitive_def!(bool) => BOOLEAN.into(),
            }
        }
        DataType::Literal(literal) => literal.to_ts(),
        DataType::Nullable(def) => {
            let dt = datatype_inner(ctx, def, type_map)?;

            if dt.ends_with(&format!(" | {NULL}")) {
                dt
            } else {
                format!("{dt} | {NULL}")
            }
        }
        DataType::Map(def) => {
            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key in {}]: {} }}",
                datatype_inner(ctx.clone(), &def.0, type_map)?,
                datatype_inner(ctx, &def.1, type_map)?
            )
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => {
            let dt = datatype_inner(ctx, def, type_map)?;
            if dt.contains(' ') && !dt.ends_with('}') {
                format!("({dt})[]")
            } else {
                format!("{dt}[]")
            }
        }
        DataType::Struct(item) => struct_datatype(
            ctx.with(PathItem::Type(item.name().clone())),
            item.name(),
            item,
            type_map,
        )?,
        DataType::Enum(item) => enum_datatype(
            ctx.with(PathItem::Variant(item.name.clone())),
            item,
            type_map,
        )?,
        DataType::Tuple(tuple) => tuple_datatype(ctx, tuple, type_map)?,
        DataType::Result(result) => {
            let mut variants = vec![
                datatype_inner(ctx.clone(), &result.0, type_map)?,
                datatype_inner(ctx, &result.1, type_map)?,
            ];
            variants.dedup();
            variants.join(" | ")
        }
        DataType::Reference(DataTypeReference { name, generics, .. }) => match &generics[..] {
            [] => name.to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|v| datatype_inner(ctx.with(PathItem::Type(name.clone())), v, type_map))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");

                format!("{name}<{generics}>")
            }
        },
        DataType::Generic(GenericType(ident)) => ident.to_string(),
    })
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    type_map: &TypeMap,
) -> Output {
    match fields {
        [(field, ty)] => Ok(inner_comments(
            ctx.clone(),
            field.deprecated(),
            field.docs(),
            datatype_inner(ctx, ty, type_map)?,
            true,
        )),
        fields => Ok(format!(
            "[{}]",
            fields
                .iter()
                .map(|(field, ty)| Ok(inner_comments(
                    ctx.clone(),
                    field.deprecated(),
                    field.docs(),
                    datatype_inner(ctx.clone(), ty, type_map)?,
                    true
                )))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn tuple_datatype(ctx: ExportContext, tuple: &TupleType, type_map: &TypeMap) -> Output {
    match &tuple.elements[..] {
        [] => Ok(NULL.to_string()),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| datatype_inner(ctx.clone(), v, type_map))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn struct_datatype(ctx: ExportContext, key: &str, s: &StructType, type_map: &TypeMap) -> Output {
    match &s.fields {
        StructFields::Unit => Ok(NULL.into()),
        StructFields::Unnamed(s) => {
            unnamed_fields_datatype(ctx, &skip_fields(s.fields()).collect::<Vec<_>>(), type_map)
        }
        StructFields::Named(s) => {
            let fields = skip_fields_named(s.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                return Ok(format!("Record<{STRING}, {NEVER}>"));
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) =
                fields.iter().partition(|(_, (f, _))| f.flatten);

            let mut field_sections = flattened
                .into_iter()
                .map(|(key, (field, ty))| {
                    datatype_inner(ctx.with(PathItem::Field(key.clone())), ty, type_map).map(
                        |type_str| {
                            inner_comments(
                                ctx.clone(),
                                field.deprecated(),
                                field.docs(),
                                format!("({type_str})"),
                                true,
                            )
                        },
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            let mut unflattened_fields = non_flattened
                .into_iter()
                .map(|(key, field_ref)| {
                    let (field, _) = field_ref;

                    Ok(inner_comments(
                        ctx.clone(),
                        field.deprecated(),
                        field.docs(),
                        object_field_to_ts(
                            ctx.with(PathItem::Field(key.clone())),
                            key.clone(),
                            field_ref,
                            type_map,
                        )?,
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            if let Some(tag) = &s.tag {
                unflattened_fields.push(format!("{tag}: \"{key}\""));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            Ok(field_sections.join(" & "))
        }
    }
}

fn enum_variant_datatype(
    ctx: ExportContext,
    type_map: &TypeMap,
    name: Cow<'static, str>,
    variant: &EnumVariant,
) -> Result<Option<String>> {
    match &variant.inner {
        // TODO: Remove unreachable in type system
        EnumVariants::Unit => unreachable!("Unit enum variants have no type!"),
        EnumVariants::Named(obj) => {
            let mut fields = if let Some(tag) = &obj.tag {
                let sanitised_name = sanitise_key(name, true);
                vec![format!("{tag}: {sanitised_name}")]
            } else {
                vec![]
            };

            fields.extend(
                skip_fields_named(obj.fields())
                    .map(|(name, field_ref)| {
                        let (field, _) = field_ref;

                        Ok(inner_comments(
                            ctx.clone(),
                            field.deprecated(),
                            field.docs(),
                            object_field_to_ts(
                                ctx.with(PathItem::Field(name.clone())),
                                name.clone(),
                                field_ref,
                                type_map,
                            )?,
                            true,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            Ok(Some(match &fields[..] {
                [] => format!("Record<{STRING}, {NEVER}>").to_string(),
                fields => format!("{{ {} }}", fields.join("; ")),
            }))
        }
        EnumVariants::Unnamed(obj) => {
            let fields = skip_fields(obj.fields())
                .map(|(_, ty)| datatype_inner(ctx.clone(), ty, type_map))
                .collect::<Result<Vec<_>>>()?;

            Ok(match &fields[..] {
                [] => {
                    // If the actual length is 0, we know `#[serde(skip)]` was not used.
                    if obj.fields.len() == 0 {
                        Some("[]".to_string())
                    } else {
                        // We wanna render `{tag}` not `{tag}: {type}` (where `{type}` is what this function returns)
                        None
                    }
                }
                // If the actual length is 1, we know `#[serde(skip)]` was not used.
                [field] if obj.fields.len() == 1 => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

fn enum_datatype(ctx: ExportContext, e: &EnumType, type_map: &TypeMap) -> Output {
    if e.variants().is_empty() {
        return Ok(NEVER.to_string());
    }

    Ok(match &e.repr {
        EnumRepr::Untagged => {
            let mut variants = e
                .variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .map(|(name, variant)| {
                    Ok(match variant.inner {
                        EnumVariants::Unit => NULL.to_string(),
                        _ => inner_comments(
                            ctx.clone(),
                            variant.deprecated(),
                            variant.docs(),
                            enum_variant_datatype(
                                ctx.with(PathItem::Variant(name.clone())),
                                type_map,
                                name.clone(),
                                variant,
                            )?
                            .expect("Invalid Serde type"),
                            true,
                        ),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            variants.join(" | ")
        }
        repr => {
            let mut variants = e
                .variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .map(|(variant_name, variant)| {
                    let sanitised_name = sanitise_key(variant_name.clone(), true);

                    Ok(inner_comments(
                        ctx.clone(),
                        variant.deprecated(),
                        variant.docs(),
                        match (repr, &variant.inner) {
                            (EnumRepr::Untagged, _) => unreachable!(),
                            (EnumRepr::Internal { tag }, EnumVariants::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Internal { tag }, EnumVariants::Unnamed(tuple)) => {
                                let mut typ = unnamed_fields_datatype(
                                    ctx.clone(),
                                    &skip_fields(tuple.fields()).collect::<Vec<_>>(),
                                    type_map,
                                )?;

                                // TODO: This `null` check is a bad fix for an internally tagged type with a `null` variant being exported as `{ type: "A" } & null` (which is `never` in TS)
                                // TODO: Move this check into the macros so it can apply to any language cause it should (it's just hard to do in the macros)
                                if typ == "null" {
                                    format!("({{ {tag}: {sanitised_name} }})")
                                } else {
                                    // We wanna be sure `... & ... | ...` becomes `... & (... | ...)`
                                    if typ.contains('|') {
                                        typ = format!("({typ})");
                                    }
                                    format!("({{ {tag}: {sanitised_name} }} & {typ})")
                                }
                            }
                            (EnumRepr::Internal { tag }, EnumVariants::Named(obj)) => {
                                let mut fields = vec![format!("{tag}: {sanitised_name}")];

                                fields.extend(
                                    skip_fields_named(obj.fields())
                                        .map(|(name, field)| {
                                            object_field_to_ts(
                                                ctx.with(PathItem::Field(name.clone())),
                                                name.clone(),
                                                field,
                                                type_map,
                                            )
                                        })
                                        .collect::<Result<Vec<_>>>()?,
                                );

                                format!("{{ {} }}", fields.join("; "))
                            }
                            (EnumRepr::External, EnumVariants::Unit) => sanitised_name.to_string(),
                            (EnumRepr::External, _) => {
                                let ts_values = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    type_map,
                                    variant_name.clone(),
                                    variant,
                                )?;
                                let sanitised_name = sanitise_key(variant_name.clone(), false);

                                match ts_values {
                                    Some(ts_values) => {
                                        format!("{{ {sanitised_name}: {ts_values} }}")
                                    }
                                    None => format!(r#""{sanitised_name}""#),
                                }
                            }
                            (EnumRepr::Adjacent { tag, .. }, EnumVariants::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Adjacent { tag, content }, _) => {
                                let ts_values = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    type_map,
                                    variant_name.clone(),
                                    variant,
                                )?
                                .expect("Invalid Serde type");

                                format!("{{ {tag}: {sanitised_name}; {content}: {ts_values} }}")
                            }
                        },
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            variants.join(" | ")
        }
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
            Self::char(v) => format!(r#""{v}""#),
            Self::None => NULL.to_string(),
        }
    }
}

/// convert an object field into a Typescript string
fn object_field_to_ts(
    ctx: ExportContext,
    key: Cow<'static, str>,
    (field, ty): NonSkipField,
    type_map: &TypeMap,
) -> Output {
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional {
        true => (format!("{field_name_safe}?").into(), ty),
        false => (field_name_safe, ty),
    };

    Ok(format!("{key}: {}", datatype_inner(ctx, ty, type_map)?))
}

/// sanitise a string to be a valid Typescript key
fn sanitise_key<'a>(field_name: Cow<'static, str>, force_string: bool) -> Cow<'a, str> {
    let valid = field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && field_name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    if force_string || !valid {
        format!(r#""{field_name}""#).into()
    } else {
        field_name
    }
}

pub(crate) fn sanitise_type_name(ctx: ExportContext, loc: NamedLocation, ident: &str) -> Output {
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(ExportError::ForbiddenName(loc, ctx.export_path(), name));
    }

    if let Some(first_char) = ident.chars().nth(0) {
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(ExportError::InvalidName(
                loc,
                ctx.export_path(),
                ident.to_string(),
            ));
        }
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        return Err(ExportError::InvalidName(
            loc,
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    Ok(ident.to_string())
}

const ANY: &str = "any";
const NUMBER: &str = "number";
const STRING: &str = "string";
const BOOLEAN: &str = "boolean";
const NULL: &str = "null";
const NEVER: &str = "never";
const BIGINT: &str = "bigint";
