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

#[derive(Type, Serialize, Deserialize, Default)]
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

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Wrapper<T> {
    inner: T,
}

/// A `#[serde(default)]`-only type must force a split of every dependent,
/// with each half referencing the matching half — including through generic
/// instantiation, which exercises the reference-generics rewrite path.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericParent {
    w: Wrapper<WithFieldDefault>,
}

/// Flatten lowers to an intersection; each phase of the parent must
/// intersect with the matching phase of the flattened `default`-split type.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenParent {
    #[serde(flatten)]
    inner: WithFieldDefault,
    own: bool,
}

/// Container references through `Vec<T>` and `HashMap<K, V>` values must
/// also be redirected to the matching phase half.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct CollectionParent {
    list: Vec<WithFieldDefault>,
    map: std::collections::HashMap<String, WithFieldDefault>,
}

/// Enum variant payloads referencing a `default`-split type must also be
/// redirected per phase.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum EnumParent {
    Payload(WithFieldDefault),
    Nothing,
}

/// Container-level `#[serde(default)]` combined with an `Option<T>` field:
/// serialize must be `a: number | null` (always emitted, nullable),
/// deserialize `a?: number | null`.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct ContainerDefaultWithOption {
    a: Option<i32>,
}

/// serde accepts container `#[serde(default)]` on a zero-field struct with
/// named-field syntax; both directions are `{}` so a split would only emit a
/// redundant identical pair.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct EmptyWithDefault {}

/// `#[serde(default)]` on a struct-variant field goes through the same
/// field-level local-difference check as struct fields and must split the
/// enum.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantFieldDefault {
    A {
        #[serde(default)]
        x: i32,
    },
}

/// `#[serde(skip, default = "...")]` is a common pattern for non-wire
/// cache/state fields: `skip` removes the field from BOTH phases, so its
/// `default` never affects the wire shape and must not force a phase split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkippedDefault {
    #[serde(skip, default)]
    cache: i32,
    a: i32,
}

/// A dependent of [`SkippedDefault`] must not be dragged into a split either.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkippedDefaultParent {
    inner: SkippedDefault,
}

/// Same pattern with an explicit default path on a struct-variant field.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkippedDefaultVariant {
    A {
        #[serde(skip, default = "i32::default")]
        cache: i32,
        x: i32,
    },
}

/// Container `#[serde(default)]` on a non-wire cache/state struct whose
/// fields are ALL `#[serde(skip)]`-ped: both phases render `{}`, so the
/// container default is wire-irrelevant and must not force a phase split.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct AllSkipped {
    #[serde(skip)]
    a: u8,
    #[serde(skip)]
    b: String,
}

/// A dependent of [`AllSkipped`] must not be dragged into a split either.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AllSkippedParent {
    inner: AllSkipped,
}

/// Control for [`AllSkipped`]: one field surviving deserialize is enough for
/// the container default to matter, so this must still split.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct MixedSkipped {
    #[serde(skip)]
    cache: u8,
    live: i32,
}

/// `#[specta(skip)]` hides a field from the exported shape entirely
/// (`field.ty` is `None`) while serde still handles it on the wire, so a
/// `#[serde(default)]` on it can't show up in either exported phase.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HiddenDefault {
    #[specta(skip)]
    #[serde(default)]
    cache: i32,
    a: i32,
}

/// A dependent of [`HiddenDefault`] must not be dragged into a split either.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HiddenDefaultParent {
    inner: HiddenDefault,
}

/// Container-default counterpart of [`HiddenDefault`]: every field hidden by
/// specta leaves nothing in the exported shape for the default to widen.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct AllHidden {
    #[specta(skip)]
    cache: i32,
}

/// A split source referenced ONLY behind a field skipped in both serde
/// phases: the field is removed from both halves, so the split must not
/// cascade into this container (contrast the live-field dependents above,
/// e.g. [`SkippedDefaultParent`]/[`CollectionParent`], which must split).
///
/// The skip pair is spelled out because the specta macro special-cases a
/// bare `#[serde(skip)]` on fields/variants into `ty: None` (the referenced
/// type is then never even collected); the explicit pair keeps the reference
/// in the datatype, exercising the dependency-walk filter.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipChildParent {
    #[serde(skip_serializing, skip_deserializing)]
    child: WithFieldDefault,
    a: i32,
}

