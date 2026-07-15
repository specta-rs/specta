// Regression tests for the `specta_serde::Format` (unified mode) contract:
//
//   "If serde metadata produces different serialize and deserialize shapes,
//   this formatter returns an error instead of guessing."
//
// One-sided directional renames and one-sided skips are asymmetric serde
// constructs (the field/variant/container behaves differently when
// serializing vs deserializing) but `Format` used to silently guess by
// treating the missing side as "no override" and picking whichever side was
// set. That produced a unified type that was silently wrong for one
// direction. These constructs must now be rejected under `Format` and
// require `specta_serde::PhasesFormat` instead, which already understands
// how to split them into separate `*_Serialize` / `*_Deserialize` shapes.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

// --- One-sided field rename ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldOneSidedRenameSerialize {
    #[serde(rename(serialize = "serName"))]
    field_a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FieldOneSidedRenameDeserialize {
    #[serde(rename(deserialize = "derName"))]
    field_a: String,
}

// --- One-sided container rename ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename(serialize = "ContainerOneSidedRenameSerializeTarget"))]
struct ContainerOneSidedRenameSerialize {
    value: String,
}

// --- One-sided container `rename_all` ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase"))]
struct ContainerRenameAllOneSided {
    field_one: String,
}

// --- One-sided enum variant rename ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantOneSidedRename {
    #[serde(rename(serialize = "VariantSer"))]
    A(String),
}

// --- One-sided `rename_all_fields` on an enum ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all_fields(serialize = "camelCase"))]
enum EnumRenameAllFieldsOneSided {
    A { field_one: String },
}

// --- One-sided field skip ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipSer {
    a: String,
    #[serde(skip_serializing)]
    b: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SkipDe {
    a: String,
    #[serde(skip_deserializing)]
    b: String,
}

// --- One-sided variant skip ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VarSkip {
    A(String),
    #[serde(skip_deserializing)]
    B(String),
}

// --- Controls: these must keep working unchanged under `Format` ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PlainSkip {
    a: String,
    #[serde(skip)]
    b: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SymmetricRename {
    #[serde(rename(serialize = "same_name", deserialize = "same_name"))]
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
struct SymmetricRenameAll {
    field_one: String,
}

#[test]
fn one_sided_field_rename_requires_phases_format() {
    for (name, err) in [
        (
            "FieldOneSidedRenameSerialize",
            Typescript::default()
                .export(
                    &Types::default().register::<FieldOneSidedRenameSerialize>(),
                    specta_serde::Format,
                )
                .expect_err("one-sided serialize-only field rename should require PhasesFormat"),
        ),
        (
            "FieldOneSidedRenameDeserialize",
            Typescript::default()
                .export(
                    &Types::default().register::<FieldOneSidedRenameDeserialize>(),
                    specta_serde::Format,
                )
                .expect_err("one-sided deserialize-only field rename should require PhasesFormat"),
        ),
    ] {
        let msg = err.to_string();
        assert!(
            msg.contains("field rename") && msg.contains("PhasesFormat"),
            "{name}: unexpected error: {msg}"
        );
    }
}

#[test]
fn one_sided_field_rename_splits_correctly_under_phases_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FieldOneSidedRenameSerialize>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided field renames");

    assert!(rendered.contains("FieldOneSidedRenameSerialize_Serialize"));
    assert!(rendered.contains("FieldOneSidedRenameSerialize_Deserialize"));

    let serialize_ty = extract_type(&rendered, "FieldOneSidedRenameSerialize_Serialize");
    let deserialize_ty = extract_type(&rendered, "FieldOneSidedRenameSerialize_Deserialize");

    // Serialize direction uses the renamed field name.
    assert!(serialize_ty.contains("serName"), "got: {serialize_ty}");
    assert!(!serialize_ty.contains("field_a"), "got: {serialize_ty}");
    // Deserialize direction keeps the original field name (no rename applies).
    assert!(deserialize_ty.contains("field_a"), "got: {deserialize_ty}");
    assert!(!deserialize_ty.contains("serName"), "got: {deserialize_ty}");
}

