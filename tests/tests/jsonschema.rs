use serde::Serialize;
use specta::{Format, Type, Types};
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

// Regression tests for the P1 defect in https://github.com/specta-rs/specta/issues/590.
//
// `#[serde(deny_unknown_fields)]` used to be discarded at derive time, so JSON
// Schema had no way to honour per-type strictness and instead applied one global
// policy to every object.

/// An ordinary struct must stay permissive in the deserialization-compatible
/// mode, while a `#[serde(deny_unknown_fields)]` struct must stay closed — and
/// the two schemas must no longer be identical.
#[test]
fn jsonschema_deny_unknown_fields_closes_only_marked_objects() {
    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    struct Permissive {
        a: i32,
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        a: i32,
    }

    // Both in one graph, matching a real crate that mixes the two.
    let types = Types::default()
        .register::<Permissive>()
        .register::<Strict>();

    let permissive = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "Permissive")
        .expect("permissive export");
    let strict = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "Strict")
        .expect("strict export");

    let permissive_object = &permissive["$defs"]["Permissive"];
    let strict_object = &strict["$defs"]["Strict"];

    assert!(
        permissive_object.get("additionalProperties").is_none(),
        "an ordinary struct must stay permissive in deserialization-compatible mode: {permissive_object}"
    );
    assert_eq!(
        strict_object["additionalProperties"],
        serde_json::json!(false),
        "a deny-marked struct must stay closed: {strict_object}"
    );
    assert_ne!(
        permissive_object, strict_object,
        "ordinary and deny-marked structs must no longer produce identical schemas"
    );

    let permissive_validator = jsonschema::validator_for(&permissive).expect("valid schema");
    let strict_validator = jsonschema::validator_for(&strict).expect("valid schema");

    // Serde accepts the extra key for `Permissive` and rejects it for `Strict`.
    assert!(permissive_validator.is_valid(&serde_json::json!({ "a": 1, "extra": true })));
    assert!(!strict_validator.is_valid(&serde_json::json!({ "a": 1, "extra": true })));
    assert!(strict_validator.is_valid(&serde_json::json!({ "a": 1 })));
}

/// Serde applies a container's `deny_unknown_fields` to each named-field enum
/// variant payload, and the recorded policy must survive both format phases.
#[test]
fn jsonschema_deny_unknown_fields_applies_to_enum_payloads_in_both_phases() {
    // Internally tagged variants render as one object whose properties are the
    // tag plus the payload fields, so the variant object *is* the payload.
    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t", deny_unknown_fields)]
    enum StrictEnum {
        V { a: i32 },
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t")]
    enum OpenEnum {
        V { a: i32 },
    }

    let types = Types::default()
        .register::<StrictEnum>()
        .register::<OpenEnum>();

    let strict = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "StrictEnum")
        .expect("strict enum export");
    let open = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "OpenEnum")
        .expect("open enum export");

    assert_eq!(
        strict["$defs"]["StrictEnum"]["additionalProperties"],
        serde_json::json!(false),
        "a deny-marked enum's variant payload must stay closed: {}",
        strict["$defs"]["StrictEnum"]
    );
    assert!(
        open["$defs"]["OpenEnum"]
            .get("additionalProperties")
            .is_none(),
        "an ordinary enum's variant payload must stay permissive: {}",
        open["$defs"]["OpenEnum"]
    );

    // Both phases must retain the recorded policy (the raw attribute is what
    // `specta-jsonschema` reads), including the split phase types.
    for mapped in [
        specta_serde::Format
            .map_types(&types)
            .expect("unified phase"),
        specta_serde::PhasesFormat
            .map_types(&types)
            .expect("split phases"),
    ] {
        assert!(
            mapped.as_ref().into_sorted_iter().any(|ndt| {
                matches!(
                    ndt.ty.as_ref(),
                    Some(specta::datatype::DataType::Enum(enm))
                        if enm
                            .attributes
                            .contains_key("serde:container:deny_unknown_fields")
                )
            }),
            "deny_unknown_fields must survive the format phase"
        );
    }
}

/// Serde honours the outer container's `deny_unknown_fields` across a
/// `#[serde(flatten)]`, but ignores the flattened inner struct's own — so only
/// the outer marker may close the merged intersection object.
#[test]
fn jsonschema_flattened_deny_unknown_fields_matches_serde() {
    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    struct OpenInner {
        b: i32,
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(deny_unknown_fields)]
    struct StrictInner {
        b: i32,
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(deny_unknown_fields)]
    struct StrictOuter {
        a: i32,
        #[serde(flatten)]
        inner: OpenInner,
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    struct OpenOuter {
        a: i32,
        #[serde(flatten)]
        inner: StrictInner,
    }

    let types = Types::default()
        .register::<StrictOuter>()
        .register::<OpenOuter>();

    let strict = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "StrictOuter")
        .expect("strict outer export");
    let open = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "OpenOuter")
        .expect("open outer export");

    let strict_outer = &strict["$defs"]["StrictOuter"];
    let open_outer = &open["$defs"]["OpenOuter"];
    assert_eq!(
        strict_outer["additionalProperties"],
        serde_json::json!(false),
        "the outer container's deny must close the flattened object: {strict_outer}"
    );
    assert!(
        open_outer.get("additionalProperties").is_none(),
        "a flattened inner struct's deny must not close the merged object: {open_outer}"
    );

    let strict_validator = jsonschema::validator_for(&strict).expect("valid schema");
    let open_validator = jsonschema::validator_for(&open).expect("valid schema");
    let payload = serde_json::json!({ "a": 1, "b": 2, "extra": true });
    assert!(!strict_validator.is_valid(&payload));
    assert!(open_validator.is_valid(&payload));
}

