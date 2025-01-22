use std::borrow::Cow;

use super::{DataType, Fields, GenericType, NamedDataType, SpectaID};

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub(crate) name: Cow<'static, str>,
    // Associating a SpectaID will allow exporter to lookup more detailed information about the type to provide better errors.
    pub(crate) sid: Option<SpectaID>,
    pub(crate) generics: Vec<GenericType>,
    pub(crate) fields: Fields,
}

impl StructType {
    /// Convert a [`StructType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Struct(self)
    }

    /// Convert a [`StructType`] to a named [`NamedDataType`].
    ///
    /// This can easily be converted to a [`DataType`] by putting it inside the [DataType::Named] variant.
    pub fn to_named(self, name: impl Into<Cow<'static, str>>) -> NamedDataType {
        NamedDataType {
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            ext: None,
            inner: DataType::Struct(self),
        }
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn sid(&self) -> Option<SpectaID> {
        self.sid
    }

    pub fn generics(&self) -> &Vec<GenericType> {
        &self.generics
    }

    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    pub fn tag(&self) -> Option<&Cow<'static, str>> {
        match &self.fields {
            Fields::Unit => None,
            Fields::Unnamed(_) => None,
            Fields::Named(named) => named.tag.as_ref(),
        }
    }
}

impl From<StructType> for DataType {
    fn from(t: StructType) -> Self {
        t.to_anonymous()
    }
}
