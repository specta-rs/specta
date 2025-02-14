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
    let mut type_map = TypeCollection::default();
    ActualType::definition(&mut type_map);

    assert_eq!(type_map.len(), 2);
    let mut iter = type_map.into_iter();

    let first = iter.next().unwrap().1;
    // https://github.com/oscartbeaumont/specta/issues/171
    assert_eq!(
        ts::legacy::export_named_datatype(&Default::default(), first, &type_map).unwrap(),
        "export type ActualType = { a: GenericType<string> }".to_string()
    );

    let second = iter.next().unwrap().1;
    assert_eq!(
        ts::legacy::export_named_datatype(&Default::default(), second, &type_map).unwrap(),
        "export type GenericType<T> = null | T".to_string()
    );
}
