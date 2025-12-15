//! Helpers for generating [Type::reference] implementations

use std::{fmt, hash, sync::Arc};

use crate::{TypeCollection, datatype::NamedDataType};

use super::{DataType, Generic};

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub(crate) id: ArcId,
    // TODO: Should this be a map-type???
    pub(crate) generics: Vec<(Generic, DataType)>, // TODO: Cow<'static, [(Generic, DataType)]>,
    pub(crate) inline: bool,
}

impl Reference {
    #[doc(hidden)] // TODO: I wanna remove this and come up with a better solution for `specta-serde`.
    pub fn type_identifier(&self) -> String {
        match &self.id {
            ArcId::Static(id) => format!("s:{:p}", *id),
            ArcId::Dynamic(id) => format!("d:{}", Arc::as_ptr(id) as u128),
        }
    }

    /// Get a reference to a [NamedDataType] from a [TypeCollection].
    pub fn get<'a>(&self, types: &'a TypeCollection) -> Option<&'a NamedDataType> {
        types.0.get(&self.id)?.as_ref()
    }

    /// Construct a new reference to an opaque type.
    ///
    /// An opaque type is unable to represents using the [DataType] system and requires specific exporter integration to handle it.
    ///
    /// This should NOT be used in a [Type::definition] method as that likely means unnecessary memory.
    ///
    /// An opaque [Reference] is equal when cloned and can be compared using the [Self::ref_eq] or [PartialEq].
    ///
    pub fn opaque() -> Self {
        Self {
            id: ArcId::Dynamic(Default::default()),
            generics: Vec::with_capacity(0),
            inline: false,
        }
    }

    // TODO: Remove this
    /// Construct a new reference to a type with a fixed reference.
    ///
    /// # Safety
    ///
    /// It's critical that this reference points to a `static ...: () = ();` which is uniquely created for this reference. If it points to a `const` or `Box::leak`d value, the reference will not maintain it's invariants.
    ///
    pub const fn opaque_from_sentinel(sentinel: &'static ()) -> Reference {
        Self {
            id: ArcId::Static(sentinel),
            generics: Vec::new(),
            inline: false,
        }
    }

    /// Compare if two references are pointing to the same type.
    ///
    /// Unlike `PartialEq::eq`, this method only compares the types, not the generics, inline and other reference attributes.
    pub fn ref_eq(&self, other: &Self) -> bool {
        self.id == other.id
    }

    /// Get the generic parameters set on this reference which will be filled in by the [NamedDataType].
    pub fn generics(&self) -> &[(Generic, DataType)] {
        &self.generics
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

/// A unique identifier for a type.
///
/// `Arc<()>` is a great way of creating a virtual ID which
/// can be compared to itself but for any types defined with the macro
/// it requires a program-length allocation which is cringe so we use the pointer
/// to a static which is much more error-prone.
#[derive(Clone)]
pub(crate) enum ArcId {
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
