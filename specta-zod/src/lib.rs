//! TODO
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

// mod context;
// mod error;
// mod export_config;

// use context::{ExportContext, PathItem};
// use specta::{
//     datatype::*,
//     internal::{detect_duplicate_type_names, skip_fields, skip_fields_named, NonSkipField},
//     *,
// };
// use std::{borrow::Cow, collections::VecDeque};

// pub use context::*;
// pub use error::*;
// pub use export_config::*;

// macro_rules! primitive_def {
//     ($($t:ident)+) => {
//         $(PrimitiveType::$t)|+
//     }
// }

// type Result<T> = std::result::Result<T, ExportError>;

// pub(crate) type Output = Result<String>;

// /// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
// ///
// /// Eg. `export const Foo = z.object({ demo: z.string() });`
// pub fn export_ref<T: NamedType>(_: &T, conf: &ExportConfig) -> Output {
//     export::<T>(conf)
// }

// /// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
// ///
// /// Eg. `export const Foo = z.object({ demo: string; });`
// pub fn export<T: NamedType>(conf: &ExportConfig) -> Output {
//     let mut types = TypeCollection::default();
//     let named_data_type = T::definition_named_data_type(&mut types);
//     // is_valid_ty(&named_data_type.inner, &types)?;
//     let result = export_named_datatype(conf, &named_data_type, &types);

//     if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
//         return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
//     }

//     result
// }

// /// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
// ///
// /// Eg. `z.object({ demo: z.string() });`
// pub fn inline_ref<T: Type>(_: &T, conf: &ExportConfig) -> Output {
//     inline::<T>(conf)
// }

// /// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
// ///
// /// Eg. `z.object({ demo: z.string() });`
// pub fn inline<T: Type>(conf: &ExportConfig) -> Output {
//     let mut types = TypeCollection::default();
//     let ty = T::inline(&mut types, specta::Generics::Definition);
//     // is_valid_ty(&ty, &types)?;
//     let result = datatype(conf, &ty, &types);

//     if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
//         return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
//     }

//     result
// }

// /// Convert a DataType to a Zod validator
// ///
// /// Eg. `export const Name = z.object({ demo: z.string() });`
// pub fn export_named_datatype(
//     conf: &ExportConfig,
//     typ: &NamedDataType,
//     types: &TypeCollection,
// ) -> Output {
//     // TODO: Duplicate type name detection?

//     // is_valid_ty(&typ.inner, types)?;
//     export_datatype_inner(
//         ExportContext {
//             cfg: conf,
//             path: vec![],
//             is_export: true,
//         },
//         typ,
//         types,
//     )
// }

// #[allow(clippy::ptr_arg)]
// fn inner_comments(
//     ctx: ExportContext,
//     _deprecated: Option<&DeprecatedType>,
//     _docs: &Cow<'static, str>,
//     other: String,
//     start_with_newline: bool,
// ) -> String {
//     if !ctx.is_export {
//         return other;
//     }

//     // let comments = ctx
//     //     .cfg
//     //     .comment_exporter
//     //     .map(|v| v(CommentFormatterArgs { docs, deprecated }))
//     //     .unwrap_or_default();
//     let comments = "";

//     let prefix = match start_with_newline && !comments.is_empty() {
//         true => "\n",
//         false => "",
//     };

//     format!("{prefix}{comments}{other}")
// }

// fn export_datatype_inner(
//     ctx: ExportContext,
//     typ: &NamedDataType,
//     types: &TypeCollection,
// ) -> Output {
//     let ctx = ctx.with(
//         typ.ext()
//             .map(|v| PathItem::TypeExtended(typ.name().clone(), *v.impl_location()))
//             .unwrap_or_else(|| PathItem::Type(typ.name().clone())),
//     );
//     let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, typ.name())?;

//     let _generics = typ
//         .inner
//         .generics()
//         .filter(|generics| !generics.is_empty())
//         .map(|generics| format!("<{}>", generics.join(", ")))
//         .unwrap_or_default();

//     let inline_zod = datatype_inner(ctx.clone(), &typ.inner, types)?;

//     // {generics}
//     Ok(inner_comments(
//         ctx,
//         typ.deprecated(),
//         typ.docs(),
//         format!("export const {name} = {inline_zod}"),
//         false,
//     ))
// }

