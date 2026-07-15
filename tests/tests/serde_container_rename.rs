//! Regression tests for container-level `#[serde(rename = "...")]` on enums.
//!
//! `#[serde(rename = "...")]` on a struct renames the exported type, but the
//! same attribute on an enum was silently ignored (the underlying rewrite only
//! ever inspected `DataType::Struct`). These tests pin down that enums and
//! structs behave identically for container renames: plain `rename`,
//! two-sided `rename(serialize = ..., deserialize = ...)`, one-sided/differing
//! renames (which require `PhasesFormat`), symmetric renames on types that
//! split for unrelated reasons, references to renamed enums, generics, and
//! name collisions.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedStructX")]
struct StructRename {
    a: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedEnumX")]
enum EnumRename {
    A,
}

/// Control test pinning the (already correct) struct behaviour, so the enum
/// tests below can be read as demonstrating parity with it.
#[test]
fn struct_container_rename_is_honored() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<StructRename>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        ts.contains("export type RenamedStructX = "),
        "expected struct container rename to be honored, got:\n{ts}"
    );
    assert!(
        !ts.contains("StructRename"),
        "the original (un-renamed) struct name should not appear, got:\n{ts}"
    );
}

/// This is the bug: the enum equivalent of `struct_container_rename_is_honored`
/// was silently ignoring the container rename.
#[test]
fn enum_container_rename_is_honored() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<EnumRename>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        ts.contains("export type RenamedEnumX = "),
        "expected enum container rename to be honored, got:\n{ts}"
    );
    assert!(
        !ts.contains("EnumRename"),
        "the original (un-renamed) enum name should not appear, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename(serialize = "TwoSidedEnumRename", deserialize = "TwoSidedEnumRename"))]
enum EnumTwoSidedRename {
    A,
    B,
}

/// `rename(serialize = X, deserialize = X)` with equal values on both sides
/// must behave the same as a plain `rename = X` under the unified `Format`.
#[test]
fn enum_two_sided_equal_rename_is_honored_under_format() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<EnumTwoSidedRename>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        ts.contains("export type TwoSidedEnumRename = "),
        "expected two-sided (equal) enum container rename to be honored, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
#[serde(rename(
    serialize = "StructPhaseSpecificRenameSerializeControl",
    deserialize = "StructPhaseSpecificRenameDeserializeControl"
))]
struct StructPhaseSpecificRenameControl {
    a: String,
}

/// Control test: a struct with a one-sided/differing container rename
/// requires `PhasesFormat` and splits into `*_Serialize` / `*_Deserialize`
/// named types using the *renamed* base name.
#[test]
fn struct_phase_specific_rename_splits_under_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<StructPhaseSpecificRenameControl>(),
            specta_serde::Format,
        )
        .expect_err("differing phase renames should require PhasesFormat");
    assert!(err.to_string().contains("StructPhaseSpecificRename"));

    let ts = Typescript::default()
        .export(
            &Types::default().register::<StructPhaseSpecificRenameControl>(),
            specta_serde::PhasesFormat,
        )
        .expect("export should succeed");

    // Differing phase renames are used verbatim (no `_Serialize`/`_Deserialize`
    // suffix): the user already authored a distinct name per phase.
    assert!(
        ts.contains("export type StructPhaseSpecificRenameSerializeControl = "),
        "expected serialize-phase renamed name, got:\n{ts}"
    );
    assert!(
        ts.contains("export type StructPhaseSpecificRenameDeserializeControl = "),
        "expected deserialize-phase renamed name, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename(
    serialize = "EnumPhaseSpecificRenameSerialize",
    deserialize = "EnumPhaseSpecificRenameDeserialize"
))]
enum EnumPhaseSpecificRename {
    A,
    B(String),
}

/// The enum equivalent of `struct_phase_specific_rename_splits_under_phases_format`:
/// a one-sided/differing container rename on an enum should require
/// `PhasesFormat` and split using the renamed base name, exactly like a struct.
#[test]
fn enum_phase_specific_rename_splits_under_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<EnumPhaseSpecificRename>(),
            specta_serde::Format,
        )
        .expect_err("differing phase renames should require PhasesFormat");
    assert!(err.to_string().contains("EnumPhaseSpecificRename"));

    let ts = Typescript::default()
        .export(
            &Types::default().register::<EnumPhaseSpecificRename>(),
            specta_serde::PhasesFormat,
        )
        .expect("export should succeed");

    // Exactly like structs, differing phase renames are used verbatim (no
    // `_Serialize`/`_Deserialize` suffix).
    assert!(
        ts.contains("export type EnumPhaseSpecificRenameSerialize = "),
        "expected serialize-phase renamed name, got:\n{ts}"
    );
    assert!(
        ts.contains("export type EnumPhaseSpecificRenameDeserialize = "),
        "expected deserialize-phase renamed name, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedSymmetricSplitStruct")]
