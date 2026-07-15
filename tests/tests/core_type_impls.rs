use std::time::SystemTime;

use specta::{
    Type, Types,
    datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference},
};

fn inline_data_type(data_type: &DataType) -> &DataType {
    match data_type {
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => inline_data_type(dt),
            _ => data_type,
        },
        _ => data_type,
    }
}

fn named_data_type<T: Type>(name: &str) -> DataType {
    let mut types = Types::default();
    T::definition(&mut types);
    types
        .into_unsorted_iter()
        .find(|data_type| data_type.name == name)
        .and_then(|data_type| data_type.ty.clone())
        .unwrap_or_else(|| panic!("missing named datatype {name}"))
}

fn assert_untagged(data_type: &DataType) {
    let DataType::Enum(data_type) = inline_data_type(data_type) else {
        panic!("expected enum, got {data_type:#?}");
    };
    assert_eq!(
        data_type
            .attributes
            .get_named_as::<bool>("serde:container:untagged"),
        Some(&true)
    );
}

fn assert_variable_list<T: Type>(element: Primitive) {
    let mut types = Types::default();
    let data_type = T::definition(&mut types);
    let DataType::List(list) = inline_data_type(&data_type) else {
        panic!("expected list, got {data_type:#?}");
    };
    assert_eq!(list.length, None);
    assert_eq!(list.ty.as_ref(), &DataType::Primitive(element));
}

#[test]
fn nested_passthrough_wrappers_use_concrete_generics() {
    let mut types = Types::default();
    let data_type = <Box<Box<u32>>>::definition(&mut types);

    assert_eq!(
        inline_data_type(&data_type),
        &DataType::Primitive(Primitive::u32)
    );
    assert!(!format!("{data_type:#?}").contains("Recursive"));
}

#[derive(Type)]
#[specta(collect = false)]
struct RecursiveNode {
    child: Option<Box<RecursiveNode>>,
}

#[test]
fn concrete_wrapper_generics_preserve_named_recursion() {
    let data_type = named_data_type::<RecursiveNode>("RecursiveNode");
    let debug = format!("{data_type:#?}");

    assert!(debug.contains("Reference"), "{debug}");
    assert!(!debug.contains("Recursive("), "{debug}");
}

#[test]
fn serialized_value_enums_are_untagged() {
    assert_untagged(&named_data_type::<serde_json::Value>("Value"));

    let mut types = Types::default();
    assert_untagged(&serde_yaml::Value::definition(&mut types));

    let mut types = Types::default();
    assert_untagged(&serde_yaml::Number::definition(&mut types));

    let mut types = Types::default();
    assert_untagged(&toml::Value::definition(&mut types));

    assert_untagged(&named_data_type::<bson::Bson>("Bson"));
}

#[test]
fn result_uses_externally_tagged_enum_shape() {
    let DataType::Enum(result) = named_data_type::<Result<u32, String>>("Result") else {
        panic!("Result should be modeled as an enum");
    };

    assert_eq!(
        result
            .variants
            .iter()
            .map(|(name, _)| name.as_ref())
            .collect::<Vec<_>>(),
        ["Ok", "Err"]
    );
    assert!(result.attributes.is_empty());
    assert!(result
        .variants
        .iter()
        .all(|(_, variant)| matches!(&variant.fields, Fields::Unnamed(fields) if fields.fields.len() == 1)));
}

#[test]
fn system_time_uses_serde_field_shape() {
    let DataType::Struct(system_time) = named_data_type::<SystemTime>("SystemTime") else {
        panic!("SystemTime should be modeled as a struct");
    };
    let Fields::Named(fields) = system_time.fields else {
        panic!("SystemTime should have named fields");
    };

    assert_eq!(
        fields
            .fields
            .iter()
            .map(|(name, field)| (name.as_ref(), field.ty.as_ref()))
            .collect::<Vec<_>>(),
        [
            (
                "secs_since_epoch",
                Some(&DataType::Primitive(Primitive::u64))
            ),
            (
                "nanos_since_epoch",
                Some(&DataType::Primitive(Primitive::u32))
            ),
        ]
    );
}

#[test]
fn bounded_sequence_containers_are_variable_length_lists() {
    assert_variable_list::<heapless::Vec<i32, 8>>(Primitive::i32);
    assert_variable_list::<heapless::Deque<i32, 8>>(Primitive::i32);
    assert_variable_list::<heapless::HistoryBuf<i32, 8>>(Primitive::i32);
    assert_variable_list::<heapless::BinaryHeap<i32, heapless::binary_heap::Min, 8>>(
        Primitive::i32,
    );
    assert_variable_list::<arrayvec::ArrayVec<i32, 8>>(Primitive::i32);
}

#[test]
fn smallvec_uses_its_item_type() {
    assert_variable_list::<smallvec::SmallVec<[i32; 8]>>(Primitive::i32);
}

#[test]
fn nested_smallvecs_do_not_trigger_false_recursion() {
    let mut types = Types::default();
    let data_type = smallvec::SmallVec::<[smallvec::SmallVec<[i32; 4]>; 4]>::definition(&mut types);
    let DataType::List(outer) = data_type else {
        panic!("outer SmallVec should be modeled as a list");
    };
    let DataType::List(inner) = outer.ty.as_ref() else {
        panic!("inner SmallVec should be modeled as a list");
    };

    assert_eq!(outer.length, None);
    assert_eq!(inner.length, None);
    assert_eq!(inner.ty.as_ref(), &DataType::Primitive(Primitive::i32));
}

#[test]
fn uhlc_id_uses_its_byte_array_shape() {
    let mut types = Types::default();
    let data_type = uhlc::ID::definition(&mut types);
    let DataType::List(list) = inline_data_type(&data_type) else {
        panic!("uhlc::ID should be modeled as an array");
    };

    assert_eq!(list.length, Some(16));
    assert_eq!(list.ty.as_ref(), &DataType::Primitive(Primitive::u8));
}

#[derive(Type)]
#[specta(collect = false)]
struct IterA;

#[derive(Type)]
#[specta(collect = false)]
struct IterB;

#[test]
fn unsorted_iterator_len_tracks_consumption() {
    let types = Types::default().register::<IterA>().register::<IterB>();
    let mut iter = types.into_unsorted_iter();

    assert_eq!(iter.len(), 2);
    assert!(iter.next().is_some());
    assert_eq!(iter.len(), 1);
    assert!(iter.next().is_some());
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
}
