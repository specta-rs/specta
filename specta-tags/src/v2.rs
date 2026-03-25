use std::borrow::Cow;

use specta::{
    Types,
    datatype::{DataType, Fields, GenericReference, NamedReference, Primitive, Reference},
};
use specta_serde::internal::SerdeContainerAttrs;

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

/// TODO
#[derive(Debug)]
pub struct TransformPlan {
    plan: PlanNode,
}

impl TransformPlan {
    /// TODO
    pub fn analyze(dt: DataType, types: &Types) -> Self {
        // Scan all `DataType` references, etc. and collect tags and their object location for `Self::map` to use.
        //
        // You should match on `NamedDataType`'s name and module path to determine known named types.

        Self {
            plan: Analyzer::default().analyze(&dt, types, &[], &mut Vec::new()),
        }
    }

    /// TODO
    ///
    /// This should produce something like
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
    Object(Vec<(String, PlanNode)>),
    Map(Box<PlanNode>),
    Enum {
        repr: EnumRepr,
        variants: Vec<EnumVariantPlan>,
    },
}

impl PlanNode {
    fn is_identity(&self) -> bool {
        match self {
            Self::Identity => true,
            Self::Leaf(_) => false,
            Self::Nullable(inner) | Self::List(inner) | Self::Map(inner) => inner.is_identity(),
            Self::Tuple(items) => items.iter().all(Self::is_identity),
            Self::Object(fields) => fields.iter().all(|(_, v)| v.is_identity()),
            Self::Enum { variants, .. } => variants.iter().all(|v| v.plan.is_identity()),
        }
    }
}

#[derive(Debug)]
struct EnumVariantPlan {
    name: String,
    kind: EnumVariantKind,
    plan: PlanNode,
}

#[derive(Debug, Clone, Copy)]
enum EnumVariantKind {
    Unit,
    Named,
    Unnamed,
}

#[derive(Debug, Clone)]
enum EnumRepr {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

impl Default for EnumRepr {
    fn default() -> Self {
        Self::External
    }
}

#[derive(Clone, Copy)]
enum KnownNamedTag {
    Date,
    Uint8Array,
}

const BUILTIN_MATCHERS: &[(&str, &str, KnownNamedTag)] = &[
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
                    let (kind, plan) = match variant.fields() {
                        Fields::Unit => (EnumVariantKind::Unit, PlanNode::Identity),
                        Fields::Unnamed(fields) => {
                            let mut items = fields
                                .fields()
                                .iter()
                                .filter_map(|field| field.ty())
                                .map(|ty| self.analyze(ty, types, generics, stack))
                                .collect::<Vec<_>>();

                            let plan = if items.is_empty() {
                                PlanNode::Identity
                            } else if items.len() == 1 {
                                items.remove(0)
                            } else {
                                PlanNode::Tuple(items)
                            };

                            (EnumVariantKind::Unnamed, plan)
                        }
                        Fields::Named(fields) => {
                            let object = fields
                                .fields()
                                .iter()
                                .filter_map(|(field_name, field)| {
                                    let ty = field.ty()?;
                                    let plan = self.analyze(ty, types, generics, stack);
                                    (!plan.is_identity()).then(|| (field_name.to_string(), plan))
                                })
                                .collect::<Vec<_>>();

                            let plan = if object.is_empty() {
                                PlanNode::Identity
                            } else {
                                PlanNode::Object(object)
                            };

                            (EnumVariantKind::Named, plan)
                        }
                    };

                    if !plan.is_identity() {
                        variants.push(EnumVariantPlan {
                            name: name.to_string(),
                            kind,
                            plan,
                        });
                    }
                }

