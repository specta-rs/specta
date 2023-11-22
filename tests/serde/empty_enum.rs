use specta::Type;

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

#[test]
fn empty_enums() {
    // `never & { tag = "a" }` would collease to `never` so we don't need to include it.
    assert_ts!(A, "never");
    assert_ts!(B, "never");
    assert_ts!(C, "never");
    assert_ts!(D, "never");
}
