//! Unified serde shapes must accept the data both directions accept.
//!
//! Serde writes `null` for a `None` field but also accepts a missing key when
//! deserializing, so neither "required" nor "optional" alone describes the
//! shared shape.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Input {
    x: Option<u32>,
    required: u32,
}

#[test]
fn a_unified_option_field_is_optional() {
    let ts = Typescript::default()
        .export(&Types::default().register::<Input>(), specta_serde::Format)
        .expect("export succeeds");

    assert!(
        ts.contains("x?: number | null,"),
        "a unified shape must accept a missing key: {ts}"
    );
    assert!(
        ts.contains("required: number,"),
        "a non-optional field stays required: {ts}"
    );
}

#[test]
fn the_exported_shape_matches_serde() {
    // Missing key is accepted on deserialize.
    assert!(serde_json::from_str::<Input>(r#"{"required":1}"#).is_ok());
    // Serialize always writes the key, even when it is null.
    assert_eq!(
        serde_json::to_string(&Input {
            x: None,
            required: 1
        })
        .unwrap(),
        r#"{"x":null,"required":1}"#
    );
}
