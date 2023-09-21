#[allow(deprecated)]
#[derive(Type)]
#[specta(export = false)]
#[deprecated]
struct DeprecatedType {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
#[deprecated = "Look at you big man using a deprecation message"]
struct DeprecatedTypeWithMsg {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
#[deprecated(note = "Look at you big man using a deprecation message")]
struct DeprecatedTypeWithMsg2 {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct DeprecatedFields {
    a: i32,
    #[deprecated]
    b: String,
    #[deprecated = "This field is cringe!"]
    c: String,
    #[deprecated(note = "This field is cringe!")]
    d: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct DeprecatedTupleVariant(
    #[deprecated] String,
    #[deprecated = "Nope"] String,
    #[deprecated(note = "Nope")] i32,
);

#[test]
fn test_deprecated_types() {
    assert_ts_export!(DeprecatedType, "");
    assert_ts_export!(DeprecatedTypeWithMsg, "");
    assert_ts_export!(DeprecatedTypeWithMsg2, "");
    assert_ts_export!(DeprecatedFields, "");
    assert_ts_export!(DeprecatedTupleVariant, "");
}
