use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum A {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum B {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a", content = "b")]
enum C {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum D {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Inner;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Inner2 {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct Inner3();

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum E {
    A(Inner),
    B(Inner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum F {
    A(Inner2),
    B(Inner2),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum G {
    A(Inner3),
    B(Inner3),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
enum H {
    #[specta(skip)]
    A(Inner3),
    B(Inner2),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, transparent)]
pub struct Demo(());

#[derive(Type, Serialize, Deserialize)]
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
    insta::assert_snapshot!(assert_ts_inline2::<A>().unwrap(), @r#"never"#);
    insta::assert_snapshot!(assert_ts_inline2::<B>().unwrap(), @r#"never"#);
    insta::assert_snapshot!(assert_ts_inline2::<C>().unwrap(), @r#"never"#);
    insta::assert_snapshot!(assert_ts_inline2::<D>().unwrap(), @r#"never"#);

    insta::assert_snapshot!(assert_ts_inline2::<E>().unwrap(), @r#"({ a: "A" }) | ({ a: "B" })"#);
    insta::assert_snapshot!(assert_ts_inline2::<F>().unwrap(), @r#"({ a: "A" }) | ({ a: "B" })"#);
    insta::assert_snapshot!(assert_ts_inline2::<G>().unwrap_err(), @"Attempted to export  with tagging but the variant is a tuple struct.\n");
    insta::assert_snapshot!(assert_ts_inline2::<H>().unwrap(), @r#"({ a: "B" })"#);
    insta::assert_snapshot!(assert_ts_inline2::<I>().unwrap(), @r#"({ a: "A" }) | ({ a: "B" })"#);
}
