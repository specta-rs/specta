//! Regression tests for <https://github.com/specta-rs/specta/issues/597>.
//!
//! Several built-in `Type` impls used to map to a shape Serde never produces.
//! Each test here first pins the shape the real Serde serializer emits, then
//! asserts Specta's `DataType`/exporter output matches it.
//!
//! # Format contract
//!
//! Specta models the Serde *data model* with one static type graph, so it can
//! only describe the shape a JSON-style serializer sees. Native YAML/TOML have
//! their own encodings for tags and datetimes; those are asserted below to
//! document the divergence, and the JSON shape is what Specta maps to.

use std::ffi::{CStr, CString};
#[cfg(any(unix, windows))]
use std::ffi::{OsStr, OsString};

use specta::{
    Type, Types,
    datatype::{DataType, Fields, NamedFields, NamedReferenceType, Primitive, Reference},
};
use specta_typescript::Typescript;
use specta_util::Remapper;

fn inline_data_type(data_type: &DataType) -> &DataType {
    match data_type {
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => inline_data_type(dt),
            _ => data_type,
        },
        _ => data_type,
    }
}

fn named_fields(data_type: &DataType) -> &NamedFields {
    let DataType::Struct(structure) = inline_data_type(data_type) else {
        panic!("expected a struct, got {data_type:#?}");
    };
    let Fields::Named(fields) = &structure.fields else {
        panic!("expected named fields, got {:?}", structure.fields);
    };
    fields
}

fn assert_byte_list<T: Type + ?Sized>() {
    let mut types = Types::default();
    let data_type = T::definition(&mut types);
    let DataType::List(list) = inline_data_type(&data_type) else {
        panic!(
            "{} should serialize as a byte list, got {data_type:#?}",
            std::any::type_name::<T>()
        );
    };
    assert_eq!(list.ty.as_ref(), &DataType::Primitive(Primitive::u8));
}

// ---------------------------------------------------------------------------
// C strings
// ---------------------------------------------------------------------------

#[test]
fn c_string_and_c_str_match_serde_bytes() {
    let owned = CString::new("hi").unwrap();
    assert_eq!(
        serde_json::to_value(&owned).unwrap(),
        serde_json::json!([104, 105])
    );

    let borrowed = c"hi";
    assert_eq!(
        serde_json::to_value(borrowed).unwrap(),
        serde_json::json!([104, 105])
    );

    assert_byte_list::<CString>();
    assert_byte_list::<CStr>();
}

// ---------------------------------------------------------------------------
// OS strings (platform specific)
// ---------------------------------------------------------------------------

#[cfg(any(unix, windows))]
fn assert_os_string_shape<T: Type + ?Sized>(variant: &str, element: Primitive) {
    let mut types = Types::default();
    let data_type = T::definition(&mut types);
    let DataType::Enum(enumeration) = inline_data_type(&data_type) else {
        panic!(
            "{} should serialize as an externally tagged enum, got {data_type:#?}",
            std::any::type_name::<T>()
        );
    };
    assert!(
        enumeration.attributes.is_empty(),
        "OS strings must stay externally tagged"
    );
    assert_eq!(enumeration.variants.len(), 1);

    let (name, definition) = &enumeration.variants[0];
    assert_eq!(name.as_ref(), variant);

    let Fields::Unnamed(fields) = &definition.fields else {
        panic!("expected a newtype variant, got {:?}", definition.fields);
    };
    assert_eq!(fields.fields.len(), 1);
    let payload = inline_data_type(fields.fields[0].ty.as_ref().expect("field type"));
    let DataType::List(list) = payload else {
        panic!("expected a list payload, got {payload:#?}");
    };
    assert_eq!(list.ty.as_ref(), &DataType::Primitive(element));
    assert_eq!(list.length, None);
}

#[cfg(unix)]
#[test]
fn os_string_matches_unix_serde_shape() {
    let owned = OsString::from("hi");
    assert_eq!(
        serde_json::to_value(&owned).unwrap(),
        serde_json::json!({ "Unix": [104, 105] })
    );

    let borrowed = OsStr::new("hi");
    assert_eq!(
        serde_json::to_value(borrowed).unwrap(),
        serde_json::json!({ "Unix": [104, 105] })
    );

    assert_os_string_shape::<OsString>("Unix", Primitive::u8);
    assert_os_string_shape::<OsStr>("Unix", Primitive::u8);
}

