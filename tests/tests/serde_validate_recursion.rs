// Regression tests for https://github.com/specta-rs/specta/pull/517:
// `validate_internally_tag_enum_datatype` (specta-serde/src/validate.rs)
// recursed through named references without a visited set, so a self- or
// mutually-recursive untagged enum used as an internally-tagged enum's
// payload hung the export forever. serde_json serializes these shapes fine
// at runtime, so the export must succeed rather than error.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use serde::Serialize;
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum Rec {
    A(Box<Rec>),
    B(HashMap<String, String>),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum TagRec {
    X(Rec),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum RecA {
    A(Box<RecB>),
    Terminal(HashMap<String, String>),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum RecB {
    B(Box<RecA>),
    Terminal(HashMap<String, String>),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum TagMutual {
    Y(RecA),
}

/// Runs `f` on a background thread, panicking if it doesn't finish within
/// `timeout`, so a recursion regression fails the test instead of hanging
/// CI. The thread is leaked on timeout; the process exits shortly after.
fn with_timeout<T: Send + 'static>(timeout: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(timeout)
        .expect("operation timed out - likely an infinite recursion regression")
}

#[test]
fn serde_json_confirms_self_recursive_variant_serializes_fine() {
    let mut map = HashMap::new();
    map.insert("k".to_string(), "v".to_string());
    let value = TagRec::X(Rec::B(map));

    let json = serde_json::to_string(&value).expect("serde_json should serialize this fine");
    assert_eq!(json, r#"{"t":"X","k":"v"}"#);
}

#[test]
fn serde_json_confirms_mutually_recursive_variant_serializes_fine() {
    let mut map = HashMap::new();
    map.insert("k".to_string(), "v".to_string());
    let value = TagMutual::Y(RecA::Terminal(map));

    let json = serde_json::to_string(&value).expect("serde_json should serialize this fine");
    assert_eq!(json, r#"{"t":"Y","k":"v"}"#);
}

#[test]
fn self_recursive_untagged_enum_inside_internal_tag_does_not_hang() {
    let ts = with_timeout(Duration::from_secs(10), || {
        Typescript::default().export(&Types::default().register::<TagRec>(), specta_serde::Format)
    })
    .expect("export should succeed - serde_json serializes this fine at runtime");

    insta::assert_snapshot!("serde-validate-recursion-self-recursive-typescript", ts);
}

#[test]
fn mutually_recursive_untagged_enums_inside_internal_tag_does_not_hang() {
    let ts = with_timeout(Duration::from_secs(10), || {
        Typescript::default().export(
            &Types::default().register::<TagMutual>(),
            specta_serde::Format,
        )
    })
    .expect("export should succeed - serde_json serializes this fine at runtime");

    insta::assert_snapshot!("serde-validate-recursion-mutually-recursive-typescript", ts);
}