/// Nothing to inherit: [`SkipChildParent`] doesn't split, so neither does a
/// type referencing it.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipChildGrandparent {
    parent: SkipChildParent,
}

/// Variant counterpart: a both-phase-skipped variant's payload never appears
/// in either half, so it must not tie the enum to the payload's split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipVariantEnum {
    #[serde(skip_serializing, skip_deserializing)]
    Hidden(WithFieldDefault),
    Visible {
        x: i32,
    },
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

/// Split propagation through generic instantiation: `GenericParent_Serialize`
/// must reference `Wrapper<WithFieldDefault_Serialize>` (and likewise for
/// deserialize). `Wrapper` itself has no directional difference and must not
/// split.
#[test]
fn default_split_propagates_through_generic_instantiation() {
    let rendered = Typescript::default()
        .export(&Types::default().register::<GenericParent>(), PhasesFormat)
        .expect("PhasesFormat should propagate `default` splits through generics");

    assert!(
        rendered.contains("Wrapper<WithFieldDefault_Serialize>"),
        "serialize half must instantiate the serialize shape: {rendered}"
    );
    assert!(
        rendered.contains("Wrapper<WithFieldDefault_Deserialize>"),
        "deserialize half must instantiate the deserialize shape: {rendered}"
    );
    assert!(
        !rendered.contains("Wrapper_Serialize"),
        "`Wrapper` has no directional difference of its own and must not split: {rendered}"
    );

    insta::assert_snapshot!("serde-default-phases-generic-propagation", rendered);
}

/// Split propagation through `#[serde(flatten)]`: each phase of the parent
/// must merge in the matching phase shape of the flattened type.
#[test]
fn default_split_propagates_through_flatten() {
    let rendered = Typescript::default()
        .export(&Types::default().register::<FlattenParent>(), PhasesFormat)
        .expect("PhasesFormat should propagate `default` splits through flatten");

    insta::assert_snapshot!("serde-default-phases-flatten-propagation", rendered);
}

/// Split propagation through `Vec<T>`, `HashMap<K, V>` values, and enum
/// variant payloads.
#[test]
fn default_split_propagates_through_collections_and_enum_payloads() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<CollectionParent>()
                .register::<EnumParent>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should propagate `default` splits through collections");

    assert!(
        rendered.contains("WithFieldDefault_Serialize[]"),
        "Vec element in the serialize half must use the serialize shape: {rendered}"
    );
    insta::assert_snapshot!("serde-default-phases-collection-propagation", rendered);
}

/// Container-level `#[serde(default)]` with an `Option<T>` field: the field
/// is always emitted (as `T | null`) on serialize but may be omitted on
/// deserialize.
#[test]
fn container_default_with_option_field_renders_expected_shapes() {
    // serde_json ground truth first.
    let none = ContainerDefaultWithOption { a: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":null}"#);
    let deserialized: ContainerDefaultWithOption = serde_json::from_str("{}").unwrap();
    assert_eq!(deserialized.a, None);

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ContainerDefaultWithOption>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support container `default` with `Option<T>` fields");

    insta::assert_snapshot!("serde-default-phases-container-default-option", rendered);
}

/// The zero-field guard: container `#[serde(default)]` on an empty struct has
/// no field to widen, so the type must not split into a byte-identical pair.
#[test]
fn container_default_on_zero_field_struct_does_not_split() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<EmptyWithDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept container `default` on empty structs");

    assert!(
        !rendered.contains("EmptyWithDefault_Serialize"),
        "an empty struct with container `default` must not split: {rendered}"
    );
    // serde_json ground truth: both directions are `{}`.
    assert_eq!(serde_json::to_string(&EmptyWithDefault {}).unwrap(), "{}");
    let EmptyWithDefault {} = serde_json::from_str("{}").unwrap();
}

/// `#[serde(default)]` on a struct-variant field must split the containing
/// enum: required on serialize, optional on deserialize.
#[test]
fn variant_field_default_splits_enum() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VariantFieldDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support `#[serde(default)]` on variant fields");

    insta::assert_snapshot!("serde-default-phases-variant-field-default", rendered);
}

