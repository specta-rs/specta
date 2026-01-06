use specta::{Type, TypeCollection};
use specta_typescript as ts;

#[derive(Type)]
#[specta(untagged)]
pub enum GenericType<T> {
    Undefined,
    Value(T),
}

#[derive(Type)]
pub struct ActualType {
    a: GenericType<String>,
}

#[test]
fn test_generic_type_in_type_map() {
    let mut types = TypeCollection::default();
    ActualType::definition(&mut types);

    insta::assert_snapshot!(types.len(), @"2");
    let mut iter = types.into_sorted_iter();

    let first = iter.next().unwrap();
    // https://github.com/oscartbeaumont/specta/issues/171
    insta::assert_snapshot!(specta_typescript::primitives::export(&Default::default(), &types, &first).unwrap(), @"export type ActualType = { a: GenericType<string> };");

    let second = iter.next().unwrap();
    insta::assert_snapshot!(specta_typescript::primitives::export(&Default::default(), &types, &second).unwrap(), @"export type GenericType<T> = null | T;");
}
