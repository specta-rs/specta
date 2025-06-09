use specta::{Type, TypeCollection};
use specta_typescript::Typescript;

mod one {
    use super::*;

    #[derive(Type)]
    #[specta(collect = false)]
    pub struct One {
        pub a: String,
    }
}

mod two {
    use super::*;

    #[derive(Type)]
    #[specta(collect = false)]
    pub struct One {
        pub b: String,
        pub c: i32,
    }
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Demo {
    pub one: one::One,
    pub two: two::One,
}

#[test]
fn test_duplicate_ty_name() {
    assert!(Typescript::default()
        .export(&TypeCollection::default().register::<Demo>())
        .is_err_and(|err| err
            .to_string()
            .starts_with("Detected multiple types with the same name:")));
}
