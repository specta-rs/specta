#![allow(dead_code)]

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Rename1 {
    a: i32,
    #[specta(rename = "bb")]
    b: i32,
}

#[test]
fn test() {
    insta::assert_snapshot!(crate::ts::inline::<Rename1>(&Default::default()).unwrap(), @"{ a: number; bb: number }");
}
