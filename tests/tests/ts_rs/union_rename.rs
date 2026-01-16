use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Serialize, Deserialize)]
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
    insta::assert_snapshot!(crate::ts::inline::<RenamedEnum>(&Default::default()).unwrap(), @r#""ASDF" | "b" | "c""#);
}
