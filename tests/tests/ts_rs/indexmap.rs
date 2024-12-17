#![cfg(feature = "indexmap")]

use indexmap::{IndexMap, IndexSet};
use specta::Type;

use crate::ts::assert_ts;

#[test]
fn indexmap() {
    #[derive(Type)]
    #[specta(export = false)]
    #[allow(dead_code)]
    struct Indexes {
        map: IndexMap<String, String>,
        indexset: IndexSet<String>,
    }

    assert_ts!(
        Indexes,
        "{ map: Partial<{ [key in string]: string }>; indexset: string[] }"
    );
}
