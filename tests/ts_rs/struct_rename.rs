use specta::{ts::inline, Type};

#[test]
fn rename_all() {
    #[derive(Type)]
    #[specta(rename_all = "UPPERCASE")]
    struct Rename {
        a: i32,
        b: i32,
    }

    assert_eq!(inline::<Rename>(), "{ A: number, B: number }");
}

#[test]
fn rename_special_char() {
    #[derive(Type)]
    struct RenameSerdeSpecialChar {
        #[specta(rename = "a/b")]
        b: i32,
    }

    assert_eq!(inline::<RenameSerdeSpecialChar>(), r#"{ "a/b": number }"#);
}
