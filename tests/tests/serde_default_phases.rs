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

fn is_false(value: &bool) -> bool {
    !value
}

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

/// Serde's `missing_field` helper accepts an omitted `Option<T>` field as
/// `None`, even without `#[serde(default)]`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionSkippedWhenNone {
    #[serde(skip_serializing_if = "Option::is_none")]
    featured: Option<bool>,
}

/// A nullable Specta override does not alter serde's missing-field behavior.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NullableOverrideSkippedWhenNone {
    #[serde(skip_serializing_if = "is_false")]
    #[specta(type = Option<bool>)]
    featured: bool,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleOptionSkippedWhenNone(
    u32,
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
);

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

#[test]
fn option_skip_serializing_if_is_optional_only_when_deserializing() {
    let input: OptionSkippedWhenNone = serde_json::from_str("{}").unwrap();
    assert_eq!(input.featured, None);

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<OptionSkippedWhenNone>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support omitted Option fields");

    assert!(
        rendered.contains("featured?: boolean | null"),
        "deserialize shape must accept omitted nullable fields: {rendered}"
    );
    assert!(
        rendered.contains("OptionSkippedWhenNone_Serialize = {\n\tfeatured?: boolean,\n};"),
        "serialize shape must continue omitting null: {rendered}"
    );
}

#[test]
fn nullable_override_does_not_make_deserialize_field_optional() {
    assert!(serde_json::from_str::<NullableOverrideSkippedWhenNone>("{}").is_err());

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<NullableOverrideSkippedWhenNone>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support nullable overrides");

    assert!(
        rendered.contains("featured: boolean | null"),
        "a nullable override must not make a non-Option field optional: {rendered}"
    );
}

#[test]
fn tuple_option_is_not_optional_when_deserializing() {
    assert!(serde_json::from_str::<TupleOptionSkippedWhenNone>("[1]").is_err());

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TupleOptionSkippedWhenNone>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support tuple fields");

    assert!(
        rendered.contains("[number, string | null]"),
        "deserialize tuple shape must retain its required Option element: {rendered}"
    );
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

/// serde ignores `default` on flattened fields in BOTH directions, so
/// `#[serde(flatten, default)]` must not split (ground truth asserted
/// below). An inner type with its own container default still splits itself
/// and propagates through flatten — pinned by
/// [`default_split_propagates_through_flatten`] as the control.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenFieldDefault {
    base: i32,
    #[serde(flatten, default)]
    inner: NoDefault,
}

/// Outer container `#[serde(default)]` cannot widen a flattened field either
/// (ground truth below), so a container default over ONLY flattened fields
/// must not split.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct FlattenOnlyContainerDefault {
    #[serde(flatten)]
    inner: NoDefault,
}

/// Control: one plain field alongside the flatten keeps the container
/// default observable (the plain field is widened), so this must split.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct FlattenPlusPlainContainerDefault {
    base: i32,
    #[serde(flatten)]
    inner: NoDefault,
}

#[test]
fn default_on_flattened_field_does_not_split() {
    // serde_json ground truth: `default` never rescues a flattened field.
    // Field-level `flatten, default`: base-only input is REJECTED.
    let r: Result<FlattenFieldDefault, _> = serde_json::from_str(r#"{"base":1}"#);
    assert!(
        r.is_err(),
        "serde ignores `default` on flattened fields; base-only input must fail"
    );
    // And serialize always inlines the inner keys.
    let v = FlattenFieldDefault {
        base: 1,
        inner: NoDefault { a: 2 },
    };
    assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"base":1,"a":2}"#);
    // Outer container default: still rejected, even for `{}`.
    let r: Result<FlattenOnlyContainerDefault, _> = serde_json::from_str("{}");
    assert!(
        r.is_err(),
        "an outer container default cannot rescue flattened keys either"
    );

    // Both phases therefore share one shape: no split, no cascade.
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<FlattenFieldDefault>()
                .register::<FlattenOnlyContainerDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept `flatten, default` fields");
    for split_name in [
        "FlattenFieldDefault_Serialize",
        "FlattenOnlyContainerDefault_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "`default` is inert on flattened fields and must not cause a \
             phase split (found `{split_name}`): {rendered}"
        );
    }

    // Control: a plain sibling field keeps the container default observable.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlattenPlusPlainContainerDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept container default with flatten");
    assert!(
        rendered.contains("FlattenPlusPlainContainerDefault_Serialize"),
        "a plain field alongside a flatten still makes the container default \
         directional: {rendered}"
    );
}

