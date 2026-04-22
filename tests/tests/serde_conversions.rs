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
            specta_serde::format,
        )
        .expect_err("apply should reject asymmetric serde conversions");

    assert!(
        err.to_string()
            .contains("Incompatible container conversion"),
        "unexpected error: {err}"
    );
}

#[test]
fn format_phases_splits_container_and_dependents_for_conversions() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<Parent>(),
            specta_serde::format_phases,
        )
        .expect("format_phases should support asymmetric serde conversions");

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
            specta_serde::format,
        )
        .expect("apply should accept symmetric serde conversions");
}

#[test]
fn format_phases_accepts_generic_try_from_container_conversion() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<GenericParent<String>>(),
            specta_serde::format,
        )
        .expect_err("apply should reject deserialize-only container conversions");
    assert!(
        err.to_string()
            .contains("Incompatible container conversion")
    );

    Typescript::default()
        .export(
            &Types::default().register::<GenericParent<String>>(),
            specta_serde::format_phases,
        )
        .expect(
            "format_phases should resolve nested generic references from container conversions",
        );
}

#[test]
fn custom_codec_requires_explicit_override() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<CustomCodecNoOverride>(),
            specta_serde::format,
        )
        .expect_err("custom serde codecs should require #[specta(type = ...)]");

    assert!(err.to_string().contains("Unsupported serde attribute"));

    Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithOverride>(),
            specta_serde::format,
        )
        .expect("override should satisfy custom serde codecs");
}

#[test]
fn custom_codec_variant_requires_explicit_override() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantCodecNoOverride>(),
            specta_serde::format,
        )
        .expect_err("variant custom serde codecs should require #[specta(type = ...)]");
    assert!(err.to_string().contains("Unsupported serde attribute"));

    Typescript::default()
        .export(
            &Types::default().register::<VariantCodecWithOverride>(),
            specta_serde::format,
        )
        .expect("variant override should satisfy custom serde codecs");
}

#[test]
fn phased_override_requires_format_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithPhasedOverride>(),
            specta_serde::format,
        )
        .expect_err("apply should reject phased overrides");
    assert!(err.to_string().contains("requires `format_phases`"));

    Typescript::default()
        .export(
            &Types::default().register::<CustomCodecWithPhasedOverride>(),
            specta_serde::format_phases,
        )
        .expect("format_phases should accept phased overrides");
}

#[test]
fn field_only_phased_override_requires_format_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::format,
        )
        .expect_err("apply should reject phased field overrides");
    assert!(err.to_string().contains("requires `format_phases`"));

    Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::format_phases,
        )
        .expect("phased export should remove phased opaque references");
}

#[test]
fn format_phases_exports_field_only_phased_override() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FieldOnlyPhasedOverride>(),
            specta_serde::format_phases,
        )
        .expect("format_phases should resolve phased overrides during export");

    insta::assert_snapshot!(
        "serde-conversions-format-phases-exports-field-only-phased-override",
        rendered
    );
}

#[test]
fn skip_serializing_if_requires_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<SkipSerializingIfOnly>(),
            specta_serde::format,
        )
        .expect_err("skip_serializing_if should require format_phases");
    assert!(err.to_string().contains("skip_serializing_if"));

    Typescript::default()
        .export(
            &Types::default().register::<SkipSerializingIfOnly>(),
            specta_serde::format_phases,
        )
        .expect("format_phases should accept skip_serializing_if");
}

#[test]
fn field_phase_specific_rename_requires_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FieldPhaseSpecificRename>(),
            specta_serde::format,
        )
        .expect_err("field-level phase-specific renames should require format_phases");
    assert!(err.to_string().contains("Incompatible field rename"));

    Typescript::default()
        .export(
            &Types::default().register::<FieldPhaseSpecificRename>(),
            specta_serde::format_phases,
        )
        .expect("format_phases should accept field-level phase-specific renames");
}

#[test]
fn identifier_enums_require_phases() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifierValid>(),
            specta_serde::format,
        )
        .expect_err("identifier enums should require format_phases");
    assert!(
        err.to_string()
            .contains("identifier enums require `format_phases`")
    );

    Typescript::default()
        .export(
            &Types::default().register::<VariantIdentifierValid>(),
            specta_serde::format_phases,
        )
        .expect("valid variant_identifier enum should pass in format_phases");
    Typescript::default()
        .export(
            &Types::default().register::<FieldIdentifierValid>(),
            specta_serde::format_phases,
        )
        .expect("valid field_identifier enum should pass in format_phases");
}
