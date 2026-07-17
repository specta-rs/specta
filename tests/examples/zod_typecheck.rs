use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{NamedDataType, Primitive},
};
use specta_typescript::Typescript;
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

// The TypeScript fixture consumes a generic constrained by this type. The
// previous intersection-of-unions lowering expanded it into 2^16 branches.
#[derive(Type, Serialize, Deserialize)]
struct AliasHeavy {
    #[serde(alias = "first_old")]
    first: String,
    #[serde(alias = "second_old")]
    second: String,
    #[serde(alias = "third_old")]
    third: String,
    #[serde(alias = "fourth_old")]
    fourth: String,
    #[serde(alias = "fifth_old")]
    fifth: String,
    #[serde(alias = "sixth_old")]
    sixth: String,
    #[serde(alias = "seventh_old")]
    seventh: String,
    #[serde(alias = "eighth_old")]
    eighth: String,
    #[serde(alias = "ninth_old")]
    ninth: String,
    #[serde(alias = "tenth_old")]
    tenth: String,
    #[serde(alias = "eleventh_old")]
    eleventh: String,
    #[serde(alias = "twelfth_old")]
    twelfth: String,
    #[serde(alias = "thirteenth_old")]
    thirteenth: String,
    #[serde(alias = "fourteenth_old")]
    fourteenth: String,
    #[serde(alias = "fifteenth_old")]
    fifteenth: String,
    #[serde(alias = "sixteenth_old")]
    sixteenth: String,
}

