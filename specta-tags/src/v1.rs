use std::borrow::Cow;

use serde::{Serialize, Serializer, ser::SerializeSeq};
use specta::{
    Types,
    datatype::{DataType, Fields, GenericReference, NamedReference, Primitive, Reference},
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
        types: &Types,
        generics: &[(GenericReference, DataType)],
    ) -> TransformSpec {
        self.analyze_inner(dt, types, generics, &mut Vec::new())
    }

    fn analyze_inner(
        &self,
        dt: &DataType,
        types: &Types,
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
                    fields
                        .fields()
                        .iter()
                        .filter_map(|field| field.ty())
                        .map(|ty| self.analyze_inner(ty, types, generics, stack))
                        .collect(),
                ),
                Fields::Named(fields) => TransformSpec::Object(
                    fields
                        .fields()
                        .iter()
                        .filter_map(|(name, field)| field.ty().map(|ty| (name, ty)))
                        .map(|(name, ty)| {
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
                                let fields = fields
                                    .fields()
                                    .iter()
                                    .filter_map(|field| field.ty())
                                    .map(|ty| self.analyze_inner(ty, types, generics, stack))
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
                                    fields
                                        .fields()
                                        .iter()
                                        .filter_map(|(name, field)| field.ty().map(|ty| (name, ty)))
                                        .map(|(name, ty)| {
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

#[derive(Debug, Clone, Default)]
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

impl Serialize for TransformSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Identity => serializer.serialize_u8(0),
            Self::BigInt => serializer.serialize_u8(1),
            Self::Date => serializer.serialize_u8(2),
            Self::Bytes => serializer.serialize_u8(3),
            Self::Nullable(inner) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&4u8)?;
                seq.serialize_element(inner)?;
                seq.end()
            }
            Self::List(inner) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&5u8)?;
                seq.serialize_element(inner)?;
                seq.end()
            }
            Self::Tuple(items) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&6u8)?;
                seq.serialize_element(items)?;
                seq.end()
            }
            Self::Object(fields) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&7u8)?;
                seq.serialize_element(fields)?;
                seq.end()
            }
            Self::Map(inner) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&8u8)?;
                seq.serialize_element(inner)?;
                seq.end()
            }
            Self::Enum(variants) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&9u8)?;
                seq.serialize_element(variants)?;
                seq.end()
            }
        }
    }
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

#[derive(Debug, Clone)]
pub struct EnumVariantTransformSpec {
    pub name: String,
    pub kind: EnumVariantTransformKind,
    pub spec: TransformSpec,
}

impl Serialize for EnumVariantTransformSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.name)?;
        seq.serialize_element(&self.kind.code())?;
        seq.serialize_element(&self.spec)?;
        seq.end()
    }
}

#[derive(Debug, Clone)]
pub enum EnumVariantTransformKind {
    Unit,
    Named,
    Unnamed,
}

impl EnumVariantTransformKind {
    const fn code(&self) -> u8 {
        match self {
            Self::Unit => 0,
            Self::Named => 1,
            Self::Unnamed => 2,
        }
    }
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
    | 0
    | 1
    | 2
    | 3
    | [4, __TS_TransformSpec]
    | [5, __TS_TransformSpec]
    | [6, __TS_TransformSpec[]]
    | [7, [string, __TS_TransformSpec][]]
    | [8, __TS_TransformSpec]
    | [9, __TS_EnumVariantTransformSpec[]];

type __TS_EnumVariantTransformSpec = [
    name: string,
    kind: 0 | 1 | 2,
    spec: __TS_TransformSpec,
];
"#;

const TRANSFORM_FN_PREFIX_TS: &str = "function __TS_transform<T>(value: T, spec: __TS_TransformSpec): T {\n    if (spec == null || spec === 0) return value;\n\n    const rawValue = value as any;\n    const op = Array.isArray(spec) ? spec[0] : spec;\n\n    switch (op) {\n";
const TRANSFORM_FN_PREFIX_JS: &str = "function __TS_transform(value, spec) {\n    if (spec == null || spec === 0) return value;\n\n    const rawValue = value;\n    const op = Array.isArray(spec) ? spec[0] : spec;\n\n    switch (op) {\n";

const CASE_BIGINT: &str = r#"        case 1:
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

const CASE_DATE: &str = "        case 2:\n            return (typeof rawValue === \"string\" ? new Date(rawValue) : value);\n";
const CASE_BYTES: &str = "        case 3:\n            return (Array.isArray(rawValue) && rawValue.every((v) => typeof v === \"number\") ? Uint8Array.from(rawValue) : value);\n";
const CASE_NULLABLE: &str = "        case 4:\n            return (Array.isArray(spec) ? (rawValue == null ? value : __TS_transform(value, spec[1])) : value);\n";
const CASE_LIST: &str = "        case 5:\n            return (Array.isArray(spec) && Array.isArray(rawValue) ? rawValue.map((item) => __TS_transform(item, spec[1])) : value);\n";
const CASE_TUPLE: &str = "        case 6:\n            return (Array.isArray(spec) && Array.isArray(spec[1]) && Array.isArray(rawValue)\n                ? rawValue.map((item, index) => __TS_transform(item, spec[1][index] ?? 0))\n                : value);\n";
const CASE_OBJECT: &str = r#"        case 7: {
            if (!Array.isArray(spec) || !Array.isArray(spec[1])) return value;
            if (rawValue == null || typeof rawValue !== "object" || Array.isArray(rawValue)) return value;

