//! This file is run with the `trybuild` crate to assert compilation errors in the Specta macros.

use specta::{Type, specta};

// Invalid inflection
#[derive(Type)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase123")]
pub enum Demo2 {}

// Specta doesn't support Trait objects
#[derive(Type)]
#[specta(collect = false)]
pub struct Error {
    pub(crate) cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

// Enums can only flatten if
// at least one of their variants can flatten

#[derive(Type)]
#[specta(collect = false)]
enum UnitExternal {
    Unit,
}

#[derive(Type)]
#[specta(collect = false)]
enum UnnamedMultiExternal {
    UnnamedMulti(String, String),
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenExternal {
    #[serde(flatten)]
    unit: UnitExternal,
    #[serde(flatten)]
    unnamed_multi: UnnamedMultiExternal,
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
enum UnnamedUntagged {
    Unnamed(String),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
enum UnnamedMultiUntagged {
    Unnamed(String, String),
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenUntagged {
    #[serde(flatten)]
    unnamed: UnnamedUntagged,
    #[serde(flatten)]
    unnamed_multi: UnnamedMultiUntagged,
}

// Adjacent can always flatten

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "tag")]
enum UnnamedInternal {
    Unnamed(String),
}

// Internal can't be used with unnamed multis

#[derive(Type)]
#[specta(collect = false)]
struct FlattenInternal {
    #[serde(flatten)]
    unnamed: UnnamedInternal,
}

// Invalid attributes
#[derive(Type)]
#[specta(collect = false)]
#[specta(noshot = true)]
struct InvalidAttrs1;

#[derive(Type)]
#[specta(collect = false)]
#[specta(noshot)]
struct InvalidAttrs2;

#[derive(Type)]
#[specta(collect = false)]
struct InvalidAttrs3 {
    #[specta(noshot = true)]
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct InvalidAttrs4 {
    #[specta(noshot)]
    a: String,
}

// Legacy `#[specta(...)]` migration errors
#[derive(Type)]
#[specta(collect = false)]
#[specta(rename = "Renamed")]
struct LegacyContainerRename;

#[derive(Type)]
#[specta(collect = false)]
struct LegacyFieldRename {
    #[specta(rename = "renamed")]
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
enum LegacyVariantRename {
    #[specta(rename = "renamed")]
    A,
}

const INTERNAL_RENAME_KEY: &str = "renamed";

#[derive(Type)]
#[specta(collect = false)]
struct InternalRenameFromPath {
    #[specta(rename_from_path = INTERNAL_RENAME_KEY)]
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
#[specta(transparent)]
pub enum TransparentEnum {}

#[derive(Type)]
#[specta(collect = false)]
#[specta]
pub struct InvalidSpectaAttribute1;

#[derive(Type)]
#[specta(collect = false)]
#[specta = "todo"]
pub struct InvalidSpectaAttribute2;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[specta]
pub fn testing() {}

// TODO: https://docs.rs/trybuild/latest/trybuild/#what-to-test
