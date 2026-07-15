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

/// Shorthand for the exports below, which all register a single root type.
fn export<T: Type>() -> String {
    Typescript::default()
        .export(&Types::default().register::<T>(), specta_serde::Format)
        .expect("export should succeed")
}

/// A mutual cycle where the two members have *different* terminal branches.
/// A `DiffA` value can be `DiffA::A(Box<DiffB::B(42)>)`, whose wire value is
/// just `42`, so `DiffA`'s static type must include `DiffB`'s terminals too:
/// both aliases collapse to the union of *all* terminals in the cycle, each
/// listing its own terminals first (merge order is discovery order, so the
/// output is deterministic).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum DiffA {
    A(Box<DiffB>),
    B(HashMap<String, String>),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum DiffB {
    A(Box<DiffA>),
    B(u32),
}

#[test]
fn mutual_cycle_with_different_terminals_merges_all_terminals() {
    // serde ground truth: a `DiffA` value's wire form can be a bare number.
    let json = serde_json::to_string(&DiffA::A(Box::new(DiffB::B(42)))).unwrap();
    assert_eq!(json, "42");

    let ts = export::<DiffA>();
    assert!(
        ts.contains("export type DiffA = { [key in string]: string } | number;"),
        "DiffA must union its own terminal with DiffB's: {ts}"
    );
    assert!(
        ts.contains("export type DiffB = number | { [key in string]: string };"),
        "DiffB must union its own terminal with DiffA's: {ts}"
    );
}

/// A cycle member referenced from *outside* the cycle (a struct field) must
/// still resolve to the collapsed alias - the field keeps referencing
/// `DiffB` by name and `DiffB`'s own export is the collapsed union.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct HoldsDiffB {
    x: DiffB,
}

#[test]
fn cycle_member_referenced_from_outside_the_cycle_stays_a_named_reference() {
    let ts = export::<HoldsDiffB>();
    assert!(ts.contains("x: DiffB"), "field must reference DiffB: {ts}");
    assert!(
        ts.contains("export type DiffB = number | { [key in string]: string };"),
        "DiffB's standalone export must still be collapsed: {ts}"
    );
}

/// Three-cycles collapse the same way as two-cycles: every member unions
/// every terminal reachable in the cycle, its own first.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum Tri1 {
    Next(Box<Tri2>),
    T(String),
}
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum Tri2 {
    Next(Box<Tri3>),
    T(u32),
}
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum Tri3 {
    Next(Box<Tri1>),
    T(bool),
}

#[test]
fn three_cycle_merges_all_terminals_deterministically() {
    let ts = export::<Tri1>();
    assert!(
        ts.contains("export type Tri1 = string | number | boolean;"),
        "{ts}"
    );
    assert!(
        ts.contains("export type Tri2 = number | boolean | string;"),
        "{ts}"
    );
    assert!(
        ts.contains("export type Tri3 = boolean | string | number;"),
        "{ts}"
    );
}

/// A cycle that passes through a *tagged* enum is not alias-transparent:
/// the tag adds wire structure, `ExtBack` renders as an object
/// (`{ V: ThroughExt }`) which defers alias resolution, and the recursion
/// is legal TypeScript. Nothing may be collapsed.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ThroughExt {
    X(Box<ExtBack>),
    Y(u32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum ExtBack {
    V(Box<ThroughExt>),
}

#[test]
fn cycle_through_externally_tagged_enum_is_not_collapsed() {
    let ts = export::<ThroughExt>();
    assert!(
        ts.contains("export type ThroughExt = ExtBack | number;"),
        "the tagged branch must be kept as a reference: {ts}"
    );
    assert!(
        ts.contains("export type ExtBack = { V: ThroughExt };"),
        "the tagged enum must keep its object shape: {ts}"
    );
}

/// Self-recursive *generic* untagged enums collapse with the parameter kept
/// intact.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum GenRec<T> {
    A(Box<GenRec<T>>),
    B(T),
}

#[test]
fn generic_self_recursive_untagged_enum_collapses_keeping_the_parameter() {
    let ts = export::<GenRec<u32>>();
    assert!(ts.contains("export type GenRec<T> = T;"), "{ts}");
}

/// A generic *mutual* cycle whose members name their parameter differently:
/// merging `GenP`'s `Y(T)` into `GenQ`'s body must rename `T` to `U` (and
/// vice versa), not emit a dangling parameter (`tsc` TS2304).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum GenP<T> {
    X(Box<GenQ<T>>),
    Y(T),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum GenQ<U> {
    X(Box<GenP<U>>),
    Z(String),
}

