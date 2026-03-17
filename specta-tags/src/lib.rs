//! Shared semantic mapping analysis and JS/TS runtime generation.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use std::borrow::Cow;

use serde::Serialize;
use specta::{
    datatype::{
        skip_fields, skip_fields_named, DataType, Fields, GenericReference, NamedReference,
        Primitive, Reference,
    },
    TypeCollection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTarget {
    TypeScript,
    JavaScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Date,
    Bytes,
}

#[derive(Debug, Clone, Default)]
pub struct Analyzer {
    matchers: Vec<Matcher>,
    list_u8_is_bytes: bool,
}

#[derive(Debug, Clone)]
struct Matcher {
    module_path: Cow<'static, str>,
    name: Cow<'static, str>,
    kind: SemanticKind,
}

impl Analyzer {
    pub fn with_builtins() -> Self {
        let mut this = Self::default();
        for (module_path, name, kind) in BUILTIN_MATCHERS {
            this = this.with_named_type(*module_path, *name, *kind);
        }
        this
    }

    pub fn with_named_type(
        mut self,
        module_path: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        kind: SemanticKind,
    ) -> Self {
        self.matchers.push(Matcher {
            module_path: module_path.into(),
            name: name.into(),
            kind,
        });
        self
    }

    pub fn with_list_u8_is_bytes(mut self, enabled: bool) -> Self {
        self.list_u8_is_bytes = enabled;
        self
    }

    pub fn analyze(
        &self,
        dt: &DataType,
        types: &TypeCollection,
        generics: &[(GenericReference, DataType)],
    ) -> TransformSpec {
        self.analyze_inner(dt, types, generics, &mut Vec::new())
    }

    fn analyze_inner(
        &self,
        dt: &DataType,
        types: &TypeCollection,
        generics: &[(GenericReference, DataType)],
        stack: &mut Vec<NamedReference>,
    ) -> TransformSpec {
        match dt {
            DataType::Primitive(Primitive::i64)
            | DataType::Primitive(Primitive::u64)
            | DataType::Primitive(Primitive::i128)
            | DataType::Primitive(Primitive::u128) => TransformSpec::BigInt,
            DataType::Primitive(_) => TransformSpec::Identity,
            DataType::List(list) => {
                if self.list_u8_is_bytes && matches!(list.ty(), DataType::Primitive(Primitive::u8))
                {
                    TransformSpec::Bytes
                } else {
                    TransformSpec::List(Box::new(self.analyze_inner(
                        list.ty(),
                        types,
                        generics,
                        stack,
                    )))
                }
            }
            DataType::Map(map) => TransformSpec::Map(Box::new(self.analyze_inner(
                map.value_ty(),
                types,
                generics,
                stack,
            ))),
            DataType::Struct(st) => match st.fields() {
                Fields::Unit => TransformSpec::Identity,
                Fields::Unnamed(fields) => TransformSpec::Tuple(
                    skip_fields(fields.fields())
                        .map(|(_, ty)| self.analyze_inner(ty, types, generics, stack))
                        .collect(),
                ),
                Fields::Named(fields) => TransformSpec::Object(
                    skip_fields_named(fields.fields())
                        .map(|(name, (_, ty))| {
                            (
                                name.to_string(),
                                self.analyze_inner(ty, types, generics, stack),
                            )
                        })
                        .collect(),
                ),
            },
            DataType::Enum(e) => TransformSpec::Enum(
                e.variants()
                    .iter()
                    .filter(|(_, variant)| !variant.skip())
                    .map(|(name, variant)| {
                        let (kind, spec) = match variant.fields() {
                            Fields::Unit => {
                                (EnumVariantTransformKind::Unit, TransformSpec::Identity)
                            }
                            Fields::Unnamed(fields) => {
                                let fields = skip_fields(fields.fields())
                                    .map(|(_, ty)| self.analyze_inner(ty, types, generics, stack))
                                    .collect::<Vec<_>>();
                                let spec = if fields.len() == 1 {
                                    fields.into_iter().next().unwrap_or_default()
                                } else {
                                    TransformSpec::Tuple(fields)
                                };
                                (EnumVariantTransformKind::Unnamed, spec)
                            }
                            Fields::Named(fields) => {
                                let spec = TransformSpec::Object(
                                    skip_fields_named(fields.fields())
                                        .map(|(name, (_, ty))| {
                                            (
                                                name.to_string(),
                                                self.analyze_inner(ty, types, generics, stack),
                                            )
                                        })
                                        .collect(),
                                );
                                (EnumVariantTransformKind::Named, spec)
                            }
                        };

                        EnumVariantTransformSpec {
                            name: name.to_string(),
                            kind,
                            spec,
                        }
                    })
                    .collect(),
            ),
            DataType::Tuple(tuple) => TransformSpec::Tuple(
                tuple
                    .elements()
                    .iter()
                    .map(|ty| self.analyze_inner(ty, types, generics, stack))
                    .collect(),
            ),
            DataType::Nullable(inner) => {
                TransformSpec::Nullable(Box::new(self.analyze_inner(inner, types, generics, stack)))
            }
            DataType::Reference(Reference::Named(r)) => {
                if let Some(ndt) = r.get(types) {
                    if let Some(kind) = self.resolve_named_kind(ndt.module_path(), ndt.name()) {
                        return kind.into();
                    }

                    if stack.contains(r) {
                        return TransformSpec::Identity;
                    }

                    stack.push(r.clone());
                    let spec = self.analyze_inner(ndt.ty(), types, r.generics(), stack);
                    stack.pop();
                    spec
                } else {
                    TransformSpec::Identity
                }
            }
            DataType::Reference(Reference::Generic(generic)) => generics
                .iter()
                .find(|(key, _)| key == generic)
                .map(|(_, dt)| self.analyze_inner(dt, types, &[], stack))
                .unwrap_or_default(),
            DataType::Reference(Reference::Opaque(_)) => TransformSpec::Identity,
        }
    }

    fn resolve_named_kind(&self, module_path: &str, name: &str) -> Option<SemanticKind> {
        self.matchers
            .iter()
            .find(|m| m.module_path == module_path && m.name == name)
            .map(|m| m.kind)
    }
}

const BUILTIN_MATCHERS: &[(&str, &str, SemanticKind)] = &[
    // Date-like
    ("std::time", "SystemTime", SemanticKind::Date),
    ("toml::value", "Datetime", SemanticKind::Date),
    ("chrono", "NaiveDateTime", SemanticKind::Date),
    ("chrono", "NaiveDate", SemanticKind::Date),
    ("chrono", "Date", SemanticKind::Date),
    ("chrono", "DateTime", SemanticKind::Date),
    ("time", "PrimitiveDateTime", SemanticKind::Date),
    ("time", "OffsetDateTime", SemanticKind::Date),
    ("time", "Date", SemanticKind::Date),
    ("jiff", "Timestamp", SemanticKind::Date),
    ("jiff", "Zoned", SemanticKind::Date),
    ("jiff::civil", "Date", SemanticKind::Date),
    ("jiff::civil", "DateTime", SemanticKind::Date),
    ("bson", "DateTime", SemanticKind::Date),
    // Byte-like
    ("bytes", "Bytes", SemanticKind::Bytes),
    ("bytes", "BytesMut", SemanticKind::Bytes),
];

#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum TransformSpec {
    #[default]
    Identity,
    BigInt,
    Date,
    Bytes,
    Nullable(Box<TransformSpec>),
    List(Box<TransformSpec>),
    Tuple(Vec<TransformSpec>),
    Object(Vec<(String, TransformSpec)>),
    Map(Box<TransformSpec>),
    Enum(Vec<EnumVariantTransformSpec>),
}

