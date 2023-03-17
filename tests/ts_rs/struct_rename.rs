use crate::ts::assert_ts;

use specta::Type;

#[test]
fn rename_all() {
    #[derive(Type)]
    #[specta(export = false)]
    #[specta(rename_all = "UPPERCASE")]
    struct Rename {
        a: i32,
        b: i32,
    }

    assert_ts!(Rename, "{ A: number; B: number }");
}

// #[test]
// fn serde_rename_special_char() {
//     #[derive(serde::Serialize, Type)]
//     #[specta(export = false)]
//     struct RenameSerdeSpecialChar {
//         #[serde(rename = "a/b")]
//         b: i32,
//     }

//     assert_ts!(RenameSerdeSpecialChar, r#"{ "a/b": number }"#);
// }
