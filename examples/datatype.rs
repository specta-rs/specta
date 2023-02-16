//! This file show how to use an advanced API of Specta.
//! You probably shouldn't be using this in application code but if your building a library on Specta it will be useful.

use specta::{datatype::LiteralType, ts, DataTypeFrom, DataTypeItem};

// TODO: Do up some docs on the difference between working with `DataType` and `DataTypeItem` if gonna support both

#[derive(DataTypeFrom)]
pub struct MyEnum(pub Vec<DataTypeItem>);

// TODO: Also support this???
// #[derive(DataTypeFrom)]
// pub struct MyEnum(pub Vec<DataType>);

fn main() {
    let e = MyEnum(vec![
        DataTypeItem::Literal(LiteralType::String("A".to_string())),
        DataTypeItem::Literal(LiteralType::String("B".to_string())),
    ]);
    let ts = ts::export_datatype(&Default::default(), &e.into()).unwrap();

    println!("{ts}");
    assert_eq!(ts, "export type MyEnum = \"A\" | \"B\"");
}
