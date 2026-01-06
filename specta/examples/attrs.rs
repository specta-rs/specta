// use specta::{Type, TypeCollection, datatype::DataType};

// #[derive(Type)]
// #[todo(bruh, foo = 42, bar = "hello")]
// #[todo("brother")] // TODO: Fix this
// #[todo(3.14159265)]
// struct Testing {}

// #[derive(Type)]
// #[todo("enum_test")]
// enum TestEnum {
//     #[todo("variant1")]
//     First,
//     #[todo("variant2")]
//     Second(String),
//     #[todo("variant3")]
//     Third { value: i32 },
// }

fn main() {
    // let mut types = TypeCollection::default();

    // // Test struct
    // let dt = Testing::definition(&mut types);
    // let _ndt = match &dt {
    //     DataType::Reference(reference) => reference.get(&types).unwrap(),
    //     _ => todo!(),
    // };

    // match _ndt.ty() {
    //     DataType::Struct(s) => {
    //         println!("Struct attributes: {:#?}", s.attributes());
    //     }
    //     _ => panic!("Expected struct"),
    // }

    // // Test enum
    // let dt_enum = TestEnum::definition(&mut types);
    // let _ndt_enum = match &dt_enum {
    //     DataType::Reference(reference) => reference.get(&types).unwrap(),
    //     _ => todo!(),
    // };

    // match _ndt_enum.ty() {
    //     DataType::Enum(e) => {
    //         println!("Enum attributes: {:#?}", e.attributes());
    //     }
    //     _ => panic!("Expected enum"),
    // }
}
