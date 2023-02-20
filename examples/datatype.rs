//! This file show how to use an advanced API of Specta.
//! You probably shouldn't be using this in application code but if your building a library on Specta it will be useful.

use specta::{datatype::LiteralType, impl_location, sid, ts, DataType, DataTypeFrom};

#[derive(DataTypeFrom)]
pub struct MyEnum(pub Vec<DataType>);

fn main() {
    // let e = MyEnum(vec![
    //     DataType::Literal(LiteralType::String("A".to_string())),
    //     DataType::Literal(LiteralType::String("B".to_string())),
    // ]);

    // let e = DataType::Enum(specta::CustomDataType::Named {
    //     name: "TODO",
    //     sid: sid!("TODO"),
    //     impl_location: impl_location!(),
    //     comments: &[],
    //     export: None,
    //     deprecated: None,
    //     item: e.into(),
    // });

    // let ts = ts::export_datatype(&Default::default(), &e.into()).unwrap();

    // println!("{ts}");
    // assert_eq!(ts, "export type MyEnum = \"A\" | \"B\"");
}
