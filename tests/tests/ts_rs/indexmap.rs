#![cfg(feature = "indexmap")]
// TODO: This feature guard is bogus because this test package doesn't define any features.

use indexmap::{IndexMap, IndexSet};
use specta::Type;

#[test]
fn indexmap() {
    #[derive(Type)]
    #[specta(collect = false)]
    #[allow(dead_code)]
    struct Indexes {
        map: IndexMap<String, String>,
        indexset: IndexSet<String>,
    }

    insta::assert_snapshot!(crate::ts::inline::<Indexes>(&Default::default()).unwrap(), @"{ map: Partial<{ [key in string]: string }>; indexset: string[] }");
}
