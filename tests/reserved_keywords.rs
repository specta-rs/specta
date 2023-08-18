use specta::{
    ts::{ExportConfig, ExportPath, NamedLocation, TsExportError},
    Type,
};

mod astruct {
    use super::*;

    // Typescript reserved type name
    #[derive(Type)]
    #[specta(export = false)]
    #[allow(non_camel_case_types)]
    pub struct r#enum {
        a: String,
    }
}

mod atuplestruct {
    use super::*;

    // Typescript reserved type name
    #[derive(Type)]
    #[specta(export = false)]
    #[allow(non_camel_case_types)]
    pub struct r#enum(String);
}

mod aenum {
    use super::*;

    // Typescript reserved type name
    #[derive(Type)]
    #[specta(export = false)]
    #[allow(non_camel_case_types)]
    pub enum r#enum {
        A(String),
    }
}

#[test]
fn test_ts_reserved_keyworks() {
    assert_eq!(
        specta::ts::export::<astruct::r#enum>(&ExportConfig::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::export::<atuplestruct::r#enum>(&ExportConfig::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::export::<aenum::r#enum>(&ExportConfig::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
}
