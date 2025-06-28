use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false)]
struct UnitA;

#[derive(Type)]
#[specta(collect = false)]
struct UnitB {}

#[derive(Type)]
#[specta(collect = false)]
struct UnitC();

#[test]
fn test() {
    assert_ts!(UnitA, "null");
    assert_ts!(UnitB, "Record<string, never>");
    assert_ts!(UnitC, "[]");
}
