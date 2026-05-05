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
#[non_exhaustive]
pub struct Function {
    /// Whether the function is async.
    pub asyncness: bool,
    /// The function's name.
    pub name: Cow<'static, str>,
    /// The name and type of each of the function's arguments.
    pub args: Vec<(Cow<'static, str>, DataType)>,
    /// The return type of the function.
    pub result: Option<DataType>,
    /// The function's documentation. Detects both `///` and `#[doc = ...]` style documentation.
    pub docs: Cow<'static, str>,
    /// The deprecated status of the function.
    pub deprecated: Option<Deprecated>,
}

impl Function {
    /// Whether the function is async.
    pub fn asyncness(&self) -> bool {
        self.asyncness
    }

    /// The function's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name and type of each of the function's arguments.
    pub fn args(&self) -> &[(Cow<'static, str>, DataType)] {
        &self.args
    }

    /// The return type of the function.
    pub fn result(&self) -> Option<&DataType> {
        self.result.as_ref()
    }
}
