use std::borrow::Cow;

use super::{DataType, Deprecated};

/// Contains type information about a function annotated with the `#[specta]` attribute.
/// Returned by the `fn_datatype!` macro.
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

impl Function {
    /// Is this function defined with the `async` keyword?
    pub fn asyncness(&self) -> bool {
        self.asyncness
    }

    /// Set the `async` status of the function.
    pub fn set_asyncness(&mut self, asyncness: bool) {
        self.asyncness = asyncness;
    }

    /// Get the name of the function.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// Get a mutable reference to the name of the function.
    pub fn name_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.name
    }

    /// Set the name of the function.
    pub fn set_name(&mut self, name: Cow<'static, str>) {
        self.name = name;
    }

    /// Get the arguments of the function.
    pub fn args(&self) -> &[(Cow<'static, str>, DataType)] {
        &self.args
    }

    /// Get the arguments of the function as mutable references.
    pub fn args_mut(&mut self) -> &mut Vec<(Cow<'static, str>, DataType)> {
        &mut self.args
    }

    /// Get the result of the function.
    pub fn result(&self) -> Option<&DataType> {
        self.result.as_ref()
    }

    /// Get the result of the function as mutable reference.
    pub fn result_mut(&mut self) -> Option<&mut DataType> {
        self.result.as_mut()
    }

    /// Set the result of the function.
    pub fn set_result(&mut self, result: DataType) {
        self.result = Some(result);
    }

    /// Get the documentation of the function.
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// Get the documentation of the function as mutable reference.
    pub fn docs_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.docs
    }

    /// Set the documentation of the function.
    pub fn set_docs(&mut self, docs: Cow<'static, str>) {
        self.docs = docs;
    }

    /// Get the deprecated status of the function.
    pub fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }

    /// Get the deprecated status of the function as mutable reference.
    pub fn deprecated_mut(&mut self) -> Option<&mut Deprecated> {
        self.deprecated.as_mut()
    }

    /// Set the deprecated status of the function.
    pub fn set_deprecated(&mut self, deprecated: Deprecated) {
        self.deprecated = Some(deprecated);
    }
}
