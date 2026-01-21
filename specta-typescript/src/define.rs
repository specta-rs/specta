use std::borrow::Cow;

use specta::datatype::Reference;

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Define(pub(crate) Cow<'static, str>);

/// Define a custom Typescript string which can be used as a `DataType::Reference`.
///
/// This is an advanced feature which should be used with caution.
pub fn define(raw: impl Into<Cow<'static, str>>) -> Reference {
    Reference::opaque(Define(raw.into()))
}
