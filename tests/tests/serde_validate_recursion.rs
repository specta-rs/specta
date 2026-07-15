// Regression tests for https://github.com/specta-rs/specta/pull/517:
// `validate_internally_tag_enum_datatype` (specta-serde/src/validate.rs)
// recursed through named references without a visited set, so a self- or
// mutually-recursive untagged enum used as an internally-tagged enum's
// payload hung the export forever. serde_json serializes these shapes fine
// at runtime, so the export must succeed rather than error.
//
// Fixing the hang exposed a follow-up bug flagged on that same PR
// (https://github.com/specta-rs/specta/pull/517#discussion_r3584346217):
// once the export stopped hanging, it produced TypeScript that `tsc`
// rejects with TS2456 ("Type alias circularly references itself"), e.g.
// `export type Rec = Rec | { [key in string]: string };`. `Rec::A` is a
// `#[serde(untagged)]` newtype variant, so it adds no wire structure - every
// finite `Rec` value is, at the wire level, exactly a `Rec::B` map value.
// The fix collapses such alias-transparent self/mutual references out of
// the union instead of emitting an illegal alias. See
// `collapse_untagged_alias_cycle` in `specta-typescript/src/primitives.rs`.

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

/// Legitimate recursion through object structure, which TypeScript has no
/// trouble with (`type Tree = { children: Tree[] }`) because array/object
/// types defer resolution instead of eagerly inlining. Exporting this must
/// not change: it doesn't go through any alias-transparent hop at all.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Tree {
    children: Vec<Tree>,
}

/// An untagged enum whose newtype variant references a *different*
/// recursive type (`Tree`) non-cyclically - `Tree` never refers back to
/// `WrapsTree`, so no alias-transparent cycle exists and this must render
/// exactly as before (a plain reference to `Tree` in the union).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum WrapsTree {
    Node(Box<Tree>),
    Leaf(u32),
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

/// `Rec::A(Box<Rec>)` used to make `Rec`'s own alias directly reference
/// itself in a union member (`export type Rec = Rec | ...;`), which `tsc`
/// rejects with TS2456. Every finite `Rec` value bottoms out at `Rec::B`
/// (untagged newtype variants add no wire structure), so the direct
/// self-reference is a fixpoint identity and can be dropped: `Rec`'s real
/// wire type is exactly the map type.
#[test]
fn self_recursive_untagged_enum_collapses_to_non_recursive_branch() {
    let ts = Typescript::default()
        .export(&Types::default().register::<Rec>(), specta_serde::Format)
        .expect("export should succeed");

    assert_eq!(
        ts.trim_start_matches(
            "// This file has been generated by Specta. Do not edit this file manually.\n"
        )
        .trim(),
        "export type Rec = { [key in string]: string };"
    );
    assert!(
        !ts.contains("Rec | "),
        "Rec's own alias must not reference itself: {ts}"
    );
}

/// Same as above but for a *mutual* 2-cycle: `RecA` and `RecB` each wrap a
/// `Box` of the other, so naively rendering `RecA` produces `export type
/// RecA = RecB | map;` and `RecB` produces `export type RecB = RecA |
/// map;` - a circular alias pair `tsc` also rejects with TS2456. Both
/// `RecA` and `RecB` bottom out at the same terminal map type, so both
/// collapse to it, exactly like the self-recursive case.
#[test]
fn mutually_recursive_untagged_enums_collapse_to_non_recursive_branch() {
    // `RecA` and `RecB` reference each other, so registering either one
    // pulls in both - assert on each type's own alias line rather than the
    // whole output, since both `export type ... = ...;` statements are
    // present in both exports.
    let ts = Typescript::default()
        .export(&Types::default().register::<RecA>(), specta_serde::Format)
        .expect("export should succeed");

    assert!(
        ts.contains("export type RecA = { [key in string]: string };"),
        "RecA should collapse to the terminal map type: {ts}"
    );
    assert!(
        !ts.contains("RecA = RecB") && !ts.contains("RecB | "),
        "RecA's own alias must not reference RecB, directly or otherwise: {ts}"
    );

    let ts = Typescript::default()
        .export(&Types::default().register::<RecB>(), specta_serde::Format)
        .expect("export should succeed");

    assert!(
        ts.contains("export type RecB = { [key in string]: string };"),
        "RecB should collapse to the terminal map type: {ts}"
    );
    assert!(
        !ts.contains("RecB = RecA") && !ts.contains("RecA | "),
        "RecB's own alias must not reference RecA, directly or otherwise: {ts}"
    );
}

/// Legitimate recursion through object structure must not be affected by
/// the untagged-alias-cycle collapse: `Tree` isn't an enum at all, so
/// `collapse_untagged_alias_cycle` bails out immediately, and the existing
/// `{ children: Tree[] }` rendering (already legal TypeScript) is
/// untouched.
#[test]
fn struct_object_recursion_is_unaffected() {
    let ts = Typescript::default()
        .export(&Types::default().register::<Tree>(), specta_serde::Format)
        .expect("export should succeed");

    insta::assert_snapshot!("serde-validate-recursion-tree-object-recursion", ts);
}

/// An untagged enum referencing a *different* recursive type non-cyclically
/// must also be unaffected: `WrapsTree -> Tree` is a real edge in the
/// alias-transparent graph, but `Tree` never points back to `WrapsTree`, so
/// there's no cycle and `WrapsTree` keeps referencing `Tree` by name.
#[test]
fn untagged_enum_referencing_different_recursive_type_non_cyclically_is_unaffected() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<WrapsTree>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    insta::assert_snapshot!("serde-validate-recursion-wraps-tree-non-cyclic", ts);
}
