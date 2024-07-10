use std::borrow::Cow;

use crate::{DataType, DeprecatedType};

/// Contains type information about a function annotated with [`specta`](macro@crate::specta).
/// Returned by [`fn_datatype`].
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
    pub(crate) deprecated: Option<DeprecatedType>,
}

impl Function {
    pub fn asyncness(&self) -> bool {
        self.asyncness
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn args(&self) -> impl Iterator<Item = &(Cow<'static, str>, DataType)> {
        self.args.iter()
    }

    pub fn result(&self) -> Option<&DataType> {
        self.result.as_ref()
    }

    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }
}
