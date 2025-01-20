//! Helpers for generating [Type::reference] implementations.

use crate::SpectaID;

use super::DataType;

/// A reference datatype.
///
/// TODO: Explain how to construct this.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
    pub(crate) generics: Vec<DataType>,
    // When creating a reference we generate a `DataType` replacing all `DataType::Generic` with the current generics.
    // This allows us to avoid a runtime find-and-replace on it.
    pub(crate) dt: Box<DataType>,
    // TODO: This is a Typescript-specific thing
    pub(crate) inline: bool,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(sid: SpectaID, generics: impl Into<Vec<DataType>>, dt: DataType, inline: bool) -> Self {
        Self { sid, generics: generics.into(), dt: Box::new(dt), inline, }
    }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn generics(&self) -> &[DataType] {
        &self.generics
    }

    pub fn inline(&self) -> bool {
        self.inline
    }

    pub fn datatype(&self) -> &DataType {
        &self.dt
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}
