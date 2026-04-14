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

fn type_names(types: &Types) -> Vec<String> {
    types
        .into_unsorted_iter()
        .map(|ndt| ndt.name.to_string())
        .collect()
}

#[test]
fn apply_rejects_asymmetric_container_conversion() {
    let err = specta_serde::apply(Types::default().register::<IntoOnly>())
        .expect_err("apply should reject asymmetric serde conversions");

    assert!(
        err.to_string()
            .contains("Incompatible container conversion"),
        "unexpected error: {err}"
    );
}

#[test]
fn apply_phases_splits_container_and_dependents_for_conversions() {
    let types = specta_serde::apply_phases(Types::default().register::<Parent>())
        .expect("apply_phases should support asymmetric serde conversions");
    let names = type_names(&types);

    assert!(names.iter().any(|name| name == "IntoOnly_Serialize"));
    assert!(names.iter().any(|name| name == "IntoOnly_Deserialize"));
    assert!(names.iter().any(|name| name == "Parent_Serialize"));
    assert!(names.iter().any(|name| name == "Parent_Deserialize"));
}

#[test]
fn apply_accepts_symmetric_container_conversion() {
    specta_serde::apply(Types::default().register::<Symmetric>())
        .expect("apply should accept symmetric serde conversions");
}

#[test]
fn apply_phases_accepts_generic_try_from_container_conversion() {
    let err = specta_serde::apply(Types::default().register::<GenericParent<String>>())
        .expect_err("apply should reject deserialize-only container conversions");
    assert!(
        err.to_string()
            .contains("Incompatible container conversion")
    );

    specta_serde::apply_phases(Types::default().register::<GenericParent<String>>())
        .expect("apply_phases should resolve nested generic references from container conversions");
}

#[test]
fn custom_codec_requires_explicit_override() {
    let err = specta_serde::apply(Types::default().register::<CustomCodecNoOverride>())
        .expect_err("custom serde codecs should require #[specta(type = ...)]");

    assert!(err.to_string().contains("Unsupported serde attribute"));

    specta_serde::apply(Types::default().register::<CustomCodecWithOverride>())
        .expect("override should satisfy custom serde codecs");
}

#[test]
fn custom_codec_variant_requires_explicit_override() {
    let err = specta_serde::apply(Types::default().register::<VariantCodecNoOverride>())
        .expect_err("variant custom serde codecs should require #[specta(type = ...)]");
    assert!(err.to_string().contains("Unsupported serde attribute"));

    specta_serde::apply(Types::default().register::<VariantCodecWithOverride>())
        .expect("variant override should satisfy custom serde codecs");
}

#[test]
fn phased_override_requires_apply_phases() {
    let err = specta_serde::apply(Types::default().register::<CustomCodecWithPhasedOverride>())
        .expect_err("apply should reject phased overrides");
    assert!(err.to_string().contains("requires `apply_phases`"));

    specta_serde::apply_phases(Types::default().register::<CustomCodecWithPhasedOverride>())
        .expect("apply_phases should accept phased overrides");
}

#[test]
fn field_only_phased_override_requires_apply_phases() {
    let err = specta_serde::apply(Types::default().register::<FieldOnlyPhasedOverride>())
        .expect_err("apply should reject phased field overrides");
    assert!(err.to_string().contains("requires `apply_phases`"));

    let raw_err = Typescript::default()
        .format(crate::raw_format)
        .export(&Types::default().register::<FieldOnlyPhasedOverride>())
        .expect_err("raw export should fail on unresolved phased opaque reference");
    assert!(raw_err.to_string().contains("unsupported opaque reference"));

    let phased_types =
        specta_serde::apply_phases(Types::default().register::<FieldOnlyPhasedOverride>())
            .expect("apply_phases should accept phased field overrides");
    Typescript::default()
        .format(crate::raw_format)
        .export(&phased_types)
        .expect("phased export should remove phased opaque references");
}

#[test]
fn format_phases_exports_field_only_phased_override() {
    let rendered = Typescript::default()
        .format(specta_serde::format_phases)
        .export(&Types::default().register::<FieldOnlyPhasedOverride>())
        .expect("format_phases should resolve phased overrides during export");

    assert!(rendered.contains("FieldOnlyPhasedOverride_Serialize"));
    assert!(rendered.contains("FieldOnlyPhasedOverride_Deserialize"));
}

#[test]
fn skip_serializing_if_requires_phases() {
    let err = specta_serde::apply(Types::default().register::<SkipSerializingIfOnly>())
        .expect_err("skip_serializing_if should require apply_phases");
    assert!(err.to_string().contains("skip_serializing_if"));

    specta_serde::apply_phases(Types::default().register::<SkipSerializingIfOnly>())
        .expect("apply_phases should accept skip_serializing_if");
}

#[test]
fn field_phase_specific_rename_requires_phases() {
    let err = specta_serde::apply(Types::default().register::<FieldPhaseSpecificRename>())
        .expect_err("field-level phase-specific renames should require apply_phases");
    assert!(err.to_string().contains("Incompatible field rename"));

    specta_serde::apply_phases(Types::default().register::<FieldPhaseSpecificRename>())
        .expect("apply_phases should accept field-level phase-specific renames");
}

#[test]
fn identifier_enums_require_phases() {
    let err = specta_serde::apply(Types::default().register::<VariantIdentifierValid>())
        .expect_err("identifier enums should require apply_phases");
    assert!(
        err.to_string()
            .contains("identifier enums require `apply_phases`")
    );

    specta_serde::apply_phases(Types::default().register::<VariantIdentifierValid>())
        .expect("valid variant_identifier enum should pass in apply_phases");
    specta_serde::apply_phases(Types::default().register::<FieldIdentifierValid>())
        .expect("valid field_identifier enum should pass in apply_phases");
}
