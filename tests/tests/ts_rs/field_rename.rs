#![allow(dead_code)]

use crate::ts::assert_ts;
use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Rename1 {
    a: i32,
    #[serde(rename = "bb")]
    b: i32,
}

#[test]
fn test() {
    assert_ts!(Rename1, "{ a: number; bb: number }")
}
