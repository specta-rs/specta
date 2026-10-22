//! Runtime-aware TypeScript type remapping.
//!
//! Semantic types are Rust types whose TypeScript runtime value should be more
//! specific than their JSON-compatible wire representation.
//!
//! This enables the following default rules:
//!  - [`bytes::Bytes`](https://docs.rs/bytes/latest/bytes/struct.Bytes.html) to become [`Uint8Array`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)
//!  - [`bytes::BytesMut`](https://docs.rs/bytes/latest/bytes/struct.BytesMut.html) to become [`Uint8Array`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)
//!  - [`url::Url`](https://docs.rs/url/latest/url/struct.Url.html) to become [`Url`](https://developer.mozilla.org/en-US/docs/Web/API/URL/URL)
//!  - [`chrono::DateTime`](https://docs.rs/chrono/latest/chrono/struct.DateTime.html) to become [`Date`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
//!  - [`chrono::NaiveDate`](https://docs.rs/chrono/latest/chrono/struct.NaiveDate.html) to become [`Date`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
//!  - [`jiff::Timestamp`](https://docs.rs/jiff/latest/jiff/struct.Timestamp.html) to become [`Date`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
//!  - [`jiff::civil::Date`](https://docs.rs/jiff/latest/jiff/civil/struct.Date.html) to become [`Date`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
//!
//! This could also allow you to map your own Rust types into custom JavaScript types like custom classes. Refer to [`semantic::Configuration::define`](Configuration::define) for building your own rules.
//!
//! This is intended the be implemented by frameworks like [Tauri Specta](https://github.com/specta-rs/tauri-specta), [TauRPC](https://github.com/MatsDK/TauRPC) and [rspc](https://github.com/specta-rs/rspc) as they have control of the runtime and type layer.
//!
//! <div class="warning">
//!
//! **WARNING:** The current implementation relies on the frontend and backend being versioned in-step. This works for a Tauri desktop application but may become an issue for a HTTP API unless you have something like [Skew Protection](https://vercel.com/docs/skew-protection).
//!
//! We will likely lift this as a hard restriction in the future!
//!
//! </div>
//!
//! <details>
//! <summary>Implementing into your own framework</summary>
//!
//! # Implementing into your own framework
//!
//! A framework needs to be integrated properly for this feature to work, as it requires both type-level and runtime JS to make it work properly.
//!
//! I would highly recommend reading [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) and understanding it as it's the core work which inspired this feature.
//!
//! Documentation coming soon... For now refer to [specta-rs/tauri-specta#219](https://github.com/specta-rs/tauri-specta/pull/219) which was the original implementation into [Tauri Specta](https://github.com/specta-rs/tauri-specta).
//!
//! </details>
//!

use std::{borrow::Cow, collections::BTreeSet, fmt, sync::Arc};

use specta::{
    Type, Types,
    datatype::{DataType, Enum, Field, Fields, NamedReferenceType, Primitive, Reference, Variant},
};

use crate::{
    Error, define,
    primitives::{escape_typescript_string_literal, is_identifier},
};

/// A JavaScript expression that converts between a semantic
/// TypeScript runtime value and its JSON-compatible representation.
///
/// The closure receives the JavaScript identifier/expression being transformed
/// and must return a JavaScript expression using that value.
///
/// # Examples
///
/// Convert a JSON string into a TypeScript `Date`:
///
/// ```rust
/// use specta_typescript::semantic::Transform;
///
/// let transform = Transform::new(|value| format!("new Date({value})"));
/// # let _ = transform;
/// ```
///
/// Convert a `Uint8Array` into a JSON array of numbers:
///
/// ```rust
/// use specta_typescript::semantic::Transform;
///
/// let transform = Transform::new(|value| format!("[...{value}]"));
/// # let _ = transform;
/// ```
///
/// Use [`Transform::identity`] when the TypeScript runtime value already has
/// the same representation as the value crossing the wire or when JSON.stringify/JSON.parse
/// is already able to handle the transformation for you.
#[derive(Clone)]
#[non_exhaustive]
pub struct Transform(
    /// The runtime transform function
    ///
    /// This is called with the argument being a Typescript identifier.
    /// It should output some transformation on the identifier.
    /// Eg. `|i| format!("new Date({i})")` could be one valid implementation.
    Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,
);

impl fmt::Debug for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(r) => write!(f, "{r:p}"),
            None => write!(f, "<none>"),
        }
    }
}

