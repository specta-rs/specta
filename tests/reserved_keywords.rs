use specta::{ts::TsExportError, Type};

use crate::ts::assert_ts_export_err;

// Typescript reserved field name
#[derive(Type)]
pub struct ReservedFieldName {
    r#enum: String,
}

// Typescript reserved type name
#[derive(Type)]
#[allow(non_camel_case_types)]
pub struct r#enum {
    a: String,
}

#[test]
fn test_macro_in_decls() {
    let err = Box::new(TsExportError::ForbiddenFieldName(
        "ReservedFieldName".to_string(),
        "enum",
    ));
    assert_ts_export_err!(
        ReservedFieldName,
        // TsExportError::ForbiddenFieldName(field_name, "enum")
        // TODO: Clean up error handling with Specta cause this is bad
        TsExportError::WithCtx {
            ty_name: Some("ReservedFieldName"),
            field_name: None,
            err,
        }
    );
    assert_ts_export_err!(r#enum, TsExportError::ForbiddenTypeName("enum"));
}
