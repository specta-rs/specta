use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false)]
struct ExampleId(pub ulid::Ulid);

#[test]
fn ulid() {
    assert_ts!(ulid::Ulid, "string");
    assert_ts!(ExampleId, "string");
}
