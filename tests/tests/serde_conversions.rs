use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Wire {
    value: i32,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(into = "Wire")]
struct IntoOnly {
    value: i32,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(from = "Wire", into = "Wire")]
struct Symmetric {
    value: i32,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericWire<T> {
    values: Vec<T>,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(try_from = "GenericWire<T>")]
struct GenericTryFrom<T> {
    values: Vec<T>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericParent<T> {
    child: GenericTryFrom<T>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Parent {
    child: IntoOnly,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct CustomCodecNoOverride {
    #[serde(with = "codec")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct CustomCodecWithOverride {
    #[specta(type = String)]
    #[serde(with = "codec")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct CustomCodecWithPhasedOverride {
    #[specta(type = specta_serde::Phased<String, i32>)]
    #[serde(with = "codec")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldOnlyPhasedOverride {
    #[specta(type = specta_serde::Phased<String, i32>)]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipSerializingIfOnly {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct DefaultedSkipSerializingIfOnly {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum TupleVariantSkipSerializingIfOnly {
    Value(#[serde(skip_serializing_if = "Option::is_none")] Option<String>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NonTrailingTupleSkipSerializingIf(
    #[serde(skip_serializing_if = "std::ops::Not::not")] bool,
    u32,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct IndependentlyOmittedTupleSuffix(
    u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")] bool,
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ConditionalThenSerializeSkipped(
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
    #[serde(skip_serializing)] u32,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ConditionalThenSkippedTuple(
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
    #[serde(skip)] u8,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkippedThenConditionalTuple(
    #[serde(skip)] u8,
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
);

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(from = "Wire", into = "Wire")]
struct ConvertedConditionalTuple(
    #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
    u32,
);

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(from = "Wire", into = "Wire")]
enum ConvertedConditionalEnum {
    Value(
        #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
        u32,
    ),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum SerializeSkippedConditionalVariant {
    #[serde(skip_serializing)]
    Value(
        #[serde(skip_serializing_if = "Option::is_none")] Option<String>,
        u32,
    ),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldAlias {
    #[serde(alias = "old_value")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldMultipleAliases {
    #[serde(alias = "old_value", alias = "legacy_value")]
    value: String,
}

mod alias_collision {
    // The collision is intentional: serde gives the earlier alias precedence.
    #![allow(unreachable_patterns)]

    use super::*;

    #[derive(Debug, Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    pub(super) struct FieldAliasCollision {
        #[serde(alias = "b")]
        pub(super) a: u32,
        #[serde(default)]
        pub(super) b: String,
    }

    #[derive(Debug, Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    pub(super) struct SharedFieldAlias {
        #[serde(alias = "legacy")]
        pub(super) a: u32,
        #[serde(default, alias = "legacy")]
        pub(super) b: String,
    }

    #[derive(Clone, Debug, Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    #[serde(from = "Wire", into = "Wire")]
    pub(super) struct ConvertedFieldAliasCollision {
        #[serde(alias = "b")]
        pub(super) a: u32,
        #[serde(default)]
        pub(super) b: String,
    }

    #[derive(Clone, Debug, Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    #[serde(from = "Wire", into = "Wire")]
    pub(super) enum ConvertedVariantFieldAliasCollision {
        Value {
            #[serde(alias = "b")]
            a: u32,
            #[serde(default)]
            b: String,
        },
    }

    impl From<ConvertedFieldAliasCollision> for Wire {
        fn from(_: ConvertedFieldAliasCollision) -> Self {
            Self { value: 0 }
        }
    }

    impl From<Wire> for ConvertedFieldAliasCollision {
        fn from(_: Wire) -> Self {
            Self {
                a: 0,
                b: String::new(),
            }
        }
    }

    impl From<ConvertedVariantFieldAliasCollision> for Wire {
        fn from(_: ConvertedVariantFieldAliasCollision) -> Self {
            Self { value: 0 }
        }
    }

    impl From<Wire> for ConvertedVariantFieldAliasCollision {
        fn from(_: Wire) -> Self {
            Self::Value {
                a: 0,
                b: String::new(),
            }
        }
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenedAliasInner {
    nested: bool,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldAliasWithFlatten {
    #[serde(alias = "old_value")]
    value: String,
    #[serde(flatten)]
    inner: FlattenedAliasInner,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InternalFieldAliasWithFlatten {
    Value {
        #[serde(alias = "old_value")]
        value: String,
        #[serde(flatten)]
        inner: FlattenedAliasInner,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantAlias {
    #[serde(alias = "OldValue")]
    Value,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantMultipleAliases {
    #[serde(alias = "OldValue", alias = "LegacyValue")]
    Value,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InternalVariantAlias {
    #[serde(alias = "OldValue")]
    Value { value: String },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum AdjacentVariantAlias {
    #[serde(alias = "OldValue")]
    Value(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldPhaseSpecificRename {
    #[serde(rename(serialize = "serialized_value", deserialize = "deserialized_value"))]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantCodecNoOverride {
    #[serde(
        serialize_with = "codec_variant::serialize",
        deserialize_with = "codec_variant::deserialize"
    )]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantCodecWithOverride {
    #[serde(
        serialize_with = "codec_variant::serialize",
        deserialize_with = "codec_variant::deserialize"
    )]
    #[specta(r#type = String)]
    A(String),
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(variant_identifier)]
enum VariantIdentifierValid {
    Alpha,
    Beta,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(field_identifier)]
enum FieldIdentifierValid {
    Alpha,
    Beta,
    Other(String),
}

mod codec {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(_value: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("codec")
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
    }
}

mod codec_variant {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
    }
}

impl From<IntoOnly> for Wire {
    fn from(value: IntoOnly) -> Self {
        Self { value: value.value }
    }
}

impl From<Symmetric> for Wire {
    fn from(value: Symmetric) -> Self {
        Self { value: value.value }
    }
}

impl From<Wire> for Symmetric {
    fn from(value: Wire) -> Self {
        Self { value: value.value }
    }
}

impl From<ConvertedConditionalTuple> for Wire {
    fn from(_: ConvertedConditionalTuple) -> Self {
        Self { value: 0 }
    }
}

impl From<Wire> for ConvertedConditionalTuple {
    fn from(_: Wire) -> Self {
        Self(None, 0)
    }
}

impl From<ConvertedConditionalEnum> for Wire {
    fn from(_: ConvertedConditionalEnum) -> Self {
        Self { value: 0 }
    }
}

impl From<Wire> for ConvertedConditionalEnum {
    fn from(_: Wire) -> Self {
        Self::Value(None, 0)
    }
}

#[allow(clippy::infallible_try_from)]
impl<T> TryFrom<GenericWire<T>> for GenericTryFrom<T> {
    type Error = std::convert::Infallible;

    fn try_from(value: GenericWire<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            values: value.values,
        })
    }
}

#[test]
fn apply_rejects_asymmetric_container_conversion() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<IntoOnly>(),
            specta_serde::Format,
        )
        .expect_err("apply should reject asymmetric serde conversions");

    assert!(
        err.to_string()
            .contains("Incompatible container conversion"),
        "unexpected error: {err}"
    );
}

#[test]
fn phases_format_splits_container_and_dependents_for_conversions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<Parent>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support asymmetric serde conversions");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-splits-container-and-dependents",
        rendered
    );
}

#[test]
fn apply_accepts_symmetric_container_conversion() {
    Typescript::default()
        .export(
            &Types::default().register::<Symmetric>(),
            specta_serde::Format,
        )
        .expect("apply should accept symmetric serde conversions");
}

#[test]
fn phases_format_accepts_generic_try_from_container_conversion() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<GenericParent<String>>(),
            specta_serde::Format,
        )
        .expect_err("apply should reject deserialize-only container conversions");
    assert!(
        err.to_string()
            .contains("Incompatible container conversion")
    );

    Typescript::default()
        .export(
            &Types::default().register::<GenericParent<String>>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should resolve nested generic references from container conversions");
}

#[test]
fn custom_codec_requires_explicit_override() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<CustomCodecNoOverride>(),
            specta_serde::Format,
        )
        .expect_err("custom serde codecs should require #[specta(type = ...)]");

    assert!(err.to_string().contains("Unsupported serde attribute"));

    Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithOverride>(),
            specta_serde::Format,
        )
        .expect("override should satisfy custom serde codecs");
}

#[test]
fn custom_codec_variant_requires_explicit_override() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantCodecNoOverride>(),
            specta_serde::Format,
        )
        .expect_err("variant custom serde codecs should require #[specta(type = ...)]");
    assert!(err.to_string().contains("Unsupported serde attribute"));

