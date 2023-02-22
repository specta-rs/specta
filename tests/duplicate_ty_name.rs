use specta::{
    ts::{export, TsExportError},
    ImplLocation, Type,
};

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
    assert_eq!(
        export::<Demo>(&Default::default()),
        #[cfg(not(feature = "windows"))]
        Err(TsExportError::DuplicateTypeName(
            "One",
            Some(ImplLocation::internal_new(
                "tests/duplicate_ty_name.rs:19:14"
            )),
            Some(ImplLocation::internal_new(
                "tests/duplicate_ty_name.rs:9:14"
            ))
        ))
        #[cfg(feature = "windows")]
        Err(TsExportError::DuplicateTypeName(
            "One",
            Some(ImplLocation::internal_new(
                "tests\\duplicate_ty_name.rs:19:14"
            )),
            Some(ImplLocation::internal_new(
                "tests\\duplicate_ty_name.rs:9:14"
            ))
        ))
    );
}
