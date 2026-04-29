use std::borrow::Cow;

use super::{DataType, Deprecated};

/// Runtime type information for a function annotated with `#[specta]`.
///
/// Values are produced by [`fn_datatype!`](crate::function::fn_datatype) and
/// [`collect_functions!`](crate::function::collect_functions). Function metadata
/// is intentionally separate from [`Types`](crate::Types): the function's
/// argument and result datatypes reference entries collected into the `Types`
/// value passed to those macros.
#[derive(Debug, Clone)]
pub struct Function {
    /// Whether the function is async.
    pub(crate) asyncness: bool,
    /// The function's name.
    pub(crate) name: Cow<'static, str>,
    /// The name and type of each of the function's arguments.
    pub(crate) args: Vec<(Cow<'static, str>, DataType)>,
    /// The return type of the function.
    pub(crate) result: Option<DataType>,
    /// The function's documentation. Detects both `///` and `#[doc = ...]` style documentation.
    pub(crate) docs: Cow<'static, str>,
    /// The deprecated status of the function.
    pub(crate) deprecated: Option<Deprecated>,
}

// TODO(v2): We are keeping the accessors as this submodule is likely going to go.
impl Function {
    /// Returns whether the function was declared with the `async` keyword.
    pub fn asyncness(&self) -> bool {
        self.asyncness
    }

    /// Sets whether the function should be treated as async.
    pub fn set_asyncness(&mut self, asyncness: bool) {
        self.asyncness = asyncness;
    }

    /// Returns the exported function name.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// Returns a mutable reference to the exported function name.
    pub fn name_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.name
    }

    /// Sets the exported function name.
    pub fn set_name(&mut self, name: Cow<'static, str>) {
        self.name = name;
    }

    /// Returns the exported argument names and datatypes in source order.
    pub fn args(&self) -> &[(Cow<'static, str>, DataType)] {
        &self.args
    }

    /// Returns the argument list for in-place mutation.
    pub fn args_mut(&mut self) -> &mut Vec<(Cow<'static, str>, DataType)> {
        &mut self.args
    }

    /// Returns the function result datatype, if exported.
    pub fn result(&self) -> Option<&DataType> {
        self.result.as_ref()
    }

    /// Returns the result datatype for in-place mutation, if exported.
    pub fn result_mut(&mut self) -> Option<&mut DataType> {
        self.result.as_mut()
    }

    /// Sets the function result datatype.
    pub fn set_result(&mut self, result: DataType) {
        self.result = Some(result);
    }

    /// Returns documentation collected from the function item.
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// Returns function documentation for in-place mutation.
    pub fn docs_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.docs
    }

    /// Sets function documentation.
    pub fn set_docs(&mut self, docs: Cow<'static, str>) {
        self.docs = docs;
    }

    /// Returns deprecation metadata for the function, if present.
    pub fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }

    /// Returns deprecation metadata for in-place mutation, if present.
    pub fn deprecated_mut(&mut self) -> Option<&mut Deprecated> {
        self.deprecated.as_mut()
    }

    /// Sets deprecation metadata for the function.
    pub fn set_deprecated(&mut self, deprecated: Deprecated) {
        self.deprecated = Some(deprecated);
    }
}