#[cfg(windows)]
#[test]
fn os_string_matches_windows_serde_shape() {
    let owned = OsString::from("hi");
    assert_eq!(
        serde_json::to_value(&owned).unwrap(),
        serde_json::json!({ "Windows": [104, 0, 105, 0] })
    );

    let borrowed = OsStr::new("hi");
    assert_eq!(
        serde_json::to_value(borrowed).unwrap(),
        serde_json::json!({ "Windows": [104, 0, 105, 0] })
    );

    assert_os_string_shape::<OsString>("Windows", Primitive::u16);
    assert_os_string_shape::<OsStr>("Windows", Primitive::u16);
}

// ---------------------------------------------------------------------------
// YAML tags
// ---------------------------------------------------------------------------

fn tagged_value() -> serde_yaml::value::TaggedValue {
    serde_yaml::value::TaggedValue {
        tag: serde_yaml::value::Tag::new("!Thing"),
        value: serde_yaml::Value::Bool(true),
    }
}

#[test]
fn yaml_tagged_value_matches_serde_single_entry_map() {
    let tagged = tagged_value();

    // JSON-style serializers see a single-entry map keyed by the tag string.
    assert_eq!(
        serde_json::to_value(&tagged).unwrap(),
        serde_json::json!({ "!Thing": true })
    );
    // Native YAML keeps the tag syntax instead.
    assert_eq!(serde_yaml::to_string(&tagged).unwrap(), "!Thing true\n");

    // Specta cannot express the dynamic one-entry key, so it maps to a
    // string-keyed map of `serde_yaml::Value`.
    let mut types = Types::default();
    let data_type = serde_yaml::value::TaggedValue::definition(&mut types);
    let DataType::Map(map) = inline_data_type(&data_type) else {
        panic!("TaggedValue should map to a string-keyed map, got {data_type:#?}");
    };
    assert_eq!(map.key_ty(), &DataType::Primitive(Primitive::str));

    let DataType::Enum(value) = inline_data_type(map.value_ty()) else {
        panic!(
            "TaggedValue values should be `serde_yaml::Value`, got {:#?}",
            map.value_ty()
        );
    };
    assert_eq!(
        value
            .attributes
            .get_named_as::<bool>("serde:container:untagged"),
        Some(&true)
    );
}

#[test]
fn nested_yaml_value_tagged_matches_serde() {
    let value = serde_yaml::Value::Tagged(Box::new(tagged_value()));
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        serde_json::json!({ "!Thing": true })
    );
    assert_eq!(serde_yaml::to_string(&value).unwrap(), "!Thing true\n");

    // The `Tagged` variant of `serde_yaml::Value` carries the map shape.
    let mut types = Types::default();
    let data_type = serde_yaml::Value::definition(&mut types);
    let DataType::Enum(value) = inline_data_type(&data_type) else {
        panic!("serde_yaml::Value should be an untagged enum, got {data_type:#?}");
    };
    let (_, tagged) = value
        .variants
        .iter()
        .find(|(name, _)| name.as_ref() == "Tagged")
        .expect("serde_yaml::Value should have a Tagged variant");

    let Fields::Unnamed(fields) = &tagged.fields else {
        panic!(
            "Tagged should be a newtype variant, got {:?}",
            tagged.fields
        );
    };
    assert_eq!(fields.fields.len(), 1);
    let DataType::Map(map) = inline_data_type(fields.fields[0].ty.as_ref().expect("field type"))
    else {
        panic!("Tagged payload should be a map");
    };
    assert_eq!(map.key_ty(), &DataType::Primitive(Primitive::str));
}

// ---------------------------------------------------------------------------
// TOML datetimes
// ---------------------------------------------------------------------------

#[test]
fn toml_datetime_matches_serde_private_struct() {
    let datetime: toml::value::Datetime = "2026-01-01".parse().unwrap();

    // JSON-style serializers see the private serde struct.
    assert_eq!(
        serde_json::to_value(datetime).unwrap(),
        serde_json::json!({ "$__toml_private_datetime": "2026-01-01" })
    );
    // Native TOML emits a datetime literal when the value is nested.
    let mut table = toml::map::Map::new();
    table.insert("when".into(), toml::Value::Datetime(datetime));
    assert_eq!(toml::to_string(&table).unwrap(), "when = 2026-01-01\n");

    let mut types = Types::default();
    let data_type = toml::value::Datetime::definition(&mut types);
    let fields = named_fields(&data_type);
    assert_eq!(fields.fields.len(), 1);
    assert_eq!(fields.fields[0].0.as_ref(), "$__toml_private_datetime");
    assert_eq!(
        fields.fields[0].1.ty.as_ref(),
        Some(&DataType::Primitive(Primitive::str))
    );
}