struct SymmetricRenameSplitStruct {
    a: String,
    #[serde(skip_serializing)]
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged, rename = "RenamedSymmetricSplitUntagged")]
enum SymmetricRenameSplitUntaggedEnum {
    A(String),
    #[serde(skip_serializing)]
    B(i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedSymmetricSplitTagged")]
enum SymmetricRenameSplitTaggedEnum {
    A,
    #[serde(skip_serializing)]
    B(String),
}

/// A symmetric container rename on a type that splits for an unrelated reason
/// (here a directional skip) must keep the phase suffix on the *renamed* base
/// name; the rename alone can't distinguish the two phases.
#[test]
fn symmetric_rename_on_split_types_keeps_phase_suffix() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<SymmetricRenameSplitStruct>(),
            specta_serde::PhasesFormat,
        )
        .expect("struct export should succeed");
    assert!(
        ts.contains("export type RenamedSymmetricSplitStruct_Serialize = ")
            && ts.contains("export type RenamedSymmetricSplitStruct_Deserialize = "),
        "expected suffixed renamed struct names, got:\n{ts}"
    );

    let ts = Typescript::default()
        .export(
            &Types::default().register::<SymmetricRenameSplitUntaggedEnum>(),
            specta_serde::PhasesFormat,
        )
        .expect("untagged enum export should succeed");
    assert!(
        ts.contains("export type RenamedSymmetricSplitUntagged_Serialize = ")
            && ts.contains("export type RenamedSymmetricSplitUntagged_Deserialize = "),
        "expected suffixed renamed untagged enum names, got:\n{ts}"
    );

    let ts = Typescript::default()
        .export(
            &Types::default().register::<SymmetricRenameSplitTaggedEnum>(),
            specta_serde::PhasesFormat,
        )
        .expect("tagged enum export should succeed");
    assert!(
        ts.contains("export type RenamedSymmetricSplitTagged_Serialize = ")
            && ts.contains("export type RenamedSymmetricSplitTagged_Deserialize = "),
        "expected suffixed renamed tagged enum names, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "ReferencedEnumNew")]
enum ReferencedEnum {
    A,
    B(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ReferencingStruct {
    field: ReferencedEnum,
    opt: Option<ReferencedEnum>,
}

/// References from other types must resolve to the *renamed* enum name.
#[test]
fn references_use_renamed_enum_name() {
    let ts = Typescript::default()
        .export(
            &Types::default()
                .register::<ReferencedEnum>()
                .register::<ReferencingStruct>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        ts.contains("field: ReferencedEnumNew,") && ts.contains("opt: ReferencedEnumNew | null,"),
        "expected references to use the renamed enum name, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedGenericEnum")]
enum GenericEnumRename<T> {
    Value(T),
    Empty,
}

/// Container rename must also apply to generic enums, matching how generic
/// structs already behave.
#[test]
fn generic_enum_container_rename_is_honored() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<GenericEnumRename<i32>>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        ts.contains("RenamedGenericEnum"),
        "expected generic enum container rename to be honored, got:\n{ts}"
    );
    assert!(
        !ts.contains("GenericEnumRename"),
        "the original (un-renamed) generic enum name should not appear, got:\n{ts}"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "CollidingStructName")]
struct CollidingStructA {
    a: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "CollidingStructName")]
struct CollidingStructB {
    b: i32,
}

/// Control test: two distinct structs renamed to the same name are caught as
/// a duplicate export name.
#[test]
fn struct_container_rename_collision_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default()
                .register::<CollidingStructA>()
                .register::<CollidingStructB>(),
            specta_serde::Format,
        )
        .expect_err("two structs renamed to the same name should collide");

    assert!(err.to_string().contains("CollidingStructName"));
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "CollidingEnumName")]
enum CollidingEnumA {
    A,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "CollidingEnumName")]
enum CollidingEnumB {
    B,
}

/// The enum equivalent of `struct_container_rename_collision_is_rejected`:
/// two distinct enums renamed to the same name must be rejected identically.
#[test]
fn enum_container_rename_collision_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default()
                .register::<CollidingEnumA>()
                .register::<CollidingEnumB>(),
            specta_serde::Format,
        )
        .expect_err("two enums renamed to the same name should collide");

    assert!(err.to_string().contains("CollidingEnumName"));
}