    Typescript::default()
        .export(
            &Types::default().register::<VariantCodecWithOverride>(),
            specta_serde::Format,
        )
        .expect("variant override should satisfy custom serde codecs");
}

#[test]
fn phased_override_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithPhasedOverride>(),
            specta_serde::Format,
        )
        .expect_err("apply should reject phased overrides");
    assert!(err.to_string().contains("requires `PhasesFormat`"));

    Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithPhasedOverride>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should accept phased overrides");
}

#[test]
fn field_only_phased_override_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::Format,
        )
        .expect_err("apply should reject phased field overrides");
    assert!(err.to_string().contains("requires `PhasesFormat`"));

    Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::PhasesFormat,
        )
        .expect("phased export should remove phased opaque references");
}

#[test]
fn phases_format_exports_field_only_phased_override() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should resolve phased overrides during export");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-exports-field-only-phased-override",
        rendered
    );
}

#[test]
fn format_unifies_skip_serializing_if() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkipSerializingIfOnly>()
                .register::<DefaultedSkipSerializingIfOnly>(),
            specta_serde::Format,
        )
        .expect("Format should safely unify conditional omission");

    insta::assert_snapshot!("serde-conversions-format-unified-option-is-none", rendered);
}

#[test]
fn option_is_none_omits_null_in_serialize_phase() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<SkipSerializingIfOnly>()
                .register::<DefaultedSkipSerializingIfOnly>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split skip_serializing_if phases");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-option-is-none-omits-null",
        rendered
    );
}