impl TransformSpec {
    pub fn is_identity(&self) -> bool {
        match self {
            Self::Identity => true,
            Self::Nullable(inner) | Self::List(inner) | Self::Map(inner) => inner.is_identity(),
            Self::Tuple(items) => items.iter().all(Self::is_identity),
            Self::Object(fields) => fields.iter().all(|(_, spec)| spec.is_identity()),
            Self::Enum(variants) => variants.iter().all(|variant| variant.spec.is_identity()),
            Self::BigInt | Self::Date | Self::Bytes => false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize transform spec")
    }

    fn collect_requirements(&self, req: &mut RuntimeRequirements) {
        req.needs_transform = true;
        match self {
            Self::Identity => {}
            Self::BigInt => req.needs_bigint = true,
            Self::Date => req.needs_date = true,
            Self::Bytes => req.needs_bytes = true,
            Self::Nullable(inner) => {
                req.needs_nullable = true;
                inner.collect_requirements(req);
            }
            Self::List(inner) => {
                req.needs_list = true;
                inner.collect_requirements(req);
            }
            Self::Tuple(items) => {
                req.needs_tuple = true;
                items.iter().for_each(|item| item.collect_requirements(req));
            }
            Self::Object(fields) => {
                req.needs_object = true;
                fields
                    .iter()
                    .for_each(|(_, item)| item.collect_requirements(req));
            }
            Self::Map(inner) => {
                req.needs_map = true;
                inner.collect_requirements(req);
            }
            Self::Enum(variants) => {
                req.needs_enum = true;
                variants
                    .iter()
                    .for_each(|variant| variant.spec.collect_requirements(req));
            }
        }
    }
}

impl From<SemanticKind> for TransformSpec {
    fn from(value: SemanticKind) -> Self {
        match value {
            SemanticKind::Date => Self::Date,
            SemanticKind::Bytes => Self::Bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumVariantTransformSpec {
    pub name: String,
    pub kind: EnumVariantTransformKind,
    pub spec: TransformSpec,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnumVariantTransformKind {
    Unit,
    Named,
    Unnamed,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeRequirements {
    pub needs_transform: bool,
    pub needs_result_helper: bool,
    needs_bigint: bool,
    needs_date: bool,
    needs_bytes: bool,
    needs_nullable: bool,
    needs_list: bool,
    needs_tuple: bool,
    needs_object: bool,
    needs_map: bool,
    needs_enum: bool,
}

impl RuntimeRequirements {
    pub fn from_specs<'a>(specs: impl IntoIterator<Item = &'a TransformSpec>) -> Self {
        let mut req = Self::default();
        for spec in specs {
            spec.collect_requirements(&mut req);
        }
        req
    }

    pub fn with_result_helper(mut self, enabled: bool) -> Self {
        self.needs_result_helper = enabled;
        self
    }

    pub fn is_empty(&self) -> bool {
        !self.needs_transform && !self.needs_result_helper
    }
}

pub const RUNTIME_RESERVED_NAMES: &[&str] = &[
    "__TS_transform",
    "__TS_transformEnum",
    "__TS_transformResult",
];

pub fn render_runtime(target: RuntimeTarget, req: &RuntimeRequirements) -> Cow<'static, str> {
    if req.is_empty() {
        return Cow::Borrowed("");
    }

    let mut out = String::new();

    if target == RuntimeTarget::TypeScript {
        out.push_str(TRANSFORM_SPEC_TS);
        out.push('\n');
    }

    if req.needs_transform {
        out.push_str(match target {
            RuntimeTarget::TypeScript => TRANSFORM_FN_PREFIX_TS,
            RuntimeTarget::JavaScript => TRANSFORM_FN_PREFIX_JS,
        });

        if req.needs_bigint {
            out.push_str(CASE_BIGINT);
        }
        if req.needs_date {
            out.push_str(CASE_DATE);
        }
        if req.needs_bytes {
            out.push_str(CASE_BYTES);
        }
        if req.needs_nullable {
            out.push_str(CASE_NULLABLE);
        }
        if req.needs_list {
            out.push_str(CASE_LIST);
        }
        if req.needs_tuple {
            out.push_str(CASE_TUPLE);
        }
        if req.needs_object {
            out.push_str(CASE_OBJECT);
        }
        if req.needs_map {
            out.push_str(CASE_MAP);
        }
        if req.needs_enum {
            out.push_str(CASE_ENUM);
        }

        out.push_str(TRANSFORM_FN_SUFFIX);

        if req.needs_enum {
            out.push_str(match target {
                RuntimeTarget::TypeScript => ENUM_HELPERS_TS,
                RuntimeTarget::JavaScript => ENUM_HELPERS_JS,
            });
        }
    }

    if req.needs_result_helper {
        out.push_str(match target {
            RuntimeTarget::TypeScript => TRANSFORM_RESULT_TS,
            RuntimeTarget::JavaScript => TRANSFORM_RESULT_JS,
        });
    }

    Cow::Owned(out)
}

const TRANSFORM_SPEC_TS: &str = r#"type __TS_TransformSpec =
    | { t: "identity" }
    | { t: "big_int" }
    | { t: "date" }
    | { t: "bytes" }
    | { t: "nullable"; v: __TS_TransformSpec }
    | { t: "list"; v: __TS_TransformSpec }
    | { t: "tuple"; v: __TS_TransformSpec[] }
    | { t: "object"; v: [string, __TS_TransformSpec][] }
    | { t: "map"; v: __TS_TransformSpec }
    | { t: "enum"; v: __TS_EnumVariantTransformSpec[] };

type __TS_EnumVariantTransformSpec = {
    name: string;
    kind: "unit" | "named" | "unnamed";
    spec: __TS_TransformSpec;
};
"#;

const TRANSFORM_FN_PREFIX_TS: &str = "function __TS_transform<T>(value: T, spec: __TS_TransformSpec): T {\n    if (!spec || spec.t === \"identity\") return value;\n\n    const rawValue = value as any;\n\n    switch (spec.t) {\n";
const TRANSFORM_FN_PREFIX_JS: &str = "function __TS_transform(value, spec) {\n    if (!spec || spec.t === \"identity\") return value;\n\n    switch (spec.t) {\n";

const CASE_BIGINT: &str = r#"        case "big_int":
            if (typeof rawValue === "bigint") return value;
            if (typeof rawValue === "number" && Number.isInteger(rawValue)) return BigInt(rawValue);
            if (typeof rawValue === "string") {
                try {
                    return BigInt(rawValue);
                } catch {
                    return value;
                }
            }
            return value;
"#;

const CASE_DATE: &str = "        case \"date\":\n            return (typeof rawValue === \"string\" ? new Date(rawValue) : value);\n";
const CASE_BYTES: &str = "        case \"bytes\":\n            return (Array.isArray(rawValue) && rawValue.every((v) => typeof v === \"number\") ? Uint8Array.from(rawValue) : value);\n";
const CASE_NULLABLE: &str = "        case \"nullable\":\n            return rawValue == null ? value : __TS_transform(value, spec.v);\n";
const CASE_LIST: &str = "        case \"list\":\n            return (Array.isArray(rawValue) ? rawValue.map((item) => __TS_transform(item, spec.v)) : value);\n";
const CASE_TUPLE: &str = "        case \"tuple\":\n            return (Array.isArray(rawValue)\n                ? rawValue.map((item, index) => __TS_transform(item, spec.v[index] || { t: \"identity\" }))\n                : value);\n";
const CASE_OBJECT: &str = r#"        case "object": {
            if (rawValue == null || typeof rawValue !== "object" || Array.isArray(rawValue)) return value;

            let out = rawValue;
            for (const [key, nested] of spec.v) {
                if (!Object.prototype.hasOwnProperty.call(rawValue, key)) continue;
                const next = __TS_transform(rawValue[key], nested);
                if (next !== rawValue[key]) {
                    if (out === rawValue) out = { ...rawValue };
                    out[key] = next;
                }
            }
            return out;
        }
"#;
const CASE_MAP: &str = r#"        case "map": {
            if (rawValue == null || typeof rawValue !== "object" || Array.isArray(rawValue)) return value;

            let out = rawValue;
            for (const key of Object.keys(rawValue)) {
                const next = __TS_transform(rawValue[key], spec.v);
                if (next !== rawValue[key]) {
                    if (out === rawValue) out = { ...rawValue };
                    out[key] = next;
                }
            }
            return out;
        }
"#;
const CASE_ENUM: &str =
    "        case \"enum\":\n            return __TS_transformEnum(value, spec.v);\n";

const TRANSFORM_FN_SUFFIX: &str = "        default:\n            return value;\n    }\n}\n\n";

const ENUM_HELPERS_TS: &str = r#"function __TS_transformEnum<T>(value: T, variants: __TS_EnumVariantTransformSpec[]): T {
    for (const variant of variants) {
        const transformed = __TS_transformEnumVariant(value, variant);
        if (transformed !== undefined) return transformed;
    }
    return value;
}

function __TS_transformEnumVariant<T>(value: T, variant: __TS_EnumVariantTransformSpec): T | undefined {
    const rawValue = value as any;

    if (variant.kind === "unit") return undefined;

    if (rawValue != null && typeof rawValue === "object" && !Array.isArray(rawValue)) {
        if (Object.prototype.hasOwnProperty.call(rawValue, variant.name)) {
            const next = __TS_transform(rawValue[variant.name], variant.spec);
            if (next === rawValue[variant.name]) return value;
            return { ...rawValue, [variant.name]: next } as T;
        }

        if (rawValue.type === variant.name && Object.prototype.hasOwnProperty.call(rawValue, "data")) {
            const next = __TS_transform(rawValue.data, variant.spec);
            if (next === rawValue.data) return value;
            return { ...rawValue, data: next } as T;
        }

        if (rawValue.tag === variant.name && Object.prototype.hasOwnProperty.call(rawValue, "content")) {
            const next = __TS_transform(rawValue.content, variant.spec);
            if (next === rawValue.content) return value;
            return { ...rawValue, content: next } as T;
        }
    }

    const direct = __TS_transform(value, variant.spec);
    if (direct !== value) return direct;

    return undefined;
}

"#;

const ENUM_HELPERS_JS: &str = r#"function __TS_transformEnum(value, variants) {
    for (const variant of variants) {
        const transformed = __TS_transformEnumVariant(value, variant);
        if (transformed !== undefined) return transformed;
    }
    return value;
}

