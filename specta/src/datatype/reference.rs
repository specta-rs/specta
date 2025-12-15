//! Helpers for generating [Type::reference] implementations.

use std::{fmt, hash, sync::Arc};

use crate::{SpectaID, specta_id::SpectaIDInner};

use super::{DataType, Generic};

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq)]
pub struct Reference(ReferenceInner);

#[derive(Debug, Clone, PartialEq)]
enum ReferenceInner {
    Opaque(ArcId),
    // TODO: Replace this
    Legacy {
        sid: SpectaID,
        generics: Vec<(Generic, DataType)>,
        inline: bool,
    },
}

impl Reference {
    /// Construct a new reference to an opaque type.
    ///
    /// An opaque type is unable to represents using the [DataType] system and requires specific exporter integration to handle it.
    ///
    /// An opaque [Reference] is equal when cloned and can be compared using the [Self::ref_eq] or [PartialEq].
    ///
    pub fn opaque() -> Reference {
        Reference(ReferenceInner::Opaque(ArcId::default()))
    }

    /// Compare if two references are pointing to the same type.
    ///
    /// Unlike `PartialEq::eq`, this method only compares the types, not the generics and inline attributes.
    pub fn ref_eq(&self, other: &Reference) -> bool {
        match (&self.0, &other.0) {
            (ReferenceInner::Opaque(id1), ReferenceInner::Opaque(id2)) => {
                Arc::ptr_eq(&id1.0, &id2.0)
            }
            _ => false,
        }
    }

    // TODO: Remove this method
    /// TODO: Explain invariant.
    pub fn construct(
        sid: SpectaID,
        generics: impl Into<Vec<(Generic, DataType)>>,
        inline: bool,
    ) -> Self {
        Self(ReferenceInner::Legacy {
            sid,
            generics: generics.into(),
            inline,
        })
    }

    /// Get the [SpectaID] of the [NamedDataType] this [Reference] points to.
    pub fn sid(&self) -> SpectaID {
        match &self.0 {
            ReferenceInner::Opaque { .. } => SpectaID(SpectaIDInner::Virtual(0)), // TODO: Fix this
            ReferenceInner::Legacy { sid, .. } => *sid,
        }
    }

    /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    pub fn generics(&self) -> &[(Generic, DataType)] {
        match &self.0 {
            ReferenceInner::Opaque { .. } => &[],
            ReferenceInner::Legacy { generics, .. } => generics,
        }
    }

    /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    pub fn generics_mut(&mut self) -> &mut Vec<(Generic, DataType)> {
        match &mut self.0 {
            ReferenceInner::Opaque { .. } => todo!(), // TODO: Fix this
            ReferenceInner::Legacy { generics, .. } => generics,
        }
    }

    /// Get whether this reference should be inlined
    pub fn inline(&self) -> bool {
        match &self.0 {
            ReferenceInner::Opaque { .. } => false,
            ReferenceInner::Legacy { inline, .. } => *inline,
        }
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}

#[derive(Clone, Default)]
struct ArcId(Arc<()>);

impl PartialEq for ArcId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ArcId {}

impl hash::Hash for ArcId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state)
    }
}

impl fmt::Debug for ArcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArcId({:?})", Arc::as_ptr(&self.0))
    }
}
