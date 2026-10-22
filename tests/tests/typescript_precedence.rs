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

branded!(#[derive(Serialize)] pub struct Brand(Option<String>) as "Brand");

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

/// Locate a TypeScript compiler, if one is installed.
///
/// `SPECTA_TSC` overrides the lookup with a whitespace-separated command (for
/// example `bunx tsc`). CI does not install Node, so the assignability checks
/// are skipped there and the exact-string assertions above remain the
/// authoritative coverage.
fn typescript_compiler() -> Option<Vec<String>> {
    if let Ok(command) = std::env::var("SPECTA_TSC") {
        let parts = command
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        return (!parts.is_empty()).then_some(parts);
    }

    Command::new("tsc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| vec!["tsc".to_string()])
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