/// A variant removed in BOTH phases (`filter_enum_variants_for_phase` drops
/// it from each half) must not contribute phase differences from its payload
/// attrs — `#[serde(default)]`, asymmetric renames, or anything else.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum DeadVariantDefault {
    #[serde(skip_serializing, skip_deserializing)]
    Dead {
        #[serde(default)]
        x: i32,
    },
    Live {
        y: i32,
    },
}

/// A dependent of [`DeadVariantDefault`] must not be dragged into a split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct DeadVariantDefaultParent {
    inner: DeadVariantDefault,
}

/// Field counterpart of the same disease: a field removed in BOTH phases
/// carrying some other directional attr (asymmetric rename here) must not
/// split its container either.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
struct DeadFieldRename {
    #[serde(
        skip_serializing,
        skip_deserializing,
        rename(serialize = "s", deserialize = "d")
    )]
    cache: i32,
    a: i32,
}

/// Control: a ONE-sided skip keeps the field in one phase and is genuinely
/// directional, so it must still split.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
struct OneSidedSkip {
    #[serde(skip_serializing)]
    cache: i32,
    a: i32,
}

/// Dead-in-both-phases variants and fields render nowhere, so none of their
/// attrs can constitute a phase difference. Live-variant `default` splitting
/// is pinned by [`variant_field_default_splits_enum`] as the control.
#[test]
fn dead_in_both_phases_positions_do_not_split() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<DeadVariantDefaultParent>()
                .register::<DeadFieldRename>()
                .register::<OneSidedSkip>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept dead-in-both-phases positions");

    for split_name in [
        "DeadVariantDefault_Serialize",
        "DeadVariantDefaultParent_Serialize",
        "DeadFieldRename_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "attrs on a position removed from both phases cannot be a phase \
             difference (found `{split_name}`): {rendered}"
        );
    }
    assert!(
        rendered.contains("OneSidedSkip_Serialize"),
        "a one-sided skip is genuinely directional and must still split: {rendered}"
    );
}

/// A trailing `#[serde(default)]` tuple element may be omitted on
/// deserialize (`[1]` is accepted) but is always emitted on serialize.
/// serde_derive rejects a required field after a defaulted one, so defaults
/// always form a suffix and TypeScript's trailing `?` can express them.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleDefault(u8, #[serde(default)] u8);

