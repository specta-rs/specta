// use specta::{Generics, Type, TypeCollection};
// use specta_typescript as ts;

// #[derive(Type)]
// #[specta(untagged)]
// pub enum GenericType<T> {
//     Undefined,
//     Value(T),
// }

// #[derive(Type)]
// pub struct ActualType {
//     a: GenericType<String>,
// }

// #[test]
// fn test_generic_type_in_types() {
//     let mut types = TypeCollection::default();
//     ActualType::definition(&mut types, Generics::NONE);

//     assert_eq!(types.len(), 1);
//     let first = types.iter().next().unwrap().1;
//     // https://github.com/oscartbeaumont/specta/issues/171
//     assert_eq!(
//         ts::export_named_datatype(&Default::default(), first, &types).unwrap(),
//         "export type GenericType<T> = null | T".to_string()
//     );
// }
