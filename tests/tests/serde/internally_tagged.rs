use std::collections::HashMap;

use specta::Type;

use crate::ts::assert_ts_inline2;

// This type won't even compile with Serde macros
#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum A {
    // For internal tagging all variants must be a unit, named or *unnamed with a single variant*.
    A(String, u32),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum B {
    // Is not a map-type so invalid.
    A(String),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum C {
    // Is not a map-type so invalid.
    A(Vec<String>),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum D {
    // Is a map type so valid.
    A(HashMap<String, String>),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum E {
    // Null is valid (although it's not a map-type)
    A(()),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum F {
    // `FInner` is untagged so this is *only* valid if it is (which it is)
    A(FInner),
}

#[derive(Type)]
#[serde(collect = false, untagged)]
pub enum FInner {
    A(()),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum G {
    // `GInner` is untagged so this is *only* valid if it is (which it is not)
    A(GInner),
}

#[derive(Type)]
#[serde(collect = false, untagged)]
pub enum GInner {
    A(String),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum H {
    // `HInner` is transparent so this is *only* valid if it is (which it is)
    A(HInner),
}

#[derive(Type)]
#[serde(collect = false, transparent)]
pub struct HInner(());

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum I {
    // `IInner` is transparent so this is *only* valid if it is (which it is not)
    A(IInner),
}

#[derive(Type)]
#[serde(collect = false, transparent)]
pub struct IInner(String);

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum L {
    // Internally tag enum with inlined field that is itself internally tagged
    #[specta(inline)]
    A(LInner),
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum LInner {
    A,
    B,
}

#[derive(Type)]
#[serde(collect = false, tag = "type")]
pub enum M {
    // Internally tag enum with inlined field that is untagged
    // `MInner` is `null` - Test `B` in `untagged.rs`
    #[specta(inline)]
    A(MInner),
}

#[derive(Type)]
#[serde(collect = false, untagged)]
pub enum MInner {
    A,
    B,
}

#[test]
fn internally_tagged() {
    insta::assert_snapshot!(
        assert_ts_inline2::<A>().unwrap_err(),
        @"#[specta(tag = \"...\")] cannot be used with tuple variants\n"
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<B>().unwrap_err(),
        @"#[specta(tag = \"...\")] cannot be used with tuple variants\n"
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<C>().unwrap_err(),
        @"#[specta(tag = \"...\")] cannot be used with tuple variants\n"
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<D>().unwrap(),
        @r#"({ type: "A" } & { [key in string]: string })"#
    );

    insta::assert_snapshot!(assert_ts_inline2::<E>().unwrap(), @r#"({ type: "A" })"#);
    insta::assert_snapshot!(
        assert_ts_inline2::<F>().unwrap(),
        @r#"({ type: "A" } & FInner)"#
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<G>().unwrap_err(),
        @"#[specta(tag = \"...\")] cannot be used with tuple variants\n"
    );
    insta::assert_snapshot!(assert_ts_inline2::<H>().unwrap(), @r#"({ type: "A" })"#);
    insta::assert_snapshot!(
        assert_ts_inline2::<I>().unwrap_err(),
        @"#[specta(tag = \"...\")] cannot be used with tuple variants\n"
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<L>().unwrap(),
        @r#"({ type: "A" } & LInner)"#
    );
    insta::assert_snapshot!(assert_ts_inline2::<M>().unwrap(), @r#"({ type: "A" })"#);
}
