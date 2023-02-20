// use specta::{ts, DataType, DataTypeFrom, LiteralType};

// use crate::ts::assert_ts;

// #[derive(DataTypeFrom)]
// struct Procedures1 {
//     pub queries: Vec<DataType>,
// }

// // Testing using `DataTypeFrom` and `Type` together.
// #[derive(DataTypeFrom, specta::Type)] // This derive bit gets passed into the macro
// #[specta(export = false)]
// #[specta(rename = "ProceduresDef")]
// struct Procedures2 {
//     #[specta(type = String)] // This is a lie but just for the test
//     pub queries: Vec<DataTyp>,
// }

// #[test]
// fn test_datatype() {
//     let dt: DataType = Procedures1 { queries: vec![] }.into();
//     assert_eq!(
//         &ts::datatype(&Default::default(), &dt),
//         Ok("{ queries: never }".into())
//     );

//     let dt: DataType = Procedures1 {
//         queries: vec![
//             DataTypeItem::Literal(LiteralType::String("A".to_string())),
//             DataTypeItem::Literal(LiteralType::String("B".to_string())),
//             DataTypeItem::Literal(LiteralType::bool(true)),
//             DataTypeItem::Literal(LiteralType::i32(42)),
//         ],
//     }
//     .into();
//     assert_eq!(
//         &ts::datatype(&Default::default(), &dt),
//         Ok(r#"{ queries: "A" | "B" | true | 42 }"#.into())
//     );

//     let dt: DataType = Procedures2 { queries: vec![] }.into();
//     assert_eq!(
//         &ts::datatype(&Default::default(), &dt),
//         Ok("{ queries: never }".into())
//     );

//     assert_ts!(Procedures2, "{ queries: string }");
// }
