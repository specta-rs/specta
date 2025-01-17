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
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(sid: SpectaID, generics: impl Into<Vec<DataType>>) -> Self {
        Self { sid, generics: generics.into() }
    }

    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    pub fn generics(&self) -> &[DataType] {
        &self.generics
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}
