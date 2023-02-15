use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
struct UnitA;

#[derive(Type)]
#[specta(export = false)]
struct UnitB {}

#[derive(Type)]
#[specta(export = false)]
struct UnitC();

#[test]
fn test() {
    assert_ts!(UnitA, "null");
    assert_ts!(UnitB, "null");
    assert_ts!(UnitC, "null");
}
