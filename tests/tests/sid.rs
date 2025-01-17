use specta::{Type, TypeCollection};

#[derive(Type)]
#[specta(export = false)]
pub struct A {}

#[derive(Type)]
#[specta(export = false)]
pub struct B {}

#[derive(Type)]
#[specta(export = false)]
pub struct C {}

#[derive(Type)]
#[specta(export = false)]
pub struct Z {}

#[derive(Type)]
#[specta(export = false)]
pub struct BagOfTypes {
    // Fields are outta order intentionally so we don't fluke the test
    a: A,
    z: Z,
    b: B,
    c: C,
}

#[test]
fn test_sid() {
    // TODO: This is so hard for an end-user to work with. Add some convenience API's!!!
    let mut type_map = TypeCollection::default();
    // We are calling this for it's side-effects
    BagOfTypes::definition(&mut type_map);

    // `TypeCollection` is a `BTreeMap` so it's sorted by SID. It should be sorted alphabetically by name
    assert_eq!(
        type_map
            .into_iter()
            .map(|(_, t)| t.name().clone())
            .collect::<Vec<_>>(),
        ["A", "B", "BagOfTypes", "C", "Z"]
    );
}
