use std::borrow::Cow;

use specta::datatype::Reference;

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Define(pub(crate) Cow<'static, str>);

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Any;

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Unknown;

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Never;

/// Defines a custom Python type expression.
///
/// This is an advanced escape hatch. The expression is emitted verbatim.
pub fn define(raw: impl Into<Cow<'static, str>>) -> Reference {
    Reference::opaque(Define(raw.into()))
}
