//! Helpers for generating [Type::reference] implementations.

use crate::SpectaID;

use super::{DataType, Generic};

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Reference {
    pub(crate) sid: SpectaID,
    pub(crate) generics: Vec<(Generic, DataType)>,
    pub(crate) inline: bool,
}

impl Reference {
    /// TODO: Explain invariant.
    pub fn construct(
        sid: SpectaID,
        generics: impl Into<Vec<(Generic, DataType)>>,
        inline: bool,
    ) -> Self {
        Self {
            sid,
            generics: generics.into(),
            inline,
        }
    }

    /// Get the [SpectaID] of the [NamedDataType] this [Reference] points to.
    pub fn sid(&self) -> SpectaID {
        self.sid
    }

    /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    pub fn generics(&self) -> &[(Generic, DataType)] {
        &self.generics
    }

    /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    pub fn generics_mut(&mut self) -> &mut Vec<(Generic, DataType)> {
        &mut self.generics
    }

    /// Get whether this reference should be inlined
    pub fn inline(&self) -> bool {
        self.inline
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}
