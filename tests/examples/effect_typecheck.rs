#![allow(missing_docs, dead_code)]

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_effect::{Effect, Layout};

#[derive(Type, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    nickname: Option<String>,
    active: bool,
}

#[derive(Type, Serialize, Deserialize)]
struct Generic<T> {
    value: T,
    values: Vec<T>,
}

#[derive(Type, Serialize, Deserialize)]
struct Defaults<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type, Serialize, Deserialize)]
struct GenericMap<K = bool, V = String>
where
    K: Eq + std::hash::Hash,
{
    values: HashMap<K, V>,
}

#[derive(Type, Serialize, Deserialize)]
struct ChainedMap<K = bool, U = K>
where
    K: Eq + std::hash::Hash,
    U: Eq + std::hash::Hash,
{
    marker: Option<K>,
    values: HashMap<U, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum Message<T> {
    Hello(T),
    Ping,
    Goodbye { reason: Option<String> },
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalFields {
    #[serde(default)]
    nickname: Option<String>,
    required: i32,
}

#[derive(Type, Serialize, Deserialize)]
struct Complex {
    user: User,
    generic: Generic<i32>,
    defaulted: Defaults,
    messages: Vec<Message<String>>,
    booleans: HashMap<bool, Vec<i32>>,
    pair: (i32, Option<String>),
    optional: OptionalFields,
    chained: ChainedMap,
    default_map: DefaultMapHolder,
}

#[derive(Type, Serialize, Deserialize)]
struct Node {
    value: i32,
    children: Vec<Node>,
}

#[derive(Type, Serialize, Deserialize)]
struct ProtoField {
    #[serde(rename = "__proto__")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
struct Flattened {
    id: u32,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct Aliased {
    #[serde(alias = "old_value")]
    value: String,
    other: i32,
}

#[derive(Type)]
struct DefaultedKey<T = bool>(T);

#[derive(Type)]
struct DefaultedKeyMap {
    values: HashMap<DefaultedKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalFlattenInner {
    a: i32,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalFlatten {
    id: u32,
    #[serde(flatten)]
    inner: Option<OptionalFlattenInner>,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalMapFlatten {
    id: u32,
    #[serde(flatten)]
    inner: Option<HashMap<String, String>>,
}

#[derive(Type)]
struct MutualA {
    b: Option<Box<MutualB>>,
}

#[derive(Type)]
struct MutualB {
    a: Option<Box<MutualA>>,
}

#[derive(Type)]
struct GenericWrapper<T> {
    value: T,
}

#[derive(Type)]
struct CycleA {
    b: Option<Vec<GenericWrapper<CycleB>>>,
}

#[derive(Type)]
struct CycleB {
    a: Option<Vec<GenericWrapper<CycleA>>>,
}

#[derive(Type)]
struct DefaultCycleA<T = DefaultCycleB> {
    value: Option<Box<T>>,
}

#[derive(Type)]
struct DefaultCycleB {
    a: DefaultCycleA,
}

#[derive(Type)]
struct GenericMapDefault<T = GenericMap<String, i32>> {
    value: T,
}

#[derive(Type, Serialize, Deserialize)]
struct DefaultMapHolder<T = bool, U = GenericMap<T, String>>
where
    T: Eq + std::hash::Hash,
{
    marker: T,
    value: U,
}

fn main() {
    let types = specta::Types::default()
        .register::<Complex>()
        .register::<GenericMap>()
        .register::<ChainedMap>()
        .register::<Node>()
        .register::<ProtoField>()
        .register::<Flattened>()
        .register::<Aliased>()
        .register::<DefaultedKeyMap>()
        .register::<OptionalFlatten>()
        .register::<OptionalMapFlatten>()
        .register::<MutualA>()
        .register::<CycleA>()
        .register::<DefaultCycleA>()
        .register::<GenericMapDefault>()
        .register::<DefaultMapHolder>();

    let generated = Path::new(env!("CARGO_MANIFEST_DIR")).join("effect-typecheck/generated");
    Effect::default()
        .export_to(generated.join("bindings.ts"), &types, specta_serde::Format)
        .unwrap();
    Effect::default()
        .layout(Layout::Namespaces)
        .export_to(
            generated.join("namespaces.ts"),
            &types,
            specta_serde::Format,
        )
        .unwrap();
    Effect::default()
        .layout(Layout::ModulePrefixedName)
        .export_to(generated.join("prefixed.ts"), &types, specta_serde::Format)
        .unwrap();
    Effect::default()
        .layout(Layout::Files)
        .export_to(generated.join("files"), &types, specta_serde::Format)
        .unwrap();
}
