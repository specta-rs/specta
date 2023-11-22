use std::collections::HashMap;

use specta::{ts::ExportError, SerdeError, Type};

use crate::ts::{assert_ts, assert_ts_export};

#[derive(Type)]
#[specta(export = false)]
pub struct Recursive {
    demo: Box<Recursive>,
}

#[derive(Type)]
#[specta(transparent, export = false)]
pub struct RecursiveMapKeyTrick(RecursiveMapKey);

#[derive(Type)]
#[specta(export = false)]
pub struct RecursiveMapKey {
    demo: HashMap<RecursiveMapKeyTrick, String>,
}

#[derive(Type)]
#[specta(export = false)]
pub struct RecursiveMapValue {
    demo: HashMap<String, RecursiveMapValue>,
}

#[derive(Type)]
#[specta(export = false)]
pub struct RecursiveInline {
    #[specta(flatten)]
    demo: Box<RecursiveInline>,
}

#[derive(Type)]
#[specta(transparent, export = false)]
pub struct RecursiveTransparent(Box<RecursiveInline>);

#[test]
fn test_recursive_types() {
    assert_ts!(Recursive, "{ demo: Recursive }");
    assert_ts_export!(Recursive, "export type Recursive = { demo: Recursive }");

    // Just check it doesn't overflow while doing this check
    assert_ts!(error; RecursiveMapKey, ExportError::Serde(SerdeError::InvalidMapKey));
    assert_ts_export!(
        error;
        RecursiveMapKey,
        ExportError::Serde(SerdeError::InvalidMapKey)
    );

    assert_ts!(
        RecursiveMapValue,
        "{ demo: { [key in string]: RecursiveMapValue } }"
    );
    assert_ts_export!(
        RecursiveMapValue,
        "export type RecursiveMapValue = { demo: { [key in string]: RecursiveMapValue } }"
    );

    assert_ts!(RecursiveTransparent, "");
    assert_ts_export!(RecursiveTransparent, "");
}
