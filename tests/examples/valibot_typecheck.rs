use std::{borrow::Cow, collections::HashMap, iter, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, Map, NamedDataType, Primitive},
};
use specta_util::Remapper;
use specta_valibot::{Layout, Valibot, primitives, runtime_helpers};

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
    floating: f64,
    fixed_array: [u8; 2],
    tuple: (String, bool),
    integer_keys: HashMap<i32, String>,
    string_keys: HashMap<String, String>,
    boolean_keys: HashMap<bool, String>,
    newtype_keys: HashMap<IntegerKey, String>,
    boolean_newtype_keys: HashMap<BooleanKey, String>,
    enum_keys: HashMap<FiniteKey, String>,
    generic_finite_keys: HashMap<GenericKey<FiniteKey>, String>,
    nested_generic_finite_keys: HashMap<OuterKey<FiniteKey>, String>,
    remote_keys: HashMap<keys::RemoteKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct NarrowFloats {
    single: f32,
}

#[derive(Type, Serialize, Deserialize)]
struct EmptyObject {}

#[derive(Type, Serialize, Deserialize)]
struct OptionalObject {
    value: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
struct DefaultTuple(u8, #[serde(default)] u8);

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
    any: specta_valibot::Any<String>,
    unknown: specta_valibot::Unknown<String>,
    never: specta_valibot::Never<String>,
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
struct FlattenFields {
    id: String,
}

#[derive(Type, Serialize, Deserialize)]
enum FlattenPayload {
    First { value: i32 },
    Second { label: String },
}

#[derive(Type, Serialize, Deserialize)]
struct ReferencedFlatten {
    #[serde(flatten)]
    fields: FlattenFields,
    #[serde(flatten)]
    payload: FlattenPayload,
}

#[derive(Type, Serialize, Deserialize)]
struct DangerousFlattenFields {
    #[serde(rename = "__proto__")]
    proto_key: String,
    #[serde(rename = "constructor")]
    constructor_key: String,
    #[serde(rename = "prototype")]
    prototype_key: String,
}

#[derive(Type, Serialize, Deserialize)]
struct DangerousReferencedFlatten {
    #[serde(flatten)]
    fields: DangerousFlattenFields,
    #[serde(flatten)]
    payload: FlattenPayload,
}

#[derive(Type, Serialize, Deserialize)]
enum FlattenLeft {
    A(()),
    C(()),
}

#[derive(Type, Serialize, Deserialize)]
enum FlattenRight {
    B(()),
    D(()),
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenSiblingEnums {
    #[serde(flatten)]
    left: FlattenLeft,
    #[serde(flatten)]
    right: FlattenRight,
}

#[derive(Type, Serialize, Deserialize)]
enum WrappedFlattenEnum {
    A(()),
    B(()),
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedEnumNewtype<T>(T);

#[derive(Type, Serialize, Deserialize)]
struct FlattenedEnumNewtypeHolder {
    id: u32,
    #[serde(flatten)]
    value: FlattenedEnumNewtype<WrappedFlattenEnum>,
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedStringMap {
    id: u32,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
enum FlattenedExternalMapPayload {
    #[serde(rename = "wire_a")]
    A(()),
    B(()),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum FlattenedOpenEnum {
    WithMap(flattened_maps::StringMap),
    Empty,
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedExternalOpenEnum {
    #[serde(flatten)]
    payload: FlattenedExternalMapPayload,
    #[serde(flatten)]
    extra: FlattenedOpenEnum,
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedExternalGeneric<T> {
    #[serde(flatten)]
    payload: FlattenedExternalMapPayload,
    #[serde(flatten)]
    extra: T,
}

// https://github.com/specta-rs/specta/pull/558
#[derive(Type, Serialize, Deserialize)]
struct FlattenedExternalMap {
    id: u32,
    #[serde(flatten)]
    payload: FlattenedExternalMapPayload,
    #[serde(flatten)]
    extra: flattened_maps::StringMap,
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedFiniteMap {
    #[serde(rename = "wire_id")]
    id: u32,
    #[serde(flatten)]
    extra: HashMap<FiniteKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalFlattenedMap {
    id: u32,
    #[serde(flatten)]
    extra: Option<HashMap<String, String>>,
}

#[derive(Type, Serialize, Deserialize)]
struct GenericFlatten<T> {
    id: u32,
    #[serde(flatten)]
    extra: T,
}

#[derive(Type, Serialize, Deserialize)]
struct GenericFlattenHolder {
    value: GenericFlatten<HashMap<String, String>>,
}

mod flattened_maps {
    use super::*;

    #[derive(Type, Serialize, Deserialize)]
    pub struct StringMap(pub HashMap<String, String>);
}

#[derive(Type, Serialize, Deserialize)]
struct CrossModuleFlattenedMap {
    id: u32,
    #[serde(flatten)]
    extra: flattened_maps::StringMap,
}

#[derive(Type, Serialize, Deserialize)]
struct NestedMapInner {
    name: String,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct NestedFlattenedMap {
    id: u32,
    #[serde(flatten)]
    inner: NestedMapInner,
}

#[derive(Type, Serialize, Deserialize)]
struct OverlappingNestedMapInner {
    id: u32,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
struct OverlappingNestedFlattenedMap {
    id: u32,
    #[serde(flatten)]
    inner: OverlappingNestedMapInner,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum FlattenedMapEnum {
    A {
        #[serde(flatten)]
        extra: HashMap<String, String>,
    },
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedMapEnumHolder {
    id: u32,
    #[serde(flatten)]
    value: FlattenedMapEnum,
}

#[derive(Type, Serialize, Deserialize)]
enum AllSkippedTupleVariant {
    A(#[serde(skip)] u8, #[serde(skip)] u8),
    B(String),
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
            specta_valibot::define("v.string()").into(),
        )
        .remap_types(
            Types::default()
                .register::<Recursive>()
                .register::<Generic>()
                .register::<WireTypes>()
                .register::<NarrowFloats>()
                .register::<EmptyObject>()
                .register::<OptionalObject>()
                .register::<DefaultTuple>()
                .register::<DefinedMapKey>()
                .register::<OptionalFlatten>()
                .register::<ProtoField>()
                .register::<GenericMapHolder>()
                .register::<OpaqueTypes>()
                .register::<ExternalEnum>()
                .register::<ContextualExternalWrapper>()
                .register::<MapOnlyExternalWrapper>()
                .register::<ReferencedFlatten>()
                .register::<DangerousReferencedFlatten>()
                .register::<FlattenSiblingEnums>()
                .register::<FlattenedEnumNewtypeHolder>()
                .register::<FlattenedStringMap>()
                .register::<FlattenedExternalMap>()
                .register::<FlattenedExternalOpenEnum>()
                .register::<FlattenedExternalGeneric<flattened_maps::StringMap>>()
                .register::<FlattenedFiniteMap>()
                .register::<OptionalFlattenedMap>()
                .register::<GenericFlattenHolder>()
                .register::<CrossModuleFlattenedMap>()
                .register::<NestedFlattenedMap>()
                .register::<OverlappingNestedFlattenedMap>()
                .register::<FlattenedMapEnumHolder>()
                .register::<AllSkippedTupleVariant>()
                .register::<UntaggedMatchingField>()
                .register::<UsesKeywordModule>(),
        );
    let v_type = NamedDataType::new("PreludeCollision", &mut types, |_, ndt| {
        ndt.module_path = "v".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesV", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(specta::datatype::DataType::Reference(
            v_type.reference(vec![]),
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
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("valibot-typecheck/generated");
    std::fs::create_dir_all(&out).unwrap();

    Valibot::default()
        .export_to(out.join("bindings.ts"), &types, specta_serde::Format)
        .unwrap();
    let mut low_level_types = Types::default();
    let low_level_record = NamedDataType::new("LowLevelRecord", &mut low_level_types, |_, ndt| {
        ndt.ty = Some(Map::new(Primitive::str.into(), Primitive::str.into()).into());
    });
    let low_level_export = primitives::export(
        &Valibot::default(),
        &low_level_types,
        iter::once(&low_level_record),
        "",
    )
    .unwrap();
    let low_level_inline = primitives::inline(
        &Valibot::default(),
        &low_level_types,
        &Map::new(Primitive::str.into(), Primitive::str.into()).into(),
    )
    .unwrap();
    let inline_reference = specta::datatype::inline(&mut low_level_types, EmptyObject::definition);
    let DataType::Reference(inline_reference) = inline_reference else {
        panic!("inline named type must produce a reference");
    };
    let low_level_reference =
        primitives::reference(&Valibot::default(), &low_level_types, &inline_reference).unwrap();
    std::fs::write(
        out.join("low-level.ts"),
        format!(
            "import * as v from \"valibot\";\n{}\n{low_level_export}\nexport const LowLevelInlineSchema = {low_level_inline};\nexport const LowLevelReferenceSchema = {low_level_reference};\n",
            runtime_helpers(),
        ),
    )
    .unwrap();
    Valibot::default()
        .layout(Layout::Namespaces)
        .export_to(out.join("namespaces.ts"), &types, specta_serde::Format)
        .unwrap();
    let mut manual_namespace_types = Types::default();
    let manual_namespace_type = NamedDataType::new(
        "ManualNamespaceType",
        &mut manual_namespace_types,
        |_, ndt| {
            ndt.module_path = "".into();
            ndt.ty = Some(Primitive::str.into());
        },
    );
    NamedDataType::new(
        "AutomaticNamespaceType",
        &mut manual_namespace_types,
        |_, ndt| {
            ndt.module_path = "".into();
            ndt.ty = Some(DataType::Reference(manual_namespace_type.reference(vec![])));
        },
    );
    Valibot::default()
        .layout(Layout::Namespaces)
        .framework_runtime(|exporter| {
            let manual = exporter
                .types
                .into_unsorted_iter()
                .filter(|ndt| ndt.name == "ManualNamespaceType");
            Ok(Cow::Owned(format!(
                "{}\nexport const AdaptedManualNamespaceTypeSchema = ManualNamespaceTypeSchema;",
                exporter.export(manual, "")?
            )))
        })
        .export_to(
            out.join("namespaces-manual.ts"),
            &manual_namespace_types,
            specta_serde::Format,
        )
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
    Valibot::default()
        .layout(Layout::ModulePrefixedName)
        .export_to(
            out.join("module-prefixed.ts"),
            &module_prefixed_types,
            specta_serde::Format,
        )
        .unwrap();
    Valibot::default()
        .layout(Layout::Files)
        .with_raw("export const runtime = true;")
        .export_to(out.join("files"), &types, specta_serde::Format)
        .unwrap();

    let mut manual_file_types = Types::default();
    let sibling = NamedDataType::new("Sibling", &mut manual_file_types, |_, ndt| {
        ndt.module_path = "nested".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("ManualNested", &mut manual_file_types, |_, ndt| {
        ndt.module_path = "nested".into();
        ndt.ty = Some(DataType::Reference(sibling.reference(vec![])));
    });
    Valibot::default()
        .layout(Layout::Files)
        .framework_runtime(|exporter| {
            let manual = exporter
                .types
                .into_unsorted_iter()
                .filter(|ndt| ndt.name == "ManualNested");
            Ok(Cow::Owned(exporter.export(manual, "")?))
        })
        .export_to(
            out.join("manual-files"),
            &manual_file_types,
            specta_serde::Format,
        )
        .unwrap();

    let mut separate_declaration_spaces = Types::default();
    for name in ["Foo", "FooSchema"] {
        NamedDataType::new(name, &mut separate_declaration_spaces, |_, ndt| {
            ndt.module_path = "source".into();
            ndt.ty = Some(Primitive::str.into());
        });
    }
    Valibot::default()
        .layout(Layout::Files)
        .export_to(
            out.join("separate-declaration-spaces"),
            &separate_declaration_spaces,
            specta_serde::Format,
        )
        .unwrap();
}
