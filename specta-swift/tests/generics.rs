//! A generic declaration has to constrain its parameters, otherwise its own
//! `Codable` conformance does not hold and its variant wrapper structs refer to
//! `T` without declaring it. `GenericStyle` picks where the bound goes: on the
//! parameter, or in a `where` clause.

#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use specta::{Type, Types};
use specta_swift::{GenericStyle, Swift};

#[derive(Type)]
struct Boxed<T> {
    value: T,
}

#[derive(Type)]
enum Response<T> {
    Data { value: T, count: u32 },
    Empty,
}

fn export(swift: &Swift) -> String {
    let types = Types::default()
        .register::<Boxed<String>>()
        .register::<Response<u32>>();
    swift.export(&types, specta_serde::Format).unwrap()
}

#[test]
fn test_protocol_style_constrains_the_parameter() {
    let output = export(&Swift::default());

    assert!(
        output.contains("public struct Boxed<T: Codable>: Codable {"),
        "{output}"
    );
    assert!(
        output.contains("public enum Response<T: Codable> {"),
        "{output}"
    );
    assert!(
        output.contains("public struct ResponseDataData<T: Codable>: Codable {"),
        "a wrapper struct must repeat the enum's parameters: {output}"
    );
    assert!(
        output.contains("case data(ResponseDataData<T>)"),
        "a case payload must pass them through: {output}"
    );
}

#[test]
fn test_where_style_constrains_the_declaration() {
    let output = export(&Swift::default().generics(GenericStyle::Typealias));

    assert!(
        output.contains("public struct Boxed<T>: Codable where T: Codable {"),
        "{output}"
    );
    assert!(
        output.contains("public enum Response<T> where T: Codable {"),
        "{output}"
    );
    assert!(
        output.contains("public struct ResponseDataData<T>: Codable where T: Codable {"),
        "{output}"
    );
}