impl Transform {
    /// Construct a runtime transform from a JavaScript identifier mapper.
    ///
    /// The mapper should return a JavaScript expression, not a statement.
    ///
    /// ```rust
    /// use specta_typescript::semantic::Transform;
    ///
    /// let transform = Transform::new(|ident| format!("new URL({ident})"));
    /// # let _ = transform;
    /// ```
    pub fn new(runtime: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(runtime)))
    }

    /// Construct an identity runtime transform.
    ///
    /// This is useful when a rule only changes the exported TypeScript type, or
    /// when one direction does not need runtime conversion.
    ///
    /// ```rust
    /// use specta_typescript::semantic::Transform;
    ///
    /// let transform = Transform::identity();
    /// # let _ = transform;
    /// ```
    pub fn identity() -> Self {
        Self(None)
    }

    fn apply(&self, ident: &str) -> String {
        match &self.0 {
            Some(runtime) => runtime(ident),
            None => ident.to_owned(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DataTypeFn(Arc<dyn Fn(DataType) -> DataType + Send + Sync>);

impl DataTypeFn {
    pub(crate) fn new(f: impl Fn(DataType) -> DataType + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl fmt::Debug for DataTypeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DataTypeFn")
            .field(&format!("{:p}", self.0))
            .finish()
    }
}

/// A rule for a specific named data type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Rule {
    /// Matched against [`NamedDataType::name`](specta::datatype::NamedDataType::name) to determine if rule should apply
    pub name: Cow<'static, str>,
    /// Matched against [`NamedDataType::module_path`](specta::datatype::NamedDataType::module_path) to determine if rule should apply
    pub module_path: Cow<'static, str>,
    /// The type transformation function
    ///
    /// This must match up with the type produced or consumed by the runtime.
    pub(crate) data_type: DataTypeFn,
    /// The type transformation for serializing.
    /// This is JS -> Rust
    pub(crate) serialize: Option<Transform>,
    /// The type transformation for deserializing.
    /// This is Rust -> JS
    pub(crate) deserialize: Option<Transform>,
}

/// Configuration for runtime-aware TypeScript type remapping.
///
/// By default this contains a set of default rules as defined on [the module](crate::semantic). If you don't want them use [`Configuration::empty()`](Configuration::empty) instead.
///
/// You can add your own rules via [`Configuration::define(...)`](Configuration::define).
///
#[derive(Debug, Clone)]
pub struct Configuration {
    rules: Vec<Rule>,
    lossless_bigint: bool,
    lossless_floats: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            rules: vec![
                // Uint8Array
                Rule {
                    name: "Bytes".into(),
                    module_path: "bytes".into(),
                    data_type: DataTypeFn::new(|_| define("Uint8Array").into()),
                    serialize: Some(Transform::new(|i| format!("[...{i}]"))),
                    deserialize: Some(Transform::new(|i| format!("new Uint8Array({i})"))),
                },
                Rule {
                    name: "BytesMut".into(),
                    module_path: "bytes".into(),
                    data_type: DataTypeFn::new(|_| define("Uint8Array").into()),
                    serialize: Some(Transform::new(|i| format!("[...{i}]"))),
                    deserialize: Some(Transform::new(|i| format!("new Uint8Array({i})"))),
                },
                // URL
                Rule {
                    name: "Url".into(),
                    module_path: "url".into(),
                    data_type: DataTypeFn::new(|_| define("URL").into()),
                    serialize: None,
                    deserialize: Some(Transform::new(|i| format!("new URL({i})"))),
                },
                // Date
                Rule {
                    name: "DateTime".into(),
                    module_path: "chrono".into(),
                    data_type: DataTypeFn::new(|_| define("Date").into()),
                    serialize: None,
                    deserialize: Some(Transform::new(|i| format!("new Date({i})"))),
                },
                Rule {
                    name: "NaiveDate".into(),
                    module_path: "chrono".into(),
                    data_type: DataTypeFn::new(|_| define("Date").into()),
                    serialize: Some(Transform::new(|i| {
                        format!("{i}.toISOString().slice(0, 10)")
                    })),
                    deserialize: Some(Transform::new(|i| format!("new Date({i})"))),
                },
                Rule {
                    name: "Timestamp".into(),
                    module_path: "jiff".into(),
                    data_type: DataTypeFn::new(|_| define("Date").into()),
                    serialize: Some(Transform::new(|i| format!("{i}.toISOString()"))),
                    deserialize: Some(Transform::new(|i| format!("new Date({i})"))),
                },
                Rule {
                    name: "Date".into(),
                    module_path: "jiff::civil".into(),
                    data_type: DataTypeFn::new(|_| define("Date").into()),
                    serialize: Some(Transform::new(|i| {
                        format!("{i}.toISOString().slice(0, 10)")
                    })),
                    deserialize: Some(Transform::new(|i| format!("new Date({i})"))),
                },
                // We don't implement support for `chrono::NaiveDateTime`, and many `jiff` types as lack of timezone is an issue with JS's `Date`
            ],
            lossless_bigint: false,
            lossless_floats: false,
        }
    }
}

impl Configuration {
    /// Construct a [`Configuration`] without the default rules.
    ///
    /// Prefer [`Configuration::default`] when possible; the default
    /// rules cover common ecosystem types and may grow over time.
    pub fn empty() -> Self {
        Self {
            rules: Default::default(),
            lossless_bigint: false,
            lossless_floats: false,
        }
    }

    /// Exposes the rules applies to this instance for manual manipulation.
    ///
    /// This could be used to filter the default rules if you want to exclude certain ones.
    pub fn rules_mut(&mut self) -> &mut Vec<Rule> {
        &mut self.rules
    }

    /// Define a new rule for a given type `T`.
    ///
    /// `dt` receives the original [`DataType`] for `T` and must return the
    /// TypeScript-facing [`DataType`] that should replace it. `serialize`
    /// transforms TypeScript runtime values before sending them to Rust.
    /// `deserialize` transforms values received from Rust into TypeScript
    /// runtime values.
    ///
    /// This only works for named types, such as types generated by the
    /// [`Type`] derive macro. It does not work for primitives.
    ///
    /// ```rust
    /// use specta::Type;
    /// use specta_typescript::{define, semantic::{Configuration, Transform}};
    ///
    /// #[derive(Type)]
    /// struct MyCustomUrl(String);
    ///
    /// let mut semantic_types = Configuration::empty();
    /// semantic_types.define::<MyCustomUrl>(
    ///     |_| define("URL").into(), // Runtime Specta Type
    ///     Some(Transform::new(|value| format!("{value}.toString()"))), // JS -> JSON
    ///     Some(Transform::new(|value| format!("new URL({value})"))), // JSON -> JS
    /// );
    /// ```
    pub fn define<T: Type>(
        mut self,
        dt: impl Fn(DataType) -> DataType + Send + Sync + 'static,
        serialize: Option<Transform>,
        deserialize: Option<Transform>,
    ) -> Self {
        let mut types = Types::default();
        let ndt = match T::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => types.get(&r),
            _ => None,
        };
        if let Some(ndt) = ndt {
            self.rules.push(Rule {
                name: ndt.name.clone(),
                module_path: ndt.module_path.clone(),
                data_type: DataTypeFn(Arc::new(dt)),
                serialize,
                deserialize,
            });
        }

