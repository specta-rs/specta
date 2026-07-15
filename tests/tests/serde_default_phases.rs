// Regression tests for `#[serde(default)]` not being treated as a phase
// difference by `specta_serde::PhasesFormat`.
//
// `#[serde(default)]` is directional: on deserialize the field may be
// absent (serde fills it in from `Default::default()`), but on serialize
// serde *always* emits the field. Before this fix, `PhasesFormat` never
// split types that only differed via `default`, so it rendered the
// deserialize-side "optional" shape for both directions, which is wrong for
// serialize.

use serde::{Deserialize, Serialize};
use specta::{Format as _, Type, Types, datatype::DataType};
use specta_serde::{Phase, PhasesFormat, select_phase_datatype};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WithFieldDefault {
    #[serde(default)]
    a: i32,
}

#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct WithContainerDefault {
    a: i32,
    b: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WithOptionDefault {
    #[serde(default)]
    a: Option<i32>,
}

#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
struct NoDefault {
    a: i32,
}

/// A field with both `default` and `skip_serializing_if` was already a
/// direction-difference before this fix (via `skip_serializing_if`). Adding
/// `default` to the local-difference check must not double-split it or
/// change its already-correct shape.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WithDefaultAndSkipIf {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    a: Option<i32>,
}

fn named_field_optional(dt: &DataType, types: &Types, field_name: &str) -> bool {
    let DataType::Reference(specta::datatype::Reference::Named(reference)) = dt else {
        panic!("expected named reference, got {dt:?}");
    };
    let named = types.get(reference).expect("reference should resolve");
    let Some(DataType::Struct(strct)) = &named.ty else {
        panic!("expected struct type");
    };
    let specta::datatype::Fields::Named(fields) = &strct.fields else {
        panic!("expected named fields");
    };
    fields
        .fields
        .iter()
        .find_map(|(name, field)| (name == field_name).then_some(field.optional))
        .unwrap_or_else(|| panic!("field `{field_name}` should exist"))
}

#[test]
fn field_default_splits_into_serialize_and_deserialize_phases() {
    let mut types = Types::default();
    let dt = WithFieldDefault::definition(&mut types);
    let resolved = PhasesFormat
        .map_types(&types)
        .expect("PhasesFormat should support `#[serde(default)]` fields")
        .into_owned();

    let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
    let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

    // serde ALWAYS emits a defaulted field on serialize.
    assert!(
        !named_field_optional(&serialize, &resolved, "a"),
        "serialize-phase shape for a `#[serde(default)]` field must NOT be optional"
    );
    // serde allows the field to be absent on deserialize, falling back to `Default::default()`.
    assert!(
        named_field_optional(&deserialize, &resolved, "a"),
        "deserialize-phase shape for a `#[serde(default)]` field must be optional"
    );
}

#[test]
fn field_default_split_renders_expected_typescript_shapes() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<WithFieldDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support `#[serde(default)]` fields");

    insta::assert_snapshot!("serde-default-phases-field-default", rendered);
}

#[test]
fn container_default_splits_into_serialize_and_deserialize_phases() {
    let mut types = Types::default();
    let dt = WithContainerDefault::definition(&mut types);
    let resolved = PhasesFormat
        .map_types(&types)
        .expect("PhasesFormat should support container-level `#[serde(default)]`")
        .into_owned();

    let serialize = select_phase_datatype(&dt, &resolved, Phase::Serialize);
    let deserialize = select_phase_datatype(&dt, &resolved, Phase::Deserialize);

    for field in ["a", "b"] {
        assert!(
            !named_field_optional(&serialize, &resolved, field),
            "serialize-phase shape for container `#[serde(default)]` field `{field}` must NOT be optional"
        );
        assert!(
            named_field_optional(&deserialize, &resolved, field),
            "deserialize-phase shape for container `#[serde(default)]` field `{field}` must be optional"
        );
    }
}

#[test]
fn container_default_split_renders_expected_typescript_shapes() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<WithContainerDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support container-level `#[serde(default)]`");

    insta::assert_snapshot!("serde-default-phases-container-default", rendered);
}

/// Control: a struct with NO serde-directional differences at all must not be
/// split by `PhasesFormat`, and `Format` (the unified formatter) must accept
/// it unchanged. This pins down that we haven't started splitting everything.
#[test]
fn no_default_is_not_split_by_phases_format() {
    let rendered = Typescript::default()
        .export(&Types::default().register::<NoDefault>(), PhasesFormat)
        .expect("PhasesFormat should accept plain structs");

    assert!(
        !rendered.contains("NoDefault_Serialize"),
        "a struct with no directional serde attributes must not be split: {rendered}"
    );
}

/// Control: the unified `Format` compromise (single type, deserialize-side
/// widening applied) is documented behavior and must NOT change. A struct
/// with `#[serde(default)]` still renders a single type with an optional
/// field under `Format`.
#[test]
fn unified_format_keeps_single_optional_shape_for_default() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<WithFieldDefault>(),
            specta_serde::Format,
        )
        .expect("Format should accept `#[serde(default)]` fields as a documented compromise");

    insta::assert_snapshot!("serde-default-phases-unified-format-control", rendered);
}

/// serde_json ground truth: a `#[serde(default)]` field is ALWAYS present on
/// serialize (the attribute only affects deserialize), and `Option<T>`
/// serializes `None` as `null` rather than omitting the key (there is no
/// `skip_serializing_if` here).
#[test]
fn option_default_field_matches_serde_json_serialize_behavior() {
    let some = WithOptionDefault { a: Some(5) };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"a":5}"#);

    let none = WithOptionDefault { a: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":null}"#);

    // And omitting the field entirely on the wire is accepted on deserialize,
    // falling back to `Option::default()` (`None`).
    let deserialized: WithOptionDefault = serde_json::from_str("{}").unwrap();
    assert_eq!(deserialized.a, None);
}

#[test]
fn option_default_field_split_renders_expected_typescript_shapes() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<WithOptionDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support `#[serde(default)]` on an `Option<T>` field");

    insta::assert_snapshot!("serde-default-phases-option-default", rendered);
}

/// `default` combined with `skip_serializing_if` was already split before
/// this fix (via the `skip_serializing_if` local-difference check). Confirm
/// adding `default` to the same check doesn't produce a different shape.
#[test]
fn default_with_skip_serializing_if_shape_is_unchanged() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<WithDefaultAndSkipIf>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support `default` + `skip_serializing_if`");

    insta::assert_snapshot!("serde-default-phases-default-with-skip-if", rendered);
}
