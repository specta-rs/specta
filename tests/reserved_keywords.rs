use specta::{
    ts::{ExportConfiguration, ExportPath, NamedLocation, TsExportError},
    Type,
};

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
pub enum ReservedEnumVariant {
    r#enum(String),
}

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
        specta::ts::inline::<ReservedFieldName>(&ExportConfiguration::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Field,
            ExportPath::new_unsafe("ReservedFieldName.enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::inline::<ReservedEnumVariant>(&ExportConfiguration::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Variant,
            ExportPath::new_unsafe("ReservedEnumVariant::enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::export::<astruct::r#enum>(&ExportConfiguration::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::export::<atuplestruct::r#enum>(&ExportConfiguration::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
    assert_eq!(
        specta::ts::export::<aenum::r#enum>(&ExportConfiguration::default()),
        Err(TsExportError::ForbiddenName(
            NamedLocation::Type,
            ExportPath::new_unsafe("enum"),
            "enum"
        ))
    );
}