/// A standalone schema for `base`'s deserialize-phase shape in `format`.
///
/// `PhasesFormat` only splits a type into `_Serialize`/`_Deserialize` variants
/// when its two shapes differ; a symmetric type keeps its original name, so the
/// split name is preferred but not required.
fn deserialize_phase_schema(types: &Types, format: &dyn Format, base: &str) -> serde_json::Value {
    let document = JsonSchema::default()
        .allow_additional_properties(true)
        .export_value(types, format)
        .unwrap_or_else(|err| panic!("export of `{base}` failed: {err}"));
    let defs = document["$defs"].clone();
    let split = format!("{base}_Deserialize");
    let key = if defs.get(&split).is_some() {
        split
    } else {
        base.to_string()
    };
    serde_json::json!({ "$ref": format!("#/$defs/{key}"), "$defs": defs })
}

/// An externally or adjacently tagged variant's *named payload* object must be
/// closed when the enum container carries `deny_unknown_fields` — Serde rejects
/// unknown keys there too. The tag wrapper is strict either way; this covers the
/// payload nested inside it.
///
/// Asserted against real `serde_json` accept/reject under the
/// deserialization-compatible mode, in both format phases.
#[test]
fn jsonschema_deny_unknown_fields_closes_external_and_adjacent_payloads() {
    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(deny_unknown_fields)]
    enum ExternalDeny {
        V { a: i32 },
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t", content = "c", deny_unknown_fields)]
    enum AdjacentDeny {
        V { a: i32 },
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    enum ExternalOpen {
        V { a: i32 },
    }

    #[derive(Type, Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t", content = "c")]
    enum AdjacentOpen {
        V { a: i32 },
    }

    let types = Types::default()
        .register::<ExternalDeny>()
        .register::<AdjacentDeny>()
        .register::<ExternalOpen>()
        .register::<AdjacentOpen>();

    let external_extra = serde_json::json!({ "V": { "a": 1, "extra": true } });
    let adjacent_extra = serde_json::json!({ "t": "V", "c": { "a": 1, "extra": true } });

    // Ground truth straight from Serde: the deny-marked payloads reject the
    // unknown key, the ordinary ones accept it.
    assert!(serde_json::from_value::<ExternalDeny>(external_extra.clone()).is_err());
    assert!(serde_json::from_value::<AdjacentDeny>(adjacent_extra.clone()).is_err());
    assert!(serde_json::from_value::<ExternalOpen>(external_extra.clone()).is_ok());
    assert!(serde_json::from_value::<AdjacentOpen>(adjacent_extra.clone()).is_ok());

    for (phase, format) in [
        ("unified", &specta_serde::Format as &dyn Format),
        ("phases", &specta_serde::PhasesFormat as &dyn Format),
    ] {
        for (shape, name, json, closed) in [
            ("external deny", "ExternalDeny", &external_extra, true),
            ("adjacent deny", "AdjacentDeny", &adjacent_extra, true),
            ("external open", "ExternalOpen", &external_extra, false),
            ("adjacent open", "AdjacentOpen", &adjacent_extra, false),
        ] {
            let schema = deserialize_phase_schema(&types, format, name);
            let validator = jsonschema::validator_for(&schema)
                .unwrap_or_else(|err| panic!("{phase}: {shape} schema invalid: {err}"));
            assert_eq!(
                validator.is_valid(json),
                !closed,
                "{phase}: the {shape} schema must {} a payload with an unknown key",
                if closed { "reject" } else { "accept" }
            );
        }
    }

    // Structural check so a regression is obvious without reading the validator
    // result: the deny-marked payload object itself is closed.
    let external = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "ExternalDeny")
        .expect("external export");
    let adjacent = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(&types, specta_serde::Format, "AdjacentDeny")
        .expect("adjacent export");
    assert_eq!(
        external["$defs"]["ExternalDeny"]["properties"]["V"]["additionalProperties"],
        serde_json::json!(false),
        "external payload must be closed: {}",
        external["$defs"]["ExternalDeny"]
    );
    assert_eq!(
        adjacent["$defs"]["AdjacentDeny"]["properties"]["c"]["additionalProperties"],
        serde_json::json!(false),
        "adjacent payload must be closed: {}",
        adjacent["$defs"]["AdjacentDeny"]
    );
}
