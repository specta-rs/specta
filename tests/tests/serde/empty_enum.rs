use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type)]
#[specta(collect = false)]
enum A {}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum B {}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a", content = "b")]
enum C {}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
enum D {}

#[derive(Type)]
#[specta(collect = false)]
pub struct Inner;

#[derive(Type)]
#[specta(collect = false)]
pub struct Inner2 {}

#[derive(Type)]
#[specta(collect = false)]
pub struct Inner3();

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum E {
    A(Inner),
    B(Inner),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum F {
    A(Inner2),
    B(Inner2),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum G {
    A(Inner3),
    B(Inner3),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum H {
    #[specta(skip)]
    A(Inner3),
    B(Inner2),
}

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct Demo(());

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum I {
    A(Demo),
    B(Demo),
}

// https://github.com/oscartbeaumont/specta/issues/174
#[test]
fn empty_enums() {
    // `never & { tag = "a" }` would coalesce to `never` so we don't need to include it.
    assert_eq!(assert_ts_inline2::<A>(), Ok(r#"never"#.into()));
    assert_eq!(assert_ts_inline2::<B>(), Ok(r#"never"#.into()));
    assert_eq!(assert_ts_inline2::<C>(), Ok(r#"never"#.into()));
    assert_eq!(assert_ts_inline2::<D>(), Ok(r#"never"#.into()));

    assert_eq!(
        assert_ts_inline2::<E>(),
        Ok(r#"({ a: "A" }) | ({ a: "B" })"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<F>(),
        Ok(r#"({ a: "A" }) | ({ a: "B" })"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<G>(),
        Err("Attempted to export  with tagging but the variant is a tuple struct.\n".into())
    );
    assert_eq!(assert_ts_inline2::<H>(), Ok(r#"({ a: "B" })"#.into()));
    assert_eq!(
        assert_ts_inline2::<I>(),
        Ok(r#"({ a: "A" }) | ({ a: "B" })"#.into())
    );
}
