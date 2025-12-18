use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false)]
struct Skip {
    a: i32,
    b: i32,
    #[specta(skip)]
    c: String,
}

#[test]
fn test_def() {
    assert_ts!(Skip, "{ a: number; b: number }");
}
