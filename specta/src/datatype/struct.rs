use std::borrow::Cow;

use crate::{
    datatype::{DataType, Fields, Generic},
    SpectaID,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub(crate) name: Cow<'static, str>,
    // Associating a SpectaID will allow exporter to lookup more detailed information about the type to provide better errors.
    pub(crate) sid: Option<SpectaID>,
    pub(crate) generics: Vec<Generic>,
    pub(crate) fields: Fields,
}

impl Struct {
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn sid(&self) -> Option<SpectaID> {
        self.sid
    }

    pub fn generics(&self) -> &Vec<Generic> {
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

impl From<Struct> for DataType {
    fn from(t: Struct) -> Self {
        Self::Struct(t)
    }
}
