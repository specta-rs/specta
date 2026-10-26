//! `add_protocol` is documented as "a protocol that all generated types should
//! conform to", but it only emitted `import <protocol>`, which does not compile
//! for a protocol. It now appends the protocol to each declaration's conformance
//! list, and `add_import` keeps the import behaviour for modules.
//!
//! Refs <https://github.com/specta-rs/specta/issues/621#issuecomment-6376290428>

#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

#[derive(Type)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Type)]
struct Boxed<T> {
    value: T,
}

#[derive(Type)]
enum Role {
    Admin,
    Guest,
}

#[derive(Type)]
enum Shape {
    Empty,
    Named { corner: Point },
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    Ping,
    Data { value: String },
}

#[derive(Type)]
struct Timed {
    elapsed: Duration,
}

fn export(swift: &Swift) -> String {
    let types = Types::default()
        .register::<Point>()
        .register::<Boxed<String>>()
        .register::<Role>()
        .register::<Shape>()
        .register::<Message>()
        .register::<Timed>();
    swift.export(&types, specta_serde::Format).unwrap()
}

#[test]
fn test_add_protocol_applies_to_every_generated_type() {
    let output = export(
        &Swift::default()
            .unknown_variants(true)
            .add_protocol("Equatable")
            .add_protocol("Hashable"),
    );

    // `JSONValue` and `RustDuration` are generated alongside the registered
    // types, and are part of the public API.
    assert!(
        output.contains("public enum JSONValue: Codable, Equatable, Sendable, Hashable {"),
        "{output}"
    );
    assert!(
        output.contains("public struct RustDuration: Codable, Sendable, Equatable, Hashable {"),
        "{output}"
    );

    let declarations = output
        .lines()
        .filter(|line| line.starts_with("public struct ") || line.starts_with("public enum "))
        .collect::<Vec<_>>();
    assert!(!declarations.is_empty());
    for declaration in declarations {
        assert!(
            declaration.contains(", Equatable") && declaration.ends_with("Hashable {"),
            "configured protocols are missing: {declaration}"
        );
    }
}

#[test]
fn test_add_protocol_does_not_emit_an_import() {
    let output = export(&Swift::default().add_protocol("Equatable"));

    assert!(
        !output.contains("import Equatable"),
        "a protocol is a conformance, not a module: {output}"
    );
}

#[test]
fn test_add_import_emits_an_import() {
    let output = export(&Swift::default().add_import("MyModule"));

    assert!(output.contains("import MyModule\n"), "{output}");
    assert!(
        !output.contains(", MyModule"),
        "an import is not a conformance: {output}"
    );
}

#[test]
fn test_configured_codable_and_sendable_are_not_duplicated() {
    let output = export(
        &Swift::default()
            .add_protocol("Codable")
            .add_protocol("Sendable"),
    );

    assert!(
        output.contains("public struct Point: Codable, Sendable {"),
        "{output}"
    );
    assert!(!output.contains("Codable, Sendable, Codable"), "{output}");
}

#[test]
fn test_add_protocol_extends_the_generic_bound() {
    let output = export(&Swift::default().add_protocol("Equatable"));

    assert!(
        output.contains("public struct Boxed<T: Codable & Equatable>: Codable, Equatable {"),
        "{output}"
    );
    assert!(
        output.contains("extension Boxed: Sendable where T: Sendable {}"),
        "{output}"
    );
}