// /// Convert a DataType to a Zod validator
// ///
// /// Eg. `z.object({ demo: z.string(); })`
// pub fn datatype(conf: &ExportConfig, typ: &DataType, types: &TypeCollection) -> Output {
//     // TODO: Duplicate type name detection?

//     datatype_inner(
//         ExportContext {
//             cfg: conf,
//             path: vec![],
//             is_export: false,
//         },
//         typ,
//         types,
//     )
// }

// pub(crate) fn datatype_inner(
//     ctx: ExportContext,
//     typ: &DataType,
//     types: &TypeCollection,
// ) -> Output {
//     Ok(match &typ {
//         DataType::Any => ANY.into(),
//         DataType::Unknown => UNKNOWN.into(),
//         DataType::Primitive(p) => {
//             let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
//             match p {
//                 primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => NUMBER.into(),
//                 primitive_def!(usize isize i64 u64 i128 u128) => match ctx.cfg.bigint {
//                     BigIntExportBehavior::String => STRING.into(),
//                     BigIntExportBehavior::Number => NUMBER.into(),
//                     BigIntExportBehavior::BigInt => BIGINT.into(),
//                     BigIntExportBehavior::Fail => {
//                         return Err(ExportError::BigIntForbidden(ctx.export_path()));
//                     }
//                     BigIntExportBehavior::FailWithReason(reason) => {
//                         return Err(ExportError::Other(ctx.export_path(), reason.to_owned()))
//                     }
//                 },
//                 primitive_def!(String char) => STRING.into(),
//                 primitive_def!(bool) => BOOLEAN.into(),
//             }
//         }
//         DataType::Literal(literal) => literal.to_zod(),
//         DataType::Nullable(def) => {
//             let dt = datatype_inner(ctx, def, types)?;

//             if dt.ends_with(NULLABLE) {
//                 dt
//             } else {
//                 format!("{dt}{NULLABLE}")
//             }
//         }
//         DataType::Map(def) => {
//             format!(
//                 // We use this isn't of `Record<K, V>` to avoid issues with circular references.
//                 "z.record({}, {})",
//                 datatype_inner(ctx.clone(), def.key_ty(), types)?,
//                 datatype_inner(ctx, def.value_ty(), types)?
//             )
//         }
//         // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
//         DataType::List(def) => {
//             let dt = datatype_inner(ctx, def.ty(), types)?;

//             if let Some(length) = def.length() {
//                 format!(
//                     "z.tuple([{}])",
//                     (0..length)
//                         .map(|_| dt.clone())
//                         .collect::<Vec<_>>()
//                         .join(", ")
//                 )
//             } else {
//                 format!("z.array({dt})")
//             }
//         }
//         DataType::Struct(item) => struct_datatype(
//             ctx,
//             // ctx.with(
//             //     item.sid
//             //         .and_then(|sid| types.get(sid))
//             //         .and_then(|v| v.ext())
//             //         .map(|v| PathItem::TypeExtended(item.name().clone(), v.impl_location()))
//             //         .unwrap_or_else(|| PathItem::Type(item.name().clone())),
//             // ),
//             item.name(),
//             item,
//             types,
//         )?,
//         DataType::Enum(item) => {
//             let ctx = ctx.clone();
//             let _cfg = ctx.cfg.clone().bigint(BigIntExportBehavior::Number);
//             // if item.skip_bigint_checks {
//             //     ctx.cfg = &cfg;
//             // }

//             enum_datatype(
//                 ctx.with(PathItem::Variant(item.name().clone())),
//                 item,
//                 types,
//             )?
//         }
//         DataType::Tuple(tuple) => tuple_datatype(ctx, tuple, types)?,
//         DataType::Reference(reference) => match &reference.generics()[..] {
//             [] => reference.name().to_string(),
//             _generics => {
//                 let name = reference.name();
//                 let generics = reference
//                     .generics()
//                     .iter()
//                     .map(|(_, v)| {
//                         datatype_inner(ctx.with(PathItem::Type(name.clone())), v, types)
//                     })
//                     .collect::<Result<Vec<_>>>()?
//                     .join(", ");

//                 format!("{name}({generics})")
//             }
//         },
//         DataType::Generic(generic) => generic.to_string(),
//     })
// }

// // Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
// fn unnamed_fields_datatype(
//     ctx: ExportContext,
//     fields: &[NonSkipField],
//     types: &TypeCollection,
// ) -> Output {
//     match fields {
//         [(field, ty)] => Ok(inner_comments(
//             ctx.clone(),
//             field.deprecated(),
//             field.docs(),
//             datatype_inner(ctx, ty, types)?,
//             true,
//         )),
//         fields => Ok(format!(
//             "z.tuple([{}])",
//             fields
//                 .iter()
//                 .map(|(field, ty)| Ok(inner_comments(
//                     ctx.clone(),
//                     field.deprecated(),
//                     field.docs(),
//                     datatype_inner(ctx.clone(), ty, types)?,
//                     true
//                 )))
//                 .collect::<Result<Vec<_>>>()?
//                 .join(", ")
//         )),
//     }
// }

// fn tuple_datatype(ctx: ExportContext, tuple: &TupleType, types: &TypeCollection) -> Output {
//     match &tuple.elements()[..] {
//         [] => Ok(NULL.into()),
//         tys => Ok(format!(
//             "z.tuple([{}])",
//             tys.iter()
//                 .map(|v| datatype_inner(ctx.clone(), v, types))
//                 .collect::<Result<Vec<_>>>()?
//                 .join(", ")
//         )),
//     }
// }

// fn struct_datatype(
//     ctx: ExportContext,
//     key: &str,
//     s: &StructType,
//     types: &TypeCollection,
// ) -> Output {
//     match &s.fields() {
//         Fields::Unit => Ok(NULL.into()),
//         Fields::Unnamed(s) => {
//             unnamed_fields_datatype(ctx, &skip_fields(s.fields()).collect::<Vec<_>>(), types)
//         }
//         Fields::Named(s) => {
//             let fields = skip_fields_named(s.fields()).collect::<Vec<_>>();

//             if fields.is_empty() {
//                 return Ok(s
//                     .tag()
//                     .as_ref()
//                     .map(|tag| {
//                         let tag = sanitise_key(tag.clone(), false);
//                         format!(r#"z.object({{ {tag}: z.literal("{key}") }})"#)
//                     })
//                     .unwrap_or_else(|| format!("z.record({STRING}, {NEVER})")));
//             }

//             let (flattened, non_flattened): (Vec<_>, Vec<_>) =
//                 fields.iter().partition(|(_, (f, _))| f.flatten());

//             let mut field_sections = flattened
//                 .into_iter()
//                 .map(|(key, (field, ty))| {
//                     datatype_inner(ctx.with(PathItem::Field(key.clone())), ty, types).map(
//                         |type_str| {
//                             inner_comments(
//                                 ctx.clone(),
//                                 field.deprecated(),
//                                 field.docs(),
//                                 type_str,
//                                 true,
//                             )
//                         },
//                     )
//                 })
//                 .collect::<Result<VecDeque<String>>>()?;

//             let mut unflattened_fields = non_flattened
//                 .into_iter()
//                 .map(|(key, field_ref)| {
//                     let (field, _) = field_ref;

//                     Ok(inner_comments(
//                         ctx.clone(),
//                         field.deprecated(),
//                         field.docs(),
//                         object_field_to_ts(
//                             ctx.with(PathItem::Field(key.clone())),
//                             key.clone(),
//                             field_ref,
//                             types,
//                         )?,
//                         true,
//                     ))
//                 })
//                 .collect::<Result<Vec<_>>>()?;

//             if let Some(tag) = &s.tag() {
//                 let tag = sanitise_key(tag.clone(), false);
//                 unflattened_fields.push(format!(r#"{tag}: z.literal("{key}")"#));
//             }

//             if !unflattened_fields.is_empty() {
//                 if field_sections.is_empty() {
//                     field_sections
//                         .push_back(format!("z.object({{ {} }})", unflattened_fields.join(", ")));
//                 } else {
//                     field_sections.push_back(format!(
//                         ".and(z.object({{ {} }}))",
//                         unflattened_fields.join(", ")
//                     ));
//                 }
//             }

//             Ok(field_sections.pop_front().expect("field_sections is empty")
//                 + &{
//                     let vec: Vec<_> = field_sections.into();
//                     vec.join("")
//                 })
//         }
//     }
// }

