use specta::{
    Type, TypeCollection,
    datatype::{DataType, Primitive},
};

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

    insta::assert_snapshot!(crate::ts::inline::<TupleStruct>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(crate::ts::inline::<TupleStructWithRep>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(crate::ts::inline::<GenericTupleStruct::<String>>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(crate::ts::inline::<BracedStruct>(&Default::default()).unwrap(), @"string");
}