            let out = rawValue;
            for (const [key, nested] of spec[1]) {
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
const CASE_MAP: &str = r#"        case 8: {
            if (!Array.isArray(spec)) return value;
            if (rawValue == null || typeof rawValue !== "object" || Array.isArray(rawValue)) return value;

            let out = rawValue;
            for (const key of Object.keys(rawValue)) {
                const next = __TS_transform(rawValue[key], spec[1]);
                if (next !== rawValue[key]) {
                    if (out === rawValue) out = { ...rawValue };
                    out[key] = next;
                }
            }
            return out;
        }
"#;
const CASE_ENUM: &str = "        case 9:\n            return (Array.isArray(spec) && Array.isArray(spec[1]) ? __TS_transformEnum(value, spec[1]) : value);\n";

const TRANSFORM_FN_SUFFIX: &str = "        default:\n            return value;\n    }\n}\n\n";

const ENUM_HELPERS_TS: &str = r#"function __TS_transformEnum<T>(value: T, variants: __TS_EnumVariantTransformSpec[]): T {
    const rawValue = value as any;
    for (const variant of variants) {
        if (variant[1] === 0) continue;

        if (rawValue != null && typeof rawValue === "object" && !Array.isArray(rawValue)) {
            if (Object.prototype.hasOwnProperty.call(rawValue, variant[0])) {
                const next = __TS_transform(rawValue[variant[0]], variant[2]);
                if (next === rawValue[variant[0]]) return value;
                return { ...rawValue, [variant[0]]: next } as T;
            }

            if (rawValue.type === variant[0] && Object.prototype.hasOwnProperty.call(rawValue, "data")) {
                const next = __TS_transform(rawValue.data, variant[2]);
                if (next === rawValue.data) return value;
                return { ...rawValue, data: next } as T;
            }

            if (rawValue.tag === variant[0] && Object.prototype.hasOwnProperty.call(rawValue, "content")) {
                const next = __TS_transform(rawValue.content, variant[2]);
                if (next === rawValue.content) return value;
                return { ...rawValue, content: next } as T;
            }
        }

        const direct = __TS_transform(value, variant[2]);
        if (direct !== value) return direct;
    }
    return value;
}

"#;

const ENUM_HELPERS_JS: &str = r#"function __TS_transformEnum(value, variants) {
    for (const variant of variants) {
        if (variant[1] === 0) continue;

        if (value != null && typeof value === "object" && !Array.isArray(value)) {
            if (Object.prototype.hasOwnProperty.call(value, variant[0])) {
                const next = __TS_transform(value[variant[0]], variant[2]);
                if (next === value[variant[0]]) return value;
                return { ...value, [variant[0]]: next };
            }

            if (value.type === variant[0] && Object.prototype.hasOwnProperty.call(value, "data")) {
                const next = __TS_transform(value.data, variant[2]);
                if (next === value.data) return value;
                return { ...value, data: next };
            }

            if (value.tag === variant[0] && Object.prototype.hasOwnProperty.call(value, "content")) {
                const next = __TS_transform(value.content, variant[2]);
                if (next === value.content) return value;
                return { ...value, content: next };
            }
        }

        const direct = __TS_transform(value, variant[2]);
        if (direct !== value) return direct;
    }
    return value;
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
            &Types::default(),
            &[],
        );

        assert!(matches!(spec, TransformSpec::BigInt));
    }

    #[test]
    fn list_u8_detection_is_opt_in() {
        let dt = DataType::List(specta::datatype::List::new(DataType::Primitive(
            Primitive::u8,
        )));

        let default_spec = Analyzer::with_builtins().analyze(&dt, &Types::default(), &[]);
        assert!(!matches!(default_spec, TransformSpec::Bytes));

        let bytes_spec = Analyzer::with_builtins()
            .with_list_u8_is_bytes(true)
            .analyze(&dt, &Types::default(), &[]);
        assert!(matches!(bytes_spec, TransformSpec::Bytes));
    }

    #[test]
    fn runtime_is_minimal_for_single_bigint() {
        let req = RuntimeRequirements::from_specs([&TransformSpec::BigInt]);
        let runtime = render_runtime(RuntimeTarget::JavaScript, &req);

        assert!(runtime.contains("case 1"));
        assert!(!runtime.contains("case 2"));
        assert!(!runtime.contains("case 9"));
        assert!(!runtime.contains("function __TS_transformEnum"));
    }

    #[test]
    fn serializes_compact_leaf_opcodes() {
        assert_eq!(TransformSpec::Identity.to_json(), "0");
        assert_eq!(TransformSpec::BigInt.to_json(), "1");
        assert_eq!(TransformSpec::Date.to_json(), "2");
        assert_eq!(TransformSpec::Bytes.to_json(), "3");
    }

    #[test]
    fn serializes_compact_composite_shapes() {
        let object = TransformSpec::Object(vec![(
            "created_at".to_string(),
            TransformSpec::Nullable(Box::new(TransformSpec::Date)),
        )]);
        assert_eq!(object.to_json(), r#"[7,[["created_at",[4,2]]]]"#);

        let en = TransformSpec::Enum(vec![EnumVariantTransformSpec {
            name: "Created".to_string(),
            kind: EnumVariantTransformKind::Named,
            spec: TransformSpec::Tuple(vec![TransformSpec::BigInt]),
        }]);
        assert_eq!(en.to_json(), r#"[9,[["Created",1,[6,[1]]]]]"#);
    }
}