// fn enum_variant_datatype(
//     ctx: ExportContext,
//     types: &TypeCollection,
//     name: Cow<'static, str>,
//     variant: &EnumVariant,
// ) -> Result<Option<String>> {
//     match &variant.fields() {
//         // TODO: Remove unreachable in type system
//         Fields::Unit => unreachable!("Unit enum variants have no type!"),
//         Fields::Named(obj) => {
//             let mut fields = if let Some(tag) = &obj.tag() {
//                 let sanitised_name = sanitise_key(name, true);
//                 let tag = sanitise_key(tag.clone(), false);
//                 vec![format!(r#"{tag}: z.literal({sanitised_name})"#)]
//             } else {
//                 vec![]
//             };

//             fields.extend(
//                 skip_fields_named(obj.fields())
//                     .map(|(name, field_ref)| {
//                         let (field, _) = field_ref;

//                         Ok(inner_comments(
//                             ctx.clone(),
//                             field.deprecated(),
//                             field.docs(),
//                             object_field_to_ts(
//                                 ctx.with(PathItem::Field(name.clone())),
//                                 name.clone(),
//                                 field_ref,
//                                 types,
//                             )?,
//                             true,
//                         ))
//                     })
//                     .collect::<Result<Vec<_>>>()?,
//             );

//             Ok(Some(match &fields[..] {
//                 [] => format!("z.record({STRING}, {NEVER})").to_string(),
//                 fields => format!("z.object({{ {} }})", fields.join(", ")),
//             }))
//         }
//         Fields::Unnamed(obj) => {
//             let fields = skip_fields(obj.fields())
//                 .map(|(_, ty)| datatype_inner(ctx.clone(), ty, types))
//                 .collect::<Result<Vec<_>>>()?;

//             Ok(match &fields[..] {
//                 [] => {
//                     // If the actual length is 0, we know `#[serde(skip)]` was not used.
//                     if obj.fields().is_empty() {
//                         Some("z.tuple([])".to_string())
//                     } else {
//                         // We wanna render `{tag}` not `{tag}: {type}` (where `{type}` is what this function returns)
//                         None
//                     }
//                 }
//                 // If the actual length is 1, we know `#[serde(skip)]` was not used.
//                 [field] if obj.fields().len() == 1 => Some(field.to_string()),
//                 fields => Some(format!("z.tuple([{}])", fields.join(", "))),
//             })
//         }
//     }
// }

// fn enum_datatype(ctx: ExportContext, e: &EnumType, types: &TypeCollection) -> Output {
//     if e.variants().is_empty() {
//         return Ok(NEVER.to_string());
//     }

//     Ok(match &e.repr() {
//         EnumRepr::Untagged => {
//             let mut variants = e
//                 .variants()
//                 .iter()
//                 .filter(|(_, variant)| !variant.skip())
//                 .map(|(name, variant)| {
//                     Ok(match variant.fields() {
//                         Fields::Unit => NULL.into(),
//                         _ => inner_comments(
//                             ctx.clone(),
//                             variant.deprecated(),
//                             variant.docs(),
//                             enum_variant_datatype(
//                                 ctx.with(PathItem::Variant(name.clone())),
//                                 types,
//                                 name.clone(),
//                                 variant,
//                             )?
//                             .expect("Invalid Serde type"),
//                             true,
//                         ),
//                     })
//                 })
//                 .collect::<Result<Vec<_>>>()?;
//             variants.dedup();
//             if variants.len() == 1 {
//                 variants.pop().expect("variants is empty")
//             } else {
//                 format!("z.union([{}])", variants.join(", "))
//             }
//         }
//         repr => {
//             let mut variants = e
//                 .variants()
//                 .iter()
//                 .filter(|(_, variant)| !variant.skip())
//                 .map(|(variant_name, variant)| {
//                     let sanitised_name = sanitise_key(variant_name.clone(), true);

//                     Ok(inner_comments(
//                         ctx.clone(),
//                         variant.deprecated(),
//                         variant.docs(),
//                         match (repr, &variant.fields()) {
//                             (EnumRepr::Untagged, _) => unreachable!(),
//                             (EnumRepr::Internal { tag }, Fields::Unit) => {
//                             	let tag = sanitise_key(tag.clone(), false);
//                                 format!(r#"z.object({{ {tag}: {sanitised_name} }})"#)
//                             }
//                             (EnumRepr::Internal { tag }, Fields::Unnamed(tuple)) => {
//                            	 	let tag = sanitise_key(tag.clone(), false);
//                                 let fields = skip_fields(tuple.fields()).collect::<Vec<_>>();