        self
    }

    /// Enable lossless support for large integer types (`BigInt`s).
    ///
    /// This remaps `usize`, `isize`, `u64`, `i64`, `u128`, and `i128` so they
    /// are a [`BigInt`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt) in JavaScript.
    ///
    /// This is only safe if your serialization and deserialization layer can losslessly transmit `BigInt`s to the frontend.
    ///
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for implementation details.
    pub fn enable_lossless_bigints(mut self) -> Self {
        if !self.lossless_bigint {
            self.lossless_bigint = true;
        }

        self
    }

    /// Enable lossless float support.
    ///
    /// By enabling this, you assert that your runtime *must* preserve `NaN`,
    /// `Infinity`, and `-Infinity` values from JavaScript to Rust and we will flatten `number | null` into `number`.
    ///
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for implementation details.
    pub fn enable_lossless_floats(mut self) -> Self {
        if !self.lossless_floats {
            self.lossless_floats = true;
        }

        self
    }

    /// Transform a [`Types`] collection using the configured rules.
    ///
    /// This rewrites registered named types so their exported TypeScript shapes
    /// match the values produced or consumed by the runtime transforms.
    ///
    /// Call this after any format-specific mapping that changes the type graph,
    /// and before exporting the final TypeScript definitions.
    pub fn apply_types<'a>(&self, types: &'a Types) -> Cow<'a, Types> {
        let mut types = Cow::Borrowed(types);

        if self.has_builtin_remaps() {
            types = Cow::Owned(types.into_owned().map(|mut ndt| {
                let remap_bigint = if ndt.name.ends_with("_Serialize") {
                    serialize_bigint
                } else {
                    deserialize_bigint
                };

                ndt.generics.to_mut().iter_mut().for_each(|generic| {
                    if let Some(dt) = &mut generic.default {
                        apply_builtin_remaps(
                            dt,
                            remap_bigint,
                            self.lossless_bigint,
                            self.lossless_floats,
                        );
                    }
                });
                if let Some(dt) = &mut ndt.ty {
                    apply_builtin_remaps(
                        dt,
                        remap_bigint,
                        self.lossless_bigint,
                        self.lossless_floats,
                    );
                }

                ndt
            }));
        }

        if !self.rules.is_empty() {
            let source = types.into_owned();
            let lookup = source.clone();
            types = Cow::Owned(source.map(|mut ndt| {
                if let Some(dt) = &mut ndt.ty {
                    self.apply_rules_to_dt(&lookup, dt);
                }

                if let Some(rule) = self
                    .rules
                    .iter()
                    .find(|r| r.name == ndt.name && r.module_path == ndt.module_path)
                    && let Some(dt) = ndt.ty.take()
                {
                    ndt.ty = Some((rule.data_type.0)(dt));
                }

                ndt
            }));
        }

        types
    }

    fn apply_rules_to_dt(&self, types: &Types, dt: &mut DataType) {
        if let DataType::Reference(Reference::Named(reference)) = dt
            && let Some(rule) = self.rule_for_reference(types, reference)
        {
            *dt = (rule.data_type.0)(Self::reference_source_dt(types, reference));
            return;
        }

        match dt {
            DataType::Primitive(_) | DataType::Generic(_) => {}
            DataType::List(list) => self.apply_rules_to_dt(types, &mut list.ty),
            DataType::Map(map) => {
                self.apply_rules_to_dt(types, map.key_ty_mut());
                self.apply_rules_to_dt(types, map.value_ty_mut());
            }
            DataType::Struct(s) => self.apply_rules_to_fields(types, &mut s.fields),
            DataType::Enum(e) => {
                for (_, variant) in &mut e.variants {
                    self.apply_rules_to_fields(types, &mut variant.fields);
                }
            }
            DataType::Tuple(tuple) => {
                for dt in &mut tuple.elements {
                    self.apply_rules_to_dt(types, dt);
                }
            }
            DataType::Nullable(dt) => self.apply_rules_to_dt(types, dt),
            DataType::Intersection(dts) => {
                for dt in dts {
                    self.apply_rules_to_dt(types, dt);
                }
            }
            DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
                NamedReferenceType::Recursive(_) | NamedReferenceType::Reference { .. } => {}
                NamedReferenceType::Inline { dt, .. } => self.apply_rules_to_dt(types, dt),
            },
            DataType::Reference(Reference::Opaque(_)) => {}
        }
    }

    fn apply_rules_to_fields(&self, types: &Types, fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for field in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.apply_rules_to_dt(types, dt);
                    }
                }
            }
            Fields::Named(fields) => {
                for (_, field) in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.apply_rules_to_dt(types, dt);
                    }
                }
            }
        }
    }

    /// Scan a [`DataType`] tree applying serialize-facing rules.
    ///
    /// This assumes [`Configuration::apply_types`] has already been applied to the [`Types`].
    /// Therefore the type updates will be shallow (up until references to the `Types`).
    ///
    /// The returned JavaScript expression is built around `js_ident` and may be
    /// deeply nested for structs, tuples, lists, nullable values, and
    /// intersections.
    ///
    /// If a registered graph contains no runtime transform, `None` is returned;
    /// its type-only changes are handled by [`Configuration::apply_types`]. For
    /// inline inputs, a type-only result is retained with an identity runtime
    /// expression because there is no registered type for `apply_types` to rewrite.
    ///
    /// Prefer [`Configuration::try_apply_serialize`] when the caller can handle
    /// an explicit error: this method panics if a runtime transform is required
    /// in a position the generator cannot discriminate.
    pub fn apply_serialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.try_apply_serialize(types, dt, js_ident).expect(
            "unable to generate a semantic transform; use `try_apply_serialize` to handle this explicitly",
        )
    }

    /// Fallible counterpart to [`Configuration::apply_serialize`].
    ///
    /// Returns an [`Error`] when a non-identity transform would be required
    /// beneath a construct whose wire shape cannot be discriminated without
    /// guessing (for example an overlapping `#[serde(untagged)]` enum). Emitting
    /// a transform there would let the exported type and the runtime value
    /// diverge, so the request is rejected rather than silently skipped.
    pub fn try_apply_serialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Result<Option<(Option<DataType>, String)>, Error> {
        self.try_apply(
            serialize_bigint,
            |rule| &rule.serialize,
            types,
            dt,
            js_ident,
        )
    }

    /// Scan a [`DataType`] tree applying deserialize-facing rules.
    ///
    /// Use this for values received from Rust before exposing them to
    /// TypeScript callers.
    ///
    /// Prefer [`Configuration::try_apply_deserialize`] when the caller can
    /// handle an explicit error: this method panics if a runtime transform is
    /// required in a position the generator cannot discriminate.
    pub fn apply_deserialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.try_apply_deserialize(types, dt, js_ident).expect(
            "unable to generate a semantic transform; use `try_apply_deserialize` to handle this explicitly",
        )
    }

    /// Fallible counterpart to [`Configuration::apply_deserialize`].
    ///
    /// See [`Configuration::try_apply_serialize`] for the rejection semantics.
    pub fn try_apply_deserialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Result<Option<(Option<DataType>, String)>, Error> {
        self.try_apply(
            deserialize_bigint,
            |rule| &rule.deserialize,
            types,
            dt,
            js_ident,
        )
    }

    fn try_apply(
        &self,
        remap_bigint: fn() -> DataType,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Result<Option<(Option<DataType>, String)>, Error> {
        let mut unsupported = None;
        let result = self.apply_inner(
            transform_for_rule,
            remap_bigint,
            types,
            dt,
            js_ident,
            &mut Vec::new(),
            &mut unsupported,
        );
        match unsupported {
            Some(err) => Err(err),
            None => Ok(prune_identity_root(types, dt, js_ident, result)),
        }
    }

    fn apply_inner(
        &self,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        remap_bigint: fn() -> DataType,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
        stack: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>,
        unsupported: &mut Option<Error>,
    ) -> Option<(Option<DataType>, String)> {
        let result = match dt {
            DataType::Reference(Reference::Named(r)) => {
                if let Some(rule) = self.rule_for_reference(types, r) {
                    return Some((
                        Some((rule.data_type.0)(Self::reference_source_dt(types, r))),
                        transform_for_rule(rule).as_ref().map_or_else(
                            || js_ident.to_owned(),
                            |transform| transform.apply(js_ident),
                        ),
                    ));
                }

                match &r.inner {
                    NamedReferenceType::Inline { dt, .. } => self.apply_inner(
                        transform_for_rule,
                        remap_bigint,
                        types,
                        dt,
                        js_ident,
                        stack,
                        unsupported,
                    ),
                    NamedReferenceType::Recursive(_) => None,
                    NamedReferenceType::Reference { .. } => {
                        let ndt = types.get(r)?;

                        let ty = ndt.ty.as_ref()?;
                        let key = (ndt.name.clone(), ndt.module_path.clone());
                        if stack.contains(&key) {
                            return None;
                        }
                        stack.push(key);
                        let result = self
                            .apply_inner(
                                transform_for_rule,
                                remap_bigint,
                                types,
                                ty,
                                js_ident,
                                stack,
                                unsupported,
                            )
                            .map(|(_, runtime)| (None, runtime));
                        stack.pop();
                        result
                    }
                }
            }
            DataType::Struct(s) => self
                .apply_fields(
                    transform_for_rule,
                    remap_bigint,
                    types,
                    &s.fields,
                    js_ident,
                    stack,
                    unsupported,
                )
                .map(|(fields, runtime)| {
                    let ty = fields.map(|fields| {
                        let mut next = s.clone();
                        next.fields = fields;
                        DataType::Struct(next)
                    });
                    (ty, runtime)
                }),
            DataType::Tuple(tuple) => {
                let mut ty = tuple.clone();
                let mut changed = false;
                let parts = tuple
                    .elements
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, element)| {
                        let ident = format!("{js_ident}[{idx}]");
                        let (next_ty, runtime) = self.apply_inner(
                            transform_for_rule,
                            remap_bigint,
                            types,
                            element,
                            &ident,
                            stack,
                            unsupported,
                        )?;
                        if let Some(next_ty) = next_ty {
                            ty.elements[idx] = next_ty;
                            changed = true;
                        }
                        (runtime != ident).then_some((idx, runtime))
                    })
                    .collect::<Vec<_>>();

                if parts.is_empty() {
                    changed.then_some((Some(DataType::Tuple(ty)), js_ident.to_owned()))
                } else {
                    Some((
                        changed.then_some(DataType::Tuple(ty)),
                        array_transform(js_ident, tuple.elements.len(), parts),
                    ))
                }
            }
            DataType::Map(map) => {
                let item = "v";
                let (next_ty, runtime) = self.apply_inner(
                    transform_for_rule,
                    remap_bigint,
                    types,
                    map.value_ty(),
                    item,
                    stack,
                    unsupported,
                )?;

                let mut ty = map.clone();
                let mut changed = false;
                if let Some(next_ty) = next_ty {
                    ty.set_value_ty(next_ty);
                    changed = true;
                }

                Some((
                    changed.then_some(DataType::Map(ty)),
                    if runtime == item {
                        js_ident.to_owned()
                    } else {
                        format!(
                            "Object.fromEntries(Object.entries({js_ident}).map(([k,{item}])=>[k,{runtime}]))"
                        )
                    },
                ))
            }
            DataType::List(list) => {
                let item = "i";
                let (next_ty, runtime) = self.apply_inner(
                    transform_for_rule,
                    remap_bigint,
                    types,
                    &list.ty,
                    item,
                    stack,
                    unsupported,
                )?;
                let mut ty = list.clone();
                let mut changed = false;
                if let Some(next_ty) = next_ty {
                    ty.ty = Box::new(next_ty);
                    changed = true;
                }
                Some((
                    changed.then_some(DataType::List(ty)),
                    if runtime == item {
                        js_ident.to_owned()
                    } else {
                        format!("{js_ident}.map({item}=>{runtime})")
                    },
                ))
            }
            DataType::Nullable(inner) => {
                let (next_ty, runtime) = self.apply_inner(
                    transform_for_rule,
                    remap_bigint,
                    types,
                    inner,
                    js_ident,
                    stack,
                    unsupported,
                )?;
                Some((
                    next_ty.map(|dt| DataType::Nullable(Box::new(dt))),
                    if runtime == js_ident {
                        js_ident.to_owned()
                    } else {
                        format!("{js_ident}==null?{js_ident}:{runtime}")
                    },
                ))
            }
            DataType::Intersection(items) => {
                let mut ty = items.clone();
                let mut changed = false;
                let mut parts = items
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, item)| {
                        let (next_ty, runtime) = self.apply_inner(
                            transform_for_rule,
                            remap_bigint,
                            types,
                            item,
                            js_ident,
                            stack,
                            unsupported,
                        )?;
                        if let Some(next_ty) = next_ty {
                            ty[idx] = next_ty;
                            changed = true;
                        }
                        Some(runtime)
                    })
                    .collect::<Vec<_>>();

                parts.dedup();

                match parts.as_slice() {
                    [] => None,
                    [runtime] => Some((
                        changed.then_some(DataType::Intersection(ty)),
                        runtime.clone(),
                    )),
                    _ => Some((
                        changed.then_some(DataType::Intersection(ty)),
                        spread_transform(
                            "",
                            parts.into_iter().map(|p| format!("...{p}")).collect(),
                        ),
                    )),
                }
            }
            DataType::Enum(e) => self.apply_enum(
                transform_for_rule,
                remap_bigint,
                types,
                e,
                js_ident,
                stack,
                unsupported,
            ),
            DataType::Primitive(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => None,
        };

        self.apply_builtin_remaps(remap_bigint, dt, js_ident, result)
    }

    /// Apply the configured rules to one variant/struct [`Fields`] layout.
    ///
    /// Returns `None` when the layout needs neither a type nor a runtime
    /// change, and otherwise the (possibly unchanged) fields plus the runtime
    /// expression. This is shared by structs and enum variant payloads so the
    /// two stay in agreement.
    fn apply_fields(
        &self,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        remap_bigint: fn() -> DataType,
        types: &Types,
        fields: &Fields,
        js_ident: &str,
        stack: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>,
        unsupported: &mut Option<Error>,
    ) -> Option<(Option<Fields>, String)> {
        match fields {
            Fields::Unit => None,
            Fields::Named(named) => {
                let mut ty = named.clone();
                let mut changed = false;
                let mut parts = Vec::new();

                for (name, field) in &named.fields {
                    let Some(field_ty) = &field.ty else { continue };
                    let field_ident = js_property_access(js_ident, name);
                    let Some((next_ty, runtime)) = self.apply_inner(
                        transform_for_rule,
                        remap_bigint,
                        types,
                        field_ty,
                        &field_ident,
                        stack,
                        unsupported,
                    ) else {
                        continue;
                    };

                    if let Some(next_ty) = next_ty
                        && let Some((_, field)) = ty.fields.iter_mut().find(|(n, _)| n == name)
                    {
                        field.ty = Some(next_ty);
                        changed = true;
                    }
                    if runtime != field_ident {
                        parts.push(format!("{}:{runtime}", js_object_key(name)));
                    }
                }

                if parts.is_empty() {
                    changed.then_some((Some(Fields::Named(ty)), js_ident.to_owned()))
                } else {
                    Some((
                        changed.then_some(Fields::Named(ty)),
                        spread_transform(js_ident, parts),
                    ))
                }
            }
            Fields::Unnamed(unnamed) => {
                let mut ty = unnamed.clone();
                let mut changed = false;
                let is_newtype = unnamed.fields.len() == 1;
                let parts = unnamed
                    .fields
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, field)| {
                        let field_ty = field.ty.as_ref()?;
                        let field_ident = if is_newtype {
                            js_ident.to_owned()
                        } else {
                            format!("{js_ident}[{idx}]")
                        };
                        let (next_ty, runtime) = self.apply_inner(
                            transform_for_rule,
                            remap_bigint,
                            types,
                            field_ty,
                            &field_ident,
                            stack,
                            unsupported,
                        )?;

                        if let Some(next_ty) = next_ty {
                            ty.fields[idx].ty = Some(next_ty);
                            changed = true;
                        }

                        (runtime != field_ident).then_some((idx, runtime))
                    })
                    .collect::<Vec<_>>();

                if parts.is_empty() {
                    changed.then_some((Some(Fields::Unnamed(ty)), js_ident.to_owned()))
                } else {
                    Some((
                        changed.then_some(Fields::Unnamed(ty)),
                        if is_newtype {
                            parts.into_iter().next()?.1
                        } else {
                            array_transform(js_ident, unnamed.fields.len(), parts)
                        },
                    ))
                }
            }
        }
    }

    /// Generate a representation-aware runtime transform for an enum.
    ///
    /// The wire shape is the one produced by the format layer (for example
    /// `specta_serde`): external variants are keyed by their serialized name,
    /// internally/adjacently tagged variants carry a discriminator field, and
    /// untagged variants carry no discriminant. Each active variant's payload
    /// is transformed with the same [`Fields`] traversal structs use, then the
    /// branches are dispatched on the wire shape so the runtime value matches
    /// the remapped type.
    ///
    /// When a non-identity transform would be required beneath a shape that
    /// cannot be discriminated without guessing, this records an explicit
    /// [`Error`] rather than silently returning `None`.
    fn apply_enum(
        &self,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        remap_bigint: fn() -> DataType,
        types: &Types,
        e: &Enum,
        js_ident: &str,
        stack: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>,
        unsupported: &mut Option<Error>,
    ) -> Option<(Option<DataType>, String)> {
        let active = e
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .collect::<Vec<_>>();
        if active.is_empty() {
            return None;
        }

        let mut branches = Vec::with_capacity(active.len());
        let mut changed = false;
        let mut needs_transform = false;
        for (_, variant) in &active {
            let (fields, runtime) = match self.apply_fields(
                transform_for_rule,
                remap_bigint,
                types,
                &variant.fields,
                js_ident,
                stack,
                unsupported,
            ) {
                Some((fields, runtime)) => (fields, runtime),
                None => (None, js_ident.to_owned()),
            };
            changed |= fields.is_some();
            needs_transform |= runtime != js_ident;
            branches.push((fields, runtime));
        }

        let build = |branches: &mut Vec<(Option<Fields>, String)>| -> Option<DataType> {
            if !changed {
                return None;
            }
            let mut next = e.clone();
            let mut branch = branches.iter_mut();
            for (_, variant) in &mut next.variants {
                if variant.skip {
                    continue;
                }
                if let Some((Some(fields), _)) = branch.next() {
                    variant.fields = fields.clone();
                }
            }
            Some(DataType::Enum(next))
        };

        if !needs_transform {
            return build(&mut branches).map(|ty| (Some(ty), js_ident.to_owned()));
        }

        // A lone variant is never ambiguous: every value belongs to it.
        if branches.len() == 1 {
            let next_ty = build(&mut branches);
            let (_, runtime) = branches.into_iter().next().expect("one branch");
            return Some((next_ty, runtime));
        }

        let Some(guards) = enum_branch_guards(&active, js_ident) else {
            if unsupported.is_none() {
                *unsupported = Some(Error::unsupported_semantic_transform(
                    "a non-identity transform is required beneath an enum whose wire shape cannot be discriminated without guessing (for example overlapping `#[serde(untagged)]` variants); use a tagged enum representation or apply the rule to a non-ambiguous position",
                ));
            }
            return None;
        };

        let has_else = guards
            .iter()
            .any(|guard| matches!(guard, BranchGuard::Else));
        let mut parts = Vec::new();
        let mut fallback = None;
        for ((_, runtime), guard) in branches.iter().zip(&guards) {
            match guard {
                BranchGuard::Match(condition) => {
                    if has_else || runtime != js_ident {
                        parts.push(format!("{condition}?{runtime}"));
                    }
                }
                BranchGuard::Else => fallback = Some(runtime.clone()),
            }
        }

        let default = fallback.unwrap_or_else(|| js_ident.to_owned());
        let runtime = if parts.is_empty() {
            default
        } else {
            format!("{}:{}", parts.join(":"), default)
        };

        let next_ty = build(&mut branches);
        Some((next_ty, runtime))
    }

    fn rule_for_reference<'a>(
        &'a self,
        types: &'a Types,
        reference: &specta::datatype::NamedReference,
    ) -> Option<&'a Rule> {
        let ndt = types.get(reference)?;
        self.rules
            .iter()
            .find(|rule| rule.name == ndt.name && rule.module_path == ndt.module_path)
    }

    fn reference_source_dt(
        types: &Types,
        reference: &specta::datatype::NamedReference,
    ) -> DataType {
        match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => (**dt).clone(),
            NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive(_) => types
                .get(reference)
                .and_then(|ndt| ndt.ty.clone())
                .unwrap_or_else(|| DataType::Reference(Reference::Named(reference.clone()))),
        }
    }

    fn has_builtin_remaps(&self) -> bool {
        self.lossless_bigint || self.lossless_floats
    }

    fn apply_builtin_remaps(
        &self,
        remap_bigint: fn() -> DataType,
        dt: &DataType,
        js_ident: &str,
        result: Option<(Option<DataType>, String)>,
    ) -> Option<(Option<DataType>, String)> {
        if !self.has_builtin_remaps() {
            return result;
        }

        let source = result
            .as_ref()
            .and_then(|(dt, _)| dt.clone())
            .unwrap_or_else(|| dt.clone());
        let mut remapped = source.clone();
        apply_builtin_remaps(
            &mut remapped,
            remap_bigint,
            self.lossless_bigint,
            self.lossless_floats,
        );

        let runtime = result
            .as_ref()
            .map(|(_, runtime)| runtime.as_str())
            .unwrap_or(js_ident);
        let runtime = if is_lossless_bigint_primitive(&source)
            && self.lossless_bigint
            && remap_bigint() == deserialize_bigint()
        {
            format!("BigInt({runtime})")
        } else {
            runtime.to_owned()
        };

        if remapped == source {
            result
        } else {
            Some((Some(remapped), runtime))
        }
    }
}

