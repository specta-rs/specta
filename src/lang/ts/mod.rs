mod comments;
mod context;
mod error;
mod export_config;
mod formatter;
mod reserved_terms;

use std::borrow::Cow;

pub use comments::*;
pub use context::*;
pub use error::*;
pub use export_config::*;
pub use formatter::*;
use reserved_terms::*;

use crate::*;

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, TsExportError>;
type Output = Result<String>;

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
    })?;
    let result = export_named_datatype(conf, &named_data_type, &type_map);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(TsExportError::DuplicateTypeName(ty_name, l0, l1));
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
    let result = datatype(
        conf,
        &T::inline(
            DefOpts {
                parent_inline: false,
                type_map: &mut type_map,
            },
            &[],
        )?,
        &type_map,
    );

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(TsExportError::DuplicateTypeName(ty_name, l0, l1));
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

    export_datatype_inner(ExportContext { conf, path: vec![] }, typ, type_map)
}

fn export_datatype_inner(
    ctx: ExportContext,
    typ @ NamedDataType {
        name,
        comments,
        item,
        ..
    }: &NamedDataType,
    type_map: &TypeMap,
) -> Output {
    let ctx = ctx.with(PathItem::Type(name.clone()));
    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let inline_ts = named_datatype_inner(ctx.clone(), typ, type_map)?;

    let generics = Some(item.generics())
        .filter(|generics| !generics.is_empty())
        .map(|generics| format!("<{}>", generics.join(", ")))
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

/// Convert a NamedDataType to a TypeScript string
///
/// Eg. `{ scalar_field: number, generc_field: T }`
pub fn named_datatype(conf: &ExportConfig, typ: &NamedDataType, type_map: &TypeMap) -> Output {
    named_datatype_inner(
        ExportContext {
            conf,
            path: vec![PathItem::Type(typ.name.clone())],
        },
        typ,
        type_map,
    )
}

fn named_datatype_inner(ctx: ExportContext, typ: &NamedDataType, type_map: &TypeMap) -> Output {
    let name = Some(&typ.name);

    match &typ.item {
        NamedDataTypeItem::Object(o) => object_datatype(ctx, name, o, type_map),
        NamedDataTypeItem::Enum(e) => enum_datatype(ctx, name, e, type_map),
        NamedDataTypeItem::Tuple(t) => tuple_datatype(ctx, t, type_map),
    }
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfig, typ: &DataType, type_map: &TypeMap) -> Output {
    // TODO: Duplicate type name detection?

    datatype_inner(ExportContext { conf, path: vec![] }, typ, type_map)
}

fn datatype_inner(ctx: ExportContext, typ: &DataType, type_map: &TypeMap) -> Output {
    Ok(match &typ {
        DataType::Any => "any".into(),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
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
        DataType::Nullable(def) => {
            let dt = datatype_inner(ctx, def, type_map)?;

            if dt.ends_with(" | null") {
                dt
            } else {
                format!("{dt} | null")
            }
        }
        DataType::Map(def) => {
            let is_enum = match &def.0 {
                DataType::Enum(_) => true,
                DataType::Named(dt) => matches!(dt.item, NamedDataTypeItem::Enum(_)),
                DataType::Reference(r) => {
                    let typ = type_map
                        .get(&r.sid)
                        .unwrap_or_else(|| panic!("Type {} not found!", r.name))
                        .as_ref()
                        .unwrap_or_else(|| panic!("Type {} has no value!", r.name));

                    matches!(typ.item, NamedDataTypeItem::Enum(_))
                }
                _ => false,
            };

            let divider = if is_enum { " in" } else { ":" };

            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key{divider} {}]: {} }}",
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
        DataType::Struct(item) => object_datatype(ctx, None, item, type_map)?,
        DataType::Enum(item) => enum_datatype(ctx, None, item, type_map)?,
        DataType::Tuple(tuple) => tuple_datatype(ctx, tuple, type_map)?,
        DataType::Named(typ) => {
            named_datatype_inner(ctx.with(PathItem::Type(typ.name.clone())), typ, type_map)?
        }
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

fn tuple_datatype(ctx: ExportContext, tuple: &TupleType, type_map: &TypeMap) -> Output {
    match tuple {
        TupleType::Unnamed => Ok("[]".to_string()),
        TupleType::Named { fields, .. } => match &fields[..] {
            [] => Ok("null".to_string()),
            [ty] => datatype_inner(ctx, ty, type_map),
            tys => Ok(format!(
                "[{}]",
                tys.iter()
                    .map(|v| datatype_inner(ctx.clone(), v, type_map))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            )),
        },
    }
}

fn object_datatype(
    ctx: ExportContext,
    name: Option<&Cow<'static, str>>,
    ObjectType { fields, tag, .. }: &ObjectType,
    type_map: &TypeMap,
) -> Output {
    match &fields[..] {
        [] => Ok("Record<string, never>".to_string()),
        fields => {
            let mut field_sections = fields
                .iter()
                .filter(|f| f.flatten)
                .map(|field| {
                    datatype_inner(
                        ctx.with(PathItem::Field(field.key.clone())),
                        &field.ty,
                        type_map,
                    )
                    .map(|type_str| format!("({type_str})"))
                })
                .collect::<Result<Vec<_>>>()?;

            let mut unflattened_fields = fields
                .iter()
                .filter(|f| !f.flatten)
                .map(|f| object_field_to_ts(ctx.with(PathItem::Field(f.key.clone())), f, type_map))
                .collect::<Result<Vec<_>>>()?;

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
    _ty_name: Option<&Cow<'static, str>>,
    e: &EnumType,
    type_map: &TypeMap,
) -> Output {
    if e.variants_len() == 0 {
        return Ok("never".to_string());
    }

    Ok(match e {
        EnumType::Tagged { variants, repr, .. } => {
            let mut variants = variants
                .iter()
                .map(|(variant_name, variant)| {
                    let ctx = ctx.with(PathItem::Variant(variant_name.clone()));
                    let sanitised_name = sanitise_key(variant_name, true);

                    Ok(match (repr, variant) {
                        (EnumRepr::Internal { tag }, EnumVariant::Unit) => {
                            format!("{{ {tag}: {sanitised_name} }}")
                        }
                        (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
                            let typ =
                                datatype_inner(ctx, &DataType::Tuple(tuple.clone()), type_map)?;
                            format!("({{ {tag}: {sanitised_name} }} & {typ})")
                        }
                        (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
                            let mut fields = vec![format!("{tag}: {sanitised_name}")];

                            fields.extend(
                                obj.fields
                                    .iter()
                                    .map(|v| {
                                        object_field_to_ts(
                                            ctx.with(PathItem::Field(v.key.clone())),
                                            v,
                                            type_map,
                                        )
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                            );

                            format!("{{ {} }}", fields.join("; "))
                        }
                        (EnumRepr::External, EnumVariant::Unit) => sanitised_name.to_string(),

                        (EnumRepr::External, v) => {
                            let ts_values = datatype_inner(ctx.clone(), &v.data_type(), type_map)?;
                            let sanitised_name = sanitise_key(variant_name, false);

                            format!("{{ {sanitised_name}: {ts_values} }}")
                        }
                        (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit) => {
                            format!("{{ {tag}: {sanitised_name} }}")
                        }
                        (EnumRepr::Adjacent { tag, content }, v) => {
                            let ts_values = datatype_inner(ctx, &v.data_type(), type_map)?;

                            format!("{{ {tag}: {sanitised_name}; {content}: {ts_values} }}")
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            variants.join(" | ")
        }
        EnumType::Untagged { variants, .. } => {
            let mut variants = variants
                .iter()
                .map(|variant| {
                    Ok(match variant {
                        EnumVariant::Unit => "null".to_string(),
                        v => datatype_inner(ctx.clone(), &v.data_type(), type_map)?,
                    })
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
            Self::None => "null".to_string(),
        }
    }
}

/// convert an object field into a Typescript string
fn object_field_to_ts(ctx: ExportContext, field: &ObjectField, type_map: &TypeMap) -> Output {
    let field_name_safe = sanitise_key(&field.key, false);

    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional {
        true => (format!("{field_name_safe}?"), &field.ty),
        false => (field_name_safe, &field.ty),
    };

    Ok(format!("{key}: {}", datatype_inner(ctx, ty, type_map)?))
}

/// sanitise a string to be a valid Typescript key
fn sanitise_key(field_name: &str, force_string: bool) -> String {
    let valid = field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && field_name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    if force_string || !valid {
        format!(r#""{field_name}""#)
    } else {
        field_name.to_string()
    }
}

fn sanitise_type_name(ctx: ExportContext, loc: NamedLocation, ident: &str) -> Output {
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(TsExportError::ForbiddenName(loc, ctx.export_path(), name));
    }

    Ok(ident.to_string())
}