#[derive(Type, Serialize, Deserialize)]
struct OptionalAlias {
    #[serde(default, alias = "value_old")]
    value: Option<String>,
    #[serde(default, alias = "other_old")]
    other: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalModelPricing {
    #[serde(default, alias = "input_cost")]
    input_cost: Option<f64>,
    #[serde(default, alias = "output_cost")]
    output_cost: Option<f64>,
    #[serde(default, alias = "cache_read_cost")]
    cache_read_cost: Option<f64>,
    #[serde(default, alias = "cache_write_cost")]
    cache_write_cost: Option<f64>,
    #[serde(default, alias = "image_cost")]
    image_cost: Option<f64>,
    #[serde(default, alias = "audio_cost")]
    audio_cost: Option<f64>,
    #[serde(default, alias = "request_cost")]
    request_cost: Option<f64>,
    #[serde(default, alias = "training_cost")]
    training_cost: Option<f64>,
    #[serde(default, alias = "batch_discount")]
    batch_discount: Option<f64>,
    #[serde(default, alias = "storage_cost")]
    storage_cost: Option<f64>,
    #[serde(default, alias = "embedding_cost")]
    embedding_cost: Option<f64>,
    #[serde(default, alias = "search_cost")]
    search_cost: Option<f64>,
    #[serde(default, alias = "reasoning_cost")]
    reasoning_cost: Option<f64>,
    #[serde(default, alias = "minimum_cost")]
    minimum_cost: Option<f64>,
    #[serde(default, alias = "currency_code")]
    currency_code: Option<String>,
    #[serde(default, alias = "billing_unit")]
    billing_unit: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogProvider {
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(default, alias = "base_url")]
    base_url: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogCapability {
    #[serde(default, alias = "supports_tools")]
    supports_tools: Option<bool>,
    #[serde(default, alias = "context_window")]
    context_window: Option<u32>,
}

macro_rules! optional_alias_part {
    ($name:ident, $field:ident: $ty:ty, $alias:literal) => {
        #[derive(Type, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct $name {
            #[serde(default, alias = $alias)]
            $field: Option<$ty>,
        }
    };
}

optional_alias_part!(ThinkingEffort, effort_levels: Vec<String>, "effort_levels");
optional_alias_part!(ThinkingReasoning, include_reasoning_in_response: bool, "include_reasoning_in_response");
optional_alias_part!(ThinkingOutput, counts_against_output: bool, "counts_against_output");
optional_alias_part!(ThinkingSummary, supports_summary: bool, "supports_summary");
optional_alias_part!(ThinkingBudget, supports_budget: bool, "supports_budget");
optional_alias_part!(ThinkingMinimum, minimum_effort: String, "minimum_effort");
optional_alias_part!(ThinkingMaximum, maximum_effort: String, "maximum_effort");
optional_alias_part!(ThinkingDefault, default_effort: String, "default_effort");
optional_alias_part!(ThinkingFormat, summary_format: String, "summary_format");
optional_alias_part!(ThinkingUnit, budget_unit: String, "budget_unit");
optional_alias_part!(ThinkingTokens, reasoning_tokens: u32, "reasoning_tokens");
optional_alias_part!(ThinkingVisible, visible_tokens: u32, "visible_tokens");
optional_alias_part!(ThinkingStreaming, supports_streaming: bool, "supports_streaming");
optional_alias_part!(ThinkingTraces, supports_traces: bool, "supports_traces");
optional_alias_part!(ThinkingRedaction, supports_redaction: bool, "supports_redaction");
optional_alias_part!(ThinkingEncryption, supports_encryption: bool, "supports_encryption");
optional_alias_part!(ThinkingCache, caches_reasoning: bool, "caches_reasoning");
optional_alias_part!(ThinkingParallel, supports_parallel: bool, "supports_parallel");
optional_alias_part!(ThinkingToolUse, supports_tool_use: bool, "supports_tool_use");
optional_alias_part!(ThinkingRetention, retention_days: u32, "retention_days");

#[derive(Type, Serialize, Deserialize)]
struct ThinkingConfig {
    #[serde(flatten)]
    effort: ThinkingEffort,
    #[serde(flatten)]
    reasoning: ThinkingReasoning,
    #[serde(flatten)]
    output: ThinkingOutput,
    #[serde(flatten)]
    summary: ThinkingSummary,
    #[serde(flatten)]
    budget: ThinkingBudget,
    #[serde(flatten)]
    minimum: ThinkingMinimum,
    #[serde(flatten)]
    maximum: ThinkingMaximum,
    #[serde(flatten)]
    default: ThinkingDefault,
    #[serde(flatten)]
    format: ThinkingFormat,
    #[serde(flatten)]
    unit: ThinkingUnit,
    #[serde(flatten)]
    tokens: ThinkingTokens,
    #[serde(flatten)]
    visible: ThinkingVisible,
    #[serde(flatten)]
    streaming: ThinkingStreaming,
    #[serde(flatten)]
    traces: ThinkingTraces,
    #[serde(flatten)]
    redaction: ThinkingRedaction,
    #[serde(flatten)]
    encryption: ThinkingEncryption,
    #[serde(flatten)]
    cache: ThinkingCache,
    #[serde(flatten)]
    parallel: ThinkingParallel,
    #[serde(flatten)]
    tool_use: ThinkingToolUse,
    #[serde(flatten)]
    retention: ThinkingRetention,
}

#[derive(Type, Serialize, Deserialize)]
struct ExtensibleThinkingConfig {
    #[serde(flatten)]
    config: ThinkingConfig,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CanonicalModelThinking {
    Configured(ThinkingConfig),
    Extensible(ExtensibleThinkingConfig),
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogMetadata {
    #[serde(default, alias = "display_label")]
    display_label: Option<String>,
    #[serde(default, alias = "release_stage")]
    release_stage: Option<String>,
    #[serde(default, alias = "documentation_url")]
    documentation_url: Option<String>,
    #[serde(default, alias = "deprecation_date")]
    deprecation_date: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogLimits {
    #[serde(default, alias = "max_input_tokens")]
    max_input_tokens: Option<u32>,
    #[serde(default, alias = "max_output_tokens")]
    max_output_tokens: Option<u32>,
    #[serde(default, alias = "max_images")]
    max_images: Option<u32>,
    #[serde(default, alias = "max_tools")]
    max_tools: Option<u32>,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogEntry {
    #[serde(default, alias = "model_id")]
    model_id: Option<String>,
    #[serde(default, alias = "provider_info")]
    provider_info: Option<CatalogProvider>,
    #[serde(default, alias = "model_pricing")]
    model_pricing: Option<CanonicalModelPricing>,
    #[serde(default, alias = "model_capability")]
    model_capability: Option<CatalogCapability>,
    #[serde(default, alias = "model_thinking")]
    model_thinking: Option<CanonicalModelThinking>,
    #[serde(default, alias = "model_limits")]
    model_limits: Option<CatalogLimits>,
    #[serde(default, alias = "model_metadata")]
    model_metadata: Option<CatalogMetadata>,
}

#[derive(Type, Serialize, Deserialize)]
struct CatalogResponse {
    entries: Vec<CatalogEntry>,
}

#[derive(Type, Serialize, Deserialize)]
struct Pick {
    value: String,
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
    let alias_types = Types::default()
        .register::<AliasHeavy>()
        .register::<OptionalAlias>()
        .register::<CatalogResponse>()
        .register::<CatalogEntry>()
        .register::<CatalogProvider>()
        .register::<CanonicalModelPricing>()
        .register::<CatalogCapability>()
        .register::<CanonicalModelThinking>()
        .register::<CatalogLimits>()
        .register::<CatalogMetadata>()
        .register::<Pick>();
    Typescript::default()
        .export_to(
            out.join("serde-aliases.ts"),
            &alias_types,
            specta_serde::Format,
        )
        .unwrap();
}
