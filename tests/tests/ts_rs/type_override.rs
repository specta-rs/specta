#![allow(dead_code)]

use std::time::Instant;

use specta::Type;

struct Unsupported<T>(T);
struct Unsupported2;

#[test]
fn simple() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Override {
        a: i32,
        #[specta(type = String)]
        x: Instant,
        #[specta(type = String)]
        y: Unsupported<Unsupported<Unsupported2>>,
        #[specta(type = Option<String>)]
        z: Option<Unsupported2>,
    }

    insta::assert_snapshot!(crate::ts::inline::<Override>(&Default::default()).unwrap(), @"{ a: number; x: string; y: string; z: string | null }");
}

#[test]
fn newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct New1(#[specta(type = String)] Unsupported2);
    #[derive(Type)]
    #[specta(collect = false)]
    struct New2(#[specta(type = Option<String>)] Unsupported<Unsupported2>);

    insta::assert_snapshot!(crate::ts::inline::<New1>(&Default::default()).unwrap(), @r#"string"#);
    insta::assert_snapshot!(crate::ts::inline::<New2>(&Default::default()).unwrap(), @r#"string | null"#);
}
