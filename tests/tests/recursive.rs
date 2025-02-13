use std::collections::HashMap;

use specta::Type;
use specta_serde::Error as SerdeError;
use specta_typescript::Error;

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

#[derive(Type)]
#[specta(export = false)]
pub enum RecursiveInEnum {
    A {
        #[specta(flatten)]
        demo: Box<RecursiveInEnum>,
    },
}

#[test]
fn test_recursive_types() {
    assert_ts!(Recursive, "{ demo: Recursive }");
    assert_ts_export!(Recursive, "export type Recursive = { demo: Recursive }");

    // Just check it doesn't overflow while doing this check
    assert_ts!(error; RecursiveMapKey, Error::Serde(SerdeError::InvalidMapKey));
    assert_ts_export!(
        error;
        RecursiveMapKey,
        Error::Serde(SerdeError::InvalidMapKey)
    );

    assert_ts!(
        RecursiveMapValue,
        "{ demo: Partial<{ [key in string]: RecursiveMapValue }> }"
    );
    assert_ts_export!(
        RecursiveMapValue,
        "export type RecursiveMapValue = { demo: Partial<{ [key in string]: RecursiveMapValue }> }"
    );
}

#[test]
#[should_panic]
fn test_recursive_types_panic1() {
    assert_ts!(RecursiveTransparent, "");
}

#[test]
#[should_panic]
fn test_recursive_types_panic2() {
    assert_ts_export!(RecursiveTransparent, "");
}

#[test]
#[should_panic]
fn test_recursive_types_panic3() {
    assert_ts!(RecursiveInEnum, "");
}

#[test]
#[should_panic]
fn test_recursive_types_panic4() {
    assert_ts_export!(RecursiveInEnum, "");
}
