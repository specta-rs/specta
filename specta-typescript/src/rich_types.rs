//! TODO

use std::{borrow::Cow, fmt, sync::Arc};

use specta::{
    Type, Types,
    datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference},
};

mod rules;

/// A transformer. Defines both a type and a runtime transformer.
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
    pub fn new(runtime: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(runtime)))
    }

    /// Construct an identity runtime transform.
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

#[derive(Debug, Clone, Copy)]
enum Phase {
    Serialize,
    Deserialize,
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
pub(crate) struct Rule {
    /// NDT's name
    pub(crate) name: Cow<'static, str>,
    /// NDT's module path
    pub(crate) module_path: Cow<'static, str>,
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

/// TODO
#[derive(Debug, Clone)]
pub struct RichTypesConfiguration {
    rules: Vec<Rule>,
    remapper: Remapper,
    lossless_bigint: bool,
    lossless_floats: bool,
}

#[derive(Debug, Clone, Default)]
struct Remapper {
    rules: Vec<RemapRule>,
}

#[derive(Debug, Clone)]
struct RemapRule {
    from: DataType,
    serialize_to: DataType,
    deserialize_to: DataType,
}

impl Remapper {
    fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    fn rule(&mut self, from: DataType, serialize_to: DataType, deserialize_to: DataType) {
        self.rules.push(RemapRule {
            from,
            serialize_to,
            deserialize_to,
        });
    }

    fn remap_dt(&self, phase: Phase, mut dt: DataType) -> DataType {
        self.remap_internal(phase, &mut dt);
        dt
    }

    fn remap_types(&self, types: Types) -> Types {
        types.map(|mut ndt| {
            let phase = if ndt.name.ends_with("_Serialize") {
                Phase::Serialize
            } else {
                Phase::Deserialize
            };

            ndt.generics.to_mut().iter_mut().for_each(|generic| {
                if let Some(dt) = &mut generic.default {
                    self.remap_internal(phase, dt);
                }
            });
            if let Some(dt) = &mut ndt.ty {
                self.remap_internal(phase, dt);
            }
            ndt
        })
    }

    fn remap_internal(&self, phase: Phase, dt: &mut DataType) {
        self.remap_rules(phase, dt);

        match dt {
            DataType::Primitive(_) | DataType::Generic(_) => {}
            DataType::List(list) => self.remap_internal(phase, &mut list.ty),
            DataType::Map(map) => {
                self.remap_internal(phase, map.key_ty_mut());
                self.remap_internal(phase, map.value_ty_mut());
            }
            DataType::Struct(s) => self.remap_fields(phase, &mut s.fields),
            DataType::Enum(e) => {
                for (_, variant) in &mut e.variants {
                    self.remap_fields(phase, &mut variant.fields);
                }
            }
            DataType::Tuple(tuple) => {
                for dt in &mut tuple.elements {
                    self.remap_internal(phase, dt);
                }
            }
            DataType::Nullable(dt) => self.remap_internal(phase, dt),
            DataType::Intersection(dts) => {
                for dt in dts {
                    self.remap_internal(phase, dt);
                }
            }
            DataType::Reference(reference) => {
                let Reference::Named(reference) = reference else {
                    return;
                };

                match &mut reference.inner {
                    NamedReferenceType::Recursive => {}
                    NamedReferenceType::Inline { dt, .. } => self.remap_internal(phase, dt),
                    NamedReferenceType::Reference { generics, .. } => {
                        for (_, dt) in generics {
                            self.remap_internal(phase, dt);
                        }
                    }
                }
            }
        }
    }

    fn remap_rules(&self, phase: Phase, dt: &mut DataType) {
        for rule in &self.rules {
            if *dt == rule.from {
                *dt = match phase {
                    Phase::Serialize => rule.serialize_to.clone(),
                    Phase::Deserialize => rule.deserialize_to.clone(),
                };
            }
        }
    }

    fn remap_fields(&self, phase: Phase, fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for field in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.remap_internal(phase, dt);
                    }
                }
            }
            Fields::Named(fields) => {
                for (_, field) in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.remap_internal(phase, dt);
                    }
                }
            }
        }
    }
}

impl RichTypesConfiguration {
    /// Construct a [`RichTypesConfiguration`] without the default rules.
    ///
    /// You should prefer [`RichTypesConfiguration::default`] when possible as the default rules are likely to be more suitable for your use case.
    pub fn empty() -> Self {
        Self {
            rules: Default::default(),
            remapper: Remapper::default(),
            lossless_bigint: false,
            lossless_floats: false,
        }
    }

