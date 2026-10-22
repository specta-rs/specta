//! Regression test for the S1 defect in <https://github.com/specta-rs/specta/issues/590>.
//!
//! Serde attribute lowering used to be gated behind `specta-macros/serde`, and the only crate that
//! enabled that feature was `specta-serde`. Because Cargo unifies features across the whole
//! dependency graph, whether `#[derive(Type)]` recorded any Serde metadata depended on whether
//! *something* in the graph pulled in `specta-serde`: a consumer of `specta` + `derive` + an
//! exporter (no `specta-serde`) silently lost every `rename`, `rename_all` and `untagged`
//! attribute, and shipped a type that rejected the wire values its own `serde` impl accepted.
//!
//! This crate deliberately depends only on `specta` (with `derive`), `specta-typescript` and
//! `serde`, so it exercises exactly that configuration. It has to be a standalone example rather
//! than a case in the `specta-tests` crate, because `specta-tests` depends on `specta-serde` and
//! workspace feature unification would hide the regression.
//!
//! Run it with `cargo run -p serde-metadata-no-direct-deps`.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use specta::{
    Format, FormatError, Type, Types,
    datatype::{DataType, Fields, NamedDataType},
};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "RenamedDemo", rename_all = "camelCase")]
struct Demo {
    hello_world: String,
}

/// The smallest consumer of the recorded `serde:*` metadata.
///
/// A real application would pass `specta_serde::Format`, but this crate must not depend on
/// `specta-serde` — that dependency is exactly what used to hide the bug — so this applies only
/// the container `rename` and `rename_all` needed to observe the metadata.
struct MiniSerdeFormat;

impl Format for MiniSerdeFormat {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, FormatError> {
        let mut types = types.clone();
        types.iter_mut(apply_metadata);
        Ok(Cow::Owned(types))
    }

    fn map_type(&self, _types: &Types, dt: &DataType) -> Result<Cow<'_, DataType>, FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

fn apply_metadata(ndt: &mut NamedDataType) {
    let Some(DataType::Struct(strct)) = &mut ndt.ty else {
        return;
    };

    if let Some(rename) = strct
        .attributes
        .get_named_as::<String>(specta::markers::SERDE_CONTAINER_RENAME_SERIALIZE)
    {
        ndt.name = Cow::Owned(rename.clone());
    }

    let rule = strct
        .attributes
        .get_named_as::<String>(specta::markers::SERDE_CONTAINER_RENAME_ALL_SERIALIZE)
        .map(String::as_str);
    let Fields::Named(named) = &mut strct.fields else {
        return;
    };
    for (name, _) in &mut named.fields {
        if let Some(rule) = rule {
            *name = Cow::Owned(apply_rename_rule(rule, name));
        }
    }
}

/// `camelCase` is the only rule this example authors, so it only implements that one.
fn apply_rename_rule(rule: &str, name: &str) -> String {
    if rule != "camelCase" {
        return name.to_owned();
    }

    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for ch in name.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn main() {
    let types = Types::default().register::<Demo>();

    // 1. The metadata must be recorded even though nothing here depends on `specta-serde`.
    let attributes = types
        .into_sorted_iter()
        .find_map(|ndt| match ndt.ty.as_ref() {
            Some(DataType::Struct(strct)) => Some(strct.attributes.clone()),
            _ => None,
        })
        .expect("Demo should be registered as a named struct");
    assert_eq!(
        attributes
            .get_named_as::<String>(specta::markers::SERDE_CONTAINER_RENAME_SERIALIZE)
            .map(String::as_str),
        Some("RenamedDemo"),
        "container `rename` metadata was dropped; is serde lowering feature-gated again?"
    );
    assert_eq!(
        attributes
            .get_named_as::<String>(specta::markers::SERDE_CONTAINER_RENAME_ALL_SERIALIZE)
            .map(String::as_str),
        Some("camelCase"),
        "`rename_all` metadata was dropped; is serde lowering feature-gated again?"
    );

    // 2. An exporter reading that metadata must produce the renamed identifiers.
    let output = Typescript::default()
        .export(&types, MiniSerdeFormat)
        .expect("TypeScript export should succeed");
    assert!(
        output.contains("RenamedDemo"),
        "exported TypeScript did not use the renamed type identifier:\n{output}"
    );
    assert!(
        output.contains("helloWorld"),
        "exported TypeScript did not use the renamed field identifier:\n{output}"
    );

    println!("{output}");
}
