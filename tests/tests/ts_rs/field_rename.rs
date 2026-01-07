#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Rename1 {
    a: i32,
    #[serde(rename = "bb")]
    b: i32,
}

#[test]
fn test() {
    insta::assert_snapshot!(crate::ts::inline::<Rename1>(&Default::default()).unwrap(), @"{ a: number; bb: number }");
}