#[test]
fn generic_mutual_cycle_renames_parameters_instead_of_dangling() {
    let ts = export::<GenP<u32>>();
    assert!(ts.contains("export type GenP<T> = T | string;"), "{ts}");
    assert!(ts.contains("export type GenQ<U> = string | U;"), "{ts}");
    assert!(
        !ts.contains("= string | T") && !ts.contains("GenQ<U> = T"),
        "GenQ's body must not leak GenP's parameter name: {ts}"
    );
}

/// An untagged enum whose *only* branch is cyclic has no terminal: no finite
/// value of it can ever be serialized, so the precise type is `never`
/// (rather than a panic or an empty union).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum NoTerminal {
    A(Box<NoTerminal>),
}

#[test]
fn untagged_cycle_without_terminals_collapses_to_never() {
    let ts = export::<NoTerminal>();
    assert!(ts.contains("export type NoTerminal = never;"), "{ts}");
}

/// Variant-level `#[serde(untagged)]` (PR #512) produces the same
/// alias-transparent newtype shape inside an otherwise externally-tagged
/// enum, so the same collapse applies to it.
#[derive(Type, Serialize)]
#[specta(collect = false)]
enum MixedRec {
    V(i32),
    #[serde(untagged)]
    U(Box<MixedRec>),
}

#[test]
fn variant_level_untagged_self_cycle_collapses() {
    let ts = export::<MixedRec>();
    assert!(ts.contains("export type MixedRec = { V: number };"), "{ts}");
}

/// A cycle that hops through a *newtype struct* is still alias-transparent:
/// `NtWrap` exports as the bare alias `type NtWrap = ViaNewtype`, so the
/// naive rendering is a two-alias cycle `tsc` rejects. The enum collapses;
/// the passthrough struct keeps its bare alias body, which is legal once
/// the enum it points at no longer loops.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ViaNewtype {
    X(Box<NtWrap>),
    Y(u32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NtWrap(Box<ViaNewtype>);

#[test]
fn cycle_through_newtype_struct_collapses() {
    let ts = export::<ViaNewtype>();
    assert!(ts.contains("export type ViaNewtype = number;"), "{ts}");
    assert!(ts.contains("export type NtWrap = ViaNewtype;"), "{ts}");
}

/// Same through a `#[serde(transparent)]` struct.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ViaTransparent {
    X(Box<TransWrap>),
    Y(u32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransWrap {
    inner: Box<ViaTransparent>,
}

#[test]
fn cycle_through_serde_transparent_struct_collapses() {
    let ts = export::<ViaTransparent>();
    assert!(ts.contains("export type ViaTransparent = number;"), "{ts}");
    assert!(
        ts.contains("export type TransWrap = ViaTransparent;"),
        "{ts}"
    );
}

/// A recursive branch with an extra *skipped* field is NOT transparent:
/// serde serializes the remaining field as a one-element seq and the
/// exporter renders `[SkipRec]` - a tuple, which TypeScript resolves
/// lazily, so the recursion is legal and the branch must be kept.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum SkipRec {
    A(Box<SkipRec>, #[serde(skip)] String),
    B(u32),
}

#[test]
fn recursive_branch_with_skipped_extra_field_is_kept_as_a_tuple() {
    // serde ground truth: the skipped field leaves a one-element seq.
    let json = serde_json::to_string(&SkipRec::A(Box::new(SkipRec::B(1)), String::new())).unwrap();
    assert_eq!(json, "[1]");

    let ts = export::<SkipRec>();
    assert!(
        ts.contains("export type SkipRec = [SkipRec] | number;"),
        "{ts}"
    );
}

/// A self-reference through `Option` is still an eager alias position
/// (`OptRec | null` is a union member), so it must collapse - but dropping
/// the branch has to keep `null`, because `A(None)` really serializes as
/// `null`.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum OptRec {
    A(Option<Box<OptRec>>),
    B(u32),
}

#[test]
fn cycle_through_option_collapses_but_keeps_null() {
    // serde ground truth: the `None` case is a real `null` wire value.
    assert_eq!(serde_json::to_string(&OptRec::A(None)).unwrap(), "null");

    let ts = export::<OptRec>();
    assert!(ts.contains("export type OptRec = number | null;"), "{ts}");
}

/// Recursive branches that go through deferred structure (`Vec`) are legal
/// TypeScript and must survive the collapse of their transparent siblings.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ListRec {
    A(Box<ListRec>),
    B(Vec<ListRec>),
    C(u32),
}

#[test]
fn deferred_recursive_branches_survive_the_collapse() {
    let ts = export::<ListRec>();
    assert!(
        ts.contains("export type ListRec = ListRec[] | number;"),
        "{ts}"
    );
}