#[test]
fn one_sided_container_rename_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<ContainerOneSidedRenameSerialize>(),
            specta_serde::Format,
        )
        .expect_err("one-sided container rename should require PhasesFormat");
    let msg = err.to_string();
    assert!(
        msg.contains("container rename") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn one_sided_container_rename_splits_correctly_under_phases_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ContainerOneSidedRenameSerialize>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided container renames");

    // The serialize-facing type is named after the container rename target
    // directly (the rename fully replaces the name for that phase).
    assert!(rendered.contains("export type ContainerOneSidedRenameSerializeTarget = {"));
    // The deserialize-facing type keeps the original container name (no
    // rename applies to that direction) with the usual `_Deserialize` suffix.
    assert!(rendered.contains("export type ContainerOneSidedRenameSerialize_Deserialize = {"));
}

#[test]
fn one_sided_rename_all_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<ContainerRenameAllOneSided>(),
            specta_serde::Format,
        )
        .expect_err("one-sided rename_all should require PhasesFormat");
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn one_sided_rename_all_splits_correctly_under_phases_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ContainerRenameAllOneSided>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided rename_all");

    let serialize_ty = extract_type(&rendered, "ContainerRenameAllOneSided_Serialize");
    let deserialize_ty = extract_type(&rendered, "ContainerRenameAllOneSided_Deserialize");

    assert!(serialize_ty.contains("fieldOne"), "got: {serialize_ty}");
    assert!(
        deserialize_ty.contains("field_one"),
        "got: {deserialize_ty}"
    );
}

#[test]
fn one_sided_variant_rename_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantOneSidedRename>(),
            specta_serde::Format,
        )
        .expect_err("one-sided variant rename should require PhasesFormat");
    let msg = err.to_string();
    assert!(
        msg.contains("variant rename") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn one_sided_variant_rename_splits_correctly_under_phases_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VariantOneSidedRename>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided variant renames");

    let serialize_ty = extract_type(&rendered, "VariantOneSidedRename_Serialize");
    let deserialize_ty = extract_type(&rendered, "VariantOneSidedRename_Deserialize");

    assert!(serialize_ty.contains("VariantSer"), "got: {serialize_ty}");
    assert!(
        deserialize_ty.contains("A: string"),
        "got: {deserialize_ty}"
    );
}

#[test]
fn one_sided_rename_all_fields_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<EnumRenameAllFieldsOneSided>(),
            specta_serde::Format,
        )
        .expect_err("one-sided rename_all_fields should require PhasesFormat");
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all_fields") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn one_sided_rename_all_fields_splits_correctly_under_phases_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<EnumRenameAllFieldsOneSided>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided rename_all_fields");

    let serialize_ty = extract_type(&rendered, "EnumRenameAllFieldsOneSided_Serialize");
    let deserialize_ty = extract_type(&rendered, "EnumRenameAllFieldsOneSided_Deserialize");

    assert!(serialize_ty.contains("fieldOne"), "got: {serialize_ty}");
    assert!(
        deserialize_ty.contains("field_one"),
        "got: {deserialize_ty}"
    );
}

#[test]
fn one_sided_field_skip_requires_phases_format() {
    for (name, err) in [
        (
            "SkipSer",
            Typescript::default()
                .export(
                    &Types::default().register::<SkipSer>(),
                    specta_serde::Format,
                )
                .expect_err(
                    "skip_serializing without skip_deserializing should require PhasesFormat",
                ),
        ),
        (
            "SkipDe",
            Typescript::default()
                .export(&Types::default().register::<SkipDe>(), specta_serde::Format)
                .expect_err(
                    "skip_deserializing without skip_serializing should require PhasesFormat",
                ),
        ),
    ] {
        let msg = err.to_string();
        assert!(
            msg.contains("skip") && msg.contains("PhasesFormat"),
            "{name}: unexpected error: {msg}"
        );
    }
}