fn prune_identity_root(
    types: &Types,
    dt: &DataType,
    js_ident: &str,
    result: Option<(Option<DataType>, String)>,
) -> Option<(Option<DataType>, String)> {
    let registered_root = matches!(
        dt,
        DataType::Reference(Reference::Named(reference))
            if !matches!(reference.inner, NamedReferenceType::Inline { .. })
                && types.get(reference).is_some_and(|ndt| ndt.ty.is_some())
    );

    result.filter(|(next_ty, runtime)| {
        runtime != js_ident
            || next_ty
                .as_ref()
                .is_some_and(|next_ty| !registered_root || registered_generics_changed(dt, next_ty))
    })
}

fn registered_generics_changed(dt: &DataType, next_ty: &DataType) -> bool {
    let (
        DataType::Reference(Reference::Named(source)),
        DataType::Reference(Reference::Named(remapped)),
    ) = (dt, next_ty)
    else {
        return false;
    };
    let (
        NamedReferenceType::Reference {
            generics: source, ..
        },
        NamedReferenceType::Reference {
            generics: remapped, ..
        },
    ) = (&source.inner, &remapped.inner)
    else {
        return false;
    };

    source != remapped
}

fn is_lossless_bigint_primitive(dt: &DataType) -> bool {
    matches!(
        dt,
        DataType::Primitive(
            Primitive::usize
                | Primitive::isize
                | Primitive::u64
                | Primitive::i64
                | Primitive::u128
                | Primitive::i128
        )
    )
}

