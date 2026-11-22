//! repr(transparent) only describes the Rust memory layout.
//!
//! Serde serializes a named-field struct as an object even when the struct has
//! one field and a transparent layout. Deriving transparency from the layout
//! attribute therefore exported a scalar for a value that arrives on the wire
//! as an object.

use specta::{
    Type, Types,
    datatype::{DataType, Fields, Reference},
};

#[repr(transparent)]
#[derive(Type, serde::Serialize)]
#[specta(collect = false)]
struct TransparentLayout {
    x: u32,
}

#[repr(transparent)]
#[derive(Type, serde::Serialize)]
#[specta(collect = false)]
struct TransparentNewtype(u32);

fn declaration(types: &Types, ty: DataType) -> DataType {
    let DataType::Reference(Reference::Named(reference)) = ty else {
        panic!("expected a named reference, got {ty:?}");
    };
    types
        .get(&reference)
        .and_then(|ndt| ndt.ty.clone())
        .expect("the declaration is registered")
}

#[test]
fn repr_transparent_named_struct_keeps_its_object_shape() {
    let mut types = Types::default();
    let reference = TransparentLayout::definition(&mut types);
    let ty = declaration(&types, reference);

    let DataType::Struct(data) = ty else {
        panic!("expected a struct, got {ty:?}");
    };

    assert!(
        matches!(&data.fields, Fields::Named(fields) if fields.fields.iter().any(|(name, _)| name == "x")),
        "the named field must survive: {data:?}"
    );
}

#[test]
fn repr_transparent_alone_does_not_make_a_newtype_transparent() {
    let mut types = Types::default();
    let reference = TransparentNewtype::definition(&mut types);
    let ty = declaration(&types, reference);

    assert!(
        matches!(&ty, DataType::Struct(_)),
        "repr(transparent) must not imply specta(transparent): {ty:?}"
    );
}

// Serialization is the ground truth for the first test.
#[test]
fn the_wire_shape_is_an_object() {
    assert_eq!(
        serde_json::to_string(&TransparentLayout { x: 1 }).unwrap(),
        r#"{"x":1}"#
    );
}