//                                 // This field is only required for `{ty}` not `[...]` so we only need to check when there one field
//                                 let dont_join_ty = if tuple.fields().len() == 1 {
//                                     let (_, ty) = fields.first().expect("checked length above");
//                                     validate_type_for_tagged_intersection(
//                                         ctx.clone(),
//                                         (**ty).clone(),
//                                         types,
//                                     )?
//                                 } else {
//                                     false
//                                 };

//                                 let typ =
//                                     unnamed_fields_datatype(ctx.clone(), &fields, types)?;

//                                 if dont_join_ty {
//                                     format!(r#"z.object({{ {tag}: {sanitised_name} }})"#)
//                                 } else {
//                                     format!(
//                                         r#"z.and(z.object({{ {tag}: {sanitised_name} }}), {typ})"#
//                                     )
//                                 }
//                             }
//                             (EnumRepr::Internal { tag }, Fields::Named(obj)) => {
//                          	 	let tag = sanitise_key(tag.clone(), false);
//                                 let mut fields = vec![format!("{tag}: {sanitised_name}")];

//                                 fields.extend(
//                                     skip_fields_named(obj.fields())
//                                         .map(|(name, field)| {
//                                             object_field_to_ts(
//                                                 ctx.with(PathItem::Field(name.clone())),
//                                                 name.clone(),
//                                                 field,
//                                                 types,
//                                             )
//                                         })
//                                         .collect::<Result<Vec<_>>>()?,
//                                 );

//                                 format!("z.object({{ {} }})", fields.join(", "))
//                             }
//                             (EnumRepr::External, Fields::Unit) => format!("z.literal({})", sanitised_name),
//                             (EnumRepr::External, _) => {
//                                 let ts_values = enum_variant_datatype(
//                                     ctx.with(PathItem::Variant(variant_name.clone())),
//                                     types,
//                                     variant_name.clone(),
//                                     variant,
//                                 )?;
//                                 let sanitised_name = sanitise_key(variant_name.clone(), false);

//                                 match ts_values {
//                                     Some(ts_values) => {
//                                         format!("z.object({{ {sanitised_name}: {ts_values} }})")
//                                     }
//                                     None => format!(r#"z.literal({sanitised_name})"#),
//                                 }
//                             }
//                             (EnumRepr::Adjacent { tag, .. }, Fields::Unit) => {
//                          	 	let tag = sanitise_key(tag.clone(), false);
//                                 format!(r#"z.object({{ {tag}: z.literal({sanitised_name}) }})"#)
//                             }
//                             (EnumRepr::Adjacent { tag, content }, _) => {
//                          	 	let tag = sanitise_key(tag.clone(), false);
//                                 let content_values = enum_variant_datatype(
//                                     ctx.with(PathItem::Variant(variant_name.clone())),
//                                     types,
//                                     variant_name.clone(),
//                                     variant,
//                                 )?
//                                 .expect("Invalid Serde type");

//                                 format!(r#"z.object({{ {tag}: z.literal({sanitised_name}), {content}: {content_values} }})"#)
//                             }
//                         },
//                         true,
//                     ))
//                 })
//                 .collect::<Result<Vec<_>>>()?;
//             variants.dedup();
//             if variants.len() == 1 {
//                 variants.swap_remove(0)
//             } else {
//                 format!("z.union([{}])", variants.join(", "))
//             }
//         }
//     })
// }

// trait ToZod {
//     fn to_zod(&self) -> String;
// }
// impl ToZod for LiteralType {
//     fn to_zod(&self) -> String {
//         format!(
//             "z.literal({})",
//             match self {
//                 Self::i8(v) => v.to_string(),
//                 Self::i16(v) => v.to_string(),
//                 Self::i32(v) => v.to_string(),
//                 Self::u8(v) => v.to_string(),
//                 Self::u16(v) => v.to_string(),
//                 Self::u32(v) => v.to_string(),
//                 Self::f32(v) => v.to_string(),
//                 Self::f64(v) => v.to_string(),
//                 Self::bool(v) => v.to_string(),
//                 Self::String(v) => format!(r#""{v}""#),
//                 Self::char(v) => format!(r#""{v}""#),
//                 Self::None => return NULL.to_string(),
//                 _ => panic!("unhandled literal type!"),
//             }
//         )
//     }
// }

// /// convert an object field into a Typescript string
// fn object_field_to_ts(
//     ctx: ExportContext,
//     key: Cow<'static, str>,
//     (field, ty): NonSkipField,
//     types: &TypeCollection,
// ) -> Output {
//     let field_name_safe = sanitise_key(key, false);

