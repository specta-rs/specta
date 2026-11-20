//! Golden-file tests that export each example and compare it to the checked-in output.

use specta_rescript::ReScript;

fn assert_export_eq(actual: String, golden: &str) {
    assert_eq!(actual.replace("\r\n", "\n"), golden.replace("\r\n", "\n"));
}

#[path = "../examples/simple_usage.rs"]
mod simple_usage;

#[path = "../examples/basic_types.rs"]
mod basic_types;

#[path = "../examples/variants.rs"]
mod variants;

#[path = "../examples/generics.rs"]
mod generics;

#[path = "../examples/result_types.rs"]
mod result_types;

#[path = "../examples/comments_example.rs"]
mod comments_example;

#[path = "../examples/comprehensive_demo.rs"]
mod comprehensive_demo;

#[path = "../examples/recursive.rs"]
mod recursive;

#[test]
fn simple_usage_default_export() {
    let golden = include_str!("../examples/generated/SimpleUsage.res");
    assert_export_eq(
        ReScript::default().export(&simple_usage::types()).unwrap(),
        golden,
    );
}

#[test]
fn basic_types_export() {
    let golden = include_str!("../examples/generated/BasicTypes.res");
    assert_export_eq(
        ReScript::default()
            .without_serde()
            .export(&basic_types::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn variants_export() {
    let golden = include_str!("../examples/generated/Variants.res");
    assert_export_eq(
        ReScript::default()
            .without_serde()
            .export(&variants::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn generics_export() {
    let golden = include_str!("../examples/generated/Generics.res");
    assert_export_eq(
        ReScript::default()
            .without_serde()
            .export(&generics::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn result_types_export() {
    let golden = include_str!("../examples/generated/ResultTypes.res");
    assert_export_eq(
        ReScript::default()
            .without_serde()
            .export(&result_types::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn comments_example_export() {
    let golden = include_str!("../examples/generated/CommentsExample.res");
    assert_export_eq(
        ReScript::default()
            .export(&comments_example::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn comprehensive_demo_export() {
    let golden = include_str!("../examples/generated/ComprehensiveDemo.res");
    assert_export_eq(
        ReScript::default()
            .export(&comprehensive_demo::types())
            .unwrap(),
        golden,
    );
}

#[test]
fn recursive_export() {
    let golden = include_str!("../examples/generated/Recursive.res");
    assert_export_eq(
        ReScript::default()
            .without_serde()
            .export(&recursive::types())
            .unwrap(),
        golden,
    );
}
