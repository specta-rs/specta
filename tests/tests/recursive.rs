use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use specta::Type;
use specta_serde::Error as SerdeError;
use specta_typescript::Error;

#[derive(Type)]
#[specta(collect = false)]
pub struct Recursive {
    demo: Box<Recursive>,
}

#[derive(Type)]
#[specta(transparent, collect = false)]
pub struct RecursiveMapKeyTrick(RecursiveMapKey);

#[derive(Type)]
#[specta(collect = false)]
pub struct RecursiveMapKey {
    demo: HashMap<RecursiveMapKeyTrick, String>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct RecursiveMapValue {
    demo: HashMap<String, RecursiveMapValue>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct RecursiveInline {
    #[serde(flatten)]
    demo: Box<RecursiveInline>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(transparent, collect = false)]
pub struct RecursiveTransparent(Box<RecursiveInline>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum RecursiveInEnum {
    A {
        #[serde(flatten)]
        demo: Box<RecursiveInEnum>,
    },
}

#[test]
fn test_recursive_types() {
    insta::assert_snapshot!(crate::ts::inline::<Recursive>(&Default::default()).unwrap(), @"{ demo: Recursive }");
    insta::assert_snapshot!(crate::ts::export::<Recursive>(&Default::default()).unwrap(), @"export type Recursive = { demo: Recursive };");

    // Just check it doesn't overflow while doing this check
    insta::assert_snapshot!(crate::ts::inline::<RecursiveMapKey>(&Default::default()).unwrap_err(), @"Detect invalid Serde type: A map key must be a 'string' or 'number' type\n");
    insta::assert_snapshot!(format!("{:?}", crate::ts::export::<RecursiveMapKey>(&Default::default()).unwrap_err()), @"Serde(InvalidMapKey)");

    insta::assert_snapshot!(crate::ts::inline::<RecursiveMapValue>(&Default::default()).unwrap(), @"{ demo: { [key in string]: RecursiveMapValue } }");
    insta::assert_snapshot!(crate::ts::export::<RecursiveMapValue>(&Default::default()).unwrap(), @"export type RecursiveMapValue = { demo: { [key in string]: RecursiveMapValue } };");
}

#[test]
#[should_panic]
fn test_recursive_types_panic1() {
    crate::ts::inline::<RecursiveTransparent>(&Default::default()).unwrap();
}

#[test]
#[should_panic]
fn test_recursive_types_panic2() {
    crate::ts::export::<RecursiveTransparent>(&Default::default()).unwrap();
}

#[test]
#[should_panic]
fn test_recursive_types_panic3() {
    crate::ts::inline::<RecursiveInEnum>(&Default::default()).unwrap();
}

#[test]
#[should_panic]
fn test_recursive_types_panic4() {
    crate::ts::export::<RecursiveInEnum>(&Default::default()).unwrap();
}
