use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_zod::{Layout, Zod};

#[derive(Type, Serialize, Deserialize)]
struct Recursive {
    children: Vec<Recursive>,
}

#[derive(Type, Serialize, Deserialize)]
struct Generic<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct IntegerKey(i32);

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct GenericKey<T>(T);

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
enum FiniteKey {
    First,
    Second,
}

#[derive(Type, Serialize, Deserialize)]
struct WireTypes {
    character: char,
    integer_keys: HashMap<i32, String>,
    boolean_keys: HashMap<bool, String>,
    newtype_keys: HashMap<IntegerKey, String>,
    enum_keys: HashMap<FiniteKey, String>,
    generic_finite_keys: HashMap<GenericKey<FiniteKey>, String>,
    remote_keys: HashMap<keys::RemoteKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
enum ExternalEnum {
    Unit,
    Newtype(String),
    Tuple(i32, bool),
}

mod r#type {
    use super::*;

    #[derive(Type, Serialize, Deserialize)]
    pub struct KeywordModule {
        value: String,
    }

    #[derive(Type, Serialize, Deserialize)]
    pub struct SameKeywordModule {
        value: KeywordModule,
    }
}

mod keys {
    use super::*;

    #[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
    pub struct RemoteKey(pub i8);
}

#[derive(Type, Serialize, Deserialize)]
struct UsesKeywordModule {
    value: r#type::KeywordModule,
    same_module: r#type::SameKeywordModule,
}

fn main() {
    let types = Types::default()
        .register::<Recursive>()
        .register::<Generic>()
        .register::<WireTypes>()
        .register::<ExternalEnum>()
        .register::<UsesKeywordModule>();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("zod-typecheck/generated");
    std::fs::create_dir_all(&out).unwrap();

    Zod::default()
        .export_to(out.join("bindings.ts"), &types, specta_serde::Format)
        .unwrap();
    Zod::default()
        .layout(Layout::Namespaces)
        .export_to(out.join("namespaces.ts"), &types, specta_serde::Format)
        .unwrap();
    Zod::default()
        .layout(Layout::Files)
        .export_to(out.join("files"), &types, specta_serde::Format)
        .unwrap();
}
