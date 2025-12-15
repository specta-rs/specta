//! Helpers for generating [Type::reference] implementations

use std::{borrow::Cow, fmt, hash, sync::Arc};

use crate::{SpectaID, specta_id::SpectaIDInner};

use super::{DataType, Generic};

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    id: ArcId,
    generics: Cow<'static, [(Generic, DataType)]>,
    inline: bool,
}

impl Reference {
    /// Construct a new reference to an opaque type.
    ///
    /// An opaque type is unable to represents using the [DataType] system and requires specific exporter integration to handle it.
    ///
    /// This should NOT be used in a [Type::definition] method as that likely means unnecessary memory.
    ///
    /// An opaque [Reference] is equal when cloned and can be compared using the [Self::ref_eq] or [PartialEq].
    ///
    pub fn opaque() -> Reference {
        Reference {
            id: ArcId::Dynamic(Default::default()),
            // TODO: Allow these to be mutable would break invariant.
            generics: Cow::Borrowed(&[]),
            inline: false,
        }
    }

    // pub const fn todo(generics: &'static [Generic, DataT]) -> Reference {
    //     todo!();
    // }

    /// Construct a new reference to a type with a fixed reference.
    ///
    /// # Safety
    ///
    /// It's critical that this reference points to a `static ...: () = ();` which is uniquely created for this reference. If it points to a `const` or `Box::leak`d value, the reference will not maintain it's invariants.
    ///
    pub const fn unsafe_from_fixed_static_reference(s: &'static ()) -> Reference {
        // Reference(ReferenceInner::Opaque(ArcId::Static(s)))
        todo!();
    }

    /// Compare if two references are pointing to the same type.
    ///
    /// Unlike `PartialEq::eq`, this method only compares the types, not the generics and inline attributes.
    pub fn ref_eq(&self, other: &Reference) -> bool {
        // match (&self.0, &other.0) {
        //     (ReferenceInner::Opaque(id1), ReferenceInner::Opaque(id2)) => id1 == id2,
        //     _ => false,
        // }
        todo!();
    }

    // // TODO: Remove this method
    // /// TODO: Explain invariant.
    // pub fn construct(
    //     sid: SpectaID,
    //     generics: impl Into<Vec<(Generic, DataType)>>,
    //     inline: bool,
    // ) -> Self {
    //     Self(ReferenceInner::Legacy {
    //         sid,
    //         generics: generics.into(),
    //         inline,
    //     })
    // }

    // /// Get the [SpectaID] of the [NamedDataType] this [Reference] points to.
    // pub fn sid(&self) -> SpectaID {
    //     match &self.0 {
    //         ReferenceInner::Opaque { .. } => SpectaID(SpectaIDInner::Virtual(0)), // TODO: Fix this
    //         ReferenceInner::Legacy { sid, .. } => *sid,
    //     }
    // }

    // /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    // pub fn generics(&self) -> &[(Generic, DataType)] {
    //     match &self.0 {
    //         ReferenceInner::Opaque { .. } => &[],
    //         ReferenceInner::Legacy { generics, .. } => generics,
    //     }
    // }

    // /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    // pub fn generics_mut(&mut self) -> &mut Vec<(Generic, DataType)> {
    //     match &mut self.0 {
    //         ReferenceInner::Opaque { .. } => todo!(), // TODO: Fix this
    //         ReferenceInner::Legacy { generics, .. } => generics,
    //     }
    // }

    // /// Get whether this reference should be inlined
    // pub fn inline(&self) -> bool {
    //     match &self.0 {
    //         ReferenceInner::Opaque { .. } => false,
    //         ReferenceInner::Legacy { inline, .. } => *inline,
    //     }
    // }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}

/// `Arc<()>` is a great way of creating a virtual ID which
/// can be compared to itself but for any types defined with the macro
/// it requires a program-length allocation which is cringe so we use the pointer
/// to a static which is much more error-prone.
#[derive(Clone)]
enum ArcId {
    // A pointer to a `static ...: ()`.
    // These are all given a unique pointer.
    Static(&'static ()),
    Dynamic(Arc<()>),
}

impl PartialEq for ArcId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ArcId::Static(a), ArcId::Static(b)) => std::ptr::eq(*a, *b),
            (ArcId::Dynamic(a), ArcId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for ArcId {}

impl hash::Hash for ArcId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            ArcId::Static(ptr) => ptr.hash(state),
            ArcId::Dynamic(arc) => Arc::as_ptr(arc).hash(state),
        }
    }
}

impl fmt::Debug for ArcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            match self {
                ArcId::Static(ptr) => *ptr as *const (),
                ArcId::Dynamic(arc) => Arc::as_ptr(arc),
            }
        )
    }
}