/// Tuple-variant counterpart of [`TupleDefault`].
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum TupleVariantDefault {
    V(u8, #[serde(default)] u8),
}

/// A newtype (single unnamed field) has no container to omit the value
/// from — `default` is inert on the wire, so it must not split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NewtypeDefault(#[serde(default)] u8);

#[test]
fn trailing_tuple_default_renders_optional_element_per_phase() {
    // serde_json ground truth: trailing defaulted elements may be omitted on
    // deserialize; serialize always emits every element.
    let v: TupleDefault = serde_json::from_str("[1]").unwrap();
    assert!(v.0 == 1 && v.1 == 0);
    assert!(serde_json::from_str::<TupleDefault>("[]").is_err());
    assert_eq!(serde_json::to_string(&TupleDefault(1, 2)).unwrap(), "[1,2]");
    let TupleVariantDefault::V(a, b) = serde_json::from_str(r#"{"V":[1]}"#).unwrap();
    assert!(a == 1 && b == 0);
    // Newtype: the value IS the payload; `default` cannot make it omittable.
    assert_eq!(serde_json::to_string(&NewtypeDefault(5)).unwrap(), "5");
    assert!(serde_json::from_str::<NewtypeDefault>("null").is_err());

    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<TupleDefault>()
                .register::<TupleVariantDefault>()
                .register::<NewtypeDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support trailing tuple defaults");

    assert!(
        rendered.contains("TupleDefault_Serialize = [number, number]"),
        "serialize half must require every element: {rendered}"
    );
    assert!(
        rendered.contains("TupleDefault_Deserialize = [number, number?]"),
        "deserialize half must mark the defaulted suffix optional: {rendered}"
    );
    assert!(
        rendered.contains("{ V: [number, number?] }"),
        "tuple-variant deserialize half must mark the defaulted suffix optional: {rendered}"
    );
    assert!(
        !rendered.contains("NewtypeDefault_Serialize"),
        "newtype `default` is inert on the wire and must not split: {rendered}"
    );

    insta::assert_snapshot!("serde-default-phases-tuple-default", rendered);
}

/// Non-trailing tuple defaults can't come from serde_derive (a required
/// field after a defaulted one is the compile error "field must have
/// #[serde(default)] because previous field 0 has #[serde(default)]"), but
/// a hand-built datatype can still carry one. Sequence elements can only be
/// omitted from the end, so such a default is inert and must not split —
/// mirroring the renderer, which only honors trailing optional runs.
#[test]
fn non_trailing_hand_built_tuple_default_does_not_split() {
    use specta::datatype::{Field, NamedDataType, Primitive, Struct};

    let mut types = Types::default();
    NamedDataType::new("HandBuilt", &mut types, |_, ndt| {
        let mut s = Struct::unnamed();
        let mut first = Field::new(DataType::Primitive(Primitive::u8));
        first.attributes.insert("serde:field:default", true);
        s.field_mut(first);
        s.field_mut(Field::new(DataType::Primitive(Primitive::u8)));
        ndt.ty = Some(s.build());
    });

    let rendered = Typescript::default()
        .export(&types, PhasesFormat)
        .expect("PhasesFormat should accept a non-trailing tuple default");

    assert!(
        !rendered.contains("HandBuilt_Serialize"),
        "a non-trailing tuple default cannot be omitted from the sequence \
         and must not cause a phase split: {rendered}"
    );
    // Trailing defaults still splitting is pinned by
    // `trailing_tuple_default_renders_optional_element_per_phase`.
}

/// A defaulted trailing element whose type renders as a union must be
/// parenthesized before the optional marker: `[number, (number | null)?]`
/// (`number | null?` is invalid TypeScript).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptTupleDefault(u8, #[serde(default)] Option<u8>);

/// `#[specta(skip)]` on a variant removes it from BOTH phase rewrites
/// (`filter_enum_variants_for_phase` drops `variant.skip` unconditionally),
/// so a defaulted payload field inside it must not split the enum.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SpectaSkippedVariant {
    #[specta(skip)]
    Hidden {
        #[serde(default)]
        x: i32,
    },
    Visible {
        y: i32,
    },
}

/// Same, with the hidden payload referencing a genuinely-splitting type:
/// the dead variant must not tie the enum (or its dependents) to that split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SpectaSkippedVariantRef {
    #[specta(skip)]
    Hidden(WithFieldDefault),
    Visible {
        y: i32,
    },
}

/// A dependent of [`SpectaSkippedVariantRef`] must not split either.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SpectaSkippedVariantParent {
    inner: SpectaSkippedVariantRef,
}