#[test]
fn tuple_variant_skip_serializing_if_unifies_and_splits_owner() {
    assert_eq!(
        serde_json::to_string(&TupleVariantSkipSerializingIfOnly::Value(None)).unwrap(),
        r#"{"Value":null}"#,
    );

    let unified = Typescript::default()
        .export(
            &Types::default().register::<TupleVariantSkipSerializingIfOnly>(),
            specta_serde::Format,
        )
        .expect("Format should safely unify tuple-field conditional omission");

    insta::assert_snapshot!(
        "serde-conversions-format-unified-tuple-option-is-none",
        unified
    );

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TupleVariantSkipSerializingIfOnly>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split tuple variant field skip_serializing_if");

    assert!(rendered.contains("TupleVariantSkipSerializingIfOnly_Serialize"));
    assert!(rendered.contains("TupleVariantSkipSerializingIfOnly_Deserialize"));
    assert!(
        rendered.contains("TupleVariantSkipSerializingIfOnly_Serialize = { Value: string | null }"),
        "unexpected phased output: {rendered}"
    );
}

#[test]
fn format_rejects_non_trailing_tuple_skip_serializing_if() {
    for format in [
        &specta_serde::Format as &dyn specta::Format,
        &specta_serde::PhasesFormat,
    ] {
        for types in [
            Types::default().register::<NonTrailingTupleSkipSerializingIf>(),
            Types::default().register::<IndependentlyOmittedTupleSuffix>(),
        ] {
            let err = Typescript::default()
                .export(&types, format)
                .expect_err("tuple omission before the final live field is not representable");

            assert!(err.to_string().contains("final live field"));
        }
    }
}

#[test]
fn phases_accepts_conditional_omission_before_serialize_skipped_items() {
    Typescript::default()
        .export(
            &Types::default()
                .register::<ConditionalThenSerializeSkipped>()
                .register::<SerializeSkippedConditionalVariant>(),
            specta_serde::PhasesFormat,
        )
        .expect("serialize-skipped items do not occupy tuple positions");
}

