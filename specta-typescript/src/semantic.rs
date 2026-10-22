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
    datatype::{
        DataType, Enum, Field, Fields, NamedDataType, NamedReferenceType, Primitive, Reference,
        Variant,
    },
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
    /// This method is non-panicking. A construct that cannot be transformed is
    /// skipped: it contributes no runtime change, and enclosing structs/lists
    /// still transform their other parts. When the *root* cannot be transformed,
    /// `None` is returned. Use [`Configuration::try_apply_serialize`] to find out
    /// why a transform was skipped.
    pub fn apply_serialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.apply(
            serialize_bigint,
            |rule| &rule.serialize,
            types,
            dt,
            js_ident,
        )
    }

    /// Fallible counterpart to [`Configuration::apply_serialize`].
    ///
    /// Returns an [`Error`] when a non-identity transform is required beneath a
    /// construct whose wire shape cannot be discriminated without guessing (for
    /// example overlapping `#[serde(untagged)]` variants, or a recursive enum
    /// whose expansion would leave nested values untransformed). Emitting a
    /// transform there would let the exported type and the runtime value
    /// diverge, so the unsupported part is skipped and reported instead.
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
    /// This method is non-panicking. See [`Configuration::apply_serialize`] for
    /// the skipping semantics, and [`Configuration::try_apply_deserialize`] to
    /// find out why a transform was skipped.
    pub fn apply_deserialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.apply(
            deserialize_bigint,
            |rule| &rule.deserialize,
            types,
            dt,
            js_ident,
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

    /// Run a traversal and keep the diagnostic if a part could not be
    /// transformed. The infallible API drops the diagnostic and keeps whatever
    /// could be generated.
    fn run(
        &self,
        remap_bigint: fn() -> DataType,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> (Option<(Option<DataType>, String)>, Option<Error>) {
        let mut walk = Walk {
            transform_for_rule,
            remap_bigint,
            types,
            stack: Vec::new(),
            error: None,
            recursion_cut: false,
            current: None,
        };
        let result = self.apply_inner(&mut walk, dt, js_ident);
        (prune_identity_root(types, dt, js_ident, result), walk.error)
    }

    fn apply(
        &self,
        remap_bigint: fn() -> DataType,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.run(remap_bigint, transform_for_rule, types, dt, js_ident)
            .0
    }

    fn try_apply(
        &self,
        remap_bigint: fn() -> DataType,
        transform_for_rule: fn(&Rule) -> &Option<Transform>,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Result<Option<(Option<DataType>, String)>, Error> {
        let (result, error) = self.run(remap_bigint, transform_for_rule, types, dt, js_ident);
        match error {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }

    fn apply_inner(
        &self,
        walk: &mut Walk<'_>,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        let result = match dt {
            DataType::Reference(Reference::Named(r)) => {
                if let Some(rule) = self.rule_for_reference(walk.types, r) {
                    return Some((
                        Some((rule.data_type.0)(Self::reference_source_dt(walk.types, r))),
                        (walk.transform_for_rule)(rule).as_ref().map_or_else(
                            || js_ident.to_owned(),
                            |transform| transform.apply(js_ident),
                        ),
                    ));
                }

                match &r.inner {
                    NamedReferenceType::Inline { dt, .. } => self.apply_inner(walk, dt, js_ident),
                    NamedReferenceType::Recursive(_) => None,
                    NamedReferenceType::Reference { .. } => {
                        let ndt = walk.types.get(r)?;

                        let ty = ndt.ty.as_ref()?;
                        let key = (ndt.name.clone(), ndt.module_path.clone());
                        if walk.stack.contains(&key) {
                            walk.recursion_cut = true;
                            return None;
                        }
                        walk.stack.push(key);
                        let previous = walk.current.replace(ndt);
                        let result = self
                            .apply_inner(walk, ty, js_ident)
                            .map(|(_, runtime)| (None, runtime));
                        walk.current = previous;
                        walk.stack.pop();
                        result
                    }
                }
            }
            DataType::Struct(s) => {
                self.apply_fields(walk, &s.fields, js_ident)
                    .map(|(fields, runtime)| {
                        let ty = fields.map(|fields| {
                            let mut next = s.clone();
                            next.fields = fields;
                            DataType::Struct(next)
                        });
                        (ty, runtime)
                    })
            }
            DataType::Tuple(tuple) => {
                let mut ty = tuple.clone();
                let mut changed = false;
                let parts = tuple
                    .elements
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, element)| {
                        let ident = format!("{js_ident}[{idx}]");
                        let (next_ty, runtime) = self.apply_inner(walk, element, &ident)?;
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
                let (next_ty, runtime) = self.apply_inner(walk, map.value_ty(), item)?;

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
                let (next_ty, runtime) = self.apply_inner(walk, &list.ty, item)?;
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
                let (next_ty, runtime) = self.apply_inner(walk, inner, js_ident)?;
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
                        let (next_ty, runtime) = self.apply_inner(walk, item, js_ident)?;
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
            DataType::Enum(e) => self.apply_enum(walk, e, js_ident),
            DataType::Primitive(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => None,
        };

        self.apply_builtin_remaps(walk.remap_bigint, dt, js_ident, result)
    }

    /// Apply the configured rules to one variant/struct [`Fields`] layout.
    ///
    /// Returns `None` when the layout needs neither a type nor a runtime
    /// change, and otherwise the (possibly unchanged) fields plus the runtime
    /// expression. This is shared by structs and enum variant payloads so the
    /// two stay in agreement.
    fn apply_fields(
        &self,
        walk: &mut Walk<'_>,
        fields: &Fields,
        js_ident: &str,
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
                    let Some((next_ty, runtime)) = self.apply_inner(walk, field_ty, &field_ident)
                    else {
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
                        let (next_ty, runtime) = self.apply_inner(walk, field_ty, &field_ident)?;

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
    /// untagged variants carry no discriminant (and are told apart by their
    /// payload shape). Each active variant's payload is transformed with the
    /// same [`Fields`] traversal structs use, then the branches are dispatched
    /// on the wire shape so the runtime value matches the remapped type.
    ///
    /// When a non-identity transform would be required beneath a shape that
    /// cannot be discriminated without guessing (or beneath a recursive enum,
    /// whose single-expression expansion would be partial), this records an
    /// explicit [`Error`] and returns `None` rather than emitting an expression
    /// that would let the exported type and the runtime value diverge.
    fn apply_enum(
        &self,
        walk: &mut Walk<'_>,
        e: &Enum,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        let active = e
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .collect::<Vec<_>>();
        if active.is_empty() {
            return None;
        }

        let cut_before = walk.recursion_cut;

        let mut branches = Vec::with_capacity(active.len());
        let mut changed = false;
        let mut needs_transform = false;
        for (_, variant) in &active {
            let (fields, runtime) = match self.apply_fields(walk, &variant.fields, js_ident) {
                Some((fields, runtime)) => (fields, runtime),
                None => (None, js_ident.to_owned()),
            };
            changed |= fields.is_some();
            needs_transform |= runtime != js_ident;
            branches.push((fields, runtime));
        }

        // A recursive expansion is cut short by the cycle stack, so the single
        // expression we could emit would leave the nested values untransformed
        // while the exported type claims otherwise. Report it instead.
        if walk.recursion_cut && !cut_before {
            self.record_unsupported(walk, RECURSIVE_ENUM_REASON);
            return None;
        }

        let build = |branches: &mut Vec<(Option<Fields>, String)>| -> Option<DataType> {
            if !changed {
                return None;
            }
            let mut next = e.clone();
            for ((_, variant), (fields, _)) in next
                .variants
                .iter_mut()
                .filter(|(_, variant)| !variant.skip)
                .zip(branches.iter_mut())
            {
                if let Some(fields) = fields.take() {
                    variant.fields = fields;
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
            let (_, runtime) = branches.pop()?;
            return Some((next_ty, runtime));
        }

        let needs = branches
            .iter()
            .map(|(_, runtime)| runtime != js_ident)
            .collect::<Vec<_>>();
        let Some(guards) = enum_branch_guards(walk.types, &active, &needs, js_ident) else {
            self.record_unsupported(walk, UNSUPPORTED_ENUM_REASON);
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

    /// Record the first unsupported construct, attaching the named type
    /// currently being expanded so the diagnostic can name it.
    fn record_unsupported(&self, walk: &mut Walk<'_>, reason: &'static str) {
        if walk.error.is_some() {
            return;
        }

        let mut error = Error::unsupported_semantic_transform(reason);
        if let Some(ndt) = walk.current {
            error = error.with_named_datatype(ndt);
        }
        walk.error = Some(error);
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

/// Mutable state shared across one `apply_serialize` / `apply_deserialize` call.
struct Walk<'a> {
    transform_for_rule: fn(&Rule) -> &Option<Transform>,
    remap_bigint: fn() -> DataType,
    types: &'a Types,
    /// Named types currently being expanded, used to cut recursive cycles.
    stack: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    /// The first unsupported construct encountered, if any.
    error: Option<Error>,
    /// Set when a recursive reference was cut short by [`Walk::stack`].
    recursion_cut: bool,
    /// The named type currently being expanded, used to name errors.
    current: Option<&'a NamedDataType>,
}

const UNSUPPORTED_ENUM_REASON: &str = "the enum's wire shape cannot be discriminated without guessing (for example overlapping `#[serde(untagged)]` variants); use a tagged representation or apply the rule to a non-ambiguous position";
const RECURSIVE_ENUM_REASON: &str = "the enum is recursive, so a single expression cannot transform nested values without the exported type claiming more than the runtime value provides";

/// How a single enum branch is selected at runtime.
enum BranchGuard {
    /// A JavaScript expression that is truthy exactly for this variant.
    Match(String),
    /// The variant is the unguarded fallback: either a remapped untagged
    /// variant whose runtime shape may differ from its wire shape, or (for
    /// tagged enums) nothing.
    Else,
}

/// Work out how to select each active variant from its wire value.
///
/// Prefers a shared discriminator field (internally/adjacently tagged enums),
/// then the external representation's per-variant key or string literal, then
/// structural discrimination of untagged variants. Returns `None` when no
/// unambiguous discrimination exists.
fn enum_branch_guards(
    types: &Types,
    active: &[&(Cow<'static, str>, Variant)],
    needs: &[bool],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    tag_branch_guards(active, js_ident)
        .or_else(|| external_branch_guards(active, js_ident))
        .or_else(|| untagged_branch_guards(types, active, needs, js_ident))
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
    let (key, literals) = common_tag_key(&candidates)?;

    Some(
        literals
            .iter()
            .map(|literal| {
                BranchGuard::Match(format!(
                    "{js_ident}!=null&&{}==={}",
                    js_property_access(js_ident, &key),
                    js_string_literal(literal),
                ))
            })
            .collect(),
    )
}

/// Guards for the external representation: a non-unit variant is keyed by its
/// serialized name, while a unit (or collapsed) variant is the bare string.
/// The discriminators must be distinct so overlapping shapes are rejected
/// rather than mis-dispatched.
fn external_branch_guards(
    active: &[&(Cow<'static, str>, Variant)],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    let mut discriminators = BTreeSet::new();
    let mut guards = Vec::with_capacity(active.len());

    for (_, variant) in active {
        let (discriminator, guard) = match &variant.fields {
            Fields::Named(named) if named.fields.len() == 1 => {
                let name = named.fields[0].0.to_string();
                let guard = format!(
                    "{js_ident}!=null&&{}!==undefined",
                    js_property_access(js_ident, &name),
                );
                (name, guard)
            }
            Fields::Unnamed(unnamed) if unnamed.fields.len() == 1 => {
                let literal = string_literal_of(unnamed.fields[0].ty.as_ref()?)?;
                let guard = format!("{js_ident}==={}", js_string_literal(&literal));
                (literal, guard)
            }
            _ => return None,
        };

        if !discriminators.insert(discriminator) {
            return None;
        }
        guards.push(BranchGuard::Match(guard));
    }

    Some(guards)
}

/// Guards for untagged enums, which have no discriminant and must be told
/// apart by their payload shape.
///
/// Every variant must occupy a distinct wire shape (`null`, `boolean`,
/// `number`, `string`, `Array.isArray`, object). At most one variant may have a
/// non-identity runtime transform, because a remapped payload can change its
/// runtime shape; that variant becomes the unguarded fallback. Overlapping
/// shapes are rejected rather than guessed.
fn untagged_branch_guards(
    types: &Types,
    active: &[&(Cow<'static, str>, Variant)],
    needs: &[bool],
    js_ident: &str,
) -> Option<Vec<BranchGuard>> {
    let mut shapes = Vec::with_capacity(active.len());
    let mut fallback = None;

    for (idx, (_, variant)) in active.iter().enumerate() {
        let shape = untagged_payload_shape(types, variant)?;
        if needs[idx] && fallback.replace(idx).is_some() {
            return None;
        }
        shapes.push(shape);
    }

    let mut seen = BTreeSet::new();
    for shape in &shapes {
        if !seen.insert(*shape) {
            return None;
        }
    }

    Some(
        shapes
            .iter()
            .enumerate()
            .map(|(idx, shape)| {
                if Some(idx) == fallback {
                    BranchGuard::Else
                } else {
                    BranchGuard::Match(shape_guard(*shape, js_ident))
                }
            })
            .collect(),
    )
}

/// The wire shape of an untagged variant's payload.
fn untagged_payload_shape(types: &Types, variant: &Variant) -> Option<UntaggedShape> {
    match &variant.fields {
        Fields::Unit => Some(UntaggedShape::Null),
        Fields::Named(_) => Some(UntaggedShape::Object),
        Fields::Unnamed(unnamed) => {
            let mut live = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
            let first = live.next()?;
            if live.next().is_some() {
                return Some(UntaggedShape::Array);
            }
            wire_shape(types, first, WIRE_SHAPE_DEPTH)
        }
    }
}

/// The JavaScript value shape a wire datatype occupies, resolving newtype
/// wrappers and named references to the underlying wire type. Returns `None`
/// for shapes that cannot be classified (for example nullable payloads).
fn wire_shape(types: &Types, ty: &DataType, depth: usize) -> Option<UntaggedShape> {
    if depth == 0 {
        return None;
    }

    match ty {
        DataType::Primitive(primitive) => Some(primitive_shape(primitive)),
        DataType::Tuple(tuple) if tuple.elements.is_empty() => Some(UntaggedShape::Null),
        DataType::List(_) => Some(UntaggedShape::Array),
        DataType::Map(_) => Some(UntaggedShape::Object),
        DataType::Struct(s) => match &s.fields {
            Fields::Named(_) => Some(UntaggedShape::Object),
            Fields::Unit => Some(UntaggedShape::Null),
            Fields::Unnamed(unnamed) => {
                let mut live = unnamed.fields.iter().filter_map(|field| field.ty.as_ref());
                let first = live.next()?;
                if live.next().is_some() {
                    Some(UntaggedShape::Array)
                } else {
                    wire_shape(types, first, depth - 1)
                }
            }
        },
        DataType::Reference(Reference::Named(r)) => {
            let source = Configuration::reference_source_dt(types, r);
            wire_shape(types, &source, depth - 1)
        }
        _ => None,
    }
}

const WIRE_SHAPE_DEPTH: usize = 32;

fn primitive_shape(primitive: &Primitive) -> UntaggedShape {
    match primitive {
        Primitive::bool => UntaggedShape::Boolean,
        Primitive::str | Primitive::char => UntaggedShape::String,
        Primitive::i8
        | Primitive::i16
        | Primitive::i32
        | Primitive::i64
        | Primitive::i128
        | Primitive::isize
        | Primitive::u8
        | Primitive::u16
        | Primitive::u32
        | Primitive::u64
        | Primitive::u128
        | Primitive::usize
        | Primitive::f16
        | Primitive::f32
        | Primitive::f64
        | Primitive::f128 => UntaggedShape::Number,
    }
}

/// A JavaScript value shape used to tell untagged variants apart.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UntaggedShape {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

fn shape_guard(shape: UntaggedShape, js_ident: &str) -> String {
    match shape {
        UntaggedShape::Null => format!("{js_ident}===null"),
        UntaggedShape::Boolean => format!("typeof {js_ident}===\"boolean\""),
        UntaggedShape::Number => format!("typeof {js_ident}===\"number\""),
        UntaggedShape::String => format!("typeof {js_ident}===\"string\""),
        UntaggedShape::Array => format!("Array.isArray({js_ident})"),
        UntaggedShape::Object => {
            format!("{js_ident}!=null&&typeof {js_ident}===\"object\"&&!Array.isArray({js_ident})")
        }
    }
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
/// shape.
fn variant_tag_candidates(variant: &Variant) -> Vec<(String, String)> {
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
    candidates: &mut Vec<(String, String)>,
) {
    for (name, field) in fields {
        if field.optional {
            continue;
        }
        let Some(ty) = &field.ty else { continue };
        if let Some(literal) = string_literal_of(ty) {
            candidates.push((name.to_string(), literal));
        }
    }
}

/// Descend the intersection/struct wrappers an internally tagged tuple variant
/// is rewritten into so its tag field is visible.
fn collect_datatype_tag_candidates(ty: &DataType, candidates: &mut Vec<(String, String)>) {
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

/// The key shared by every variant with distinct literal values, if any, plus
/// the literal for each variant in order.
fn common_tag_key(candidates: &[Vec<(String, String)>]) -> Option<(String, Vec<String>)> {
    let first = candidates.first()?;

    'keys: for (key, _) in first {
        let mut literals = Vec::with_capacity(candidates.len());
        let mut seen = BTreeSet::new();
        for variant in candidates {
            let matching = variant
                .iter()
                .filter(|(candidate, _)| candidate == key)
                .collect::<Vec<_>>();
            let [(_, literal)] = matching.as_slice() else {
                continue 'keys;
            };
            if !seen.insert(literal.clone()) {
                continue 'keys;
            }
            literals.push(literal.clone());
        }

        return Some((key.clone(), literals));
    }

    None
}