    /// Define a new rule for a given type `T`.
    ///
    /// Note: this will only worked if applied to a named type like generated by the `Type` macro. It will not work with primitives.
    ///
    pub fn define<T: Type>(
        &mut self,
        dt: impl Fn(DataType) -> DataType + Send + Sync + 'static,
        serialize: Option<Transform>,
        deserialize: Option<Transform>,
    ) -> &mut Self {
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

    /// Enable lossless support for large number types (BigInt's).
    ///
    /// This assumes your runtime is configured to handle losslessly transmitting the `BigInt`'s.
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for the implementation details around this.
    pub fn enable_lossless_bigints(&mut self) -> &mut Self {
        if !self.lossless_bigint {
            self.lossless_bigint = true;

            for primitive in [
                Primitive::usize,
                Primitive::isize,
                Primitive::u64,
                Primitive::i64,
                Primitive::u128,
                Primitive::i128,
            ] {
                self.remapper.rule(
                    DataType::Primitive(primitive),
                    crate::define("bigint | number").into(),
                    Reference::opaque(crate::opaque::BigInt).into(),
                );
            }
        }

        self
    }

    /// Enable lossless float support.
    ///
    /// By enabling this your asserting your runtime *MUST* preserve `NaN`, `Infinity` and `-Infinity` values from JS to Rust. This constrain is REQUIRED for this to not have runtime issues.
    ///
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for the implementation details around this.
    pub fn enable_lossless_floats(&mut self) -> &mut Self {
        if !self.lossless_floats {
            self.lossless_floats = true;

            // We don't map `f128` as it just can't exist in JS.
            for primitive in [Primitive::f16, Primitive::f32, Primitive::f64] {
                // This remaps `number | null` to `number`, which is correct if the runtime can handle it.
                let number: DataType = Reference::opaque(crate::opaque::Number).into();
                self.remapper
                    .rule(DataType::Primitive(primitive), number.clone(), number);
            }
        }

        self
    }

    /// Transform a [`Types`] collection using the configured rules.
    ///
    /// This will ensure all of your types match the values match what the JS transform will output.
    pub fn apply_types<'a>(&self, types: &'a Types) -> Cow<'a, Types> {
        let mut types = Cow::Borrowed(types);

        if !self.remapper.is_empty() {
            types = Cow::Owned(self.remapper.remap_types(types.into_owned()));
        }

        if !self.rules.is_empty() {
            types = Cow::Owned(types.into_owned().map(|mut ndt| {
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

    /// Scan a [`DataType`] tree applying all rules and building a JS runtime transform, and an updated type.
    ///
    /// This assumes [`RichTypesConfiguration::apply_types`] has already been applied to the [`Types`].
    /// Therefore the type updates will be shallow (up until references to the `Types`).
    ///
    /// The JavaScript transform will be deeply nested and will be applied around the JS identifier which is provided.
    ///
    /// If no rules are matched, `None` is returned, if the type doesn't require transformations, `Some((None, runtime_str))` is returned.
    ///
    pub fn apply_serialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.apply_inner(Phase::Serialize, types, dt, js_ident, &mut Vec::new())
    }

    /// Scan a [`DataType`] tree applying deserialize-facing rules.
    pub fn apply_deserialize(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.apply_inner(Phase::Deserialize, types, dt, js_ident, &mut Vec::new())
    }

    /// Scan a [`DataType`] tree applying serialize-facing rules.
    pub fn apply(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        self.apply_serialize(types, dt, js_ident)
    }

    fn apply_inner(
        &self,
        phase: Phase,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
        stack: &mut Vec<(Cow<'static, str>, Cow<'static, str>)>,
    ) -> Option<(Option<DataType>, String)> {
        let result = match dt {
            DataType::Reference(Reference::Named(r)) => match &r.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    self.apply_inner(phase, types, dt, js_ident, stack)
                }
                NamedReferenceType::Recursive => None,
                NamedReferenceType::Reference { .. } => {
                    let ndt = types.get(r)?;
                    let rule = self
                        .rules
                        .iter()
                        .find(|r| r.name == ndt.name && r.module_path == ndt.module_path);

                    if let Some(rule) = rule {
                        return Some((
                            ndt.ty.clone().map(|dt| (rule.data_type.0)(dt)),
                            match phase {
                                Phase::Serialize => &rule.serialize,
                                Phase::Deserialize => &rule.deserialize,
                            }
                            .as_ref()
                            .map_or_else(
                                || js_ident.to_owned(),
                                |transform| transform.apply(js_ident),
                            ),
                        ));
                    }

                    let ty = ndt.ty.as_ref()?;
                    let key = (ndt.name.clone(), ndt.module_path.clone());
                    if stack.contains(&key) {
                        return None;
                    }
                    stack.push(key);
                    let result = self
                        .apply_inner(phase, types, ty, js_ident, stack)
                        .map(|(_, runtime)| (None, runtime));
                    stack.pop();
                    result
                }
            },
            DataType::Struct(s) => match &s.fields {
                Fields::Named(fields) => {
                    let mut ty = s.clone();
                    let mut changed = false;
                    let mut parts = Vec::new();

                    for (name, field) in &fields.fields {
                        let Some(field_ty) = &field.ty else { continue };
                        let field_ident = format!("{js_ident}.{name}");
                        let Some((next_ty, runtime)) =
                            self.apply_inner(phase, types, field_ty, &field_ident, stack)
                        else {
                            continue;
                        };

                        if let Some(next_ty) = next_ty
                            && let Fields::Named(fields) = &mut ty.fields
                            && let Some((_, field)) =
                                fields.fields.iter_mut().find(|(n, _)| n == name)
                        {
                            field.ty = Some(next_ty);
                            changed = true;
                        }
                        parts.push(format!("{name}:{runtime}"));
                    }

                    if parts.is_empty() {
                        None
                    } else {
                        Some((
                            changed.then_some(DataType::Struct(ty)),
                            spread_transform(js_ident, parts),
                        ))
                    }
                }
                Fields::Unnamed(fields) => {
                    let mut ty = s.clone();
                    let mut changed = false;
                    let parts = fields
                        .fields
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, field)| {
                            let field_ty = field.ty.as_ref()?;
                            let field_ident = format!("{js_ident}[{idx}]");
                            let (next_ty, runtime) =
                                self.apply_inner(phase, types, field_ty, &field_ident, stack)?;

                            if let Some(next_ty) = next_ty
                                && let Fields::Unnamed(fields) = &mut ty.fields
                            {
                                fields.fields[idx].ty = Some(next_ty);
                                changed = true;
                            }

                            Some(format!("{idx}:{runtime}"))
                        })
                        .collect::<Vec<_>>();

                    if parts.is_empty() {
                        None
                    } else {
                        Some((
                            changed.then_some(DataType::Struct(ty)),
                            spread_transform(js_ident, parts),
                        ))
                    }
                }
                Fields::Unit => None,
            },
            DataType::Tuple(tuple) => {
                let mut ty = tuple.clone();
                let mut changed = false;
                let parts = tuple
                    .elements
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, element)| {
                        let ident = format!("{js_ident}[{idx}]");
                        let (next_ty, runtime) =
                            self.apply_inner(phase, types, element, &ident, stack)?;
                        if let Some(next_ty) = next_ty {
                            ty.elements[idx] = next_ty;
                            changed = true;
                        }
                        Some(format!("{idx}:{runtime}"))
                    })
                    .collect::<Vec<_>>();