#[test]
fn optional_tuple_element_with_union_type_is_parenthesized() {
    // serde_json ground truth: the trailing `Option` default may be omitted
    // or null; serialize always emits it.
    let v: OptTupleDefault = serde_json::from_str("[1]").unwrap();
    assert!(v.0 == 1 && v.1.is_none());
    assert_eq!(
        serde_json::to_string(&OptTupleDefault(1, None)).unwrap(),
        "[1,null]"
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<OptTupleDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support defaulted Option tuple elements");

    assert!(
        rendered.contains("OptTupleDefault_Deserialize = [number, (number | null)?]"),
        "a union-typed optional tuple element must be parenthesized: {rendered}"
    );
    assert!(
        rendered.contains("OptTupleDefault_Serialize = [number, number | null]"),
        "the serialize half stays required and unparenthesized: {rendered}"
    );
}

#[test]
fn specta_skipped_variant_is_dead_in_both_phases() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SpectaSkippedVariant>()
                .register::<SpectaSkippedVariantParent>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept `#[specta(skip)]` variants");

    for split_name in [
        "SpectaSkippedVariant_Serialize",
        "SpectaSkippedVariantRef_Serialize",
        "SpectaSkippedVariantParent_Serialize",
    ] {
        assert!(
            !rendered.contains(split_name),
            "a `#[specta(skip)]` variant is removed from both phases and \
             must not split its enum or dependents (found `{split_name}`): {rendered}"
        );
    }
}

/// The derive erases a `#[specta(skip)]` variant's payload types, but a
/// hand-built datatype can carry `Variant::skip` with a live defaulted
/// payload. `filter_enum_variants_for_phase` drops `skip` variants from
/// BOTH phases, so such a payload must not split the enum.
#[test]
fn hand_built_skip_variant_with_default_payload_does_not_split() {
    use specta::datatype::{Enum, Field, NamedDataType, Primitive, Variant};

    let mut types = Types::default();
    NamedDataType::new("HandBuiltEnum", &mut types, |_, ndt| {
        let mut hidden_payload = Field::new(DataType::Primitive(Primitive::u8));
        hidden_payload
            .attributes
            .insert("serde:field:default", true);
        let hidden = Variant::named().skip().field("x", hidden_payload).build();
        let visible = Variant::named()
            .field("y", Field::new(DataType::Primitive(Primitive::u8)))
            .build();

        let mut e = Enum::default();
        e.variants.push(("Hidden".into(), hidden));
        e.variants.push(("Visible".into(), visible));
        ndt.ty = Some(DataType::Enum(e));
    });

    let rendered = Typescript::default()
        .export(&types, PhasesFormat)
        .expect("PhasesFormat should accept `Variant::skip` with live payloads");

    assert!(
        !rendered.contains("HandBuiltEnum_Serialize"),
        "a `Variant::skip` variant is removed from both phases and its \
         payload must not split the enum: {rendered}"
    );
}

/// Container `#[serde(default)]` on a tuple struct fills every missing
/// trailing element from the container's `Default` instance, so a shorter
/// array (even `[]`) deserializes while serialize still emits all elements.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct TupleContainerDefault(u8, u8);

/// A newtype keeps its bare-value representation even with a container
/// default — there is no sequence to shorten, so the default is inert.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct NewtypeContainerDefault(u8);

#[test]
fn container_default_on_tuple_struct_splits_with_all_optional_elements() {
    // serde_json ground truth.
    let v: TupleContainerDefault = serde_json::from_str("[]").unwrap();
    assert!(v.0 == 0 && v.1 == 0);
    let v: TupleContainerDefault = serde_json::from_str("[1]").unwrap();
    assert!(v.0 == 1 && v.1 == 0);
    assert_eq!(
        serde_json::to_string(&TupleContainerDefault(1, 2)).unwrap(),
        "[1,2]"
    );
    // Newtype: bare value, nothing to omit.
    assert_eq!(
        serde_json::to_string(&NewtypeContainerDefault(5)).unwrap(),
        "5"
    );
    assert!(serde_json::from_str::<NewtypeContainerDefault>("[]").is_err());
    assert!(serde_json::from_str::<NewtypeContainerDefault>("null").is_err());

    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<TupleContainerDefault>()
                .register::<NewtypeContainerDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support container defaults on tuple structs");

    assert!(
        rendered.contains("TupleContainerDefault_Serialize = [number, number]"),
        "serialize always emits every element: {rendered}"
    );
    assert!(
        rendered.contains("TupleContainerDefault_Deserialize = [number?, number?]"),
        "every element may be omitted on deserialize: {rendered}"
    );
    assert!(
        !rendered.contains("NewtypeContainerDefault_Serialize"),
        "a newtype container default is inert and must not split: {rendered}"
    );
}

