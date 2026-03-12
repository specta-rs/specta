use std::borrow::Cow;

use specta::{
    TypeCollection,
    datatype::{
        Attribute, AttributeMeta, DataType, Enum, EnumVariant, Field, NamedDataTypeBuilder, Struct,
    },
};
use specta_typescript::Typescript;

fn main() {
    let mut types = TypeCollection::default();

    let mut value_a = Field::new(DataType::Primitive(specta::datatype::Primitive::bool));
    value_a.set_attributes(vec![Attribute {
        path: "serde".into(),
        kind: AttributeMeta::Path("flatten".into()),
    }]);
    let payload_a = Struct::named()
        .field("value", value_a)
        .field(
            "__phase",
            Field::new(DataType::Primitive(specta::datatype::Primitive::String)),
        )
        .build();

    let mut value_b = Field::new(DataType::Primitive(specta::datatype::Primitive::i32));
    value_b.set_attributes(vec![Attribute {
        path: "serde".into(),
        kind: AttributeMeta::Path("flatten".into()),
    }]);
    let payload_b = Struct::named()
        .field("value", value_b)
        .field(
            "__phase",
            Field::new(DataType::Primitive(specta::datatype::Primitive::String)),
        )
        .build();

    let mut e = Enum::new();
    e.attributes_mut().push(Attribute {
        path: "serde".into(),
        kind: AttributeMeta::Path("untagged".into()),
    });
    e.variants_mut().push((
        Cow::Borrowed("A"),
        EnumVariant::unnamed().field(Field::new(payload_a)).build(),
    ));
    e.variants_mut().push((
        Cow::Borrowed("B"),
        EnumVariant::unnamed().field(Field::new(payload_b)).build(),
    ));

    NamedDataTypeBuilder::new("MyType", vec![], e.into()).build(&mut types);

    let result = Typescript::default().export(&types).unwrap();
    println!("{}", result);
}
