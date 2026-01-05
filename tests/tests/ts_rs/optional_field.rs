use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Optional {
    a: Option<i32>,
    #[specta(optional)]
    b: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    c: Option<String>,
    #[serde(default)]
    d: bool,
}

#[test]
fn test() {
    insta::assert_snapshot!(crate::ts::inline::<Optional>(&Default::default()).unwrap(), @"{ a: number | null; b?: number | null; c?: string | null; d?: boolean }");
}
