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

/// A trailing `#[serde(default)]` tuple element is optional on deserialize
/// (serde accepts `[1]`): the deserialize half's array schema must lower
/// `minItems` below the full arity while serialize keeps it exact.
#[derive(Type, Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct JsonTupleDefault(u8, #[serde(default)] u8);

#[test]
fn jsonschema_tuple_default_phases() {
    let rendered = JsonSchema::default()
        .export(
            &Types::default().register::<JsonTupleDefault>(),
            specta_serde::PhasesFormat,
        )
        .expect("JsonSchema should support defaulted tuple elements under PhasesFormat");

    insta::assert_snapshot!("jsonschema-tuple-default-phases", rendered);
}

/// A skip-reduced tuple struct's `ty: None` marker slots (kept so the
/// declared arity survives for other exporters) are OFF-wire: serde emits
/// `[2]` and accepts `[]`/`[2]`. The array schema must be sized over live
/// elements only — one prefix item, `minItems: 0` / `maxItems: 1` on
/// deserialize and `minItems: 1` on serialize — not two.
#[derive(Type, Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct JsonSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[test]
fn jsonschema_skip_slot_tuple_phases() {
    let rendered = JsonSchema::default()
        .export(
            &Types::default().register::<JsonSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("JsonSchema should support skip-reduced defaulted tuple structs");

    assert!(
        !rendered.contains("\"maxItems\": 2"),
        "the off-wire skipped slot must not count toward the schema size: {rendered}"
    );
    insta::assert_snapshot!("jsonschema-skip-slot-tuple-phases", rendered);
}

/// String formats are opt-in: plain JSON Schema output is unchanged unless
/// the knob is set.
#[test]
fn jsonschema_string_formats_are_opt_in() {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    struct Stamped {
        at: chrono::DateTime<chrono::Utc>,
    }

    let types = specta::Types::default().register::<Stamped>();
    let plain = specta_jsonschema::JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .expect("plain export");
    assert!(
        plain["$defs"]["Stamped"]["properties"]["at"]
            .get("format")
            .is_none(),
        "formats must not appear unless enabled"
    );

    let formatted = specta_jsonschema::JsonSchema::default()
        .string_formats(true)
        .export_value(&types, specta_serde::Format)
        .expect("formatted export");
    assert_eq!(
        formatted["$defs"]["Stamped"]["properties"]["at"]["format"],
        "date-time"
    );
}