#[test]
fn skip_serializing_only_field_is_dropped_from_serialize_but_required_in_deserialize() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<SkipSer>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided field skips");

    let serialize_ty = extract_type(&rendered, "SkipSer_Serialize");
    let deserialize_ty = extract_type(&rendered, "SkipSer_Deserialize");

    // The always-skipped-on-serialize field must not appear in the wire shape
    // Rust produces when serializing ...
    assert!(!serialize_ty.contains('b'), "got: {serialize_ty}");
    // ... but deserialize still requires it verbatim (serde never skips
    // reading it), so it must appear, and not be marked optional.
    assert!(
        deserialize_ty.contains("b: string"),
        "got: {deserialize_ty}"
    );
}

#[test]
fn skip_deserializing_only_field_is_dropped_from_deserialize_but_present_in_serialize() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<SkipDe>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided field skips");

    let serialize_ty = extract_type(&rendered, "SkipDe_Serialize");
    let deserialize_ty = extract_type(&rendered, "SkipDe_Deserialize");

    // serde always emits the field when serializing (skip_deserializing has
    // no effect on serialize) so it must appear, and not be marked optional.
    assert!(serialize_ty.contains("b: string"), "got: {serialize_ty}");
    // serde never reads the field back when deserializing, so it must be
    // absent from the deserialize-facing shape.
    assert!(!deserialize_ty.contains('b'), "got: {deserialize_ty}");
}

#[test]
fn one_sided_variant_skip_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VarSkip>(),
            specta_serde::Format,
        )
        .expect_err(
            "mismatched variant skip_serializing/skip_deserializing should require PhasesFormat",
        );
    let msg = err.to_string();
    assert!(
        msg.contains("skip") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn skip_deserializing_only_variant_is_present_in_serialize_and_dropped_from_deserialize() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VarSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("PhasesFormat should split one-sided variant skips");

    let serialize_ty = extract_type(&rendered, "VarSkip_Serialize");
    let deserialize_ty = extract_type(&rendered, "VarSkip_Deserialize");

    // serde can still emit `{"B": "..."}` when serializing.
    assert!(serialize_ty.contains('B'), "got: {serialize_ty}");
    // serde will never construct the `B` variant from wire input, so
    // deserialize must not accept it.
    assert!(!deserialize_ty.contains('B'), "got: {deserialize_ty}");
}

#[test]
fn plain_skip_still_exports_fine_under_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<PlainSkip>(),
            specta_serde::Format,
        )
        .expect("plain #[serde(skip)] (symmetric) should still export under Format");

    assert!(rendered.contains('a'));
    assert!(!rendered.contains(": string,\n\tb"), "got: {rendered}");
}

#[test]
fn symmetric_rename_still_exports_fine_under_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<SymmetricRename>(),
            specta_serde::Format,
        )
        .expect(
            "symmetric rename(serialize = .., deserialize = ..) should still export under Format",
        );

    assert!(rendered.contains("same_name"));
}

#[test]
fn symmetric_rename_all_still_exports_fine_under_format() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<SymmetricRenameAll>(),
            specta_serde::Format,
        )
        .expect("symmetric rename_all(serialize = .., deserialize = ..) should still export under Format");

    assert!(rendered.contains("fieldOne"));
}

/// Extracts the body text of a single named `export type <name> = { ... };`
/// (or similar) declaration from a rendered TypeScript module, for making
/// assertions about one type in isolation without over-matching other types
/// in the same module.
fn extract_type<'a>(rendered: &'a str, name: &str) -> &'a str {
    let marker = format!("{name} =");
    let start = rendered
        .find(&marker)
        .unwrap_or_else(|| panic!("could not find `{marker}` in:\n{rendered}"));
    let rest = &rendered[start..];
    let end = rest.find(";\n").map(|idx| idx + 1).unwrap_or(rest.len());
    &rest[..end]
}