fn apply_builtin_remaps(
    dt: &mut DataType,
    remap_bigint: fn() -> DataType,
    lossless_bigint: bool,
    lossless_floats: bool,
) {
    if let DataType::Primitive(primitive) = dt
        && let Some(remapped) = remap_primitive(
            primitive.clone(),
            remap_bigint,
            lossless_bigint,
            lossless_floats,
        )
    {
        *dt = remapped;
        return;
    }

    match dt {
        DataType::Primitive(_) | DataType::Generic(_) => {}
        DataType::List(list) => {
            apply_builtin_remaps(&mut list.ty, remap_bigint, lossless_bigint, lossless_floats)
        }
        DataType::Map(map) => {
            apply_builtin_remaps(
                map.key_ty_mut(),
                remap_bigint,
                lossless_bigint,
                lossless_floats,
            );
            apply_builtin_remaps(
                map.value_ty_mut(),
                remap_bigint,
                lossless_bigint,
                lossless_floats,
            );
        }
        DataType::Struct(s) => apply_builtin_remaps_to_fields(
            &mut s.fields,
            remap_bigint,
            lossless_bigint,
            lossless_floats,
        ),
        DataType::Enum(e) => {
            for (_, variant) in &mut e.variants {
                apply_builtin_remaps_to_fields(
                    &mut variant.fields,
                    remap_bigint,
                    lossless_bigint,
                    lossless_floats,
                );
            }
        }
        DataType::Tuple(tuple) => {
            for dt in &mut tuple.elements {
                apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
            }
        }
        DataType::Nullable(dt) => {
            apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
        }
        DataType::Intersection(dts) => {
            for dt in dts {
                apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
            }
        }
        DataType::Reference(reference) => {
            let Reference::Named(reference) = reference else {
                return;
            };

            match &mut reference.inner {
                NamedReferenceType::Recursive(_) => {}
                NamedReferenceType::Inline { dt, .. } => {
                    apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
                }
                NamedReferenceType::Reference { generics, .. } => {
                    for (_, dt) in generics {
                        apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
                    }
                }
            }
        }
    }
}

