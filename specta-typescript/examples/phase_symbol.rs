use std::borrow::Cow;

use specta::{
    Type, TypeCollection,
    datatype::{
        DataType, Enum, EnumVariant, Field, Fields, NamedDataTypeBuilder, Struct, UnnamedFields,
    },
};
use specta_typescript::Typescript;

fn main() {
    let mut types = TypeCollection::default();

    // If we flatten we get `T & U`. Specially we can maintain `unamed`.
    // If `U` (the phase marker) is inline container it will get promoted into being inline?

    let s = Struct::named().field(
        "__phase", // TODO: Make this the symbol
        // TODO: We need a way of doing literals for this :(
        Field::new(DataType::Primitive(specta::datatype::Primitive::String)),
    );
    let mut marker = Field::new(s.build());
    marker.set_inline(true);
    // marker.set_flatten(true);

    let mut e = Enum::new();
    e.variants_mut().push((
        Cow::Borrowed("A"),
        EnumVariant::unnamed()
            .field(Field::new(DataType::Primitive(
                specta::datatype::Primitive::bool,
            )))
            // .field(marker.clone())
            .build(),
    ));
    e.variants_mut().push((
        Cow::Borrowed("B"),
        EnumVariant::unnamed()
            .field(Field::new(DataType::Primitive(
                specta::datatype::Primitive::i32,
            )))
            // .field(marker)
            .build(),
    ));

    NamedDataTypeBuilder::new("MyType", vec![], e.into()).build(&mut types);

    let result = Typescript::default().export(&types).unwrap();
    println!("{}", result);
}
