use specta::{
    ts::{ExportError, ExportPath},
    Type,
};

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
enum A {}

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum B {}

#[derive(Type)]
#[specta(export = false, tag = "a", content = "b")]
enum C {}

#[derive(Type)]
#[specta(export = false, untagged)]
enum D {}

#[derive(Type)]
#[specta(export = false)]
pub struct Inner;

#[derive(Type)]
#[specta(export = false)]
pub struct Inner2 {}

#[derive(Type)]
#[specta(export = false)]
pub struct Inner3();

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum E {
    A(Inner),
    B(Inner),
}

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum F {
    A(Inner2),
    B(Inner2),
}

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum G {
    A(Inner3),
    B(Inner3),
}

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum H {
    #[specta(skip)]
    A(Inner3),
    B(Inner2),
}

#[derive(Type)]
#[specta(transparent)]
pub struct Demo(());

#[derive(Type)]
#[specta(export = false, tag = "a")]
enum I {
    A(Demo),
    B(Demo),
}

// https://github.com/oscartbeaumont/specta/issues/174
#[test]
fn empty_enums() {
    // `never & { tag = "a" }` would coalesce to `never` so we don't need to include it.
    assert_ts!(A, "never");
    assert_ts!(B, "never");
    assert_ts!(C, "never");
    assert_ts!(D, "never");

    assert_ts!(E, "({ a: \"A\" }) | ({ a: \"B\" })");
    assert_ts!(F, "({ a: \"A\" }) | ({ a: \"B\" })");
    assert_ts!(error; G, ExportError::InvalidTaggedVariantContainingTupleStruct(ExportPath::new_unsafe("G")));
    assert_ts!(H, "({ a: \"B\" })");
    assert_ts!(I, "({ a: \"A\" }) | ({ a: \"B\" })");
}
