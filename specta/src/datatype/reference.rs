use std::{
    any::{Any, TypeId},
    fmt, hash,
    sync::Arc,
};

use crate::datatype::Generic;

use super::DataType;

/// Reference to another type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Reference to a named type collected in a [`Types`].
    /// This can either produce `TypeName<Generics>` or just an inlined definition.
    Named(NamedReference),
    /// Reference to an opaque exporter-specific type.
    Opaque(OpaqueReference),
}

/// Reference to a [NamedDataType].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedReference {
    pub(crate) id: NamedId,
    pub inner: NamedReferenceType,
}

/// Internal representation of a specific [NamedDataType].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamedReferenceType {
    Recursive,
    #[non_exhaustive]
    Inline {
        dt: Box<DataType>,
    },
    #[non_exhaustive]
    Reference {
        generics: Vec<(Generic, DataType)>,
    },
}

/// Reference to a type not understood by Specta's core.
///
/// These are implemented by the language exporter to implement cool features like
/// [`specta_typescript::branded!`](https://docs.rs/specta-typescript/latest/specta_typescript/macro.branded.html),
/// [`specta_typescript::define`](https://docs.rs/specta-typescript/latest/specta_typescript/fn.define.html), and more.
///
/// This is an advanced feature designed for language exporters so should generally be avoided and is not intended to be generally useful unless your in control of the language exporter.
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
    /// Get the Rust type name of the stored opaque state.
    pub fn type_name(&self) -> &'static str {
        self.0.type_name()
    }

    /// Get the [`TypeId`] of the stored opaque state.
    pub fn type_id(&self) -> TypeId {
        self.0.as_any().type_id()
    }

    /// Attempt to downcast the opaque state to `T`.
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
        // if let Reference::Named(n) = &mut self {
        //     n.inline = true;
        // }
        // self

        todo!();
    }
}

impl From<Reference> for DataType {
    fn from(r: Reference) -> Self {
        Self::Reference(r)
    }
}

/// Unique identifier for a [NamedDataType].
///
/// For static types (from derive macros), we use a unique string based on the
/// type's module path and name. For dynamic types, we use an Arc pointer.
#[derive(Clone)]
pub(crate) enum NamedId {
    // A unique string identifying the type (module_path::TypeName).
    Static(&'static str),
    Dynamic(Arc<()>),
}

impl PartialEq for NamedId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NamedId::Static(a), NamedId::Static(b)) => a == b,
            (NamedId::Dynamic(a), NamedId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for NamedId {}

impl hash::Hash for NamedId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            NamedId::Static(s) => s.hash(state),
            NamedId::Dynamic(p) => std::ptr::hash(Arc::as_ptr(p), state),
        }
    }
}

impl fmt::Debug for NamedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedId::Static(s) => write!(f, "s:{}", s),
            NamedId::Dynamic(p) => write!(f, "d{:p}", Arc::as_ptr(p)),
        }
    }
}
