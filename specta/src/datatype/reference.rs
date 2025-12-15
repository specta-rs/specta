//! Helpers for generating [Type::reference] implementations.

use std::{fmt, hash, sync::Arc};

use crate::{SpectaID, specta_id::SpectaIDInner};

use super::{DataType, Generic};

// TODO: Rename?
// #[derive(Debug, Clone, PartialEq)]
// pub struct ReferenceToken;

#[derive(Debug, Clone, PartialEq)]
enum OpaqueReference {
    // Static(&'static ReferenceToken),
    Dynamic(ArcId),
}

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq)]
pub struct Reference(ReferenceInner);

#[derive(Debug, Clone, PartialEq)]
enum ReferenceInner {
    Opaque(OpaqueReference),
    // TODO: Replace this
    Legacy {
        sid: SpectaID,
        generics: Vec<(Generic, DataType)>,
        inline: bool,
    },
}

impl Reference {
    // /// TODO
    // ///
    // /// TODO:
    // ///  - Explain cloning semantics
    // ///  - Explain comparison semantics
    // ///
    // pub fn new() -> Reference {
    //     Reference {
    //         id: ArcId::default(),
    //         sid: SpectaID(SpectaIDInner::Virtual(0)), // TODO: Fix this
    //         generics: Default::default(),
    //         inline: Default::default(),
    //     }
    // }

    // pub fn from(dt: ()) -> Reference {
    //     // TODO: We need to handle failure of these better.
    //     // TODO: If the exporter doesn't handle them it's an error.

    //     Reference {
    //         id: ArcId::default(),
    //         sid: SpectaID(SpectaIDInner::Virtual(0)), // TODO: Fix this
    //         generics: Default::default(),
    //         inline: Default::default(),
    //     }
    // }

    // // TODO: Rename
    // // TODO: Explain invariance?
    // // TODO: Should we seal this method???
    // //
    // // TODO: This is only valid for `static`'s not `const` types.
    // pub const fn opaque2(base: &'static ReferenceToken) -> Reference {
    //     Reference(ReferenceInner::Opaque(OpaqueReference::Static(base)))
    // }

    /// TODO
    pub fn opaque() -> Reference {
        // TODO: We need to handle failure of these better.
        // TODO: If the exporter doesn't handle them it's an error.

        // Reference {
        //     id: ArcId::default(),
        //     sid: SpectaID(SpectaIDInner::Virtual(0)), // TODO: Fix this
        //     generics: Default::default(),
        //     inline: Default::default(),
        // }

        Reference(ReferenceInner::Opaque(OpaqueReference::Dynamic(
            ArcId::default(),
        )))
    }

    /// TODO
    pub fn ref_eq(&self, other: &Reference) -> bool {
        println!("{:?} {:?}", self, other);
        match (&self.0, &other.0) {
            // (
            //     ReferenceInner::Opaque(OpaqueReference::Static(id1)),
            //     ReferenceInner::Opaque(OpaqueReference::Static(id2)),
            // ) => {
            //     println!(" - A {:p} {:p}", id1, id2);
            //     std::ptr::eq(*id1, *id2)
            // }
            (
                ReferenceInner::Opaque(OpaqueReference::Dynamic(id1)),
                ReferenceInner::Opaque(OpaqueReference::Dynamic(id2)),
            ) => Arc::ptr_eq(&id1.0, &id2.0),
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
