use specta::{
    datatype::{DataType, Primitive},
    Type, TypeCollection,
};

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(collect = false, transparent)]
struct TupleStruct(String);

#[repr(transparent)]
#[derive(Type)]
#[specta(collect = false)]
struct TupleStructWithRep(String);

#[derive(Type)]
#[specta(collect = false, transparent)]
struct GenericTupleStruct<T>(T);

#[derive(Type)]
#[specta(collect = false, transparent)]
pub struct BracedStruct {
    a: String,
}

// fn inline<T: Type>() -> DataType {
//     let mut types = TypeCollection::default();
//     specta::datatype::inline(T::definition(&mut types), &types)
// }

#[test]
fn transparent() {
    // TODO: Bring back these tests
    // We check the datatype layer can TS can look correct but be wrong!
    // assert_eq!(
    //     inline::<TupleStruct>(),
    //     DataType::Primitive(Primitive::String)
    // );
    // assert_eq!(
    //     inline::<TupleStructWithRep>(),
    //     DataType::Primitive(Primitive::String)
    // );
    // assert_eq!(
    //     inline::<GenericTupleStruct::<String>>(),
    //     DataType::Primitive(Primitive::String)
    // );
    // assert_eq!(
    //     inline::<BracedStruct>(),
    //     DataType::Primitive(Primitive::String)
    // );

    assert_ts!(TupleStruct, "string");
    assert_ts!(TupleStructWithRep, "string");
    assert_ts!(GenericTupleStruct::<String>, "string");
    assert_ts!(BracedStruct, "string");
}
