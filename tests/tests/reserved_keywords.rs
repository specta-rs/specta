use specta::Type;
use specta_typescript::{
    legacy::{ExportPath, NamedLocation},
    Error, Typescript,
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
        specta_typescript::legacy::export::<astruct::r#enum>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err(Error::ForbiddenNameLegacy(
            NamedLocation::Type,
            #[cfg(not(windows))]
            ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:11:14"),
            #[cfg(windows)]
            ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:11:14"),
            "enum"
        )
        .to_string())
    );
    assert_eq!(
        specta_typescript::legacy::export::<atuplestruct::r#enum>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err(Error::ForbiddenNameLegacy(
            NamedLocation::Type,
            #[cfg(not(windows))]
            ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:23:14"),
            #[cfg(windows)]
            ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:23:14"),
            "enum"
        )
        .to_string())
    );
    assert_eq!(
        specta_typescript::legacy::export::<aenum::r#enum>(&Typescript::default())
            .map_err(|e| e.to_string()),
        Err(Error::ForbiddenNameLegacy(
            NamedLocation::Type,
            #[cfg(not(windows))]
            ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:33:14"),
            #[cfg(windows)]
            ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:33:14"),
            "enum"
        )
        .to_string())
    );
}