#[test]
fn skipped_tuple_slots_preserve_conditional_optionality() {
    assert_eq!(
        serde_json::to_string(&ConditionalThenSkippedTuple(None, 0)).unwrap(),
        "[]"
    );
    assert_eq!(
        serde_json::to_string(&SkippedThenConditionalTuple(0, None)).unwrap(),
        "[]"
    );

    let unified = Typescript::default()
        .export(
            &Types::default()
                .register::<ConditionalThenSkippedTuple>()
                .register::<SkippedThenConditionalTuple>(),
            specta_serde::Format,
        )
        .expect("skipped slots should preserve a trailing conditional element");
    assert!(unified.contains("ConditionalThenSkippedTuple = [(string | null)?]"));
    assert!(unified.contains("SkippedThenConditionalTuple = [(string | null)?]"));

    let phased = Typescript::default()
        .export(
            &Types::default()
                .register::<ConditionalThenSkippedTuple>()
                .register::<SkippedThenConditionalTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("phase splitting should retain skipped tuple slots");
    assert!(
        phased.contains("ConditionalThenSkippedTuple_Serialize = [string?]"),
        "unexpected phased output: {phased}"
    );
    assert!(
        phased.contains("SkippedThenConditionalTuple_Serialize = [string?]"),
        "unexpected phased output: {phased}"
    );
}

#[test]
fn conversions_hide_declared_tuple_conditional_omission() {
    for format in [
        &specta_serde::Format as &dyn specta::Format,
        &specta_serde::PhasesFormat,
    ] {
        Typescript::default()
            .export(
                &Types::default().register::<ConvertedConditionalTuple>(),
                format,
            )
            .expect("container conversions replace the declared tuple wire shape");

        Typescript::default()
            .export(
                &Types::default().register::<ConvertedConditionalEnum>(),
                format,
            )
            .expect("container conversions replace the declared enum wire shape");
    }
}

#[test]
fn format_unifies_aliases() {
    let field = Typescript::default()
        .export(
            &Types::default().register::<FieldAlias>(),
            specta_serde::Format,
        )
        .expect("Format should include canonical and aliased field names");

    let variant = Typescript::default()
        .export(
            &Types::default().register::<VariantAlias>(),
            specta_serde::Format,
        )
        .expect("Format should include canonical and aliased variant names");

    insta::assert_snapshot!("serde-conversions-format-unified-field-alias", field);
    insta::assert_snapshot!("serde-conversions-format-unified-variant-alias", variant);
}

#[test]
fn format_rejects_alias_colliding_with_live_key() {
    let parsed: alias_collision::FieldAliasCollision = serde_json::from_str(r#"{"b":1}"#).unwrap();
    assert_eq!(parsed.a, 1);
    assert!(parsed.b.is_empty());

    let parsed: alias_collision::SharedFieldAlias =
        serde_json::from_str(r#"{"legacy":1}"#).unwrap();
    assert_eq!(parsed.a, 1);
    assert!(parsed.b.is_empty());

    for (types, collision) in [
        (
            Types::default().register::<alias_collision::FieldAliasCollision>(),
            "field alias `b` collides with a key already accepted by `b`",
        ),
        (
            Types::default().register::<alias_collision::SharedFieldAlias>(),
            "field alias `legacy` collides with a key already accepted by `a`",
        ),
    ] {
        let err = Typescript::default()
            .export(&types, specta_serde::Format)
            .expect_err("colliding aliases cannot be represented by unified intersections");
        assert!(
            err.to_string().contains(collision),
            "unexpected error: {err}"
        );
    }

    Typescript::default()
        .export(
            &Types::default()
                .register::<alias_collision::ConvertedFieldAliasCollision>()
                .register::<alias_collision::ConvertedVariantFieldAliasCollision>(),
            specta_serde::Format,
        )
        .expect("container conversions replace the colliding declared fields");
}

#[test]
fn format_unifies_aliases_with_flattened_fields() {
    let rendered = Typescript::default()
        .export(
            &Types::default()
                .register::<FieldAliasWithFlatten>()
                .register::<InternalFieldAliasWithFlatten>(),
            specta_serde::Format,
        )
        .expect("Format should preserve flattened fields while widening aliases");

    insta::assert_snapshot!(
        "serde-conversions-format-unified-field-alias-with-flatten",
        rendered
    );
}

#[test]
fn phases_format_exports_field_aliases() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FieldAlias>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support field aliases");

    insta::assert_snapshot!("serde-conversions-format-phases-field-alias", rendered);
}

#[test]
fn phases_format_exports_multiple_field_aliases() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FieldMultipleAliases>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support multiple field aliases");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-multiple-field-aliases",
        rendered
    );
}

#[test]
fn phases_format_exports_variant_aliases() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VariantAlias>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support externally tagged variant aliases");

    insta::assert_snapshot!("serde-conversions-format-phases-variant-alias", rendered);
}

#[test]
fn phases_format_exports_multiple_variant_aliases() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VariantMultipleAliases>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support multiple variant aliases");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-multiple-variant-aliases",
        rendered
    );
}

#[test]
fn phases_format_exports_tagged_variant_aliases() {
    let internal = Typescript::default()
        .export(
            &Types::default().register::<InternalVariantAlias>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support internally tagged variant aliases");
    let adjacent = Typescript::default()
        .export(
            &Types::default().register::<AdjacentVariantAlias>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should support adjacently tagged variant aliases");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-internal-variant-alias",
        internal
    );
    insta::assert_snapshot!(
        "serde-conversions-format-phases-adjacent-variant-alias",
        adjacent
    );
}

#[test]
fn field_phase_specific_rename_requires_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FieldPhaseSpecificRename>(),
            specta_serde::Format,
        )
        .expect_err("field-level phase-specific renames should require PhasesFormat");
    assert!(err.to_string().contains("Incompatible field key"));

    Typescript::default()
        .export(
            &Types::default().register::<FieldPhaseSpecificRename>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should accept field-level phase-specific renames");
}

#[test]
fn identifier_enums_require_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifierValid>(),
            specta_serde::Format,
        )
        .expect_err("identifier enums should require PhasesFormat");
    assert!(
        err.to_string()
            .contains("identifier enums require `PhasesFormat`")
    );

    Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifierValid>(),
            specta_serde::PhasesFormat,
        )
        .expect("valid variant_identifier enum should pass in PhasesFormat");
    Typescript::default()
        .export(
            &Types::default().register::<FieldIdentifierValid>(),
            specta_serde::PhasesFormat,
        )
        .expect("valid field_identifier enum should pass in PhasesFormat");
}
