//! Analyze Specta datatypes and generate inline JavaScript transforms.
//!
//! `TransformPlan` walks a resolved `specta::DataType` tree, tags values that need
//! JavaScript-side conversion, and renders a plain JavaScript expression that applies
//! those conversions at runtime.
//!
//! This is intended for cases where a transport serializes values like `u128`, dates,
//! or bytes into JSON-compatible representations and the client needs to restore their
//! richer JavaScript forms.
//!
//! ```rust
//! use specta::{ResolvedTypes, Type, Types};
//!
//! #[derive(Type)]
//! struct Event {
//!     id: u128,
//! }
//!
//! let mut types = Types::default();
//! let dt = Event::definition(&mut types);
//! let resolved = ResolvedTypes::from_resolved_types(types);
//!
//! let plan = specta_tags::TransformPlan::analyze(dt, &resolved);
//! let js = plan.map("value");
//! assert!(js.contains("BigInt"));
//! assert!(js.contains("[\"id\"]"));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use std::borrow::Cow;

use specta::{
    ResolvedTypes, Types,
    datatype::{DataType, Fields, GenericReference, NamedReference, Primitive, Reference},
};

// TODO: Allow configuring custom named types via NDT name and module path using config params.
// TODO: Tagging-style system for `rspc` w/ runtime

// TODO: Core changes to `BigInt` handling with Typescript exporter???
// TODO: How to handle `UInt8Array`?
// TODO: Could we support `Custom` tags? If the runtime is fixed thats hard.

// TODO: Documentations -> Explain how input types *just work* (double check that though)

/// A tag is used to identify the transformation required for a given data type.
pub enum Tag {
    /// [BigInt](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt)
    BigInt,
    /// [Uint8Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)
    Uint8Array,
    /// [Date](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
    Date,
    /// A custom tag.
    ///
    /// TODO: Document this
    Custom(Box<dyn Fn(&str) -> Cow<'static, str> + Send + Sync>),
}

impl std::fmt::Debug for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BigInt => f.write_str("BigInt"),
            Self::Uint8Array => f.write_str("Uint8Array"),
            Self::Date => f.write_str("Date"),
            Self::Custom(_) => f.write_str("Custom(<fn>)"),
        }
    }
}

/// A compiled transform plan for a resolved Specta datatype.
///
/// Use [`TransformPlan::analyze`] to build the plan, then [`TransformPlan::map`] to
/// render the JavaScript expression that applies the required runtime conversions.
#[derive(Debug)]
pub struct TransformPlan {
    plan: PlanNode,
}

impl TransformPlan {
    /// Analyzes a resolved datatype and records the JavaScript conversions it needs.
    pub fn analyze(dt: DataType, types: &ResolvedTypes) -> Self {
        // Scan all `DataType` references, etc. and collect tags and their object location for `Self::map` to use.
        //
        // You should match on `NamedDataType`'s name and module path to determine known named types.

        Self {
            plan: Analyzer.analyze(&dt, types.as_types(), &[], &mut Vec::new()),
        }
    }

    /// Renders a JavaScript expression that transforms the provided input expression.
    ///
    /// When no runtime conversions are required this returns the original expression.
    /// Otherwise it returns a new expression that applies the necessary `BigInt`,
    /// `Date`, or `Uint8Array` conversions inline.
    pub fn map<'a>(&self, t: &'a str) -> Cow<'a, str> {
        // If `t` is a struct and has tags we wanna decompose it to something like:
        // `{ ...t, field_with_override: { nested_override: new Date(t.field_with_override.nested_override) } }`
        //
        // If a nested tag doesn't have tags we should just return it to avoid deconstructing it.
        //
        // Otherwise we need to traverse the structure to apply the tags inline.
        // This should just be plain JS code so should work in Typescript (via inference) and in JS.
        //
        // If it has no tags we can just return `t` as is.

        if self.plan.is_identity() {
            return t.into();
        }

        Cow::Owned(Renderer::default().render(&self.plan, t))
    }
}

