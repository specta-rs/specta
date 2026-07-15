use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{NamedDataType, Primitive},
};
use specta_util::Remapper;
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
struct BooleanKey(bool);

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct GenericKey<T>(T);

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct OuterKey<T>(GenericKey<T>);

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
    boolean_newtype_keys: HashMap<BooleanKey, String>,
    enum_keys: HashMap<FiniteKey, String>,
    generic_finite_keys: HashMap<GenericKey<FiniteKey>, String>,
    nested_generic_finite_keys: HashMap<OuterKey<FiniteKey>, String>,
    remote_keys: HashMap<keys::RemoteKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct DefinedMapKey {
    value: HashMap<i64, String>,
    named: HashMap<DefinedKey, String>,
}

#[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
struct DefinedKey(i64);

#[derive(Type, Serialize, Deserialize)]
struct OptionalFlattenInner {
    inner: String,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalFlatten {
    id: String,
    #[serde(flatten)]
    inner: Option<OptionalFlattenInner>,
}

#[derive(Type, Serialize, Deserialize)]
struct ProtoField {
    #[serde(rename = "__proto__")]
    prototype: String,
}

#[derive(Type, Serialize, Deserialize)]
struct GenericMap<K: Eq + std::hash::Hash = bool> {
    values: HashMap<K, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct GenericMapHolder {
    booleans: GenericMap<bool>,
    integers: GenericMap<i32>,
    finite: GenericMap<FiniteKey>,
    chained: ChainedDefaultMap,
}

#[derive(Type, Serialize, Deserialize)]
struct ChainedDefaultMap<T = bool, U = T>
where
    U: Eq + std::hash::Hash,
{
    marker: Option<T>,
    values: HashMap<U, String>,
}

#[derive(Type)]
#[allow(dead_code)]
struct OpaqueTypes {
    any: specta_zod::Any<String>,
    unknown: specta_zod::Unknown<String>,
    never: specta_zod::Never<String>,
}

#[derive(Type, Serialize, Deserialize)]
enum ExternalEnum {
    Unit,
    Newtype(String),
    Tuple(i32, bool),
}

#[derive(Type, Serialize, Deserialize)]
enum ContextualExternalPayload {
    Unit,
    Newtype(i32),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum ContextualExternalWrapper {
    Value(ContextualExternalPayload),
}

#[derive(Type, Serialize, Deserialize)]
enum MapOnlyExternalPayload<T> {
    First { value: T },
    Second { label: String },
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum MapOnlyExternalWrapper {
    Value(MapOnlyExternalPayload<i32>),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(untagged)]
enum UntaggedMatchingField {
    Variant {
        #[serde(rename = "Variant")]
        value: String,
    },
    Empty {},
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

    #[allow(clippy::module_inception)]
    pub mod r#type {
        use super::*;

        #[derive(Type, Serialize, Deserialize)]
        pub struct NestedKeywordModule {
            value: String,
        }
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
    nested: r#type::r#type::NestedKeywordModule,
}

fn main() {
    let mut types = Remapper::new()
        .rule(
            Primitive::i64.into(),
            specta_zod::define("z.string()").into(),
        )
        .remap_types(
            Types::default()
                .register::<Recursive>()
                .register::<Generic>()
                .register::<WireTypes>()
                .register::<DefinedMapKey>()
                .register::<OptionalFlatten>()
                .register::<ProtoField>()
                .register::<GenericMapHolder>()
                .register::<OpaqueTypes>()
                .register::<ExternalEnum>()
                .register::<ContextualExternalWrapper>()
                .register::<MapOnlyExternalWrapper>()
                .register::<UntaggedMatchingField>()
                .register::<UsesKeywordModule>(),
        );
    let z_type = NamedDataType::new("PreludeCollision", &mut types, |_, ndt| {
        ndt.module_path = "z".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesZ", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            z_type.reference(vec![]),
        ));
    });
    let root_type = NamedDataType::new("RootReference", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesRoot", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            root_type.reference(vec![]),
        ));
    });
    let parent_type = NamedDataType::new("ParentReference", &mut types, |_, ndt| {
        ndt.module_path = "parent".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesParent", &mut types, |_, ndt| {
        ndt.module_path = "parent::child".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            parent_type.reference(vec![]),
        ));
    });
    let index_type = NamedDataType::new("IndexType", &mut types, |_, ndt| {
        ndt.module_path = "index".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesIndex", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            index_type.reference(vec![]),
        ));
    });
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("zod-typecheck/generated");
    std::fs::create_dir_all(&out).unwrap();

    Zod::default()
        .export_to(out.join("bindings.ts"), &types, specta_serde::Format)
        .unwrap();
    Zod::default()
        .layout(Layout::Namespaces)
        .export_to(out.join("namespaces.ts"), &types, specta_serde::Format)
        .unwrap();
    let mut module_prefixed_types = Types::default();
    let module_prefixed_root =
        NamedDataType::new("RootType", &mut module_prefixed_types, |_, ndt| {
            ndt.module_path = "".into();
            ndt.ty = Some(Primitive::str.into());
        });
    NamedDataType::new("UsesRootType", &mut module_prefixed_types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            module_prefixed_root.reference(vec![]),
        ));
    });
    Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export_to(
            out.join("module-prefixed.ts"),
            &module_prefixed_types,
            specta_serde::Format,
        )
        .unwrap();
    Zod::default()
        .layout(Layout::Files)
        .with_raw("export const runtime = true;")
        .export_to(out.join("files"), &types, specta_serde::Format)
        .unwrap();
}