function __TS_transformEnumVariant(value, variant) {
    if (variant.kind === "unit") return undefined;

    if (value != null && typeof value === "object" && !Array.isArray(value)) {
        if (Object.prototype.hasOwnProperty.call(value, variant.name)) {
            const next = __TS_transform(value[variant.name], variant.spec);
            if (next === value[variant.name]) return value;
            return { ...value, [variant.name]: next };
        }

        if (value.type === variant.name && Object.prototype.hasOwnProperty.call(value, "data")) {
            const next = __TS_transform(value.data, variant.spec);
            if (next === value.data) return value;
            return { ...value, data: next };
        }

        if (value.tag === variant.name && Object.prototype.hasOwnProperty.call(value, "content")) {
            const next = __TS_transform(value.content, variant.spec);
            if (next === value.content) return value;
            return { ...value, content: next };
        }
    }

    const direct = __TS_transform(value, variant.spec);
    if (direct !== value) return direct;

    return undefined;
}

"#;

const TRANSFORM_RESULT_TS: &str = r#"function __TS_transformResult<T, E>(
    result: Promise<{ status: "ok"; data: T } | { status: "error"; error: E }>,
    okSpec: __TS_TransformSpec,
    errSpec: __TS_TransformSpec,
): Promise<{ status: "ok"; data: T } | { status: "error"; error: E }> {
    return result.then((value) => {
        if (value?.status === "ok") {
            return { status: "ok", data: __TS_transform(value.data, okSpec) };
        }

        if (value?.status === "error") {
            return { status: "error", error: __TS_transform(value.error, errSpec) };
        }

        return value;
    });
}
"#;

