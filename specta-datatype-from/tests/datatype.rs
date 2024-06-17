use specta::{ts, DataType, DataTypeFrom, LiteralType, PrimitiveType, StructType};

use crate::ts::assert_ts;

#[derive(DataTypeFrom)]
struct Procedures1(Vec<DataType>);

#[derive(DataTypeFrom)]
struct Procedures2 {
    pub queries: Vec<DataType>,
}

// Testing using `DataTypeFrom` and `Type` together.
#[derive(DataTypeFrom, specta::Type)] // This derive bit gets passed into the macro
#[specta(export = false)]
#[specta(rename = "ProceduresDef")]
struct Procedures3 {
    #[specta(type = String)] // This is a lie but just for the test
    pub queries: Vec<DataType>,
}

// Using a type implementing `DataTypeFrom` as a field.
#[derive(DataTypeFrom)]
struct Procedures4 {
    pub queries: Vec<Procedures2>,
}

// Using a type implementing `DataTypeFrom` as a field.
#[derive(DataTypeFrom)]
struct Procedures5(Vec<Procedures2>);

#[derive(DataTypeFrom)]
struct Procedures7();

#[derive(DataTypeFrom)]
struct Procedures8 {}

#[derive(DataTypeFrom)]
struct Procedures9(DataType, DataType);

#[derive(DataTypeFrom)]
struct Procedures10 {
    #[specta(rename = "b")]
    pub a: DataType,
}

#[test]
fn test_datatype() {
    let val: DataType = Procedures1(vec![
        LiteralType::String("A".to_string()).into(),
        LiteralType::String("B".to_string()).into(),
    ])
    .into();
    assert_eq!(
        ts::datatype(&Default::default(), &val, &Default::default()),
        Ok("\"A\" | \"B\"".into())
    );
    assert_eq!(
        ts::export_named_datatype(
            &Default::default(),
            &val.to_named("MyEnum"),
            &Default::default()
        ),
        Ok("export type MyEnum = \"A\" | \"B\"".into())
    );

    let val: StructType = Procedures2 {
        queries: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }
    .into();
    assert_eq!(
        ts::datatype(
            &Default::default(),
            &val.clone().to_anonymous(),
            &Default::default()
        ),
        Ok("{ queries: \"A\" | \"B\" }".into())
    );
    assert_eq!(
        ts::export_named_datatype(
            &Default::default(),
            &val.to_named("MyEnum"),
            &Default::default()
        ),
        Ok("export type MyEnum = { queries: \"A\" | \"B\" }".into())
    );

    let val: StructType = Procedures3 {
        queries: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }
    .into();
    assert_eq!(
        ts::datatype(
            &Default::default(),
            &val.clone().to_anonymous(),
            &Default::default()
        ),
        Ok("{ queries: \"A\" | \"B\" }".into())
    );
    assert_eq!(
        ts::export_named_datatype(
            &Default::default(),
            &val.to_named("MyEnum"),
            &Default::default()
        ),
        Ok("export type MyEnum = { queries: \"A\" | \"B\" }".into())
    );
    assert_ts!(Procedures3, "{ queries: string }");

    let val: StructType = Procedures4 {
        queries: vec![Procedures2 {
            queries: vec![
                LiteralType::String("A".to_string()).into(),
                LiteralType::String("B".to_string()).into(),
            ],
        }],
    }
    .into();
    assert_eq!(
        ts::datatype(
            &Default::default(),
            &val.clone().to_anonymous(),
            &Default::default()
        ),
        Ok("{ queries: { queries: \"A\" | \"B\" } }".into())
    );

    let val: DataType = Procedures5(vec![Procedures2 {
        queries: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }])
    .into();
    assert_eq!(
        ts::datatype(&Default::default(), &val, &Default::default()),
        Ok("{ queries: \"A\" | \"B\" }".into())
    );

    let val: DataType = Procedures7().into();
    assert_eq!(
        ts::datatype(&Default::default(), &val, &Default::default()),
        Ok("null".into()) // This is equivalent of `()` Because this is a `TupleType` not an `EnumType`.
    );

    let val: StructType = Procedures8 {}.into();
    assert_eq!(
        ts::datatype(
            &Default::default(),
            &val.clone().to_anonymous(),
            &Default::default()
        ),
        Ok("Record<string, never>".into())
    );

    let val: DataType =
        Procedures9(DataType::Any, DataType::Primitive(PrimitiveType::String)).into();
    assert_eq!(
        ts::datatype(&Default::default(), &val, &Default::default()),
        Ok("[any, string]".into())
    );

    let val: StructType = Procedures10 { a: DataType::Any }.into();
    assert_eq!(
        ts::datatype(
            &Default::default(),
            &val.clone().to_anonymous(),
            &Default::default()
        ),
        Ok("{ b: any }".into())
    );
}