#[derive(Debug)]
enum PlanNode {
    Identity,
    Leaf(Tag),
    Nullable(Box<PlanNode>),
    List(Box<PlanNode>),
    Tuple(Vec<PlanNode>),
    Object(Vec<ObjectFieldPlan>),
    Map(Box<PlanNode>),
    Enum(Vec<EnumVariantPlan>),
}

impl PlanNode {
    fn is_identity(&self) -> bool {
        match self {
            Self::Identity => true,
            Self::Leaf(_) => false,
            Self::Nullable(inner) | Self::List(inner) | Self::Map(inner) => inner.is_identity(),
            Self::Tuple(items) => items.iter().all(Self::is_identity),
            Self::Object(fields) => fields.iter().all(ObjectFieldPlan::is_identity),
            Self::Enum(variants) => variants.iter().all(|v| v.plan.is_identity()),
        }
    }
}

#[derive(Debug)]
enum ObjectFieldPlan {
    Named(String, PlanNode),
    Flattened(PlanNode),
}

impl ObjectFieldPlan {
    fn is_identity(&self) -> bool {
        match self {
            Self::Named(_, plan) | Self::Flattened(plan) => plan.is_identity(),
        }
    }
}

#[derive(Debug)]
struct EnumVariantPlan {
    matcher: EnumVariantMatcher,
    plan: PlanNode,
}

#[derive(Debug, Clone)]
enum EnumVariantMatcher {
    HasField(String),
    Tagged { field: String, value: String },
    Direct,
}

#[derive(Clone, Copy)]
enum KnownNamedTag {
    BigInt,
    Date,
    Uint8Array,
}

// TODO: Review everything in this!
const BUILTIN_MATCHERS: &[(&str, &str, KnownNamedTag)] = &[
    // BigInt-like wrapper
    // ("tauri_specta", "BigInt", KnownNamedTag::BigInt), // TODO: Fix this
    // Date-like
    ("std::time", "SystemTime", KnownNamedTag::Date),
    ("toml::value", "Datetime", KnownNamedTag::Date),
    ("chrono", "NaiveDateTime", KnownNamedTag::Date),
    ("chrono", "NaiveDate", KnownNamedTag::Date),
    ("chrono", "Date", KnownNamedTag::Date),
    ("chrono", "DateTime", KnownNamedTag::Date),
    ("time", "PrimitiveDateTime", KnownNamedTag::Date),
    ("time", "OffsetDateTime", KnownNamedTag::Date),
    ("time", "Date", KnownNamedTag::Date),
    ("jiff", "Timestamp", KnownNamedTag::Date),
    ("jiff", "Zoned", KnownNamedTag::Date),
    ("jiff::civil", "Date", KnownNamedTag::Date),
    ("jiff::civil", "DateTime", KnownNamedTag::Date),
    ("bson", "DateTime", KnownNamedTag::Date),
    // Byte-like
    ("bytes", "Bytes", KnownNamedTag::Uint8Array),
    ("bytes", "BytesMut", KnownNamedTag::Uint8Array),
];

#[derive(Default)]
struct Analyzer;