fn apply_builtin_remaps_to_fields(
    fields: &mut Fields,
    remap_bigint: fn() -> DataType,
    lossless_bigint: bool,
    lossless_floats: bool,
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(dt) = &mut field.ty {
                    apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(dt) = &mut field.ty {
                    apply_builtin_remaps(dt, remap_bigint, lossless_bigint, lossless_floats);
                }
            }
        }
    }
}

fn remap_primitive(
    primitive: Primitive,
    remap_bigint: fn() -> DataType,
    lossless_bigint: bool,
    lossless_floats: bool,
) -> Option<DataType> {
    if lossless_bigint
        && matches!(
            primitive,
            Primitive::usize
                | Primitive::isize
                | Primitive::u64
                | Primitive::i64
                | Primitive::u128
                | Primitive::i128
        )
    {
        return Some(remap_bigint());
    }

    if lossless_floats && matches!(primitive, Primitive::f16 | Primitive::f32 | Primitive::f64) {
        return Some(Reference::opaque(crate::opaque::Number).into());
    }

    None
}

fn serialize_bigint() -> DataType {
    crate::define("bigint | number").into()
}

fn deserialize_bigint() -> DataType {
    Reference::opaque(crate::opaque::BigInt).into()
}

fn spread_transform(js_ident: &str, mut parts: Vec<String>) -> String {
    if !js_ident.is_empty() {
        parts.insert(0, format!("...{js_ident}"));
    }
    format!("({{{}}})", parts.join(","))
}

