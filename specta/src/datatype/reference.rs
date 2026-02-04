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
#[derive(Clone)]
pub struct OpaqueReference(Arc<dyn DynOpaqueReference>);

pub(crate) fn tauri() -> Reference {
    Reference::Opaque(OpaqueReference(Arc::new(TauriChannelReferenceInner)))
}

trait DynOpaqueReference: Any + Send + Sync {
    fn type_name(&self) -> &'static str;
    fn hash(&self, hasher: &mut dyn hash::Hasher);
    fn eq(&self, other: &dyn Any) -> bool;
    fn as_any(&self) -> &dyn Any;
}

#[derive(PartialEq, Eq)]
struct TauriChannelReferenceInner;
impl DynOpaqueReference for TauriChannelReferenceInner {
    fn type_name(&self) -> &'static str {
        "tauri::ipc::Channel"
    }
    fn hash(&self, hasher: &mut dyn hash::Hasher) {
        hasher.write_u64(0);
    }
    fn eq(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .map(|other| self == other)
            .unwrap_or_default()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct OpaqueReferenceInner<T>(T);
impl<T: hash::Hash + Eq + Send + Sync + 'static> DynOpaqueReference for OpaqueReferenceInner<T> {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn hash(&self, mut hasher: &mut dyn hash::Hasher) {
        self.0.hash(&mut hasher)
    }
    fn eq(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<T>()
            .map(|other| self.0 == *other)
            .unwrap_or_default()
    }
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
}

impl fmt::Debug for OpaqueReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OpaqueReference")
            .field(&self.0.type_name())
            .finish()
    }
}

impl PartialEq for OpaqueReference {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other.0.as_any())
    }
}

impl Eq for OpaqueReference {}

impl hash::Hash for OpaqueReference {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl OpaqueReference {
    pub fn type_name(&self) -> &'static str {
        self.0.type_name()
    }

    pub fn type_id(&self) -> TypeId {
        self.0.as_any().type_id()
    }

    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
}

impl Reference {
    /// Construct a new reference to an opaque type.
    ///
    /// An opaque type is unable to be represented using the [DataType] system and requires specific exporter integration to handle it.
    ///
    /// Opaque [Reference]'s are compared using [PartialEq]. For example `Reference::opaque(()) == Reference::opaque(())` so you must ensure each reference you intent to be unique is implemented as such.
    pub fn opaque<T: hash::Hash + Eq + Send + Sync + 'static>(state: T) -> Self {
        Self::Opaque(OpaqueReference(Arc::new(OpaqueReferenceInner(state))))
    }

    /// Compare if two references point to the same type.
    ///
    /// This is different from using `Eq`, `PartialEq`, or `Hash` as those compare the [Reference].
    /// A [Reference] contains generics, inline and other attributes which this ignores.
    pub fn ty_eq(&self, other: &Reference) -> bool {
        match (self, other) {
            (Reference::Named(a), Reference::Named(b)) => a.id == b.id,
            (Reference::Opaque(a), Reference::Opaque(b)) => *a == *b,
            _ => false,
        }
    }

    /// Convert an existing [Reference] into an inlined one.
    ///
    /// It's not safe to go the other way incase the type is inlined which requires all [Reference]'s to be inlined.
    pub fn inline(mut self) -> Reference {
        if let Reference::Named(n) = &mut self {
            n.inline = true;
        }
        self
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
            (NamedId::Static(a), NamedId::Static(b)) => std::ptr::eq(*a, *b),
            (NamedId::Dynamic(a), NamedId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for NamedId {}

impl hash::Hash for NamedId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            NamedId::Static(p) => std::ptr::hash(*p, state),
            NamedId::Dynamic(p) => std::ptr::hash(Arc::as_ptr(p), state),
        }
    }
}

impl fmt::Debug for NamedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedId::Static(p) => write!(f, "s{:p}", *p),
            NamedId::Dynamic(p) => write!(f, "d{:p}", Arc::as_ptr(p)),
        }
    }
}