#[test]
fn nested_toml_value_datetime_matches_serde() {
    let datetime: toml::value::Datetime = "2026-01-01".parse().unwrap();
    let value = toml::Value::Datetime(datetime);

    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        serde_json::json!({ "$__toml_private_datetime": "2026-01-01" })
    );

    let mut types = Types::default();
    let data_type = toml::Value::definition(&mut types);
    let DataType::Enum(value) = inline_data_type(&data_type) else {
        panic!("toml::Value should be an untagged enum, got {data_type:#?}");
    };
    let (_, datetime_variant) = value
        .variants
        .iter()
        .find(|(name, _)| name.as_ref() == "Datetime")
        .expect("toml::Value should have a Datetime variant");

    let Fields::Unnamed(fields) = &datetime_variant.fields else {
        panic!(
            "Datetime should be a newtype variant, got {:?}",
            datetime_variant.fields
        );
    };
    assert_eq!(fields.fields.len(), 1);
    let nested = named_fields(fields.fields[0].ty.as_ref().expect("field type"));
    assert_eq!(nested.fields[0].0.as_ref(), "$__toml_private_datetime");
}

// ---------------------------------------------------------------------------
// bit-vec
// ---------------------------------------------------------------------------

#[test]
fn bit_vec_matches_derived_serde_struct() {
    let bits: bit_vec::BitVec = [true, false, true, true].iter().copied().collect();

    assert_eq!(
        serde_json::to_value(&bits).unwrap(),
        serde_json::json!({ "storage": [13], "nbits": 4 })
    );

    // The `bit-vec` feature must expose a `Type` impl for both the default
    // (`u32`) and alternate storage types.
    fn assert_type<T: Type>() {}
    assert_type::<bit_vec::BitVec>();
    assert_type::<bit_vec::BitVec<u8>>();
    assert_type::<bit_vec::BitVec<u16>>();

    let mut types = Types::default();
    let data_type = bit_vec::BitVec::<u32>::definition(&mut types);
    let fields = named_fields(&data_type);
    assert_eq!(fields.fields.len(), 2);

    assert_eq!(fields.fields[0].0.as_ref(), "storage");
    let DataType::List(storage) = fields.fields[0].1.ty.as_ref().expect("storage type") else {
        panic!("storage should be a list, got {:#?}", fields.fields[0].1.ty);
    };
    assert_eq!(storage.ty.as_ref(), &DataType::Primitive(Primitive::u32));
    assert_eq!(storage.length, None);
    assert!(!storage.unique);

    assert_eq!(fields.fields[1].0.as_ref(), "nbits");
    assert_eq!(
        fields.fields[1].1.ty.as_ref(),
        Some(&DataType::Primitive(Primitive::usize))
    );
}

// ---------------------------------------------------------------------------
// Exporter snapshots
// ---------------------------------------------------------------------------

#[derive(Type)]
#[specta(collect = false)]
struct BuiltinWireShapes {
    c_string: CString,
    toml_datetime: toml::value::Datetime,
}

#[test]
fn builtin_wire_shapes_export() {
    insta::assert_snapshot!(
        "builtin_wire_shapes",
        Typescript::default()
            .export(
                &Types::default().register::<BuiltinWireShapes>(),
                specta_serde::Format
            )
            .unwrap()
    );
}

#[derive(Type)]
#[specta(collect = false)]
struct BitVecWireShapes {
    bits: bit_vec::BitVec,
    bits_u8: bit_vec::BitVec<u8>,
}

#[test]
fn bit_vec_wire_shapes_export() {
    // `nbits` is a `usize`, so the deliberate wide-integer precision guard
    // applies exactly as it would to any other `usize` field.
    let err = Typescript::default()
        .export(
            &Types::default().register::<BitVecWireShapes>(),
            specta_serde::Format,
        )
        .expect_err("BitVec must hit the usize precision guard");
    assert!(
        err.to_string()
            .contains("forbids exporting BigInt-style types"),
        "unexpected error: {err}"
    );

    insta::assert_snapshot!(
        "bit_vec_wire_shapes",
        Typescript::default()
            .export(
                &Remapper::new()
                    .dangerous_bigints_as_number()
                    .remap_types(Types::default().register::<BitVecWireShapes>()),
                specta_serde::Format
            )
            .unwrap()
    );
}