fn array_transform(js_ident: &str, len: usize, parts: Vec<(usize, String)>) -> String {
    let mut items = (0..len)
        .map(|idx| format!("{js_ident}[{idx}]"))
        .collect::<Vec<_>>();

    for (idx, runtime) in parts {
        items[idx] = runtime;
    }

    format!("([{}])", items.join(","))
}

fn js_property_access(base: &str, name: &str) -> String {
    if is_identifier(name) {
        format!("{base}.{name}")
    } else {
        format!("{base}[\"{}\"]", escape_typescript_string_literal(name))
    }
}

fn js_object_key(name: &str) -> Cow<'_, str> {
    if is_identifier(name) {
        Cow::Borrowed(name)
    } else {
        Cow::Owned(format!("\"{}\"", escape_typescript_string_literal(name)))
    }
}

fn js_string_literal(value: &str) -> String {
    format!("\"{}\"", escape_typescript_string_literal(value))
}

/// How a single enum branch is selected at runtime.
enum BranchGuard {
    /// A JavaScript expression that is truthy exactly for this variant.
    Match(String),
    /// The branch is the fallback (`#[serde(other)]`), matching whatever the
    /// literal branches above did not.
    Else,
}

/// Work out how to select each active variant from its wire value.
///
/// Prefers a shared discriminator field (internally/adjacently tagged enums),
/// then falls back to the external representation's per-variant key or string
/// literal. Returns `None` when no unambiguous discrimination exists.
fn enum_branch_guards(
    active: &[&(Cow<'static, str>, Variant)],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    tag_branch_guards(active, js_ident).or_else(|| external_branch_guards(active, js_ident))
}

/// Guards for a tagged enum: a field present in every variant with distinct
/// string-literal values (`#[serde(tag = ...)]`, optionally with `content`).
fn tag_branch_guards(
    active: &[&(Cow<'static, str>, Variant)],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    let candidates = active
        .iter()
        .map(|(_, variant)| variant_tag_candidates(variant))
        .collect::<Vec<_>>();
    let key = common_tag_key(&candidates)?;

    Some(
        candidates
            .iter()
            .map(|candidates| {
                let literal = candidates
                    .iter()
                    .find(|(candidate, _)| candidate == &key)
                    .expect("common_tag_key only returns keys present in every variant")
                    .1
                    .clone();
                match literal {
                    Some(literal) => BranchGuard::Match(format!(
                        "{js_ident}!=null&&{}==={}",
                        js_property_access(js_ident, &key),
                        js_string_literal(&literal),
                    )),
                    None => BranchGuard::Else,
                }
            })
            .collect(),
    )
}

/// Guards for the external representation: a non-unit variant is keyed by its
/// serialized name, while a unit (or collapsed) variant is the bare string.
fn external_branch_guards(
    active: &[&(Cow<'static, str>, Variant)],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    active
        .iter()
        .map(|(_, variant)| match &variant.fields {
            Fields::Named(named) if named.fields.len() == 1 => {
                let (name, _) = &named.fields[0];
                Some(BranchGuard::Match(format!(
                    "{js_ident}!=null&&{}!==undefined",
                    js_property_access(js_ident, name),
                )))
            }
            Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                let literal = string_literal_of(unnamed.fields[0].ty.as_ref()?)?;
                Some(BranchGuard::Match(format!(
                    "{js_ident}==={}",
                    js_string_literal(&literal),
                )))
            }
            _ => None,
        })
        .collect()
}

/// The literal value of a synthetic single-variant string enum, if `ty` is one.
fn string_literal_of(ty: &DataType) -> Option<String> {
    let DataType::Enum(e) = ty else {
        return None;
    };

    let mut variants = e.variants.iter();
    let (name, variant) = variants.next()?;
    if variants.next().is_some() || !matches!(variant.fields, Fields::Unit) {
        return None;
    }

    Some(name.to_string())
}

/// Every `(key, literal)` discriminator candidate directly on a variant's wire
/// shape. A `None` literal is a widened `#[serde(other)]` string tag.
fn variant_tag_candidates(variant: &Variant) -> Vec<(String, Option<String>)> {
    let mut candidates = Vec::new();
    match &variant.fields {
        Fields::Named(named) => collect_tag_candidates(&named.fields, &mut candidates),
        Fields::Unnamed(unnamed) => {
            for field in &unnamed.fields {
                if let Some(ty) = &field.ty {
                    collect_datatype_tag_candidates(ty, &mut candidates);
                }
            }
        }
        Fields::Unit => {}
    }
    candidates
}

fn collect_tag_candidates(
    fields: &[(Cow<'static, str>, Field)],
    candidates: &mut Vec<(String, Option<String>)>,
) {
    for (name, field) in fields {
        if field.optional {
            continue;
        }
        let Some(ty) = &field.ty else { continue };
        if let Some(literal) = string_literal_of(ty) {
            candidates.push((name.to_string(), Some(literal)));
        } else if matches!(ty, DataType::Primitive(Primitive::str)) {
            candidates.push((name.to_string(), None));
        }
    }
}

/// Descend the intersection/struct wrappers an internally tagged tuple variant
/// is rewritten into so its tag field is visible.
fn collect_datatype_tag_candidates(ty: &DataType, candidates: &mut Vec<(String, Option<String>)>) {
    match ty {
        DataType::Intersection(parts) => {
            for part in parts {
                collect_datatype_tag_candidates(part, candidates);
            }
        }
        DataType::Struct(s) => {
            if let Fields::Named(named) = &s.fields {
                collect_tag_candidates(&named.fields, candidates);
            }
        }
        _ => {}
    }
}

/// The key shared by every variant with distinct literal values, if any.
fn common_tag_key(candidates: &[Vec<(String, Option<String>)>]) -> Option<String> {
    let first = candidates.first()?;

    'keys: for (key, _) in first {
        let mut literals = Vec::with_capacity(candidates.len());
        for variant in candidates {
            let matching = variant
                .iter()
                .filter(|(candidate, _)| candidate == key)
                .collect::<Vec<_>>();
            let [(_, literal)] = matching.as_slice() else {
                continue 'keys;
            };
            literals.push(literal.clone());
        }

        let mut seen = BTreeSet::new();
        let mut fallback = 0;
        for literal in &literals {
            match literal {
                Some(literal) => {
                    if !seen.insert(literal) {
                        continue 'keys;
                    }
                }
                None => {
                    fallback += 1;
                    if fallback > 1 {
                        continue 'keys;
                    }
                }
            }
        }

        return Some(key.clone());
    }

    None
}
