use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false)]
#[serde(rename_all = "lowercase")]
#[serde(rename = "SimpleEnum")]
enum RenamedEnum {
    #[serde(rename = "ASDF")]
    A,
    B,
    C,
}

#[test]
fn test_simple_enum() {
    assert_ts!(RenamedEnum, r#""ASDF" | "b" | "c""#)
}
