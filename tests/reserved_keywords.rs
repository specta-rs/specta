use specta::{ts::TsExportError, Type};

use crate::ts::assert_ts_export_err;

// Typescript reserved field name
#[derive(Type)]
#[specta(export = false)]
pub struct ReservedFieldName {
    r#enum: String,
}

// Typescript reserved type name
#[derive(Type)]
#[specta(export = false)]
#[allow(non_camel_case_types)]
pub struct r#enum {
    a: String,
}

#[test]
fn test_macro_in_decls() {
    let err = Box::new(TsExportError::ForbiddenFieldName(
        "TODO".to_string(), // TODO: ReservedFieldName
        "enum",
    ));
    assert_ts_export_err!(
        ReservedFieldName,
        // TsExportError::ForbiddenFieldName(field_name, "enum")
        // TODO: Clean up error handling with Specta cause this is bad
        TsExportError::WithCtx {
            ty_name: Some("TODO"), // TODO: ReservedFieldName
            field_name: None,
            err,
        }
    );
    assert_ts_export_err!(r#enum, TsExportError::ForbiddenTypeName("enum"));
}
