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

    assert_eq!(types.len(), 2);
    let mut iter = types.into_sorted_iter();

    let first = iter.next().unwrap();
    // https://github.com/oscartbeaumont/specta/issues/171
    assert_eq!(
        specta_typescript::primitives::export(&Default::default(), &types, &first)
            // Allows matching the value. Implementing `PartialEq` on it is really hard.
            .map_err(|e| e.to_string()),
        Ok("export type ActualType = { a: GenericType<string> };".into())
    );

    let second = iter.next().unwrap();
    assert_eq!(
        specta_typescript::primitives::export(&Default::default(), &types, &second)
            // Allows matching the value. Implementing `PartialEq` on it is really hard.
            .map_err(|e| e.to_string()),
        Ok("export type GenericType<T> = null | T;".into())
    );
}
