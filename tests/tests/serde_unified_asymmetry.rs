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

// --- Untagged enums: variant names never hit the wire ---
//
// serde's untagged representation serializes only the variant payload, so
// attributes that rename variant *labels* (container `rename_all`, variant
// `rename`) are not direction-dependent there and must keep working under
// `Format`. Attributes that rename variant *field* names (`rename_all_fields`,
// variant-level `rename_all`) still affect the payload shape and must error,
// as must directional variant skips (union membership differs per direction).

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged, rename_all(serialize = "camelCase"))]
enum UntaggedOneSidedRenameAll {
    Foo(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedOneSidedVariantRename {
    #[serde(rename(serialize = "Foo"))]
    A(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum TaggedWithUntaggedVariantOneSidedRename {
    A(String),
    #[serde(untagged, rename(serialize = "Renamed"))]
    B(u32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged, rename_all_fields(serialize = "camelCase"))]
enum UntaggedOneSidedRenameAllFields {
    A { field_one: String },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedVariantOneSidedRenameAll {
    #[serde(rename_all(serialize = "camelCase"))]
    A { field_one: String },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedOneSidedVariantSkip {
    A(String),
    #[serde(skip_serializing)]
    B(u32),
}

// --- Enums without named-field variants: `rename_all_fields` / variant
// `rename_all` have nothing to rename ---
//
// These rules only apply to named fields (`rewrite_fields_for_phase` ignores
// them for unit/newtype/tuple variants), so a directional value produces
// identical serialize and deserialize shapes and must keep working under
// `Format`. The named-field controls are covered by
// `EnumRenameAllFieldsOneSided` and `UntaggedVariantOneSidedRenameAll` above.

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all_fields(serialize = "camelCase"))]
enum TupleOnlyOneSidedRenameAllFields {
    A(String),
    B(u32, bool),
    C,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all_fields(serialize = "camelCase"))]
enum MixedNamedOneSidedRenameAllFields {
    A(String),
    B { field_one: String },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum TupleVariantOneSidedRenameAll {
    #[serde(rename_all(serialize = "camelCase"))]
    A(String, u32),
}

// --- Structs without live named field keys: `rename_all` has nothing to
// rename ---
//
// serde accepts `rename_all` on unit/newtype/tuple structs and on named
// structs whose fields are all flattened (their keys come from the flattened
// type) or skipped in both directions; in all of those cases there is no
// local field key for the rule to change, so a directional value produces
// identical shapes and must keep working under `Format`. The live-field
// control is `ContainerRenameAllOneSided` above.

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase"))]
struct TupleStructOneSidedRenameAll(String, u32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenTarget {
    field_a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase"))]
struct AllFlattenedOneSidedRenameAll {
    #[serde(flatten)]
    inner: FlattenTarget,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase"))]
struct AllSkippedOneSidedRenameAll {
    #[serde(skip)]
    field_one: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all(serialize = "camelCase"))]
struct FlattenedPlusLiveOneSidedRenameAll {
    #[serde(flatten)]
    inner: FlattenTarget,
    field_one: String,
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
fn untagged_variant_label_renames_are_not_direction_dependent_under_format() {
    // Container `rename_all` on an untagged enum only renames variant labels,
    // which serde never emits for untagged enums; both directions are just
    // the payload shape.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<UntaggedOneSidedRenameAll>(),
            specta_serde::Format,
        )
        .expect("one-sided rename_all on an untagged enum should export under Format");
    assert!(
        rendered.contains("UntaggedOneSidedRenameAll = string"),
        "got: {rendered}"
    );

    // Same for a variant `rename` on an untagged enum.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<UntaggedOneSidedVariantRename>(),
            specta_serde::Format,
        )
        .expect("one-sided variant rename on an untagged enum should export under Format");
    assert!(
        rendered.contains("UntaggedOneSidedVariantRename = string"),
        "got: {rendered}"
    );

    // And for a variant-level `#[serde(untagged)]` variant inside a tagged
    // enum: its tag is never emitted either.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TaggedWithUntaggedVariantOneSidedRename>(),
            specta_serde::Format,
        )
        .expect(
            "one-sided rename on a variant-level untagged variant should export under Format",
        );
    assert!(
        rendered.contains("{ A: string } | number"),
        "got: {rendered}"
    );
}

#[test]
fn untagged_field_name_renames_still_require_phases_format() {
    // `rename_all_fields` renames struct-variant *field* names, which do
    // appear in the untagged payload.
    let err = Typescript::default()
        .export(
            &Types::default().register::<UntaggedOneSidedRenameAllFields>(),
            specta_serde::Format,
        )
        .expect_err("one-sided rename_all_fields on an untagged enum should still error");
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all_fields") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );

    // Variant-level `rename_all` renames the variant's field names too.
    let err = Typescript::default()
        .export(
            &Types::default().register::<UntaggedVariantOneSidedRenameAll>(),
            specta_serde::Format,
        )
        .expect_err("one-sided variant rename_all on an untagged enum should still error");
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn untagged_variant_skip_asymmetry_still_requires_phases_format() {
    // Even without variant labels, a directional variant skip changes which
    // payload shapes are members of the untagged union per direction.
    let err = Typescript::default()
        .export(
            &Types::default().register::<UntaggedOneSidedVariantSkip>(),
            specta_serde::Format,
        )
        .expect_err("one-sided variant skip on an untagged enum should still error");
    let msg = err.to_string();
    assert!(
        msg.contains("skip") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn rename_all_fields_without_named_field_variants_is_not_direction_dependent() {
    // No variant has named fields, so there is nothing for
    // `rename_all_fields` to rename in either direction.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TupleOnlyOneSidedRenameAllFields>(),
            specta_serde::Format,
        )
        .expect(
            "one-sided rename_all_fields on an enum without named-field variants should export under Format",
        );
    assert!(
        rendered.contains("{ A: string }") && rendered.contains("\"C\""),
        "got: {rendered}"
    );

    // But one named-field variant anywhere in the enum makes it
    // direction-dependent again.
    let err = Typescript::default()
        .export(
            &Types::default().register::<MixedNamedOneSidedRenameAllFields>(),
            specta_serde::Format,
        )
        .expect_err(
            "one-sided rename_all_fields should still error when any variant has named fields",
        );
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all_fields") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
}

#[test]
fn variant_rename_all_on_tuple_variant_is_not_direction_dependent() {
    // Variant-level `rename_all` renames the variant's named fields; a tuple
    // variant has none, so both directions are the same payload shape. The
    // named-field control is `untagged_field_name_renames_still_require_phases_format`.
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<TupleVariantOneSidedRenameAll>(),
            specta_serde::Format,
        )
        .expect("one-sided variant rename_all on a tuple variant should export under Format");
    assert!(
        rendered.contains("{ A: [string, number] }"),
        "got: {rendered}"
    );
}

#[test]
fn struct_rename_all_without_live_named_field_keys_is_not_direction_dependent() {
    for (name, result) in [
        (
            "TupleStructOneSidedRenameAll",
            Typescript::default().export(
                &Types::default().register::<TupleStructOneSidedRenameAll>(),
                specta_serde::Format,
            ),
        ),
        (
            "AllFlattenedOneSidedRenameAll",
            Typescript::default().export(
                &Types::default().register::<AllFlattenedOneSidedRenameAll>(),
                specta_serde::Format,
            ),
        ),
        (
            "AllSkippedOneSidedRenameAll",
            Typescript::default().export(
                &Types::default().register::<AllSkippedOneSidedRenameAll>(),
                specta_serde::Format,
            ),
        ),
    ] {
        result.unwrap_or_else(|err| {
            panic!(
                "{name}: one-sided rename_all without live named field keys should export under Format: {err}"
            )
        });
    }
}

#[test]
fn struct_rename_all_with_a_live_named_field_still_requires_phases_format() {
    // One live named field key alongside flattened fields keeps the rule
    // direction-dependent.
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenedPlusLiveOneSidedRenameAll>(),
            specta_serde::Format,
        )
        .expect_err("one-sided rename_all should still error when a live named field exists");
    let msg = err.to_string();
    assert!(
        msg.contains("rename_all") && msg.contains("PhasesFormat"),
        "unexpected error: {msg}"
    );
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
