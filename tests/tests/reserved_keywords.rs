use specta::{NamedType, Type, TypeCollection};
use specta_typescript::{
    Error, Typescript,
    legacy::{ExportPath, NamedLocation},
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
        export::<astruct::r#enum>().map_err(|e| e.to_string()),
        // TODO: Fix error. Missing type name
        Err("Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`\n".into())
    );
    assert_eq!(
        export::<atuplestruct::r#enum>().map_err(|e| e.to_string()),
        // TODO: Fix error. Missing type name
        Err("Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`\n".into())
    );
    assert_eq!(
        export::<aenum::r#enum>().map_err(|e| e.to_string()),
        // TODO: Fix error. Missing type name
        Err("Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`\n".into())
    );
}

fn export<T: Type>() -> Result<String, String> {
    let mut types = TypeCollection::default();
    T::definition(&mut types);
    Typescript::default()
        .export(&types)
        .map_err(|e| e.to_string())
}