//     // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
//     let (key, ty) = match field.optional() {
//         true => (format!("{field_name_safe}").into(), ty), // TODO: optional
//         false => (field_name_safe, ty),
//     };

//     Ok(format!("{key}: {}", datatype_inner(ctx, ty, types)?))
// }

// /// sanitise a string to be a valid Typescript key
// fn sanitise_key<'a>(field_name: Cow<'static, str>, force_string: bool) -> Cow<'a, str> {
//     let valid = field_name
//         .chars()
//         .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
//         && field_name
//             .chars()
//             .next()
//             .map(|first| !first.is_numeric())
//             .unwrap_or(true);

//     if force_string || !valid {
//         format!(r#""{field_name}""#).into()
//     } else {
//         field_name
//     }
// }

// pub(crate) fn sanitise_type_name(ctx: ExportContext, loc: NamedLocation, ident: &str) -> Output {
//     // if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
//     //     return Err(ExportError::ForbiddenName(loc, ctx.export_path(), name));
//     // }

//     if let Some(first_char) = ident.chars().next() {
//         if !first_char.is_alphabetic() && first_char != '_' {
//             return Err(ExportError::InvalidName(
//                 loc,
//                 ctx.export_path(),
//                 ident.to_string(),
//             ));
//         }
//     }

//     if ident
//         .find(|c: char| !c.is_alphanumeric() && c != '_')
//         .is_some()
//     {
//         return Err(ExportError::InvalidName(
//             loc,
//             ctx.export_path(),
//             ident.to_string(),
//         ));
//     }

//     Ok(ident.to_string())
// }

// fn validate_type_for_tagged_intersection(
//     ctx: ExportContext,
//     ty: DataType,
//     types: &TypeCollection,
// ) -> Result<bool> {
//     match ty {
//         DataType::Any
//         | DataType::Unknown
//         | DataType::Primitive(_)
//         // `T & null` is `never` but `T & (U | null)` (this variant) is `T & U` so it's fine.
//         | DataType::Nullable(_)
//         | DataType::List(_)
//         | DataType::Map(_)
//         | DataType::Generic(_) => Ok(false),
//         DataType::Literal(v) => match v {
//             LiteralType::None => Ok(true),
//             _ => Ok(false),
//         },
//         DataType::Struct(v) => match v.fields() {
//             Fields::Unit => Ok(true),
//             Fields::Unnamed(_) => {
//                 Err(ExportError::InvalidTaggedVariantContainingTupleStruct(
//                    ctx.export_path()
//                 ))
//             }
//             Fields::Named(fields) => {
//                 // Prevent `{ tag: "{tag}" } & Record<string | never>`
//                 if fields.tag().is_none() && fields.fields().is_empty() {
//                     return Ok(true);
//                 }

//                 Ok(false)
//             }
//         },
//         DataType::Enum(v) => {
//             match v.repr() {
//                 EnumRepr::Untagged => {
//                     Ok(v.variants().iter().any(|(_, v)| match &v.fields() {
//                         // `{ .. } & null` is `never`
//                         Fields::Unit => true,
//                          // `{ ... } & Record<string, never>` is not useful
//                         Fields::Named(v) => v.tag().is_none() && v.fields().is_empty(),
//                         Fields::Unnamed(_) => false,
//                     }))
//                 },
//                 // All of these repr's are always objects.
//                 EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } | EnumRepr::External => Ok(false),
//             }
//         }
//         DataType::Tuple(v) => {
//             // Empty tuple is `null`
//             if v.elements().is_empty() {
//                 return Ok(true);
//             }

//             Ok(false)
//         }
//         DataType::Reference(r) => validate_type_for_tagged_intersection(
//             ctx,
//             types
//                 .get(r.sid())
//                 .expect("TypeCollection should have been populated by now")
//                 .inner
//                 .clone(),
//             types,
//         ),
//     }
// }

// const ANY: &str = "z.any()";
// const UNKNOWN: &str = "z.unknown()";
// const NUMBER: &str = "z.number()";
// const STRING: &str = "z.string()";
// const BOOLEAN: &str = "z.boolean()";
// const NULL: &str = "z.null()";
// const NULLABLE: &str = ".nullable()";
// const NEVER: &str = "z.never()";
// const BIGINT: &str = "z.bigint()";
