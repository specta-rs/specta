use specta::{DataType, DefOpts, PrimitiveType, Type};

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false, transparent)]
struct TupleStruct(String);

#[derive(Type)]
#[specta(export = false, transparent)]
struct GenericTupleStruct<T>(T);

#[derive(Type)]
#[specta(export = false, transparent)]
pub struct BracedStruct {
    a: String,
}

#[test]
fn transparent() {
    // We check the datatype layer can TS can look correct but be wrong!
    assert_eq!(
        TupleStruct::inline(
            DefOpts {
                parent_inline: false,
                type_map: &mut Default::default(),
            },
            &[]
        ),
        DataType::Primitive(PrimitiveType::String)
    );
    assert_eq!(
        GenericTupleStruct::<String>::inline(
            DefOpts {
                parent_inline: false,
                type_map: &mut Default::default(),
            },
            &[]
        ),
        DataType::Primitive(PrimitiveType::String)
    );
    assert_eq!(
        BracedStruct::inline(
            DefOpts {
                parent_inline: false,
                type_map: &mut Default::default(),
            },
            &[]
        ),
        DataType::Primitive(PrimitiveType::String)
    );

    assert_ts!(TupleStruct, "string");
    assert_ts!(GenericTupleStruct::<String>, "string");
    assert_ts!(BracedStruct, "string");
}
