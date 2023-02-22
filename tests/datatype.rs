use specta::{ts, DataType, DataTypeFrom, LiteralType, ObjectType, TupleType};

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

#[test]
fn test_datatype() {
    let val: TupleType = Procedures1(vec![
        LiteralType::String("A".to_string()).into(),
        LiteralType::String("B".to_string()).into(),
    ])
    .into();
    assert_eq!(
        ts::datatype(&Default::default(), &val.clone().to_anonymous()),
        Ok("\"A\" | \"B\"".into())
    );
    assert_eq!(
        ts::export_datatype(&Default::default(), &val.to_named("MyEnum")),
        Ok("export type MyEnum = \"A\" | \"B\"".into())
    );

    let val: ObjectType = Procedures2 {
        queries: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }
    .into();
    assert_eq!(
        ts::datatype(&Default::default(), &val.clone().to_anonymous()),
        Ok("{ queries: \"A\" | \"B\" }".into())
    );
    assert_eq!(
        ts::export_datatype(&Default::default(), &val.to_named("MyEnum")),
        Ok("export type MyEnum = { queries: \"A\" | \"B\" }".into())
    );

    let val: ObjectType = Procedures3 {
        queries: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }
    .into();
    assert_eq!(
        ts::datatype(&Default::default(), &val.clone().to_anonymous()),
        Ok("{ queries: \"A\" | \"B\" }".into())
    );
    assert_eq!(
        ts::export_datatype(&Default::default(), &val.to_named("MyEnum")),
        Ok("export type MyEnum = { queries: \"A\" | \"B\" }".into())
    );
    assert_ts!(Procedures3, "{ queries: string }");
}
