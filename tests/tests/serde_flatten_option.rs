// Regression test: `#[serde(flatten)]` on an `Option<T>` field.
//
// Serde contributes nothing when the flattened `Option<T>` is `None`, and
// merges `T`'s fields when it's `Some`. Previously `specta-serde` pushed the
// field's `Nullable(T)` type verbatim into the intersection it built for the
// flattened struct/variant, producing `Base & T | null`. Because TypeScript's
// `&` binds tighter than `|`, that parses as `(Base & T) | null` which is
// wrong in both directions: the wire value is never bare `null`, and the
// legitimate `None` output (base fields only) doesn't satisfy `Base & T`.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner1 {
    x: i32,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner2 {
    y: bool,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatOpt {
    a: i32,
    #[serde(flatten)]
    inner: Option<Inner1>,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalFlattenOpt {
    A {
        #[serde(flatten)]
        inner: Option<Inner1>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatTwoOpt {
    a: i32,
    #[serde(flatten)]
    inner1: Option<Inner1>,
    #[serde(flatten)]
    inner2: Option<Inner2>,
}

#[derive(Debug, Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(default)]
struct DefaultInner {
    z: i32,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatDefault {
    a: i32,
    #[serde(flatten)]
    inner: DefaultInner,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlatNestedOpt {
    a: i32,
    #[serde(flatten)]
    inner: Option<Option<Inner1>>,
}

// --- serde_json evidence, so the exported type can be checked against what
// serde actually puts on the wire (rather than against assumptions). ---

#[test]
fn flat_opt_serde_evidence() {
    let none = FlatOpt { a: 1, inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1}"#);

    let some = FlatOpt {
        a: 1,
        inner: Some(Inner1 { x: 2 }),
    };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"a":1,"x":2}"#);

    let de_none: FlatOpt = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert!(de_none.inner.is_none());

    let de_some: FlatOpt = serde_json::from_str(r#"{"a":1,"x":2}"#).unwrap();
    assert_eq!(de_some.inner.unwrap().x, 2);
}

#[test]
fn internal_flatten_opt_serde_evidence() {
    let none = InternalFlattenOpt::A { inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"t":"A"}"#);

    let some = InternalFlattenOpt::A {
        inner: Some(Inner1 { x: 1 }),
    };
    assert_eq!(serde_json::to_string(&some).unwrap(), r#"{"t":"A","x":1}"#);

    let de_none: InternalFlattenOpt = serde_json::from_str(r#"{"t":"A"}"#).unwrap();
    match de_none {
        InternalFlattenOpt::A { inner } => assert!(inner.is_none()),
    }

    let de_some: InternalFlattenOpt = serde_json::from_str(r#"{"t":"A","x":1}"#).unwrap();
    match de_some {
        InternalFlattenOpt::A { inner } => assert_eq!(inner.unwrap().x, 1),
    }
}

#[test]
fn flat_two_opt_serde_evidence() {
    let both_none = FlatTwoOpt {
        a: 1,
        inner1: None,
        inner2: None,
    };
    assert_eq!(serde_json::to_string(&both_none).unwrap(), r#"{"a":1}"#);

    let only_inner1 = FlatTwoOpt {
        a: 1,
        inner1: Some(Inner1 { x: 2 }),
        inner2: None,
    };
    assert_eq!(
        serde_json::to_string(&only_inner1).unwrap(),
        r#"{"a":1,"x":2}"#
    );

    let only_inner2 = FlatTwoOpt {
        a: 1,
        inner1: None,
        inner2: Some(Inner2 { y: true }),
    };
    assert_eq!(
        serde_json::to_string(&only_inner2).unwrap(),
        r#"{"a":1,"y":true}"#
    );

    let both_some = FlatTwoOpt {
        a: 1,
        inner1: Some(Inner1 { x: 2 }),
        inner2: Some(Inner2 { y: true }),
    };
    assert_eq!(
        serde_json::to_string(&both_some).unwrap(),
        r#"{"a":1,"x":2,"y":true}"#
    );
}

#[test]
fn flat_default_serde_evidence() {
    // A `#[serde(flatten, default)]` non-`Option` field is unaffected by this
    // fix: its type is never `Nullable`, so it stays a plain (mandatory)
    // intersection part.
    let de: FlatDefault = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(de.a, 1);
    assert_eq!(de.inner.z, 0);

    let with_inner: FlatDefault = serde_json::from_str(r#"{"a":1,"z":5}"#).unwrap();
    assert_eq!(with_inner.inner.z, 5);
}

#[test]
fn flat_nested_opt_serde_evidence() {
    // `Option<Option<T>>` flattens the same way `Option<T>` does.
    let none = FlatNestedOpt { a: 1, inner: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"a":1}"#);

    let some_none = FlatNestedOpt {
        a: 1,
        inner: Some(None),
    };
    assert_eq!(serde_json::to_string(&some_none).unwrap(), r#"{"a":1}"#);

    let some_some = FlatNestedOpt {
        a: 1,
        inner: Some(Some(Inner1 { x: 2 })),
    };
    assert_eq!(
        serde_json::to_string(&some_some).unwrap(),
        r#"{"a":1,"x":2}"#
    );
}

// --- Exported TypeScript, checked against the serde evidence above. ---

#[test]
fn flat_opt_exports_union_instead_of_nullable_intersection() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    let expected = "export type FlatOpt = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn internal_flatten_opt_exports_union_instead_of_nullable_intersection() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalFlattenOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    let expected =
        "export type InternalFlattenOpt = {\n\tt: \"A\",\n} & Inner1 | {\n\tt: \"A\",\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_two_opt_exports_precedence_correct_union() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatTwoOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option must not admit bare `null`:\n{rendered}"
    );

    // Four branches covering every independent Some/None combination of the
    // two flattened Options. `&` binds tighter than `|` in TypeScript, so no
    // parens are required around any branch for this to parse correctly as
    // `(Base & Inner1 & Inner2) | (Base & Inner2) | (Base & Inner1) | Base`.
    let expected = "export type FlatTwoOpt = {\n\ta: number,\n} & Inner1 & Inner2 | {\n\ta: number,\n} & Inner2 | {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_default_export_is_unaffected() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatDefault>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    let expected = "export type FlatDefault = {\n\ta: number,\n} & DefaultInner;";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}

#[test]
fn flat_nested_opt_exports_same_shape_as_single_option() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<FlatNestedOpt>(),
            specta_serde::Format,
        )
        .expect("export should succeed");

    assert!(
        !rendered.contains("| null"),
        "flattened Option<Option<T>> must not admit bare `null`:\n{rendered}"
    );

    let expected = "export type FlatNestedOpt = {\n\ta: number,\n} & Inner1 | {\n\ta: number,\n};";
    assert!(
        rendered.contains(expected),
        "expected:\n{expected}\n\ngot:\n{rendered}"
    );
}
