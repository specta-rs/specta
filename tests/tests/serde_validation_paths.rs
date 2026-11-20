// Regression tests for the diagnostic paths the validation walk reports.
//
// The walk builds paths in one reusable buffer as it enters and leaves nodes
// instead of formatting a fresh String per node, so these assertions pin the
// rendered text: the field separator ('.'), the variant separator ('::'),
// positional [index] segments, the named-reference reset (which restarts the
// path at the declaration's own name) and the '->' note added when an error
// surfaces from inside a referenced definition.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

/// The exporter wraps formatter errors, so compare from the walk's own text.
fn unified_error<T: Type>() -> String {
    let err = Typescript::default()
        .export(&Types::default().register::<T>(), specta_serde::Format)
        .expect_err("the type is not representable in unified mode")
        .to_string();
    let prefix = "Format error: type graph formatter failed: ";
    err.strip_prefix(prefix).map_or(err.clone(), str::to_string)
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PathFieldRoot {
    nested: PathFieldNested,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PathFieldNested {
    #[serde(flatten)]
    v: Vec<u8>,
}

#[test]
fn field_paths_restart_at_the_referenced_definition() {
    assert_eq!(
        unified_error::<PathFieldRoot>(),
        "Invalid `#[serde(flatten)]` target at 'PathFieldNested.v': serde can only flatten structs and maps, but this field's type serializes as a sequence, tuple, or scalar value"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum PathIndexRoot {
    V(String, #[serde(skip_deserializing)] bool),
}

#[test]
fn positional_paths_use_index_segments() {
    assert_eq!(
        unified_error::<PathIndexRoot>(),
        "Incompatible field skip for 'PathIndexRoot::V[1]' in unified mode: skip_serializing=false, skip_deserializing=true. Use PhasesFormat for one-sided serde skips"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum PathVariantRoot {
    A(String),
    #[serde(skip_deserializing)]
    B(String),
}

#[test]
fn variant_paths_use_the_variant_separator() {
    assert_eq!(
        unified_error::<PathVariantRoot>(),
        "Incompatible variant skip for 'PathVariantRoot::B' in unified mode: skip_serializing=false, skip_deserializing=true. Use PhasesFormat for one-sided serde skips"
    );
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum PathArrowRoot {
    V(PathArrowInner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum PathArrowInner {
    A(String),
}

#[test]
fn nested_reference_errors_extend_the_path_with_an_arrow() {
    assert_eq!(
        unified_error::<PathArrowRoot>(),
        "Invalid internally tagged enum at 'PathArrowRoot -> PathArrowInner -> String', variant 'A': payload cannot be merged with an internal tag"
    );
}
