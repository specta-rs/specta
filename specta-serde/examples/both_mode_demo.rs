//! Demonstration of SerdeMode::Both
//!
//! This example shows how to use SerdeMode::Both for types that need to work
//! for both serialization and deserialization.

use specta::datatype::{Field, Primitive, RuntimeAttribute, RuntimeLiteral, RuntimeMeta};
use specta::{DataType, internal};
use specta_serde::{SerdeMode, apply_to_dt};

fn main() {
    println!("=== SerdeMode::Both Demo ===\n");

    // Example 1: Struct with rename_all
    println!("1. Struct with rename_all = \"camelCase\"");
    let dt = create_struct_with_rename_all();
    match apply_to_dt(dt, SerdeMode::Both) {
        Ok(DataType::Struct(s)) => {
            println!("   Original fields: first_name, last_name, user_id");
            print!("   Transformed fields: ");
            if let specta::datatype::Fields::Named(fields) = s.fields() {
                let names: Vec<&str> = fields.fields().iter().map(|(n, _)| n.as_ref()).collect();
                println!("{}", names.join(", "));
            }
        }
        _ => println!("   Transformation failed"),
    }
    println!();

    // Example 2: Partially skipped fields
    println!("2. Struct with mode-specific skip attributes");
    let dt = create_partially_skipped_struct();
    match apply_to_dt(dt, SerdeMode::Both) {
        Ok(DataType::Struct(s)) => {
            println!(
                "   Original: name, password (skip_serializing), computed (skip_deserializing)"
            );
            print!("   Both mode keeps: ");
            if let specta::datatype::Fields::Named(fields) = s.fields() {
                let names: Vec<&str> = fields.fields().iter().map(|(n, _)| n.as_ref()).collect();
                println!("{}", names.join(", "));
                println!("   → Fields are kept because they're only skipped in ONE direction");
            }
        }
        _ => println!("   Transformation failed"),
    }
    println!();

    // Example 3: Universal skip
    println!("3. Struct with universal skip attribute");
    let dt = create_universally_skipped_struct();
    match apply_to_dt(dt, SerdeMode::Both) {
        Ok(DataType::Struct(s)) => {
            println!("   Original: name, internal_field (skip)");
            print!("   Both mode keeps: ");
            if let specta::datatype::Fields::Named(fields) = s.fields() {
                let names: Vec<&str> = fields.fields().iter().map(|(n, _)| n.as_ref()).collect();
                println!("{}", names.join(", "));
                println!("   → internal_field removed because #[serde(skip)] applies to both");
            }
        }
        _ => println!("   Transformation failed"),
    }
    println!();

    println!("=== Key Takeaways ===");
    println!("• SerdeMode::Both uses common transformation attributes");
    println!("• Fields are only skipped if skipped in BOTH serialize AND deserialize");
    println!("• Useful for APIs where the same type definition works both ways");
    println!("• Mode-specific attributes (like rename_serialize) are ignored in Both mode");
}

fn create_struct_with_rename_all() -> DataType {
    let serde_attr = RuntimeAttribute {
        path: "serde".to_string(),
        kind: RuntimeMeta::NameValue {
            key: "rename_all".to_string(),
            value: RuntimeLiteral::Str("camelCase".to_string()),
        },
    };

    let fields = internal::construct::fields_named(
        vec![
            (
                "first_name".into(),
                Field::new(DataType::Primitive(Primitive::String)),
            ),
            (
                "last_name".into(),
                Field::new(DataType::Primitive(Primitive::String)),
            ),
            (
                "user_id".into(),
                Field::new(DataType::Primitive(Primitive::u64)),
            ),
        ],
        vec![],
    );

    let mut s = specta::datatype::Struct::unit();
    s.set_fields(fields);
    s.set_attributes(vec![serde_attr]);

    DataType::Struct(s)
}

fn create_partially_skipped_struct() -> DataType {
    let skip_ser_attr = RuntimeAttribute {
        path: "serde".to_string(),
        kind: RuntimeMeta::Path("skip_serializing".to_string()),
    };

    let skip_de_attr = RuntimeAttribute {
        path: "serde".to_string(),
        kind: RuntimeMeta::Path("skip_deserializing".to_string()),
    };

    let mut field2 = Field::new(DataType::Primitive(Primitive::String));
    field2.set_attributes(vec![skip_ser_attr]);

    let mut field3 = Field::new(DataType::Primitive(Primitive::String));
    field3.set_attributes(vec![skip_de_attr]);

    let fields = internal::construct::fields_named(
        vec![
            (
                "name".into(),
                Field::new(DataType::Primitive(Primitive::String)),
            ),
            ("password".into(), field2),
            ("computed".into(), field3),
        ],
        vec![],
    );

    let mut s = specta::datatype::Struct::unit();
    s.set_fields(fields);

    DataType::Struct(s)
}

fn create_universally_skipped_struct() -> DataType {
    let skip_attr = RuntimeAttribute {
        path: "serde".to_string(),
        kind: RuntimeMeta::Path("skip".to_string()),
    };

    let mut field2 = Field::new(DataType::Primitive(Primitive::String));
    field2.set_attributes(vec![skip_attr]);

    let fields = internal::construct::fields_named(
        vec![
            (
                "name".into(),
                Field::new(DataType::Primitive(Primitive::String)),
            ),
            ("internal_field".into(), field2),
        ],
        vec![],
    );

    let mut s = specta::datatype::Struct::unit();
    s.set_fields(fields);

    DataType::Struct(s)
}
