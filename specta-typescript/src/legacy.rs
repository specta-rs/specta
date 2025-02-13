// TODO: Drop this stuff

use std::{borrow::Cow, fmt};

use specta::{internal::detect_duplicate_type_names, ImplLocation};
use specta_serde::validate_dt;

#[derive(Clone, Debug)]
pub(crate) enum PathItem {
    Type(Cow<'static, str>),
    TypeExtended(Cow<'static, str>, ImplLocation),
    Field(Cow<'static, str>),
    Variant(Cow<'static, str>),
}

#[derive(Clone)]
pub(crate) struct ExportContext<'a> {
    pub(crate) cfg: &'a Typescript,
    pub(crate) path: Vec<PathItem>,
    // `false` when inline'ing and `true` when exporting as named.
    pub(crate) is_export: bool,
}

impl ExportContext<'_> {
    pub(crate) fn with(&self, item: PathItem) -> Self {
        Self {
            path: self.path.iter().cloned().chain([item]).collect(),
            ..*self
        }
    }

    pub(crate) fn export_path(&self) -> ExportPath {
        ExportPath::new(&self.path)
    }
}

/// Represents the path of an error in the export tree.
/// This is designed to be opaque, meaning it's internal format and `Display` impl are subject to change at will.
pub struct ExportPath(String);

impl ExportPath {
    pub(crate) fn new(path: &[PathItem]) -> Self {
        let mut s = String::new();
        let mut path = path.iter().peekable();
        while let Some(item) = path.next() {
            s.push_str(match item {
                PathItem::Type(v) => v,
                PathItem::TypeExtended(_, loc) => loc.as_str(),
                PathItem::Field(v) => v,
                PathItem::Variant(v) => v,
            });

            if let Some(next) = path.peek() {
                s.push_str(match next {
                    PathItem::Type(_) => " -> ",
                    PathItem::TypeExtended(_, _) => " -> ",
                    PathItem::Field(_) => ".",
                    PathItem::Variant(_) => "::",
                });
            } else {
                break;
            }
        }

        Self(s)
    }

    #[doc(hidden)]
    pub fn new_unsafe(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl PartialEq for ExportPath {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Debug for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

use std::path::Path;

use specta::TypeCollection;

use crate::reserved_terms::RESERVED_TYPE_NAMES;
use crate::{BigIntExportBehavior, Error, NamedLocation, Typescript};
use std::fmt::Write;

use specta::datatype::{
    inline_and_flatten_ndt, DataType, DeprecatedType, EnumRepr, EnumType, EnumVariant, Fields,
    FunctionResultVariant, LiteralType, NamedDataType, PrimitiveType, StructType, TupleType,
};
use specta::{
    internal::{skip_fields, skip_fields_named, NonSkipField},
    NamedType, Type,
};

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) type Output = Result<String>;

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export_ref<T: NamedType>(_: &T, conf: &Typescript) -> Output {
    export::<T>(conf)
}

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: NamedType>(conf: &Typescript) -> Output {
    let mut types = TypeCollection::default();
    T::definition(&mut types);
    let ty = types.get(T::ID).unwrap();
    let ty = inline_and_flatten_ndt(ty.clone(), &types);

    validate_dt(ty.ty(), &types)?;
    let result = export_named_datatype(conf, &ty, &types);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        return Err(Error::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline_ref<T: Type>(_: &T, conf: &Typescript) -> Output {
    inline::<T>(conf)
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &Typescript) -> Output {
    let mut types = TypeCollection::default();

    let ty = T::definition(&mut types);
    let ty = specta::datatype::inline(ty.clone(), &types);

    validate_dt(&ty, &types)?;
    let result = datatype(conf, &FunctionResultVariant::Value(ty.clone()), &types);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        return Err(Error::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `export Name = { demo: string; }`
pub fn export_named_datatype(
    conf: &Typescript,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    // TODO: Duplicate type name detection?

    // is_valid_ty(&typ.inner, types)?;
    export_datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: true,
        },
        typ,
        types,
    )
}

#[allow(clippy::ptr_arg)]
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

    let comments = crate::js_doc::js_doc_builder(docs, deprecated).build();

    let prefix = match start_with_newline && !comments.is_empty() {
        true => "\n",
        false => "",
    };

    format!("{prefix}{comments}{other}")
}

fn export_datatype_inner(
    ctx: ExportContext,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    let name = typ.name();
    let docs = typ.docs();
    let ext = typ.ext();
    let deprecated = typ.deprecated();
    let item = &typ.inner;

    let ctx = ctx.with(
        ext.clone()
            .map(|v| PathItem::TypeExtended(name.clone(), *v.impl_location()))
            .unwrap_or_else(|| PathItem::Type(name.clone())),
    );
    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let generics = item
        .generics()
        .filter(|generics| !generics.is_empty())
        .map(|generics| format!("<{}>", generics.join(", ")))
        .unwrap_or_default();

    let mut inline_ts = String::new();
    datatype_inner(
        ctx.clone(),
        &FunctionResultVariant::Value((typ.inner).clone()),
        types,
        &mut inline_ts,
    )?;

    Ok(inner_comments(
        ctx,
        deprecated,
        docs,
        format!("export type {name}{generics} = {inline_ts}"),
        false,
    ))
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &Typescript, typ: &FunctionResultVariant, types: &TypeCollection) -> Output {
    // TODO: Duplicate type name detection?

    let mut s = String::new();
    datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: false,
        },
        typ,
        types,
        &mut s,
    )
    .map(|_| s)
}

macro_rules! primitive_def {
    ($($t:ident)+) => {
        $(PrimitiveType::$t)|+
    }
}

pub(crate) fn datatype_inner(
    ctx: ExportContext,
    typ: &FunctionResultVariant,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    let typ = match typ {
        FunctionResultVariant::Value(t) => t,
        FunctionResultVariant::Result(t, e) => {
            let mut variants = vec![
                {
                    let mut v = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionResultVariant::Value(t.clone()),
                        types,
                        &mut v,
                    )?;
                    v
                },
                {
                    let mut v = String::new();
                    datatype_inner(ctx, &FunctionResultVariant::Value(e.clone()), types, &mut v)?;
                    v
                },
            ];
            variants.dedup();
            s.push_str(&variants.join(" | "));
            return Ok(());
        }
    };

    Ok(match &typ {
        DataType::Any => s.push_str(ANY),
        DataType::Unknown => s.push_str(UNKNOWN),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
            let str = match p {
                primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => NUMBER,
                primitive_def!(usize isize i64 u64 i128 u128) => match ctx.cfg.bigint {
                    BigIntExportBehavior::String => STRING,
                    BigIntExportBehavior::Number => NUMBER,
                    BigIntExportBehavior::BigInt => BIGINT,
                    BigIntExportBehavior::Fail => {
                        return Err(Error::BigIntForbidden(ctx.export_path()));
                    }
                },
                primitive_def!(String char) => STRING,
                primitive_def!(bool) => BOOLEAN,
            };

            s.push_str(str);
        }
        DataType::Literal(literal) => match literal {
            LiteralType::i8(v) => write!(s, "{v}")?,
            LiteralType::i16(v) => write!(s, "{v}")?,
            LiteralType::i32(v) => write!(s, "{v}")?,
            LiteralType::u8(v) => write!(s, "{v}")?,
            LiteralType::u16(v) => write!(s, "{v}")?,
            LiteralType::u32(v) => write!(s, "{v}")?,
            LiteralType::f32(v) => write!(s, "{v}")?,
            LiteralType::f64(v) => write!(s, "{v}")?,
            LiteralType::bool(v) => write!(s, "{v}")?,
            LiteralType::String(v) => write!(s, r#""{v}""#)?,
            LiteralType::char(v) => write!(s, r#""{v}""#)?,
            LiteralType::None => s.write_str(NULL)?,
            _ => unreachable!(),
        },
        DataType::Nullable(def) => {
            datatype_inner(
                ctx,
                &FunctionResultVariant::Value((**def).clone()),
                types,
                s,
            )?;

            let or_null = format!(" | {NULL}");
            if !s.ends_with(&or_null) {
                s.push_str(&or_null);
            }
        }
        DataType::Map(def) => {
            // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
            // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
            s.push_str("Partial<{ [key in ");
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value(def.key_ty().clone()),
                types,
                s,
            )?;
            s.push_str("]: ");
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value(def.value_ty().clone()),
                types,
                s,
            )?;
            s.push_str(" }>");
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => {
            let mut dt = String::new();
            datatype_inner(
                ctx,
                &FunctionResultVariant::Value(def.ty().clone()),
                types,
                &mut dt,
            )?;

            let dt = if (dt.contains(' ') && !dt.ends_with('}'))
                // This is to do with maintaining order of operations.
                // Eg `{} | {}` must be wrapped in parens like `({} | {})[]` but `{}` doesn't cause `{}[]` is valid
                || (dt.contains(' ') && (dt.contains('&') || dt.contains('|')))
            {
                format!("({dt})")
            } else {
                dt
            };

            if let Some(length) = def.length() {
                s.push('[');

                for n in 0..length {
                    if n != 0 {
                        s.push_str(", ");
                    }

                    s.push_str(&dt);
                }

                s.push(']');
            } else {
                write!(s, "{dt}[]")?;
            }
        }
        DataType::Struct(item) => struct_datatype(
            ctx.with(
                item.sid()
                    .and_then(|sid| types.get(sid))
                    .and_then(|v| v.ext())
                    .map(|v| PathItem::TypeExtended(item.name().clone(), *v.impl_location()))
                    .unwrap_or_else(|| PathItem::Type(item.name().clone())),
            ),
            item.name(),
            item,
            types,
            s,
        )?,
        DataType::Enum(item) => {
            let mut ctx = ctx.clone();
            let cfg = ctx.cfg.clone().bigint(BigIntExportBehavior::Number);
            if item.skip_bigint_checks() {
                ctx.cfg = &cfg;
            }

            enum_datatype(
                ctx.with(PathItem::Variant(item.name().clone())),
                item,
                types,
                s,
            )?
        }
        DataType::Tuple(tuple) => s.push_str(&tuple_datatype(ctx, tuple, types)?),
        DataType::Reference(reference) => {
            let definition = types.get(reference.sid()).unwrap(); // TODO: Error handling

            if reference.generics().len() == 0 {
                s.push_str(&definition.name());
            } else {
                s.push_str(&definition.name());
                s.push('<');

                for (i, (_, v)) in reference.generics().iter().enumerate() {
                    if i != 0 {
                        s.push_str(", ");
                    }

                    datatype_inner(
                        ctx.with(PathItem::Type(definition.name().clone())),
                        &FunctionResultVariant::Value(v.clone()),
                        types,
                        s,
                    )?;
                }

                s.push('>');
            }
        }
        DataType::Generic(ident) => s.push_str(&ident.to_string()),
    })
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match fields {
        [(field, ty)] => {
            let mut v = String::new();
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value((*ty).clone()),
                types,
                &mut v,
            )?;
            s.push_str(&inner_comments(
                ctx,
                field.deprecated(),
                field.docs(),
                v,
                true,
            ));
        }
        fields => {
            s.push('[');

            for (i, (field, ty)) in fields.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }

                let mut v = String::new();
                datatype_inner(
                    ctx.clone(),
                    &FunctionResultVariant::Value((*ty).clone()),
                    types,
                    &mut v,
                )?;
                s.push_str(&inner_comments(
                    ctx.clone(),
                    field.deprecated(),
                    field.docs(),
                    v,
                    true,
                ));
            }

            s.push(']');
        }
    })
}

