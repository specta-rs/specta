//! A named Rust enum with no variants is uninhabited, and Swift rejects the
//! shape an empty case list used to produce: a raw type cannot be declared and
//! `Codable` cannot be synthesised, so nothing that mentions the type compiles.
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
}

fn export<T: Type>() -> String {
    let types = Types::default().register::<T>();
    Swift::default()
        .export(&types, specta_serde::Format)
        .expect("an empty enum is exportable")
}

#[test]
fn a_named_enum_with_no_variants_is_not_an_empty_string_enum() {
    let output = export::<FeatureUnimplemented>();

    // The declaration Swift refused: no raw type, no synthesised `Codable`.
    assert!(!output.contains("enum FeatureKey"), "{output}");
    assert!(
        output.contains("public struct FeatureKey: Codable, Sendable {"),
        "{output}"
    );
}

#[test]
fn a_named_enum_with_no_variants_cannot_be_constructed_or_decoded() {
    let output = export::<FeatureUnimplemented>();

    // A `private` initialiser keeps the type uninhabited outside the generated
    // file, and decoding always throws instead of inventing a value.
    assert!(output.contains("    private init() {}\n"), "{output}");
    assert!(
        output.contains("throw DecodingError.dataCorruptedError("),
        "{output}"
    );
    assert!(
        output
            .contains("debugDescription: \"FeatureKey has no cases, so no value decodes as one\""),
        "{output}"
    );
    assert!(output.contains("try container.encodeNil()"), "{output}");

    // The struct must be closed in the right place, or every later declaration
    // nests inside it.
    assert!(
        output.contains("        try container.encodeNil()\n    }\n}\n"),
        "{output}"
    );
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

    assert!(!output.contains("enum FeatureKey"), "{output}");
    assert!(
        output.contains("public struct FeatureKey: Codable, Sendable {"),
        "{output}"
    );
}