                if parts.is_empty() {
                    None
                } else {
                    Some((
                        changed.then_some(DataType::Tuple(ty)),
                        spread_transform(js_ident, parts),
                    ))
                }
            }
            DataType::List(list) => {
                let item = "i";
                let (next_ty, runtime) = self.apply_inner(phase, types, &list.ty, item, stack)?;
                let mut ty = list.clone();
                let mut changed = false;
                if let Some(next_ty) = next_ty {
                    ty.ty = Box::new(next_ty);
                    changed = true;
                }
                Some((
                    changed.then_some(DataType::List(ty)),
                    format!("{js_ident}.map({item}=>{runtime})"),
                ))
            }
            DataType::Nullable(inner) => {
                let (next_ty, runtime) = self.apply_inner(phase, types, inner, js_ident, stack)?;
                Some((
                    next_ty.map(|dt| DataType::Nullable(Box::new(dt))),
                    format!("{js_ident}==null?{js_ident}:{runtime}"),
                ))
            }
            DataType::Intersection(items) => {
                let mut ty = items.clone();
                let mut changed = false;
                let parts = items
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, item)| {
                        let (next_ty, runtime) =
                            self.apply_inner(phase, types, item, js_ident, stack)?;
                        if let Some(next_ty) = next_ty {
                            ty[idx] = next_ty;
                            changed = true;
                        }
                        Some(runtime)
                    })
                    .collect::<Vec<_>>();

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
            DataType::Map(_)
            | DataType::Enum(_)
            | DataType::Primitive(_)
            | DataType::Generic(_)
            | DataType::Reference(Reference::Opaque(_)) => None,
        };

        self.apply_remapper(phase, dt, js_ident, result)
    }

    fn apply_remapper(
        &self,
        phase: Phase,
        dt: &DataType,
        js_ident: &str,
        result: Option<(Option<DataType>, String)>,
    ) -> Option<(Option<DataType>, String)> {
        if self.remapper.is_empty() {
            return result;
        }

        let remapped = self.remapper.remap_dt(
            phase,
            result
                .as_ref()
                .and_then(|(dt, _)| dt.clone())
                .unwrap_or_else(|| dt.clone()),
        );

        if remapped == *dt {
            result
        } else if let Some((_, runtime)) = result {
            Some((Some(remapped), runtime))
        } else {
            Some((Some(remapped), js_ident.to_owned()))
        }
    }
}

fn spread_transform(js_ident: &str, mut parts: Vec<String>) -> String {
    if !js_ident.is_empty() {
        parts.insert(0, format!("...{js_ident}"));
    }
    format!("{{{}}}", parts.join(","))
}
