// These types exist only to be exported into the `tests/typescript-typecheck`
// harness, so their fields are never read.
#![allow(dead_code)]

use std::{collections::HashMap, path::Path};

use specta::{Type, Types};
use specta_typescript::{Typescript, semantic::Configuration};

#[derive(Type)]
#[specta(collect = false)]
#[specta(transparent)]
struct MaybeValidKey<T>(T);

#[derive(Type)]
#[specta(collect = false)]
struct WireMapTypes {
    floats: HashMap<f32, String>,
    integers: HashMap<i32, String>,
    booleans: HashMap<bool, String>,
    nested: HashMap<String, HashMap<f32, String>>,
    named_wrapper: HashMap<MaybeValidKey<String>, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericKeyMap<K> {
    generic: HashMap<K, String>,
    generic_wrapper: HashMap<MaybeValidKey<K>, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct LosslessMapTypes {
    wide: HashMap<u64, String>,
    floats: HashMap<f64, String>,
}

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("typescript-typecheck/generated");
    std::fs::create_dir_all(&out).unwrap();

    let types = Types::default()
        .register::<WireMapTypes>()
        .register::<GenericKeyMap<String>>();
    Typescript::default()
        .export_to(out.join("bindings.ts"), &types, specta_serde::Format)
        .unwrap();

    let lossless_types = Types::default().register::<LosslessMapTypes>();
    let lossless_types = Configuration::empty()
        .enable_lossless_bigints()
        .enable_lossless_floats()
        .apply_types(&lossless_types);
    Typescript::default()
        .export_to(
            out.join("lossless.ts"),
            &lossless_types,
            specta_serde::Format,
        )
        .unwrap();
}