fn tuple_datatype(ctx: ExportContext, tuple: &TupleType, types: &TypeCollection) -> Output {
    match &tuple.elements()[..] {
        [] => Ok(NULL.to_string()),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionResultVariant::Value(v.clone()),
                        types,
                        &mut s,
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn struct_datatype(
    ctx: ExportContext,
    key: &str,
    strct: &StructType,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match &strct.fields() {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &skip_fields(unnamed.fields()).collect::<Vec<_>>(),
            types,
            s,
        )?,
        Fields::Named(named) => {
            let fields = skip_fields_named(named.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                return Ok(match named.tag().as_ref() {
                    Some(tag) => write!(s, r#"{{ "{tag}": "{key}" }}"#)?,
                    None => write!(s, "Record<{STRING}, {NEVER}>")?,
                });
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) =
                fields.iter().partition(|(_, (f, _))| f.flatten());

            let mut field_sections = flattened
                .into_iter()
                .map(|(key, (field, ty))| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.with(PathItem::Field(key.clone())),
                        &FunctionResultVariant::Value(ty.clone()),
                        types,
                        &mut s,
                    )
                    .map(|_| {
                        inner_comments(
                            ctx.clone(),
                            field.deprecated(),
                            field.docs(),
                            format!("({s})"),
                            true,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let mut unflattened_fields = non_flattened
                .into_iter()
                .map(|(key, field_ref)| {
                    let (field, _) = field_ref;

                    let mut other = String::new();
                    object_field_to_ts(
                        ctx.with(PathItem::Field(key.clone())),
                        key.clone(),
                        field_ref,
                        types,
                        &mut other,
                    )?;

                    Ok(inner_comments(
                        ctx.clone(),
                        field.deprecated(),
                        field.docs(),
                        other,
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            if let Some(tag) = &named.tag() {
                unflattened_fields.push(format!("{tag}: \"{key}\""));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            s.push_str(&field_sections.join(" & "));
        }
    })
}

fn enum_variant_datatype(
    ctx: ExportContext,
    types: &TypeCollection,
    name: Cow<'static, str>,
    variant: &EnumVariant,
) -> Result<Option<String>> {
    match &variant.fields() {
        // TODO: Remove unreachable in type system
        Fields::Unit => unreachable!("Unit enum variants have no type!"),
        Fields::Named(obj) => {
            let mut fields = if let Some(tag) = &obj.tag() {
                let sanitised_name = sanitise_key(name, true);
                vec![format!("{tag}: {sanitised_name}")]
            } else {
                vec![]
            };

            fields.extend(
                skip_fields_named(obj.fields())
                    .map(|(name, field_ref)| {
                        let (field, _) = field_ref;

                        let mut other = String::new();
                        object_field_to_ts(
                            ctx.with(PathItem::Field(name.clone())),
                            name.clone(),
                            field_ref,
                            types,
                            &mut other,
                        )?;

                        Ok(inner_comments(
                            ctx.clone(),
                            field.deprecated(),
                            field.docs(),
                            other,
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
        Fields::Unnamed(obj) => {
            let fields = skip_fields(obj.fields())
                .map(|(_, ty)| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionResultVariant::Value(ty.clone()),
                        types,
                        &mut s,
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(match &fields[..] {
                [] => {
                    // If the actual length is 0, we know `#[serde(skip)]` was not used.
                    if obj.fields().is_empty() {
                        Some("[]".to_string())
                    } else {
                        // We wanna render `{tag}` not `{tag}: {type}` (where `{type}` is what this function returns)
                        None
                    }
                }
                // If the actual length is 1, we know `#[serde(skip)]` was not used.
                [field] if obj.fields().len() == 1 => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

fn enum_datatype(
    ctx: ExportContext,
    e: &EnumType,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    if e.variants().is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    Ok(match &e.repr() {
        EnumRepr::Untagged => {
            let mut variants = e
                .variants()
                .iter()
                .filter(|(_, variant)| !variant.skip())
                .map(|(name, variant)| {
                    Ok(match variant.fields() {
                        Fields::Unit => NULL.to_string(),
                        _ => inner_comments(
                            ctx.clone(),
                            variant.deprecated(),
                            variant.docs(),
                            enum_variant_datatype(
                                ctx.with(PathItem::Variant(name.clone())),
                                types,
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
            s.push_str(&variants.join(" | "));
        }
        repr => {
            let mut variants = e
                .variants()
                .iter()
                .filter(|(_, variant)| !variant.skip())
                .map(|(variant_name, variant)| {
                    let sanitised_name = sanitise_key(variant_name.clone(), true);

                    Ok(inner_comments(
                        ctx.clone(),
                        variant.deprecated(),
                        variant.docs(),
                        match (repr, &variant.fields()) {
                            (EnumRepr::Untagged, _) => unreachable!(),
                            (EnumRepr::Internal { tag }, Fields::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Internal { tag }, Fields::Unnamed(tuple)) => {
                                let fields = skip_fields(tuple.fields()).collect::<Vec<_>>();

                                // This field is only required for `{ty}` not `[...]` so we only need to check when there one field
                                let dont_join_ty = if tuple.fields().len() == 1 {
                                    let (_, ty) = fields.first().expect("checked length above");
                                    validate_type_for_tagged_intersection(
                                        ctx.clone(),
                                        (**ty).clone(),
                                        types,
                                    )?
                                } else {
                                    false
                                };

                                let mut typ = String::new();

                                unnamed_fields_datatype(ctx.clone(), &fields, types, &mut typ)?;

                                if dont_join_ty {
                                    format!("({{ {tag}: {sanitised_name} }})")
                                } else {
                                    // We wanna be sure `... & ... | ...` becomes `... & (... | ...)`
                                    if typ.contains('|') {
                                        typ = format!("({typ})");
                                    }
                                    format!("({{ {tag}: {sanitised_name} }} & {typ})")
                                }
                            }
                            (EnumRepr::Internal { tag }, Fields::Named(obj)) => {
                                let mut fields = vec![format!("{tag}: {sanitised_name}")];

                                for (name, field) in skip_fields_named(obj.fields()) {
                                    let mut other = String::new();
                                    object_field_to_ts(
                                        ctx.with(PathItem::Field(name.clone())),
                                        name.clone(),
                                        field,
                                        types,
                                        &mut other,
                                    )?;
                                    fields.push(other);
                                }

                                format!("{{ {} }}", fields.join("; "))
                            }
                            (EnumRepr::External, Fields::Unit) => sanitised_name.to_string(),
                            (EnumRepr::External, _) => {
                                let ts_values = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    types,
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
                            (EnumRepr::Adjacent { tag, .. }, Fields::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Adjacent { tag, content }, _) => {
                                let ts_value = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    types,
                                    variant_name.clone(),
                                    variant,
                                )?;

                                let mut s = String::new();

                                s.push_str("{ ");

                                write!(s, "{tag}: {sanitised_name}")?;
                                if let Some(ts_value) = ts_value {
                                    write!(s, "; {content}: {ts_value}")?;
                                }

                                s.push_str(" }");

                                s
                            }
                        },
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            s.push_str(&variants.join(" | "));
        }
    })
}

// impl std::fmt::Display for LiteralType {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::i8(v) => write!(f, "{v}"),
//             Self::i16(v) => write!(f, "{v}"),
//             Self::i32(v) => write!(f, "{v}"),
//             Self::u8(v) => write!(f, "{v}"),
//             Self::u16(v) => write!(f, "{v}"),
//             Self::u32(v) => write!(f, "{v}"),
//             Self::f32(v) => write!(f, "{v}"),
//             Self::f64(v) => write!(f, "{v}"),
//             Self::bool(v) => write!(f, "{v}"),
//             Self::String(v) => write!(f, r#""{v}""#),
//             Self::char(v) => write!(f, r#""{v}""#),
//             Self::None => f.write_str(NULL),
//         }
//     }
// }

/// convert an object field into a Typescript string
fn object_field_to_ts(
    ctx: ExportContext,
    key: Cow<'static, str>,
    (field, ty): NonSkipField,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional() {
        true => (format!("{field_name_safe}?").into(), ty),
        false => (field_name_safe, ty),
    };

    let mut value = String::new();
    datatype_inner(
        ctx,
        &FunctionResultVariant::Value(ty.clone()),
        types,
        &mut value,
    )?;

    Ok(write!(s, "{key}: {value}",)?)
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
        return Err(Error::ForbiddenName(loc, ctx.export_path(), name));
    }

    if let Some(first_char) = ident.chars().next() {
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(Error::InvalidName(
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
        return Err(Error::InvalidName(
            loc,
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    Ok(ident.to_string())
}

fn validate_type_for_tagged_intersection(
    ctx: ExportContext,
    ty: DataType,
    types: &TypeCollection,
) -> Result<bool> {
    match ty {
        DataType::Any
        | DataType::Unknown
        | DataType::Primitive(_)
        // `T & null` is `never` but `T & (U | null)` (this variant) is `T & U` so it's fine.
        | DataType::Nullable(_)
        | DataType::List(_)
        | DataType::Map(_)
        | DataType::Generic(_) => Ok(false),
        DataType::Literal(v) => match v {
            LiteralType::None => Ok(true),
            _ => Ok(false),
        },
        DataType::Struct(v) => match v.fields() {
            Fields::Unit => Ok(true),
            Fields::Unnamed(_) => {
                Err(Error::InvalidTaggedVariantContainingTupleStruct(
                   ctx.export_path()
                ))
            }
            Fields::Named(fields) => {
                // Prevent `{ tag: "{tag}" } & Record<string | never>`
                if fields.tag().is_none() && fields.fields().is_empty() {
                    return Ok(true);
                }

                Ok(false)
            }
        },
        DataType::Enum(v) => {
            match v.repr() {
                EnumRepr::Untagged => {
                    Ok(v.variants().iter().any(|(_, v)| match &v.fields() {
                        // `{ .. } & null` is `never`
                        Fields::Unit => true,
                         // `{ ... } & Record<string, never>` is not useful
                        Fields::Named(v) => v.tag().is_none() && v.fields().is_empty(),
                        Fields::Unnamed(_) => false,
                    }))
                },
                // All of these repr's are always objects.
                EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } | EnumRepr::External => Ok(false),
            }
        }
        DataType::Tuple(v) => {
            // Empty tuple is `null`
            if v.elements().is_empty() {
                return Ok(true);
            }

            Ok(false)
        }
        DataType::Reference(r) => validate_type_for_tagged_intersection(
            ctx,
            types
                .get(r.sid())
                .expect("TypeCollection should have been populated by now")
                .inner
                .clone(),
            types,
        ),
    }
}

const ANY: &str = "any";
const UNKNOWN: &str = "unknown";
const NUMBER: &str = "number";
const STRING: &str = "string";
const BOOLEAN: &str = "boolean";
const NULL: &str = "null";
const NEVER: &str = "never";
const BIGINT: &str = "bigint";