impl Analyzer {
    fn analyze(
        &self,
        dt: &DataType,
        types: &Types,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
    ) -> PlanNode {
        match dt {
            DataType::Primitive(Primitive::i64)
            | DataType::Primitive(Primitive::u64)
            | DataType::Primitive(Primitive::i128)
            | DataType::Primitive(Primitive::u128) => PlanNode::Leaf(Tag::BigInt),
            DataType::Primitive(_) => PlanNode::Identity,
            DataType::List(list) => {
                let inner = self.analyze(list.ty(), types, generics, stack);
                if inner.is_identity() {
                    PlanNode::Identity
                } else {
                    PlanNode::List(Box::new(inner))
                }
            }
            DataType::Map(map) => {
                let inner = self.analyze(map.value_ty(), types, generics, stack);
                if inner.is_identity() {
                    PlanNode::Identity
                } else {
                    PlanNode::Map(Box::new(inner))
                }
            }
            DataType::Struct(st) => self.analyze_fields(st.fields(), types, generics, stack),
            DataType::Enum(en) => {
                let mut variants = Vec::new();

                for (name, variant) in en.variants().iter().filter(|(_, v)| !v.skip()) {
                    if let Some(variant) =
                        self.analyze_enum_variant(name, variant, types, generics, stack)
                        && !variant.plan.is_identity()
                    {
                        variants.push(variant);
                    }
                }

                if variants.is_empty() {
                    PlanNode::Identity
                } else {
                    PlanNode::Enum(variants)
                }
            }
            DataType::Tuple(tuple) => {
                let items = tuple
                    .elements()
                    .iter()
                    .map(|ty| self.analyze(ty, types, generics, stack))
                    .collect::<Vec<_>>();

                if items.iter().all(PlanNode::is_identity) {
                    PlanNode::Identity
                } else {
                    PlanNode::Tuple(items)
                }
            }
            DataType::Nullable(inner) => {
                let inner = self.analyze(inner, types, generics, stack);
                if inner.is_identity() {
                    PlanNode::Identity
                } else {
                    PlanNode::Nullable(Box::new(inner))
                }
            }
            DataType::Reference(Reference::Named(reference)) => {
                if let Some(ndt) = reference.get(types) {
                    if let Some(tag) = self.resolve_named_tag(ndt.module_path(), ndt.name()) {
                        return match tag {
                            KnownNamedTag::BigInt => PlanNode::Leaf(Tag::BigInt),
                            KnownNamedTag::Date => PlanNode::Leaf(Tag::Date),
                            KnownNamedTag::Uint8Array => PlanNode::Leaf(Tag::Uint8Array),
                        };
                    }

                    if stack.contains(reference) {
                        return PlanNode::Identity;
                    }

                    stack.push(reference.clone());
                    let out = self.analyze(ndt.ty(), types, reference.generics(), stack);
                    stack.pop();
                    out
                } else {
                    PlanNode::Identity
                }
            }
            DataType::Reference(Reference::Generic(generic)) => generics
                .iter()
                .find(|(key, _)| key == generic)
                .map(|(_, dt)| self.analyze(dt, types, &[], stack))
                .unwrap_or(PlanNode::Identity),
            DataType::Reference(Reference::Opaque(_)) => PlanNode::Identity,
        }
    }

    fn analyze_fields(
        &self,
        fields: &Fields,
        types: &Types,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
    ) -> PlanNode {
        match fields {
            Fields::Unit => PlanNode::Identity,
            Fields::Unnamed(fields) => {
                let items = fields
                    .fields()
                    .iter()
                    .filter_map(|field| field.ty())
                    .map(|ty| self.analyze(ty, types, generics, stack))
                    .collect::<Vec<_>>();

                if items.iter().all(PlanNode::is_identity) {
                    PlanNode::Identity
                } else {
                    PlanNode::Tuple(items)
                }
            }
            Fields::Named(fields) => {
                let object = fields
                    .fields()
                    .iter()
                    .filter_map(|(name, field)| {
                        let ty = field.ty()?;
                        let plan = self.analyze(ty, types, generics, stack);
                        if plan.is_identity() {
                            None
                        } else if field.flatten() {
                            Some(ObjectFieldPlan::Flattened(plan))
                        } else {
                            Some(ObjectFieldPlan::Named(name.to_string(), plan))
                        }
                    })
                    .collect::<Vec<_>>();

                if object.is_empty() {
                    PlanNode::Identity
                } else {
                    PlanNode::Object(object)
                }
            }
        }
    }

    fn resolve_named_tag(&self, module_path: &str, name: &str) -> Option<KnownNamedTag> {
        BUILTIN_MATCHERS
            .iter()
            .find(|(module, ty_name, _)| module_path == *module && name == *ty_name)
            .map(|(_, _, kind)| *kind)
    }

