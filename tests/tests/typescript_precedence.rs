//! Regression tests for TypeScript expression precedence inside intersections
//! (https://github.com/specta-rs/specta/issues/593).
//!
//! `&` binds tighter than `|` in TypeScript, so any intersection operand that
//! renders a top-level union must be parenthesised, or the union escapes the
//! intersection and a required sibling field becomes optional. The same applies
//! to the payload of a `branded!` type, which is intersected with the brand
//! marker. These tests pin the emitted text and, where a TypeScript compiler is
//! available, assert assignability directly.

use std::{path::Path, process::Command};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, Enum, Generic, Primitive, Reference, Variant},
};
use specta_typescript::{Branded, Typescript, branded, primitives};
use tempfile::TempDir;

fn export<T: Type>() -> String {
    Typescript::default()
        .export(&Types::default().register::<T>(), specta_serde::Format)
        .expect("export should succeed")
}

fn export_phases<T: Type>() -> String {
    Typescript::default()
        .export(
            &Types::default().register::<T>(),
            specta_serde::PhasesFormat,
        )
        .expect("export should succeed")
}

/// An untagged enum renders as an XOR union, and `#[specta(inline)]` expands
/// the flattened reference in place, so the union becomes an operand of the
/// enclosing intersection.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum FlattenedUnion {
    A { a: String },
    B { b: u32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenedOuter {
    x: bool,
    #[serde(flatten)]
    #[specta(inline)]
    inner: FlattenedUnion,
}

#[test]
fn flattened_inline_union_is_parenthesised() {
    let rendered = export::<FlattenedOuter>();
    let expected = "export type FlattenedOuter = {\n\tx: boolean,\n} & (({ a: string }) & { b?: never } | ({ b: number }) & { a?: never });";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// The same precedence rule must hold through the low-level `inline` renderer
/// (used by the inline/file layouts), not just the default export.
#[test]
fn inline_api_parenthesises_flattened_union() {
    use specta::Format;

    let mut types = Types::default();
    let reference = match FlattenedOuter::definition(&mut types) {
        DataType::Reference(Reference::Named(r)) => r,
        other => panic!("expected a named reference, got {other:?}"),
    };

    // The flattened intersection is produced by the Serde format, so apply it
    // before rendering, as the exporter does.
    let types = specta_serde::Format.map_types(&types).unwrap();
    let body = types.get(&reference).unwrap().ty.clone().unwrap();

    let rendered = primitives::inline(&Typescript::default(), &types, &body).unwrap();
    let expected = "} & (({ a: string }) & { b?: never } | ({ b: number }) & { a?: never })";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// A genuine newtype renders as its bare inner type. If that inner type is a
/// union, it must still be parenthesised inside the flattened intersection.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum NewtypeUnion {
    A { a: String },
    B { b: u32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NewtypeWrapper(#[specta(inline)] NewtypeUnion);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NewtypeOuter {
    x: bool,
    #[serde(flatten)]
    #[specta(inline)]
    inner: NewtypeWrapper,
}

#[test]
fn flattened_inline_newtype_union_is_parenthesised() {
    let rendered = export::<NewtypeOuter>();
    let expected = "export type NewtypeOuter = {\n\tx: boolean,\n} & (({ a: string }) & { b?: never } | ({ b: number }) & { a?: never });";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

branded!(#[derive(Serialize, Deserialize, Default)] pub struct Brand(Option<String>) as "Brand");

#[derive(Type)]
#[specta(collect = false)]
struct BrandHolder {
    value: Brand,
}

#[test]
fn branded_union_payload_is_parenthesised() {
    let rendered = export::<BrandHolder>();
    let expected = "export type BrandHolder = {\n\tvalue: (string | null) & { readonly __brand: \"Brand\" },\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// A single aliased field renders, for deserialization, as a required sibling
/// intersected with a union of the accepted spellings. The sibling must stay
/// required in every arm.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Alias {
    other: bool,
    #[serde(alias = "a2")]
    a: u32,
}

#[test]
fn field_alias_union_keeps_sibling_required() {
    let rendered = export_phases::<Alias>();
    let expected = "export type Alias_Deserialize = {\n\tother: boolean,\n} & ({\n\ta: number,\n\ta2?: never,\n} | {\n\ta2: number,\n\ta?: never,\n});";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[derive(Type, Serialize, Deserialize, PartialEq, Debug)]
#[specta(collect = false)]
enum EmptyNamedVariant {
    #[serde(rename = "")]
    A,
}

#[test]
fn empty_renamed_unit_variant_round_trips_and_exports() {
    assert_eq!(
        serde_json::to_string(&EmptyNamedVariant::A).unwrap(),
        "\"\""
    );
    assert_eq!(
        serde_json::from_str::<EmptyNamedVariant>("\"\"").unwrap(),
        EmptyNamedVariant::A
    );

    let rendered = export::<EmptyNamedVariant>();
    assert!(
        rendered.contains("export type EmptyNamedVariant = \"\";"),
        "{rendered}"
    );
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Marker {
    x: bool,
}

/// An optional trailing tuple element whose type renders a union must be
/// parenthesised before the `?`, or the output is a TypeScript syntax error
/// (`[string, number | null?]`, TS17019).
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct OptionalFloatTuple(String, #[serde(default)] f32);

#[test]
fn optional_float_tuple_element_is_parenthesised() {
    let rendered = export::<OptionalFloatTuple>();
    let expected = "export type OptionalFloatTuple = [string, (number | null)?];";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// A branded element renders as `<payload> & { readonly __brand: ... }`, which
/// is an intersection and must be parenthesised before the trailing `?`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionalBrandTuple(String, #[serde(default)] Brand);

#[test]
fn optional_branded_tuple_element_is_parenthesised() {
    let rendered = export::<OptionalBrandTuple>();
    let expected = "export type OptionalBrandTuple = [string, ((string | null) & { readonly __brand: \"Brand\" })?];";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// Every operand that renders something other than a primary expression must be
/// parenthesised. `Marker` and `T` render bare names and stay unwrapped.
#[test]
fn intersection_operands_are_parenthesised_by_rendered_shape() {
    let mut types = Types::default();
    let marker = Marker::definition(&mut types);
    let nullable = <Option<String>>::definition(&mut types);
    let float = <f32>::definition(&mut types);

    let mut two_variant = Enum::default();
    two_variant.variants.push(("A".into(), Variant::unit()));
    two_variant.variants.push(("B".into(), Variant::unit()));

    let intersection = DataType::Intersection(vec![
        marker,
        nullable,
        float,
        DataType::Enum(two_variant),
        DataType::Reference(specta_typescript::define("Opaque")),
        DataType::Reference(Reference::opaque(Branded::new(
            "B",
            DataType::Primitive(Primitive::str),
        ))),
        DataType::Generic(Generic::new("T".into())),
    ]);

    let rendered = primitives::reference(&Typescript::default(), &types, &intersection).unwrap();
    assert_eq!(
        rendered,
        "Marker & (string | null) & (number | null) & (\"A\" | \"B\") & (Opaque) & (string & { readonly __brand: \"B\" }) & T"
    );
}

/// Wire evidence for the `InternallyTaggedL` fixture: an internally tagged enum
/// whose inlined newtype variant wraps another internally tagged enum using the
/// *same* tag key. Serde emits the inner enum's own tag into the outer map,
/// producing duplicate `type` keys; `JSON.parse` keeps the last one, so the
/// wire value collapses to `{ type: "B" }`. Serde itself cannot deserialize it
/// back (`duplicate field`), so the fixture has no round-trippable contract.
///
/// The export requires the outer tag in every arm, which is the correct
/// intersection semantics even though it cannot match the collapsed wire value.
/// Pin the emitted type so this is a deliberate decision rather than a silently
/// blessed snapshot.
#[test]
fn same_tag_key_nested_internal_enum_has_no_wire_contract() {
    use crate::types::{InternallyTaggedL, InternallyTaggedLInner};

    let value = InternallyTaggedL::A(InternallyTaggedLInner::B);
    let serialized = serde_json::to_string(&value).unwrap();
    assert_eq!(serialized, r#"{"type":"A","type":"B"}"#);
    assert!(
        serde_json::from_str::<InternallyTaggedL>(&serialized).is_err(),
        "Serde rejects the duplicate tag key it just produced"
    );

    let rendered = export::<InternallyTaggedL>();
    let expected = "export type InternallyTaggedL = {\n\ttype: \"A\",\n} & ({ type: \"A\" } | { type: \"B\" });";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

/// Locate a TypeScript compiler, if one is installed.
///
/// `SPECTA_TSC` overrides the lookup. Its value is split on whitespace, so it
/// cannot express arguments containing spaces; point it at a wrapper script if
/// that is needed. Setting it to an empty value is an error rather than a
/// silent skip.
///
/// Without an override the candidates are tried in order: `tsc`, `bunx tsc`,
/// then `npx --yes --package typescript tsc`. When none is available the
/// assignability check is skipped and the exact-string assertions above remain
/// the authoritative coverage.
fn typescript_compiler() -> Option<Vec<String>> {
    if let Ok(command) = std::env::var("SPECTA_TSC") {
        let parts = command
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert!(
            !parts.is_empty(),
            "SPECTA_TSC is set but does not resolve to a command"
        );
        return Some(parts);
    }

    [
        vec!["tsc".to_string()],
        vec!["bunx".to_string(), "tsc".to_string()],
        vec![
            "npx".to_string(),
            "--yes".to_string(),
            "--package".to_string(),
            "typescript".to_string(),
            "tsc".to_string(),
        ],
    ]
    .into_iter()
    .find(|candidate| {
        Command::new(&candidate[0])
            .args(&candidate[1..])
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

/// Negative assignability: the shapes `tsc` used to accept must now be
/// rejected, and the shapes Serde accepts must still typecheck.
#[test]
fn generated_typescript_passes_strict_typecheck() {
    let Some(mut compiler) = typescript_compiler() else {
        eprintln!(
            "skipping TypeScript assignability check: no `tsc` on PATH \
             (set SPECTA_TSC to override)"
        );
        return;
    };

    let types = Types::default()
        .register::<FlattenedOuter>()
        .register::<NewtypeOuter>()
        .register::<OptionalFloatTuple>()
        .register::<OptionalBrandTuple>()
        .register::<BrandHolder>()
        .register::<Alias>();
    let generated = Typescript::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("export should succeed");

    let assertions = r#"
// `x` must survive into every arm of the flattened union.
const flattenedA: FlattenedOuter = { x: true, a: "value" };
const flattenedB: FlattenedOuter = { x: true, b: 1 };
// @ts-expect-error `x` is required; before the fix the second arm escaped the intersection.
const missingX: FlattenedOuter = { b: 1 };

// A newtype renders as its bare inner union, which must still be parenthesised.
const newtypeA: NewtypeOuter = { x: true, a: "value" };
const newtypeB: NewtypeOuter = { x: true, b: 1 };
// @ts-expect-error `x` is required in every arm of the newtype's union.
const newtypeMissingX: NewtypeOuter = { b: 1 };

// A trailing optional float tuple element must be parenthesised to parse.
const tupleShort: OptionalFloatTuple = ["value"];
const tupleNull: OptionalFloatTuple = ["value", null];
const tupleNumber: OptionalFloatTuple = ["value", 1.5];
// @ts-expect-error the second element must be a number or null.
const tupleBad: OptionalFloatTuple = ["value", "bad"];

// A trailing optional branded tuple element is an intersection and must parse.
const brandTupleShort: OptionalBrandTuple = ["value"];
const brandTupleOk: OptionalBrandTuple = ["value", "x" as string & { readonly __brand: "Brand" }];
// @ts-expect-error the second element must be the branded type, not a bare string.
const brandTupleBad: OptionalBrandTuple = ["value", "x"];

// A branded value is its payload intersected with the brand marker.
const branded: BrandHolder["value"] = "value" as string & { readonly __brand: "Brand" };
// @ts-expect-error a bare string must not be assignable to a brand.
const unbranded: BrandHolder["value"] = "value";
// @ts-expect-error `null` must not be assignable to a brand.
const nullBrand: BrandHolder["value"] = null;

// `other` is a required sibling of the deserialize alias union.
const aliasA: Alias_Deserialize = { other: true, a: 1 };
const aliasB: Alias_Deserialize = { other: true, a2: 1 };
// @ts-expect-error `other` is required in every arm.
const missingOther: Alias_Deserialize = { a: 1 };

void flattenedA;
void flattenedB;
void missingX;
void newtypeA;
void newtypeB;
void newtypeMissingX;
void tupleShort;
void tupleNull;
void tupleNumber;
void tupleBad;
void brandTupleShort;
void brandTupleOk;
void brandTupleBad;
void branded;
void unbranded;
void nullBrand;
void aliasA;
void aliasB;
void missingOther;
"#;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();
    let file = temp.path().join("precedence.ts");
    std::fs::write(&file, format!("{generated}\n{assertions}")).unwrap();

    let program = compiler.remove(0);
    eprintln!("typechecking generated TypeScript with {program}");
    let output = Command::new(&program)
        .args(&compiler)
        .args([
            "--strict",
            "--noEmit",
            "--skipLibCheck",
            "--target",
            "es2022",
            "--module",
            "esnext",
            "--moduleResolution",
            "bundler",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap_or_else(|error| panic!("failed to run {program:?}: {error}"));

    assert!(
        output.status.success(),
        "generated TypeScript failed to typecheck:\n{}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        std::fs::read_to_string(&file).unwrap(),
    );
}
