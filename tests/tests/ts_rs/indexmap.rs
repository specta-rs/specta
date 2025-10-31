#![cfg(feature = "indexmap")]
// TODO: This feature guard is bogus because this test package doesn't define any features.

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
