use specta::Type;

use crate::ts::assert_ts;

#[test]
fn list() {
    #[derive(Type)]
    #[specta(export = false)]
    struct List {
        #[allow(dead_code)]
        data: Option<Vec<u32>>,
    }

    assert_ts!(List, "{ data: number[] | null }");
}
