use specta::{Type, TypeCollection};
use specta_typescript::Typescript;

mod one {
    use super::*;

    #[derive(Type)]
    #[specta(export = false)]
    pub struct One {
        pub a: String,
    }
}

mod two {
    use super::*;

    #[derive(Type)]
    #[specta(export = false)]
    pub struct One {
        pub b: String,
        pub c: i32,
    }
}

#[derive(Type)]
#[specta(export = false)]
pub struct Demo {
    pub one: one::One,
    pub two: two::One,
}

#[test]
fn test_duplicate_ty_name() {
    #[cfg(not(target_os = "windows"))]
    let err = r#"Detected multiple types with the same name: "One" in (ImplLocation("tests/tests/duplicate_ty_name.rs:7:14"), ImplLocation("tests/tests/duplicate_ty_name.rs:17:14"))
"#;
    #[cfg(target_os = "windows")]
    let err = r#"Detected multiple types with the same name: "One" in (ImplLocation("tests\tests\duplicate_ty_name.rs:7:14"), ImplLocation("tests\tests\duplicate_ty_name.rs:17:14"))
"#;

    assert_eq!(
        Typescript::default()
            .export(&TypeCollection::default().register::<Demo>())
            .map_err(|err| err.to_string()),
        Err(err.into())
    );
}
