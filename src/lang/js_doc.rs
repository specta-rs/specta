use crate::*;

pub use super::ts::*;

pub fn format_comment(cfg: &ExportConfig, typ: &NamedDataType, type_map: &TypeMap) -> Output {
    format_comment_inner(
        &ExportContext {
            cfg,
            path: vec![],
            // TODO: Should JS doc support per field or variant comments???
            is_export: false,
        },
        typ,
        type_map,
    )
}

fn format_comment_inner(
    ctx: &ExportContext,
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

    let inline_ts = datatype_inner(ctx.clone(), &typ.inner, type_map)?;

    Ok(comments::js_doc_internal(
        CommentFormatterArgs {
            docs,
            deprecated: deprecated.as_ref(),
        },
        item.generics()
            .into_iter()
            .flatten()
            .map(|generic| format!("@template {}", generic))
            .chain([format!(r#"@typedef {{ {inline_ts} }} {name}"#).into()]),
    ))
}
