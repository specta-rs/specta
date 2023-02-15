use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
struct Optional {
    a: Option<i32>,
    #[specta(optional)]
    b: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    c: Option<String>,
}

#[test]
fn test() {
    assert_ts!(
        Optional,
        "{ a: number | null; b?: number | null; c?: string | null }"
    );
}
