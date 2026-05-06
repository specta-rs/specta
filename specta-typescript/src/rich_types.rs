use std::{borrow::Cow, fmt, sync::Arc};

use specta::{
    Type, Types,
    datatype::{DataType, Fields, NamedReferenceType, Primitive, Reference},
};

use crate::define;

/// A rich type runtime JS transformer function.
///
/// This defines a JS function which can convert between the incoming/outgoing type and it's JSON representation.
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
    lossless_bigint: bool,
    lossless_floats: bool,
}

impl Default for RichTypesConfiguration {
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

impl RichTypesConfiguration {
    /// Construct a [`RichTypesConfiguration`] without the default rules.
    ///
    /// You should prefer [`RichTypesConfiguration::default`] when possible as the default rules are likely to be more suitable for your use case.
    pub fn empty() -> Self {
        Self {
            rules: Default::default(),
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
        }

        self
    }

    /// Transform a [`Types`] collection using the configured rules.
    ///
    /// This will ensure all of your types match the values match what the JS transform will output.
    pub fn apply_types<'a>(&self, types: &'a Types) -> Cow<'a, Types> {
        let mut types = Cow::Borrowed(types);

        if self.has_builtin_remaps() {
            types = Cow::Owned(types.into_owned().map(|mut ndt| {
                let phase = if ndt.name.ends_with("_Serialize") {
                    Phase::Serialize
                } else {
                    Phase::Deserialize
                };

                ndt.generics.to_mut().iter_mut().for_each(|generic| {
                    if let Some(dt) = &mut generic.default {
                        apply_builtin_remaps(dt, phase, self.lossless_bigint, self.lossless_floats);
                    }
                });
                if let Some(dt) = &mut ndt.ty {
                    apply_builtin_remaps(dt, phase, self.lossless_bigint, self.lossless_floats);
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
                NamedReferenceType::Recursive | NamedReferenceType::Reference { .. } => {}
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
            DataType::Reference(Reference::Named(r)) => {
                if let Some(rule) = self.rule_for_reference(types, r) {
                    return Some((
                        Some((rule.data_type.0)(Self::reference_source_dt(types, r))),
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

                match &r.inner {
                    NamedReferenceType::Inline { dt, .. } => {
                        self.apply_inner(phase, types, dt, js_ident, stack)
                    }
                    NamedReferenceType::Recursive => None,
                    NamedReferenceType::Reference { .. } => {
                        let ndt = types.get(r)?;

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
                }
            }
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
                        if runtime != field_ident {
                            parts.push(format!("{name}:{runtime}"));
                        }
                    }

                    if parts.is_empty() {
                        changed.then_some((Some(DataType::Struct(ty)), js_ident.to_owned()))
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

                            (runtime != field_ident).then_some(format!("{idx}:{runtime}"))
                        })
                        .collect::<Vec<_>>();

                    if parts.is_empty() {
                        changed.then_some((Some(DataType::Struct(ty)), js_ident.to_owned()))
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
                        (runtime != ident).then_some(format!("{idx}:{runtime}"))
                    })
                    .collect::<Vec<_>>();

                if parts.is_empty() {
                    changed.then_some((Some(DataType::Tuple(ty)), js_ident.to_owned()))
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

        self.apply_builtin_remaps(phase, dt, js_ident, result)
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
            NamedReferenceType::Reference { .. } | NamedReferenceType::Recursive => types
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
        phase: Phase,
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
            phase,
            self.lossless_bigint,
            self.lossless_floats,
        );

        if remapped == source {
            result
        } else if let Some((_, runtime)) = result {
            Some((Some(remapped), runtime))
        } else {
            Some((Some(remapped), js_ident.to_owned()))
        }
    }
}

fn apply_builtin_remaps(
    dt: &mut DataType,
    phase: Phase,
    lossless_bigint: bool,
    lossless_floats: bool,
) {
    if let DataType::Primitive(primitive) = dt
        && let Some(remapped) =
            remap_primitive(primitive.clone(), phase, lossless_bigint, lossless_floats)
    {
        *dt = remapped;
        return;
    }

    match dt {
        DataType::Primitive(_) | DataType::Generic(_) => {}
        DataType::List(list) => {
            apply_builtin_remaps(&mut list.ty, phase, lossless_bigint, lossless_floats)
        }
        DataType::Map(map) => {
            apply_builtin_remaps(map.key_ty_mut(), phase, lossless_bigint, lossless_floats);
            apply_builtin_remaps(map.value_ty_mut(), phase, lossless_bigint, lossless_floats);
        }
        DataType::Struct(s) => {
            apply_builtin_remaps_to_fields(&mut s.fields, phase, lossless_bigint, lossless_floats)
        }
        DataType::Enum(e) => {
            for (_, variant) in &mut e.variants {
                apply_builtin_remaps_to_fields(
                    &mut variant.fields,
                    phase,
                    lossless_bigint,
                    lossless_floats,
                );
            }
        }
        DataType::Tuple(tuple) => {
            for dt in &mut tuple.elements {
                apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
            }
        }
        DataType::Nullable(dt) => {
            apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
        }
        DataType::Intersection(dts) => {
            for dt in dts {
                apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
            }
        }
        DataType::Reference(reference) => {
            let Reference::Named(reference) = reference else {
                return;
            };

            match &mut reference.inner {
                NamedReferenceType::Recursive => {}
                NamedReferenceType::Inline { dt, .. } => {
                    apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
                }
                NamedReferenceType::Reference { generics, .. } => {
                    for (_, dt) in generics {
                        apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
                    }
                }
            }
        }
    }
}

fn apply_builtin_remaps_to_fields(
    fields: &mut Fields,
    phase: Phase,
    lossless_bigint: bool,
    lossless_floats: bool,
) {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(dt) = &mut field.ty {
                    apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
                }
            }
        }
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(dt) = &mut field.ty {
                    apply_builtin_remaps(dt, phase, lossless_bigint, lossless_floats);
                }
            }
        }
    }
}

fn remap_primitive(
    primitive: Primitive,
    phase: Phase,
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
        return Some(match phase {
            Phase::Serialize => crate::define("bigint | number").into(),
            Phase::Deserialize => Reference::opaque(crate::opaque::BigInt).into(),
        });
    }

    if lossless_floats && matches!(primitive, Primitive::f16 | Primitive::f32 | Primitive::f64) {
        return Some(Reference::opaque(crate::opaque::Number).into());
    }

    None
}

fn spread_transform(js_ident: &str, mut parts: Vec<String>) -> String {
    if !js_ident.is_empty() {
        parts.insert(0, format!("...{js_ident}"));
    }
    format!("({{{}}})", parts.join(","))
}
