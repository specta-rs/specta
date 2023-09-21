use crate::*;

pub use super::ts::*;

pub fn format_comment(cfg: &ExportConfig, typ: &NamedDataType, type_map: &TypeMap) -> Output {
    format_comment_inner(&ExportContext { cfg, path: vec![] }, typ, type_map)
}

fn format_comment_inner(
    ctx: &ExportContext,
    typ @ NamedDataType {
        name,
        comments,
        inner: item,
        ..
    }: &NamedDataType,
    type_map: &TypeMap,
) -> Output {
    let ctx = ctx.with(PathItem::Type(name.clone()));

    let generics = item
        .generics()
        .filter(|generics| !generics.is_empty())
        .map(|generics| generics.join(", ").into());

    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let inline_ts = datatype_inner(ctx.clone(), &typ.inner, type_map, "null")?;

    Ok(js_doc(
        &comments
            .iter()
            .cloned()
            .chain(generics.into_iter())
            .chain([format!(r#"@typedef {{ {inline_ts} }} {name}"#).into()])
            .collect::<Vec<_>>(),
    ))
}
