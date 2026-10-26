//! A generated type has to declare `Sendable` itself for Swift 6 concurrency to
//! accept it: a consumer can only add `@unchecked Sendable`, which gives up the
//! compiler's check. Everything the exporter emits is a value type built from
//! `Sendable` parts, so the conformance is checked.
//!
//! Refs <https://github.com/specta-rs/specta/issues/621>

#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

#[derive(Type)]
struct Profile {
    name: String,
    age: u32,
    tags: Vec<String>,
    settings: HashMap<String, String>,
}

#[derive(Type)]
struct Account {
    profile: Profile,
}

#[derive(Type)]
struct Newtype(i32);

#[derive(Type)]
struct Pair(String, u8);

#[derive(Type)]
struct Timings {
    elapsed: Duration,
}

/// A unit-only enum has a synthesised `Codable`.
#[derive(Type)]
enum Color {
    Red,
    Green,
    Blue,
}

/// Struct and tuple variants make the exporter write its own `Codable`.
#[derive(Type)]
enum Event {
    Started { job_id: String },
    Failed(String),
    Cancelled,
}

/// An internally tagged enum needs a hand-written `Codable` and can carry an
/// `unknown` case.
#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    Ping,
    Data { value: String },
}

/// Generic types can only conform to `Sendable` conditionally.
#[derive(Type)]
enum Response<T> {
    Success { data: T },
    Failure { message: String },
}

fn swift_export(types: &Types, swift: &Swift) -> String {
    swift.export(types, specta_serde::Format).unwrap()
}

fn export<T: Type + 'static>(swift: &Swift) -> String {
    swift_export(&Types::default().register::<T>(), swift)
}

#[test]
fn test_every_generated_declaration_is_sendable() {
    let types = Types::default()
        .register::<Profile>()
        .register::<Account>()
        .register::<Newtype>()
        .register::<Pair>()
        .register::<Timings>()
        .register::<Color>()
        .register::<Event>();
    let output = swift_export(&types, &Swift::default());

    // Includes the second declaration `Timings` pulls in: the `RustDuration`
    // helper.
    let declarations = output
        .lines()
        .filter(|line| line.starts_with("public struct ") || line.starts_with("public enum "))
        .collect::<Vec<_>>();
    assert!(
        declarations.len() >= 8,
        "expected every type to be exported: {output}"
    );

    for declaration in declarations {
        assert!(
            declaration.contains("Sendable"),
            "type declaration is not Sendable: {declaration}"
        );
    }
}

#[test]
fn test_keyed_struct_is_sendable() {
    let output = export::<Profile>(&Swift::default());

    assert!(
        output.contains("public struct Profile: Codable, Sendable {"),
        "{output}"
    );
}

#[test]
fn test_newtype_and_tuple_structs_are_sendable() {
    for (output, declaration) in [
        (
            export::<Newtype>(&Swift::default()),
            "public struct Newtype: Codable, Sendable {",
        ),
        (
            export::<Pair>(&Swift::default()),
            "public struct Pair: Codable, Sendable {",
        ),
    ] {
        assert!(output.contains(declaration), "{output}");
    }
}

#[test]
fn test_string_enum_is_sendable() {
    let output = export::<Color>(&Swift::default());

    assert!(
        output.contains("public enum Color: String, Codable, Sendable {"),
        "{output}"
    );
}

#[test]
fn test_custom_codable_enum_is_sendable() {
    let output = export::<Event>(&Swift::default());

    // The hand-written `Codable` lives in an extension, so the declaration
    // carries only `Sendable`.
    assert!(
        output.contains("public enum Event: Sendable {"),
        "the custom Codable must not be declared twice: {output}"
    );
    assert!(
        output.contains("extension Event: Codable {"),
        "the custom Codable should stay in an extension: {output}"
    );

    // The variant wrapper structs are part of the public API too.
    assert!(
        output.contains("public struct EventStartedData: Codable, Sendable {"),
        "{output}"
    );
}

#[test]
fn test_unknown_variants_json_value_is_sendable() {
    let output = export::<Message>(&Swift::default().unknown_variants(true));

    assert!(
        output.contains("public enum JSONValue: Codable, Equatable, Sendable {"),
        "{output}"
    );
    assert!(
        output.contains("public enum Message: Sendable {"),
        "{output}"
    );
}

#[test]
fn test_duration_helper_is_sendable() {
    let output = export::<Timings>(&Swift::default());

    assert!(
        output.contains("public struct RustDuration: Codable, Sendable {"),
        "{output}"
    );
}

#[test]
fn test_generic_type_is_conditionally_sendable() {
    let output = export::<Response<String>>(&Swift::default());

    assert!(
        output.contains("extension Response: Sendable where T: Sendable {}"),
        "{output}"
    );
    // A declaration cannot constrain its own conformance, so `Sendable` must not
    // be listed where it would fail to compile.
    assert!(!output.contains("enum Response<T>: Sendable"), "{output}");

    // Note that generic types do not compile yet for an unrelated, pre-existing
    // reason: the wrapper structs of a generic enum (e.g. `ResponseSuccessData`)
    // reference `T` without declaring it.
}