/// A tuple variant reduced to one live element by `#[serde(skip)]` STAYS a
/// sequence on the wire (`{"V":[2]}`), and its trailing default makes that
/// element omittable on deserialize — the payload must keep its declared
/// arity and render `[number]` / `[number?]`, not collapse to a bare
/// newtype `number`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipSlotVariant {
    V(#[serde(skip)] u8, #[serde(default)] u8),
}

/// Control: same shape without the default — no split, sequence kept.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SkipSlotControl {
    W(#[serde(skip)] u8, u8),
}

#[test]
fn skip_reduced_tuple_variant_keeps_arity_with_optional_payload() {
    // serde_json ground truth.
    assert_eq!(
        serde_json::to_string(&SkipSlotVariant::V(9, 2)).unwrap(),
        r#"{"V":[2]}"#
    );
    let SkipSlotVariant::V(a, b) = serde_json::from_str(r#"{"V":[]}"#).unwrap();
    assert!(a == 0 && b == 0);
    let SkipSlotVariant::V(a, b) = serde_json::from_str(r#"{"V":[2]}"#).unwrap();
    assert!(a == 0 && b == 2);
    assert!(serde_json::from_str::<SkipSlotVariant>(r#"{"V":2}"#).is_err());
    assert_eq!(
        serde_json::to_string(&SkipSlotControl::W(9, 2)).unwrap(),
        r#"{"W":[2]}"#
    );

    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkipSlotVariant>()
                .register::<SkipSlotControl>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support skip-reduced defaulted tuple variants");

    assert!(
        rendered.contains("SkipSlotVariant_Serialize = { V: [number] }"),
        "serialize keeps the one-live-element sequence: {rendered}"
    );
    assert!(
        rendered.contains("SkipSlotVariant_Deserialize = { V: [number?] }"),
        "deserialize marks the defaulted element optional WITHOUT collapsing \
         the sequence to a bare newtype: {rendered}"
    );
    assert!(
        !rendered.contains("SkipSlotControl_Serialize"),
        "the default-free control must not split: {rendered}"
    );
    assert!(
        rendered.contains("SkipSlotControl = { W: [number] }"),
        "the default-free control keeps its sequence shape: {rendered}"
    );
}

/// The explicit `skip_serializing, skip_deserializing` pair on an UNNAMED
/// field keeps `field.ty` populated (unlike bare `#[serde(skip)]`, which the
/// macro erases), but serde still never puts the element on the wire —
/// `TPair(1, 9)` serializes to `[1]` and `[1,9]` is rejected. The exported
/// shape must drop it exactly like the bare spelling, not retain it as a
/// required element (or leak a reference to a split child through it).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PairSkipTuple(
    u8,
    #[serde(skip_serializing, skip_deserializing)] WithFieldDefault,
);

/// Bare-skip spelling of the same shape: both must export identically.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct BareSkipTuple(u8, #[serde(skip)] WithFieldDefault);

/// Variant counterpart (the marker-preserving path): the pair-skipped slot
/// keeps the declared arity but never renders.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum PairSkipVariant {
    V(u8, #[serde(skip_serializing, skip_deserializing)] u8),
}

#[test]
fn pair_skipped_unnamed_field_is_dropped_from_export() {
    // serde_json ground truth.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TPair(u8, #[serde(skip_serializing, skip_deserializing)] u8);
    assert_eq!(serde_json::to_string(&TPair(1, 9)).unwrap(), "[1]");
    let v: TPair = serde_json::from_str("[1]").unwrap();
    assert!(v.0 == 1 && v.1 == 0);
    assert!(serde_json::from_str::<TPair>("[1,9]").is_err());
    assert_eq!(
        serde_json::to_string(&PairSkipVariant::V(1, 9)).unwrap(),
        r#"{"V":[1]}"#
    );

    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<PairSkipTuple>()
                .register::<BareSkipTuple>()
                .register::<PairSkipVariant>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should accept pair-skipped unnamed fields");

    assert!(
        !rendered.contains("PairSkipTuple_Serialize"),
        "the pair-skipped element never reaches the wire, so the tuple must \
         not split: {rendered}"
    );
    let pair_shape = rendered
        .lines()
        .find(|line| line.starts_with("export type PairSkipTuple "))
        .expect("PairSkipTuple should be exported");
    let bare_shape = rendered
        .lines()
        .find(|line| line.starts_with("export type BareSkipTuple "))
        .expect("BareSkipTuple should be exported");
    assert!(
        !pair_shape.contains("WithFieldDefault"),
        "a split child behind a pair-skipped element must not leak into the \
         exported shape: {pair_shape}"
    );
    assert_eq!(
        pair_shape.trim_start_matches("export type PairSkipTuple"),
        bare_shape.trim_start_matches("export type BareSkipTuple"),
        "explicit-pair and bare `skip` spellings must export identically: {rendered}"
    );
    assert!(
        rendered.contains("PairSkipVariant = { V: [number] }"),
        "the variant payload keeps its sequence shape without the skipped \
         slot: {rendered}"
    );
    // The child itself is still exported and split.
    assert!(rendered.contains("WithFieldDefault_Serialize"));
}

/// A skip-reduced tuple struct with a defaulted surviving element keeps its
/// sequence representation in serde (`[2]` on serialize; `[]`/`[2]` accepted
/// on deserialize), so the declared arity must survive the rewrite: the
/// halves render `[number]` / `[number?]`, not a bare newtype `number` that
/// also loses the deserialize `?`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipSlotTupleStruct(#[serde(skip)] u8, #[serde(default)] u8);

/// Container-default counterpart: every surviving element is omittable.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct SkipSlotContainerDefault(#[serde(skip)] u8, u8);

#[test]
fn skip_reduced_tuple_struct_keeps_arity_with_defaulted_element() {
    // serde_json ground truth.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct S(#[serde(skip)] u8, #[serde(default)] u8);
    assert_eq!(serde_json::to_string(&S(9, 2)).unwrap(), "[2]");
    let v: S = serde_json::from_str("[]").unwrap();
    assert!(v.0 == 0 && v.1 == 0);
    let v: S = serde_json::from_str("[2]").unwrap();
    assert!(v.0 == 0 && v.1 == 2);
    assert!(serde_json::from_str::<S>("2").is_err());

    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkipSlotTupleStruct>()
                .register::<SkipSlotContainerDefault>(),
            PhasesFormat,
        )
        .expect("PhasesFormat should support skip-reduced defaulted tuple structs");

    assert!(
        rendered.contains("SkipSlotTupleStruct_Serialize = [number]"),
        "serialize keeps the one-live-element sequence: {rendered}"
    );
    assert!(
        rendered.contains("SkipSlotTupleStruct_Deserialize = [number?]"),
        "deserialize marks the defaulted element optional without collapsing \
         to a bare newtype: {rendered}"
    );
    assert!(
        rendered.contains("SkipSlotContainerDefault_Serialize = [number]"),
        "container-default serialize half keeps the sequence: {rendered}"
    );
    assert!(
        rendered.contains("SkipSlotContainerDefault_Deserialize = [number?]"),
        "container-default deserialize half marks the element optional: {rendered}"
    );
}
