use std::cell::RefCell;

use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false)]
struct Simple {
    a: i32,
    b: String,
    c: (i32, String, RefCell<i32>),
    d: Vec<String>,
    e: Option<String>,
}

#[test]
fn test_def() {
    insta::assert_snapshot!(
        crate::ts::inline::<Simple>(&Default::default()).unwrap(),
        @"{ a: number; b: string; c: [number, string, number]; d: string[]; e: string | null }"
    );
}