const TRANSFORM_RESULT_JS: &str = r#"function __TS_transformResult(result, okSpec, errSpec) {
    return result.then((value) => {
        if (value?.status === "ok") {
            return { status: "ok", data: __TS_transform(value.data, okSpec) };
        }

        if (value?.status === "error") {
            return { status: "error", error: __TS_transform(value.error, errSpec) };
        }

        return value;
    });
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_primitive_bigint() {
        let analyzer = Analyzer::with_builtins();
        let spec = analyzer.analyze(
            &DataType::Primitive(Primitive::u128),
            &TypeCollection::default(),
            &[],
        );

        assert!(matches!(spec, TransformSpec::BigInt));
    }

    #[test]
    fn list_u8_detection_is_opt_in() {
        let dt = DataType::List(specta::datatype::List::new(DataType::Primitive(
            Primitive::u8,
        )));

        let default_spec = Analyzer::with_builtins().analyze(&dt, &TypeCollection::default(), &[]);
        assert!(!matches!(default_spec, TransformSpec::Bytes));

        let bytes_spec = Analyzer::with_builtins()
            .with_list_u8_is_bytes(true)
            .analyze(&dt, &TypeCollection::default(), &[]);
        assert!(matches!(bytes_spec, TransformSpec::Bytes));
    }

    #[test]
    fn runtime_is_minimal_for_single_bigint() {
        let req = RuntimeRequirements::from_specs([&TransformSpec::BigInt]);
        let runtime = render_runtime(RuntimeTarget::JavaScript, &req);

        assert!(runtime.contains("case \"big_int\""));
        assert!(!runtime.contains("case \"date\""));
        assert!(!runtime.contains("case \"enum\""));
        assert!(!runtime.contains("function __TS_transformEnum"));
    }
}
