use specta::{Any, Unknown};

use crate::ts::assert_ts;

#[test]
fn static_types() {
    assert_ts!(Any, "any");
    assert_ts!(Unknown, "unknown");

    assert_ts!(Any<String>, "any");
    assert_ts!(Unknown<String>, "unknown");
}
