use std::collections::HashMap;

use specta::Type;

use crate::ts::assert_ts_inline2;

// This type won't even compile with Serde macros
#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum A {
    // For internal tagging all variants must be a unit, named or *unnamed with a single variant*.
    A(String, u32),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum B {
    // Is not a map-type so invalid.
    A(String),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum C {
    // Is not a map-type so invalid.
    A(Vec<String>),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum D {
    // Is a map type so valid.
    A(HashMap<String, String>),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum E {
    // Null is valid (although it's not a map-type)
    A(()),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum F {
    // `FInner` is untagged so this is *only* valid if it is (which it is)
    A(FInner),
}

#[derive(Type)]
#[serde(export = false, untagged)]
pub enum FInner {
    A(()),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum G {
    // `GInner` is untagged so this is *only* valid if it is (which it is not)
    A(GInner),
}

#[derive(Type)]
#[serde(export = false, untagged)]
pub enum GInner {
    A(String),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum H {
    // `HInner` is transparent so this is *only* valid if it is (which it is)
    A(HInner),
}

#[derive(Type)]
#[serde(export = false, transparent)]
pub struct HInner(());

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum I {
    // `IInner` is transparent so this is *only* valid if it is (which it is not)
    A(IInner),
}

#[derive(Type)]
#[serde(export = false, transparent)]
pub struct IInner(String);

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum L {
    // Internally tag enum with inlined field that is itself internally tagged
    #[specta(inline)]
    A(LInner),
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum LInner {
    A,
    B,
}

#[derive(Type)]
#[serde(export = false, tag = "type")]
pub enum M {
    // Internally tag enum with inlined field that is untagged
    // `MInner` is `null` - Test `B` in `untagged.rs`
    #[specta(inline)]
    A(MInner),
}

#[derive(Type)]
#[serde(export = false, untagged)]
pub enum MInner {
    A,
    B,
}

#[test]
fn internally_tagged() {
    assert_eq!(
        assert_ts_inline2::<A>(),
        Err("#[specta(tag = \"...\")] cannot be used with tuple variants\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<B>(),
        Err("#[specta(tag = \"...\")] cannot be used with tuple variants\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<C>(),
        Err("#[specta(tag = \"...\")] cannot be used with tuple variants\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<D>(),
        Ok(r#"({ type: "A" } & Partial<{ [key in string]: string }>)"#.into())
    );

    assert_eq!(assert_ts_inline2::<E>(), Ok(r#"({ type: "A" })"#.into()));
    assert_eq!(
        assert_ts_inline2::<F>(),
        Ok(r#"({ type: "A" } & FInner)"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<G>(),
        Err("#[specta(tag = \"...\")] cannot be used with tuple variants\n".into())
    );
    assert_eq!(assert_ts_inline2::<H>(), Ok(r#"({ type: "A" })"#.into()));
    assert_eq!(
        assert_ts_inline2::<I>(),
        Err("#[specta(tag = \"...\")] cannot be used with tuple variants\n".into())
    );
    assert_eq!(
        assert_ts_inline2::<L>(),
        Ok(r#"({ type: "A" } & ({ type: "A" } | { type: "B" }))"#.into())
    );
    assert_eq!(assert_ts_inline2::<M>(), Ok(r#"({ type: "A" })"#.into()));
}
