use serde::Serialize;
use specta::{Type, Types};
use specta_jsonschema::JsonSchema;

#[test]
fn jsonschema_export() {
    let (types, _) = crate::types();
    insta::assert_snapshot!(
        "jsonschema-export-serde",
        JsonSchema::default()
            .export(&types, specta_serde::Format)
            .unwrap()
    );

    let (mut phased, _) = crate::types_phased();
    phased.extend(&types);
    insta::assert_snapshot!(
        "jsonschema-export-serde_phases",
        JsonSchema::default()
            .export(&phased, specta_serde::PhasesFormat)
            .unwrap()
    );
}

// Regression test for https://github.com/specta-rs/specta/issues/491

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct User {
    /// Display name for the user.
    name: String,
    scores: Vec<i32>,
}

#[test]
fn jsonschema_keeps_type_info_typescript_preserves() {
    let types = Types::default().register::<User>();
    let value = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    let defs = value
        .get("$defs")
        .or_else(|| value.get("definitions"))
        .expect("schema should contain a definitions block");
    let user = &defs["User"];

    // 1. Primitives render as concrete schema types, not refs to empty
    //    definitions.
    assert_eq!(user["properties"]["name"]["type"], "string");

    // 2. Generic parameters survive: Vec<i32> keeps its element type.
    assert_eq!(user["properties"]["scores"]["type"], "array");
    assert_eq!(user["properties"]["scores"]["items"]["type"], "integer");

    // 3. Field doc comments are emitted as descriptions.
    let description = user["properties"]["name"]["description"]
        .as_str()
        .expect("field doc comment should become a description");
    assert_eq!(description.trim(), "Display name for the user.");
}