                if variants.is_empty() {
                    PlanNode::Identity
                } else {
                    PlanNode::Enum {
                        repr: parse_enum_repr(en.attributes()),
                        variants,
                    }
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
                        (!plan.is_identity()).then(|| (name.to_string(), plan))
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
            PlanNode::Object(fields) => {
                let updates = fields
                    .iter()
                    .map(|(field, plan)| {
                        let key = js_string(field);
                        let source = format!("{input}[{key}]");
                        let value = self.render(plan, &source);
                        format!("{key}: {value}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{{ ...{input}, {updates} }}")
            }
            PlanNode::Map(inner) => {
                let key = self.next_ident("key");
                let value = self.next_ident("value");
                let inner = self.render(inner, &value);

                format!(
                    "Object.fromEntries(Object.entries({input}).map(([{key}, {value}]) => [{key}, {inner}]))"
                )
            }
            PlanNode::Enum { repr, variants } => self.render_enum(repr, variants, input),
        }
    }

    fn render_leaf(&mut self, tag: &Tag, input: &str) -> String {
        match tag {
            Tag::BigInt => format!("BigInt({input})"),
            Tag::Uint8Array => format!("Uint8Array.from({input})"),
            Tag::Date => format!("new Date({input})"),
            Tag::Custom(handler) => handler(input).into_owned(),
        }
    }

    fn render_enum(
        &mut self,
        repr: &EnumRepr,
        variants: &[EnumVariantPlan],
        input: &str,
    ) -> String {
        let value = self.next_ident("value");
        let mut out = format!("(() => {{ const {value} = {input};");

        if variants.is_empty() {
            out.push_str(&format!(" return {value}; }})()"));
            return out;
        }

        match repr {
            EnumRepr::External => {
                out.push_str(&format!(
                    " if ({value} != null && typeof {value} === \"object\" && !Array.isArray({value})) {{"
                ));

                for variant in variants {
                    if matches!(variant.kind, EnumVariantKind::Unit) || variant.plan.is_identity() {
                        continue;
                    }

                    let name = js_string(&variant.name);
                    let payload = format!("{value}[{name}]");
                    let next = self.render(&variant.plan, &payload);

                    out.push_str(&format!(
                        " if (Object.prototype.hasOwnProperty.call({value}, {name})) {{ const next = {next}; if (next !== {payload}) return {{ ...{value}, [{name}]: next }}; return {value}; }}"
                    ));
                }

                out.push_str(" }");
            }
            EnumRepr::Internal { tag } => {
                let tag_key = js_string(tag);
                out.push_str(&format!(
                    " if ({value} != null && typeof {value} === \"object\" && !Array.isArray({value})) {{"
                ));

                for variant in variants {
                    if matches!(variant.kind, EnumVariantKind::Unit) || variant.plan.is_identity() {
                        continue;
                    }

                    let name = js_string(&variant.name);
                    out.push_str(&format!(" if ({value}[{tag_key}] === {name}) {{"));

                    match variant.kind {
                        EnumVariantKind::Named => {
                            let next = self.render(&variant.plan, &value);
                            out.push_str(&format!(
                                " const next = {next}; if (next !== {value}) return next; return {value};"
                            ));
                        }
                        EnumVariantKind::Unnamed => {
                            let data_key = js_string("data");
                            let payload = format!("{value}[{data_key}]");
                            let next_payload = self.render(&variant.plan, &payload);
                            let next_direct = self.render(&variant.plan, &value);
                            out.push_str(&format!(
                                " if (Object.prototype.hasOwnProperty.call({value}, {data_key})) {{ const next = {next_payload}; if (next !== {payload}) return {{ ...{value}, [{data_key}]: next }}; return {value}; }} const direct = {next_direct}; if (direct !== {value}) return direct; return {value};"
                            ));
                        }
                        EnumVariantKind::Unit => {}
                    }

                    out.push_str(" }");
                }

                out.push_str(" }");
            }
            EnumRepr::Adjacent { tag, content } => {
                let tag_key = js_string(tag);
                let content_key = js_string(content);
                out.push_str(&format!(
                    " if ({value} != null && typeof {value} === \"object\" && !Array.isArray({value})) {{"
                ));

                for variant in variants {
                    if matches!(variant.kind, EnumVariantKind::Unit) || variant.plan.is_identity() {
                        continue;
                    }

                    let name = js_string(&variant.name);
                    let payload = format!("{value}[{content_key}]");
                    let next = self.render(&variant.plan, &payload);
                    out.push_str(&format!(
                        " if ({value}[{tag_key}] === {name} && Object.prototype.hasOwnProperty.call({value}, {content_key})) {{ const next = {next}; if (next !== {payload}) return {{ ...{value}, [{content_key}]: next }}; return {value}; }}"
                    ));
                }

                out.push_str(" }");
            }
            EnumRepr::Untagged => {}
        }

        for variant in variants {
            if variant.plan.is_identity() {
                continue;
            }

            let direct = self.render(&variant.plan, &value);
            out.push_str(&format!(
                " {{ const direct = {direct}; if (direct !== {value}) return direct; }}"
            ));
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

fn parse_enum_repr(attributes: &specta::datatype::Attributes) -> EnumRepr {
    let Some(attrs) = attributes.get::<SerdeContainerAttrs>() else {
        return EnumRepr::External;
    };

    if attrs.untagged {
        EnumRepr::Untagged
    } else {
        match (&attrs.tag, &attrs.content) {
            (Some(tag), Some(content)) => EnumRepr::Adjacent {
                tag: tag.clone(),
                content: content.clone(),
            },
            (Some(tag), None) => EnumRepr::Internal { tag: tag.clone() },
            _ => EnumRepr::External,
        }
    }
}

#[cfg(test)]
mod tests {
    use specta::{Type, Types};

    use super::TransformPlan;

    #[derive(Type)]
    struct Inner {
        bigint: u128,
        date: chrono::DateTime<chrono::Utc>,
        bytes: bytes::Bytes,
    }

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
        let plan = TransformPlan::analyze(dt, &types);
        let js = plan.map("v");

        assert!(
            js.contains("BigInt(v[\"bigint\"])")
                && js.contains("BigInt(v[\"nested\"][\"bigint\"])")
        );
        assert!(
            js.contains("new Date(v[\"date\"])")
                && js.contains("new Date(v[\"nested\"][\"date\"])")
        );
        assert!(
            js.contains("Uint8Array.from(v[\"bytes\"])")
                && js.contains("Uint8Array.from(v[\"nested\"][\"bytes\"])")
        );
        assert!(js.contains("v[\"list\"].map((__item"));

        assert!(!js.contains("typeof "));
        assert!(!js.contains("Array.isArray("));
        assert!(!js.contains("Number.isInteger("));
        assert!(!js.contains("try {"));
        assert!(!js.contains(" catch "));
    }
}
