use std::{
    any::{Any, TypeId},
    fmt, hash,
    sync::Arc,
};

use crate::datatype::Generic;

use super::DataType;

/// Reference to another datatype.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Reference to a named type collected in a [`Types`](crate::Types).
    ///
    /// This can either render as a named reference, such as `TypeName<T>`, or as
    /// an inlined datatype depending on [`NamedReference::inner`].
    Named(NamedReference),
    /// Reference to an opaque exporter-specific type.
    Opaque(OpaqueReference),
}

/// Reference to a [`NamedDataType`](crate::datatype::NamedDataType).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedReference {
    pub(crate) id: NamedId,
    /// How this named type should be referenced at the use site.
    pub inner: NamedReferenceType,
}

/// Use-site representation for a [`NamedReference`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamedReferenceType {
    /// Recursive reference encountered while resolving an inline type.
    ///
    /// Exporters can use this marker to avoid infinitely expanding recursive
    /// inline definitions that they would stack overflow resolving.
    Recursive,
    /// Inline the contained datatype at the reference site.
    /// These are emitted when `#[specta(inline)]` is used on a field or container.
    #[non_exhaustive]
    Inline {
        /// Datatype to render in place of the named reference.
        dt: Box<DataType>,
    },
    /// Render a reference to the named datatype.
    #[non_exhaustive]
    Reference {
        /// Concrete generic arguments for this use site.
        generics: Vec<(Generic, DataType)>,
    },
}

/// Reference to a type not understood by Specta's core datatype model.
///
/// These are implemented by the language exporter to implement cool features like
/// [`specta_typescript::branded!`](https://docs.rs/specta-typescript/latest/specta_typescript/macro.branded.html),
/// [`specta_typescript::define`](https://docs.rs/specta-typescript/latest/specta_typescript/fn.define.html), and more.
///
/// # Invariants
///
/// Equality and hashing are delegated to the stored opaque state. If two opaque
/// references should be distinct, their state values must compare and hash
/// distinctly.
///
/// This is an advanced feature designed for language exporters and framework
/// integrations. Most end users should prefer ordinary [`DataType`] variants.
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
    /// Returns the Rust type name of the stored opaque state.
    pub fn type_name(&self) -> &'static str {
        self.0.type_name()
    }

    /// Returns the [`TypeId`] of the stored opaque state.
    pub fn type_id(&self) -> TypeId {
        self.0.as_any().type_id()
    }

    /// Attempts to downcast the opaque state to `T`.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
}

impl Reference {
    /// Constructs a new reference to an opaque type.
    ///
    /// An opaque type cannot be represented with the core [`DataType`] model and
    /// requires specific exporter integration.
    ///
    /// Opaque [`Reference`]s are compared using the state's [`PartialEq`]
    /// implementation. For example, `Reference::opaque(()) ==
    /// Reference::opaque(())`, so unique references need unique state.
    pub fn opaque<T: hash::Hash + Eq + Send + Sync + 'static>(state: T) -> Self {
        Self::Opaque(OpaqueReference(Arc::new(OpaqueReferenceInner(state))))
    }

    /// Returns whether two references point to the same underlying type.
    ///
    /// This differs from [`Eq`], [`PartialEq`], and [`Hash`] because those compare
    /// the full [`Reference`] which includes generic arguments and inline state.
    pub fn ty_eq(&self, other: &Reference) -> bool {
        match (self, other) {
            (Reference::Named(a), Reference::Named(b)) => a.id == b.id,
            (Reference::Opaque(a), Reference::Opaque(b)) => *a == *b,
            _ => false,
        }
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
