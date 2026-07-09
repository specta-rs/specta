// Verifies the userspace `MaybeUndefined` pattern from
// https://github.com/specta-rs/specta/issues/157
//
// Inspired by: https://docs.rs/async-graphql/latest/async_graphql/types/enum.MaybeUndefined.html

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::{Type, Types, datatype::DataType};
use specta_typescript::Typescript;

/// Represents a field that may be undefined, null, or have a value.
///
/// Only meaningful as a field of a struct, paired with
/// `#[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MaybeUndefined<T> {
    /// Field was not present in the input.
    Undefined,
    /// Field was present and set to null.
    Null,
    /// Field was present and had a value.
    Value(T),
}

impl<T> MaybeUndefined<T> {
    fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
}

// Required by `#[serde(default)]` on fields using this type.
impl<T> Default for MaybeUndefined<T> {
    fn default() -> Self {
        Self::Undefined
    }
}

impl<T> Serialize for MaybeUndefined<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serializes as `T | null`. `undefined` is handled by
        // #[serde(skip_serializing_if = "MaybeUndefined::is_undefined")]
        match self {
            // NOTE: a field type cannot make itself "missing".
            // This will serialize as `null` if it is serialized.
            MaybeUndefined::Undefined | MaybeUndefined::Null => serializer.serialize_none(),
            MaybeUndefined::Value(v) => v.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for MaybeUndefined<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserializes as `T | null`. `undefined` is handled by #[serde(default)]
        Ok(match Option::<T>::deserialize(deserializer)? {
            None => MaybeUndefined::Null,
            Some(v) => MaybeUndefined::Value(v),
        })
    }
}

impl<T: Type> Type for MaybeUndefined<T> {
    fn definition(types: &mut Types) -> DataType {
        // `undefined` is Typescript-specific so it can't be expressed by
        // `DataType`; the field attributes supply the optionality.
        Option::<T>::definition(types)
    }
}

#[derive(Serialize, Deserialize, Type)]
#[specta(collect = false)]
struct UpdateUser {
    id: u32,
    #[serde(default, skip_serializing_if = "MaybeUndefined::is_undefined")]
    name: MaybeUndefined<String>,
}

#[test]
fn maybe_undefined_phases() {
    let types = Types::default().register::<UpdateUser>();
    let ts = Typescript::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("phased typescript export should succeed");
    insta::assert_snapshot!("maybe-undefined-phases", ts);
}

#[test]
fn maybe_undefined_single_phase() {
    let types = Types::default().register::<UpdateUser>();
    match Typescript::default().export(&types, specta_serde::Format) {
        Ok(ts) => insta::assert_snapshot!("maybe-undefined-single-phase", ts),
        Err(err) => insta::assert_snapshot!("maybe-undefined-single-phase-error", err.to_string()),
    }
}