    fn analyze_enum_variant(
        &self,
        name: &str,
        variant: &specta::datatype::Variant,
        types: &Types,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
    ) -> Option<EnumVariantPlan> {
        let plan = self.analyze_fields(variant.fields(), types, generics, stack);
        let matcher = match variant.fields() {
            Fields::Unit | Fields::Unnamed(_) => EnumVariantMatcher::Direct,
            Fields::Named(fields) => {
                let literal_fields = fields
                    .fields()
                    .iter()
                    .filter_map(|(field_name, field)| {
                        string_literal(field.ty()?).map(|value| (field_name.to_string(), value))
                    })
                    .collect::<Vec<_>>();

                if literal_fields.len() == 1 {
                    let (field, value) = &literal_fields[0];
                    EnumVariantMatcher::Tagged {
                        field: field.clone(),
                        value: value.clone(),
                    }
                } else if fields.fields().len() == 1 {
                    let (field_name, _) = &fields.fields()[0];
                    EnumVariantMatcher::HasField(field_name.to_string())
                } else {
                    let _ = name;
                    EnumVariantMatcher::Direct
                }
            }
        };

        Some(EnumVariantPlan { matcher, plan })
    }
}

#[derive(Default)]
struct Renderer {
    ident_counter: usize,
}

impl Renderer {
    fn render(&mut self, plan: &PlanNode, input: &str) -> String {
        match plan {
            PlanNode::Identity => input.to_string(),
            PlanNode::Leaf(tag) => self.render_leaf(tag, input),
            PlanNode::Nullable(inner) => {
                let inner = self.render(inner, input);
                format!("({input} == null ? {input} : {inner})")
            }
            PlanNode::List(inner) => {
                let item = self.next_ident("item");
                let inner = self.render(inner, &item);
                format!("{input}.map(({item}) => {inner})")
            }
            PlanNode::Tuple(items) => {
                let rendered = items
                    .iter()
                    .enumerate()
                    .map(|(idx, plan)| {
                        let source = format!("{input}[{idx}]");
                        self.render(plan, &source)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("[{rendered}]")
            }
            PlanNode::Object(fields) => self.render_object(fields, input),
            PlanNode::Map(inner) => {
                let key = self.next_ident("key");
                let value = self.next_ident("value");
                let inner = self.render(inner, &value);

                format!(
                    "Object.fromEntries(Object.entries({input}).map(([{key}, {value}]) => [{key}, {inner}]))"
                )
            }
            PlanNode::Enum(variants) => self.render_enum(variants, input),
        }
    }

    fn render_object(&mut self, fields: &[ObjectFieldPlan], input: &str) -> String {
        let value = self.next_ident("value");
        let mut out = format!("(() => {{ let {value} = {input};");

        for field in fields {
            match field {
                ObjectFieldPlan::Named(field, plan) => {
                    let key = js_string(field);
                    let source = format!("{value}[{key}]");
                    let next = self.render(plan, &source);
                    out.push_str(&format!(
                        " {{ const next = {next}; if (next !== {source}) {value} = {{ ...{value}, {key}: next }}; }}"
                    ));
                }
                ObjectFieldPlan::Flattened(plan) => {
                    let next = self.render(plan, &value);
                    out.push_str(&format!(
                        " {{ const next = {next}; if (next !== {value}) {value} = next; }}"
                    ));
                }
            }
        }

        out.push_str(&format!(" return {value}; }})()"));
        out
    }

    fn render_leaf(&mut self, tag: &Tag, input: &str) -> String {
        match tag {
            Tag::BigInt => format!("BigInt({input})"),
            Tag::Uint8Array => format!("Uint8Array.from({input})"),
            Tag::Date => format!("new Date({input})"),
            Tag::Custom(handler) => handler(input).into_owned(),
        }
    }

    fn render_enum(&mut self, variants: &[EnumVariantPlan], input: &str) -> String {
        let value = self.next_ident("value");
        let mut out = format!("(() => {{ const {value} = {input};");

        if variants.is_empty() {
            out.push_str(&format!(" return {value}; }})()"));
            return out;
        }

        for variant in variants {
            if variant.plan.is_identity() {
                continue;
            }

            let next = self.render(&variant.plan, &value);

            match &variant.matcher {
                EnumVariantMatcher::HasField(field) => {
                    let field = js_string(field);
                    out.push_str(&format!(
                        " if ({value} != null && typeof {value} === \"object\" && !Array.isArray({value}) && Object.prototype.hasOwnProperty.call({value}, {field})) {{ const next = {next}; if (next !== {value}) return next; return {value}; }}"
                    ));
                }
                EnumVariantMatcher::Tagged { field, value: tag } => {
                    let field = js_string(field);
                    let tag = js_string(tag);
                    out.push_str(&format!(
                        " if ({value} != null && typeof {value} === \"object\" && !Array.isArray({value}) && {value}[{field}] === {tag}) {{ const next = {next}; if (next !== {value}) return next; return {value}; }}"
                    ));
                }
                EnumVariantMatcher::Direct => {
                    out.push_str(&format!(
                        " {{ const next = {next}; if (next !== {value}) return next; }}"
                    ));
                }
            }
        }

        out.push_str(&format!(" return {value}; }})()"));
        out
    }

    fn next_ident(&mut self, prefix: &str) -> String {
        self.ident_counter += 1;
        format!("__{prefix}{}", self.ident_counter)
    }
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).expect("failed to encode JavaScript string")
}

fn string_literal(ty: &DataType) -> Option<String> {
    let DataType::Enum(enm) = ty else {
        return None;
    };

    let [(name, variant)] = enm.variants() else {
        return None;
    };

    matches!(variant.fields(), Fields::Unit).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use specta::{ResolvedTypes, Type, Types};
    use specta_serde::apply;

    use super::TransformPlan;

    #[allow(dead_code)]
    #[derive(Type)]
    struct Inner {
        bigint: u128,
        date: chrono::DateTime<chrono::Utc>,
        bytes: bytes::Bytes,
    }

    #[allow(dead_code)]
    #[derive(Type)]
    struct Root {
        bigint: u128,
        date: chrono::DateTime<chrono::Utc>,
        bytes: bytes::Bytes,
        nested: Inner,
        list: Vec<u128>,
    }

    #[test]
    fn map_renders_trusting_transforms() {
        let mut types = Types::default();
        let dt = Root::definition(&mut types);
        let plan = TransformPlan::analyze(dt, &ResolvedTypes::from_resolved_types(types));
        let js = plan.map("v");

        assert!(
            js.contains("BigInt(") && js.contains("[\"bigint\"]") && js.contains("[\"nested\"]")
        );
        assert!(
            js.contains("new Date(") && js.contains("[\"date\"]") && js.contains("[\"nested\"]")
        );
        assert!(
            js.contains("Uint8Array.from(")
                && js.contains("[\"bytes\"]")
                && js.contains("[\"nested\"]")
        );
        assert!(js.contains("[\"list\"]") && js.contains(".map((__item"));

        assert!(!js.contains("typeof "));
        assert!(!js.contains("Array.isArray("));
        assert!(!js.contains("Number.isInteger("));
        assert!(!js.contains("try {"));
        assert!(!js.contains(" catch "));
    }

    #[derive(Type, Serialize, Deserialize)]
    #[serde(tag = "kind")]
    enum TaggedEnum {
        A { count: u128 },
    }

    #[derive(Type, Serialize, Deserialize)]
    #[serde(tag = "kind", content = "payload")]
    enum AdjacentEnum {
        A { count: u128 },
    }

    #[test]
    fn map_renders_from_serde_applied_internal_enum_shape() {
        let resolved = apply(Types::default().register::<TaggedEnum>()).unwrap();
        let dt = resolved
            .as_types()
            .into_sorted_iter()
            .find(|ty| ty.name().as_ref() == "TaggedEnum")
            .expect("TaggedEnum should be registered")
            .ty()
            .clone();

        let js = TransformPlan::analyze(dt, &resolved).map("v");

        assert!(js.contains("[\"kind\"] === \"A\""));
        assert!(js.contains("BigInt("));
        assert!(js.contains("[\"count\"]"));
    }

    #[test]
    fn map_renders_from_serde_applied_adjacent_enum_shape() {
        let resolved = apply(Types::default().register::<AdjacentEnum>()).unwrap();
        let dt = resolved
            .as_types()
            .into_sorted_iter()
            .find(|ty| ty.name().as_ref() == "AdjacentEnum")
            .expect("AdjacentEnum should be registered")
            .ty()
            .clone();

        let js = TransformPlan::analyze(dt, &resolved).map("v");

        assert!(js.contains("[\"kind\"] === \"A\""));
        assert!(js.contains("payload"));
        assert!(js.contains("BigInt("));
    }
}
