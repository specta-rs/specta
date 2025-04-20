use std::borrow::Cow;

use crate::datatype::{DataType, Fields};

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub(crate) fields: Fields,
}

impl Struct {
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
