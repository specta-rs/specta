//! A named Rust enum with no variants is uninhabited, and Swift rejects the
//! shape an empty case list used to produce: it cannot declare a raw type or
//! synthesise `Codable`, so nothing that mentions the type compiles.
//!
//! The exporter declares the empty enum plainly and writes the conformance by
//! hand. Swift then treats the type as uninhabited, which is what the Rust type
//! says, and reports any generated code that runs after one as "will never be
//! executed" - a correct diagnostic for a type with no values.
//!
//! <https://github.com/specta-rs/specta/issues/625>

#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

/// A registry of features that are declared before their handlers exist, so it
/// is genuinely empty once every feature has shipped.
#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FeatureKey {}

#[derive(Type, Serialize, Deserialize)]
struct FeatureUnimplemented {
    feature_key: FeatureKey,
    message: String,
}

fn export<T: Type>() -> String {
    let types = Types::default().register::<T>();
    Swift::default()
        .export(&types, specta_serde::Format)
        .expect("an empty enum is exportable")
}

#[test]
fn a_named_enum_with_no_variants_does_not_declare_a_raw_type() {
    let output = export::<FeatureUnimplemented>();

    // The declaration Swift refused: no raw type, no synthesised `Codable`.
    assert!(
        output.contains("public enum FeatureKey: Sendable {\n}"),
        "{output}"
    );
    assert!(!output.contains("FeatureKey: String"), "{output}");
}

#[test]
fn a_named_enum_with_no_variants_carries_a_hand_written_codable() {
    let output = export::<FeatureUnimplemented>();

    assert!(
        output.contains("\nextension FeatureKey: Codable {\n"),
        "{output}"
    );
    // Decoding cannot invent a value, and encoding has no case to switch on.
    assert!(
        output.contains("throw DecodingError.dataCorruptedError("),
        "{output}"
    );
    assert!(
        output
            .contains("debugDescription: \"FeatureKey has no cases, so no value decodes as one\""),
        "{output}"
    );
    assert!(output.contains("        switch self {}\n"), "{output}");
}

#[test]
fn the_containing_type_references_the_named_type_rather_than_never() {
    let output = export::<FeatureUnimplemented>();

    assert!(
        output.contains("    public let featureKey: FeatureKey\n"),
        "{output}"
    );
    assert!(!output.contains("Never"), "{output}");
}

#[test]
fn an_empty_enum_exported_on_its_own_still_compiles() {
    let output = export::<FeatureKey>();

    assert!(
        output.contains("public enum FeatureKey: Sendable {\n}"),
        "{output}"
    );
    assert!(
        output.contains("extension FeatureKey: Codable {"),
        "{output}"
    );
}
