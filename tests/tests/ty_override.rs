use specta::Type;
use specta_typescript::Any;

// This test is to do with how the Macro passes the tokens
#[derive(Type)]
#[specta(collect = false)]
pub struct SpectaTypeOverride {
    #[specta(type = String)] // Ident
    string_ident: (),
    #[specta(type = u32)] // Ident
    u32_ident: (),
    #[specta(type = ::std::string::String)] // Path
    path: (),
}

// Checking that you can override the type of a field that is invalid. This is to ensure user code can override Specta in the case we have a bug/unsupported type.
#[derive(Type)]
#[specta(collect = false)]
pub struct InvalidToValidType {
    #[specta(type = Option<Any>)]
    pub(crate) cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[test]
fn type_override() {
    insta::assert_snapshot!(crate::ts::inline::<SpectaTypeOverride>(&Default::default()).unwrap(), @"{ string_ident: string; u32_ident: number; path: string }");
    insta::assert_snapshot!(crate::ts::inline::<InvalidToValidType>(&Default::default()).unwrap(), @"{ cause: any | null }");
}
