//! TODO

use std::{borrow::Cow, fmt, sync::Arc};

use specta::{
    Type, Types,
    datatype::{DataType, Primitive, Reference},
};

mod rules;

/// A rule for a specific named data type.
#[derive(Clone)]
#[non_exhaustive]
pub(crate) struct Rule {
    /// NDT's name
    pub(crate) name: Cow<'static, str>,
    /// NDT's module path
    pub(crate) module_path: Cow<'static, str>,
    /// The type transformation function
    ///
    /// This must match up with the runtime.
    pub(crate) typ: Arc<dyn Fn(DataType) -> DataType + Send + Sync>,
    /// The runtime transform function
    ///
    /// This is called with the argument being a Typescript identifier.
    /// It should output some transformation on the identifier.
    /// Eg. `|i| format!("new Date({i})")` could be one valid implementation.
    pub(crate) runtime: Arc<dyn Fn(&str) -> String + Send + Sync>,
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("name", &self.name)
            .field("module_path", &self.module_path)
            .field("typ", &format!("{:p}", self.typ))
            .field("runtime", &format!("{:p}", self.runtime))
            .finish()
    }
}

/// TODO
#[derive(Debug, Clone)]
pub struct RichTypesConfiguration {
    rules: Vec<Rule>,
    lossless_bigint: bool,
    lossless_floats: bool,
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
    pub fn define<T: Type>(
        &mut self,
        typ: impl Fn(DataType) -> DataType + Send + Sync + 'static,
        runtime: impl Fn(&str) -> String + Send + Sync + 'static,
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
                typ: Arc::new(typ),
                runtime: Arc::new(runtime),
            });
        }

        self
    }

    /// Enable lossless support for large number types (BigInt's).
    ///
    /// This assumes your runtime is configured to handle losslessly transmitting the `BigInt`'s.
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for the implementation details around this.
    pub fn enable_lossless_bigint(&mut self) -> &mut Self {
        self.lossless_bigint = true;
        self
    }

    /// Enable lossless float support.
    ///
    /// By enabling this your asserting your runtime *MUST* preserve `NaN`, `Infinity` and `-Infinity` values from JS to Rust. This constrain is REQUIRED for this to not have runtime issues.
    ///
    /// Refer to [specta-rs/specta#203](https://github.com/specta-rs/specta/issues/203) for the implementation details around this.
    pub fn enable_lossless_floats(&mut self) -> &mut Self {
        self.lossless_floats = true;
        self
    }

    /// Transform a [`Types`] collection using the configured rules.
    ///
    /// This will ensure all of your types match the values match what the JS transform will output.
    pub fn apply_types<'a>(&self, types: &'a Types) -> Cow<'a, Types> {
        let mut types = Cow::Borrowed(types);

        if self.lossless_bigint || self.lossless_floats {
            let mut remapper = specta_util::Remapper::new();

            if self.lossless_bigint {
                for primitive in [
                    Primitive::usize,
                    Primitive::isize,
                    Primitive::u64,
                    Primitive::i64,
                    Primitive::u128,
                    Primitive::i128,
                ] {
                    // TODO: For input it should be `bigint | number` when phased is enabled??? How do we know that???

                    remapper = remapper.rule(
                        DataType::Primitive(primitive),
                        Reference::opaque(crate::opaque::BigInt).into(),
                    );
                }
            }

            if self.lossless_floats {
                // We don't map `f128` as it just can't exist in JS.
                for primitive in [Primitive::f16, Primitive::f32, Primitive::f64] {
                    // This remaps `number | null` to `number`, which is correct if the runtime can handle it.
                    remapper = remapper.rule(
                        DataType::Primitive(primitive),
                        Reference::opaque(crate::opaque::Number).into(),
                    );
                }
            }

            types = Cow::Owned(remapper.remap_types(types.into_owned()));
        }

        if !self.rules.is_empty() {
            types = Cow::Owned(types.into_owned().map(|mut ndt| {
                if let Some(rule) = self
                    .rules
                    .iter()
                    .find(|r| r.name == ndt.name && r.module_path == ndt.module_path)
                    && let Some(dt) = ndt.ty.take()
                {
                    ndt.ty = Some((rule.typ)(dt));
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
    pub fn apply(
        &self,
        types: &Types,
        dt: &DataType,
        js_ident: &str,
    ) -> Option<(Option<DataType>, String)> {
        // TODO: Scan the `DataType` tree, following references and generate a string applying the runtime transform.
        // TODO: The runtime map function will need to be deeply nested but the changes to the `DataType` can stop at references as it's assumed `Self::apply_types` was already done.
        // TODO: Maybe abstract the `Remapper` onto `Self` as `Option<Remapper>` so it can be reused here and in `apply_types`.

        todo!();
    }
}
