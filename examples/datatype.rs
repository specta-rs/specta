//! This file show how to use an advanced API of Specta.
//! You probably shouldn't be using this in application code but if you're building a library on Specta it will be useful.

use specta::{datatype::LiteralType, ts, DataType, DataTypeFrom, ObjectType, TupleType};

#[derive(Clone, DataTypeFrom)]
pub struct MyEnum(pub Vec<DataType>);

#[derive(Clone, DataTypeFrom)]
pub struct MyObject {
    a: Vec<DataType>,
}

fn main() {
    //
    // Enum
    //
    let val: TupleType = MyEnum(vec![
        LiteralType::String("A".to_string()).into(),
        LiteralType::String("B".to_string()).into(),
    ])
    .into();

    let anon = val.clone().to_anonymous();
    let named = val.to_named("MyEnum");

    let anon = ts::datatype(&Default::default(), &anon, &Default::default()).unwrap();
    let named_export =
        ts::export_datatype(&Default::default(), &named, &Default::default()).unwrap();

    println!("anonymous enum: {anon}");
    println!("named enum export: {named_export}");

    assert_eq!(anon, "\"A\" | \"B\"");
    assert_eq!(named_export, "export type MyEnum = \"A\" | \"B\"");

    //
    // Object
    //
    let val: ObjectType = MyObject {
        a: vec![
            LiteralType::String("A".to_string()).into(),
            LiteralType::String("B".to_string()).into(),
        ],
    }
    .into();

    let anon = val.clone().to_anonymous();
    let named = val.to_named("MyObject");

    let anon = ts::datatype(&Default::default(), &anon, &Default::default()).unwrap();
    let named_export =
        ts::export_datatype(&Default::default(), &named, &Default::default()).unwrap();

    println!("anonymous object: {anon}");
    println!("named object export: {named_export}");

    assert_eq!(anon, "{ a: \"A\" | \"B\" }");
    assert_eq!(named_export, "export type MyObject = { a: \"A\" | \"B\" }");
}
