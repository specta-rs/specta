use crate::*;

pub use super::js_ts::*;

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
        },
        typ,
        type_map,
    )
}

fn export_datatype_inner(
    ctx: ExportContext,
    typ @ NamedDataType {
        name,
        comments,
        inner: item,
        ..
    }: &NamedDataType,
    type_map: &TypeMap,
) -> Output {
    let ctx = ctx.with(PathItem::Type(name.clone()));

    let comments = ctx
        .cfg
        .comment_exporter
        .map(|v| v(comments))
        .unwrap_or_default();

    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let generics = item
        .generics()
        .filter(|generics| !generics.is_empty())
        .map(|generics| format!("<{}>", generics.join(", ")))
        .unwrap_or_default();

    let inline_ts = datatype_inner(ctx.clone(), &typ.inner, type_map, "null")?;

    Ok(format!(
        "{comments}export type {name}{generics} = {inline_ts}"
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
        },
        typ,
        type_map,
        "null",
    )
}
