use std::{
    any::{Any, TypeId},
    fmt, hash,
    sync::Arc,
};

use crate::{TypeCollection, datatype::NamedDataType};

use super::{DataType, Generic};

/// A reference to another type.
/// This can either an [NamedReference] or [OpaqueReference].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reference {
    Named(NamedReference),
    Opaque(OpaqueReference),
}

/// A reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedReference {
    pub(crate) id: NamedId,
    // TODO: Should this be a map-type???
    pub(crate) generics: Vec<(Generic, DataType)>, // TODO: Cow<'static, [(Generic, DataType)]>,
    pub(crate) inline: bool,
}

impl NamedReference {
    /// Get a reference to a [NamedDataType] from a [TypeCollection].
    ///
    /// This is guaranteed to return a [NamedDataType] if the [TypeCollection] matches,
    /// what was used to get the original [Reference].
    pub fn get<'a>(&self, types: &'a TypeCollection) -> Option<&'a NamedDataType> {
        types.0.get(&self.id)?.as_ref()
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

/// A reference to an opaque type which is understood by the type exporter.
/// This powers [specta_typescript::branded], [specta_typescript::define] and more.
///
/// This is an advanced feature designed for language exporters so should generally be avoided.
#[derive(Debug, Clone)]
pub struct OpaqueReference {
    inner: Arc<dyn Any + Send + Sync>,
    type_name: &'static str,
    hash: fn(&(dyn Any + Send + Sync), &mut dyn hash::Hasher),
    eq: fn(&(dyn Any + Send + Sync), &(dyn Any + Send + Sync)) -> bool,
}

impl PartialEq for OpaqueReference {
    fn eq(&self, other: &Self) -> bool {
        (self.eq)(&self.inner, &other.inner)
    }
}

impl Eq for OpaqueReference {}

impl hash::Hash for OpaqueReference {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (self.hash)(&self.inner, state)
    }
}

impl OpaqueReference {
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub fn type_id(&self) -> TypeId {
        (*self.inner).type_id() // TODO: Check this is the inner type not `Arc`
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }
}

impl Reference {
    pub fn opaque<T: hash::Hash + Eq + Send + Sync + 'static>(state: T) -> Self {
        Self::Opaque(OpaqueReference {
            inner: Arc::new(state),
            type_name: std::any::type_name::<T>(),
            hash: |inner, mut hasher| {
                inner
                    .downcast_ref::<T>()
                    .expect("opaque reference failed to downcast into self")
                    .hash(&mut hasher);
            },
            eq: |a, b| {
                b.downcast_ref::<T>()
                    .map(|b| {
                        a.downcast_ref::<T>()
                            .expect("opaque reference failed to downcast into self")
                            .eq(b)
                    })
                    .unwrap_or_default()
            },
        })
    }

    /// Compare if two references point to the same type.
    ///
    /// This is different from using `Eq`, `PartialEq`, or `Hash` as those compare the [Reference].
    /// A [Reference] contains generics, inline and other attributes which this ignores.
    pub fn ty_eq(&self, other: &Reference) -> bool {
        match (self, other) {
            (Reference::Named(a), Reference::Named(b)) => a.id == b.id,
            (Reference::Opaque(a), Reference::Opaque(b)) => a == b,
            _ => false,
        }
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}

/// A unique identifier for a [NamedDataType].
///
/// `Arc<()>` is a great way of creating a virtual ID which
/// can be compared to itself but for any types defined with the macro
/// it requires a 'static allocation which is cringe so we use the pointer
/// to a static which doesn't allocate but is much more error-prone so it's only used internally.
#[derive(Clone)]
pub(crate) enum NamedId {
    // A pointer to a `static ...: ...`.
    // These are all given a unique pointer.
    Static(&'static ()),
    Dynamic(Arc<()>),
}

impl PartialEq for NamedId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NamedId::Static(a), NamedId::Static(b)) => std::ptr::eq(a, b),
            (NamedId::Dynamic(a), NamedId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for NamedId {}

impl hash::Hash for NamedId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            NamedId::Static(p) => std::ptr::hash(p, state),
            NamedId::Dynamic(p) => std::ptr::hash(Arc::as_ptr(p), state),
        }
    }
}

impl fmt::Debug for NamedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedId::Static(p) => write!(f, "s_{p:p})"),
            NamedId::Dynamic(p) => write!(f, "d_{p:p})"),
        }
    }
}