/// A field skipped in BOTH phases never appears on the wire, so its
/// `default` is wire-irrelevant and must not split the type — nor cascade a
/// split into dependents. (Plain `#[serde(default)]` splitting is covered by
/// the tests above as the control.)
#[test]
fn default_on_both_phase_skipped_field_does_not_split() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkippedDefaultParent>()
                .register::<SkippedDefaultVariant>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept `#[serde(skip, default)]` fields");

    for split_name in [
        "SkippedDefault_Serialize",
        "SkippedDefaultParent_Serialize",
        "SkippedDefaultVariant_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "`#[serde(skip, default)]` never reaches the wire and must not \
             cause a phase split (found `{split_name}`): {rendered}"
        );
    }

    // serde_json ground truth: the skipped field is absent in both phases.
    let value = SkippedDefault { cache: 9, a: 1 };
    assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"a":1}"#);
    let parsed: SkippedDefault = serde_json::from_str(r#"{"a":1,"cache":9}"#).unwrap();
    assert_eq!((parsed.cache, parsed.a), (0, 1));
}

/// Container `#[serde(default)]` where every field is skipped in both phases
/// renders `{}` for both directions, so it must not split — and must not
/// cascade a split into dependents. A single field surviving deserialize
/// (the [`MixedSkipped`] control) still splits.
#[test]
fn container_default_with_all_fields_skipped_does_not_split() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<AllSkippedParent>()
                .register::<MixedSkipped>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept container `default` with all fields skipped");

    for split_name in ["AllSkipped_Serialize", "AllSkippedParent_Serialize"] {
        assert!(
            !rendered.contains(split_name),
            "container `default` over fields skipped in both phases never \
             reaches the wire and must not cause a phase split (found \
             `{split_name}`): {rendered}"
        );
    }
    assert!(
        rendered.contains("MixedSkipped_Serialize"),
        "container `default` with a live field must still split: {rendered}"
    );

    // serde_json ground truth: nothing on the wire in either direction.
    let value = AllSkipped {
        a: 1,
        b: "x".into(),
    };
    assert_eq!(serde_json::to_string(&value).unwrap(), "{}");
    let parsed: AllSkipped = serde_json::from_str("{}").unwrap();
    assert_eq!((parsed.a, parsed.b.as_str()), (0, ""));
}

/// A `#[specta(skip)]` field is absent from both exported phases, so its
/// `#[serde(default)]` (field-level or via container default) must not split
/// the type or cascade splits into dependents. Visible `default` fields still
/// splitting is pinned by the tests above as the control.
#[test]
fn default_on_specta_hidden_field_does_not_split() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<HiddenDefaultParent>()
                .register::<AllHidden>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept `#[specta(skip)] #[serde(default)]` fields");

    for split_name in [
        "HiddenDefault_Serialize",
        "HiddenDefaultParent_Serialize",
        "AllHidden_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "a `#[specta(skip)]` field is absent from both exported phases \
             and its `default` must not cause a phase split (found \
             `{split_name}`): {rendered}"
        );
    }
    assert!(
        !rendered.contains("cache"),
        "`#[specta(skip)]` fields must stay out of the exported shape: {rendered}"
    );
}

/// Split propagation must not travel through fields or variants removed in
/// BOTH phases: the split source still splits on its own, but a container
/// referencing it only behind `#[serde(skip)]` renders identically in both
/// halves and must stay unsplit (as must its own dependents). Live-field
/// dependents splitting is pinned by the propagation tests above as the
/// control.
#[test]
fn split_does_not_propagate_through_both_phase_skipped_positions() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkipChildGrandparent>()
                .register::<SkipVariantEnum>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept split sources behind skipped fields");

    assert!(
        rendered.contains("WithFieldDefault_Serialize"),
        "the split source itself is genuinely directional and must still split: {rendered}"
    );
    for split_name in [
        "SkipChildParent_Serialize",
        "SkipChildGrandparent_Serialize",
        "SkipVariantEnum_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "a reference behind a both-phase-skipped field/variant renders \
             identically in both halves and must not cascade the split \
             (found `{split_name}`): {rendered}"
        );
    }
}
