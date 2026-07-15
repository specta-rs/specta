// Regression test for a hang in `validate_internally_tag_enum_datatype`
// (specta-serde/src/validate.rs).
//
// Internally-tagged enums with an untagged-enum payload are validated by
// recursively descending through the payload's variants and any named
// references they contain, looking for a shape that can be merged with the
// internal tag. That recursion previously had no visited set, so a
// self-recursive or mutually-recursive untagged enum reachable from an
// internally-tagged enum's payload caused unbounded recursion and the
// exporting process would never return.
//
// `serde_json` itself has no trouble serializing this shape at runtime (an
// internally-tagged enum whose payload is an untagged enum that recurses
// through itself), so the export is expected to succeed rather than fail
// validation.

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

/// Runs `f` on a background thread and panics if it doesn't complete within
/// `timeout`.
///
/// A regression of the bug this test guards against hangs forever instead of
/// failing an assertion, which would otherwise hang the test binary (and CI)
/// indefinitely. Running the risky call on its own thread and racing it
/// against a bounded `recv_timeout` turns that hang into a normal test
/// failure. The spawned thread is intentionally leaked on timeout - there's
/// no way to cancel it, but the process exits shortly after the test fails
/// anyway.
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
