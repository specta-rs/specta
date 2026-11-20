use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt, hash,
    sync::Arc,
};

use crate::{
    Types,
    datatype::{Generic, Map, NamedDataType, Primitive},
};

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
    pub(crate) id: NamedTypeId,
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
    Recursive(RecursiveInlineType),
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

/// Debug-only description of a type in a recursive inline cycle.
#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RecursiveInlineType {
    inner: Arc<RecursiveInlineTypeInner>,
}

#[derive(PartialEq, Eq, Hash)]
struct RecursiveInlineTypeInner {
    cycle: Vec<RecursiveInlineFrame>,
    generics: Vec<(Generic, DataType)>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct RecursiveInlineFrame {
    type_name: Cow<'static, str>,
    generics: Vec<Cow<'static, str>>,
}

impl RecursiveInlineType {
    pub(crate) fn from_cycle(
        cycle: Vec<RecursiveInlineFrame>,
        generics: Vec<(Generic, DataType)>,
    ) -> Self {
        Self {
            inner: Arc::new(RecursiveInlineTypeInner { cycle, generics }),
        }
    }

    /// Concrete generic arguments for the recursive named reference use site.
    pub fn generics(&self) -> &[(Generic, DataType)] {
        &self.inner.generics
    }

    fn last_frame(&self) -> Option<&RecursiveInlineFrame> {
        self.inner.cycle.last()
    }
}

impl fmt::Debug for RecursiveInlineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, ty) in self.inner.cycle.iter().enumerate() {
            if idx != 0 {
                f.write_str(" -> ")?;
            }
            write!(f, "{ty:?}")?;
        }
        Ok(())
    }
}

impl RecursiveInlineFrame {
    pub(crate) fn new(
        types: &Types,
        ndt: &NamedDataType,
        generics: &[(Generic, DataType)],
    ) -> Self {
        Self::new_inner(types, named_type_path(ndt), generics)
    }

    pub(crate) fn from_type_path(
        types: &Types,
        type_name: Cow<'static, str>,
        generics: &[(Generic, DataType)],
    ) -> Self {
        let type_name = match type_name {
            Cow::Borrowed(name) => Cow::Owned(display_type_identity(name).to_owned()),
            Cow::Owned(name) => Cow::Owned(display_type_identity(&name).to_owned()),
        };
        Self::new_inner(types, type_name, generics)
    }

    /// Describes the `id` use site with `generics`.
    ///
    /// This is only called when a cycle is actually reported, so the (allocating)
    /// path rendering and generic formatting stay off the common resolution path.
    pub(crate) fn describe(
        types: &Types,
        id: &NamedTypeId,
        generics: &[(Generic, DataType)],
    ) -> Self {
        match types.types.get(id).and_then(|ndt| ndt.as_ref()) {
            Some(ndt) => Self::new(types, ndt, generics),
            // The entry can only disappear if the definition is being rebuilt
            // after a panic; fall back to the identity itself.
            None => Self::from_type_path(types, id.0.static_path(), generics),
        }
    }

    fn new_inner(
        types: &Types,
        type_name: Cow<'static, str>,
        generics: &[(Generic, DataType)],
    ) -> Self {
        Self {
            type_name,
            generics: generics
                .iter()
                .map(|(_, dt)| render_recursive_inline_generic(types, dt))
                .collect(),
        }
    }
}

impl fmt::Debug for RecursiveInlineFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_name)?;
        if self.generics.is_empty() {
            return Ok(());
        }

        f.write_str("<")?;
        for (idx, generic) in self.generics.iter().enumerate() {
            if idx != 0 {
                f.write_str(", ")?;
            }
            f.write_str(generic)?;
        }
        f.write_str(">")
    }
}

fn named_type_path(ndt: &NamedDataType) -> Cow<'static, str> {
    if ndt.module_path.is_empty() {
        ndt.name.clone()
    } else {
        Cow::Owned(format!("{}::{}", ndt.module_path, ndt.name))
    }
}

fn render_recursive_inline_generic(types: &Types, dt: &DataType) -> Cow<'static, str> {
    match dt {
        DataType::Primitive(primitive) => Cow::Borrowed(primitive_type_name(primitive)),
        DataType::Generic(generic) => generic.name().clone(),
        DataType::Reference(Reference::Named(reference)) => {
            render_recursive_inline_named(types, reference)
        }
        DataType::Reference(Reference::Opaque(reference)) => Cow::Borrowed(reference.type_name()),
        DataType::List(list) => {
            let ty = render_recursive_inline_generic(types, &list.ty);
            match list.length {
                Some(length) => Cow::Owned(format!("[{ty}; {length}]")),
                None => Cow::Owned(format!("Vec<{ty}>")),
            }
        }
        DataType::Map(map) => render_recursive_inline_map(types, map),
        DataType::Nullable(inner) => Cow::Owned(format!(
            "Option<{}>",
            render_recursive_inline_generic(types, inner)
        )),
        DataType::Tuple(tuple) => Cow::Owned(format!(
            "({})",
            tuple
                .elements
                .iter()
                .map(|dt| render_recursive_inline_generic(types, dt).into_owned())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        dt => Cow::Owned(format!("{dt:?}")),
    }
}

fn render_recursive_inline_named(types: &Types, reference: &NamedReference) -> Cow<'static, str> {
    if let NamedReferenceType::Recursive(cycle) = &reference.inner
        && let Some(ty) = cycle.last_frame()
    {
        return Cow::Owned(format!("{ty:?}"));
    }

    let Some(ndt) = types.get(reference) else {
        return Cow::Owned(format!("{reference:?}"));
    };

    let mut out = named_type_path(ndt).into_owned();
    if let NamedReferenceType::Reference { generics } = &reference.inner
        && !generics.is_empty()
    {
        out.push('<');
        out.push_str(
            &generics
                .iter()
                .map(|(_, dt)| render_recursive_inline_generic(types, dt).into_owned())
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push('>');
    }
    Cow::Owned(out)
}

fn render_recursive_inline_map(types: &Types, map: &Map) -> Cow<'static, str> {
    Cow::Owned(format!(
        "HashMap<{}, {}>",
        render_recursive_inline_generic(types, map.key_ty()),
        render_recursive_inline_generic(types, map.value_ty())
    ))
}

fn primitive_type_name(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::i8 => "i8",
        Primitive::i16 => "i16",
        Primitive::i32 => "i32",
        Primitive::i64 => "i64",
        Primitive::i128 => "i128",
        Primitive::u8 => "u8",
        Primitive::u16 => "u16",
        Primitive::u32 => "u32",
        Primitive::u64 => "u64",
        Primitive::u128 => "u128",
        Primitive::isize => "isize",
        Primitive::usize => "usize",
        Primitive::f16 => "f16",
        Primitive::f32 => "f32",
        Primitive::f64 => "f64",
        Primitive::f128 => "f128",
        Primitive::bool => "bool",
        Primitive::str => "String",
        Primitive::char => "char",
    }
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

trait DynOpaqueReference: Any + Send + Sync {
    fn type_name(&self) -> &'static str;
    fn hash(&self, hasher: &mut dyn hash::Hasher);
    fn eq(&self, other: &dyn Any) -> bool;
    fn as_any(&self) -> &dyn Any;
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

/// Stable identity of a [`NamedDataType`].
///
/// Equality, hashing and cloning are available to exporters that need to key
/// collections by declaration. Two declarations are equal only when they are
/// the same declaration in the same graph; a copy of the graph keeps the same
/// identities.
///
/// The identity is process-local and must not be used as a persistence key or
/// as a stable sort tiebreaker.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NamedTypeId(pub(crate) NamedId);

/// How a [`NamedTypeId`] is determined.
///
/// A derived declaration is identified by the [`TypeId`] of a marker type
/// generated next to its `Type` implementation. Two Rust declarations can share
/// a module path, name and source span (for example when a `macro_rules!` macro
/// expands the same `#[derive(Type)]` twice), so the human-readable identity is
/// not unique and must not be used for equality.
///
/// Built-in implementations, whose identity is their stable Rust path, and
/// dynamic types, identified by an `Arc` pointer, keep their own variants.
#[derive(Clone)]
pub(crate) enum NamedId {
    /// A built-in implementation, identified by its stable `module_path::TypeName`.
    Static(&'static str),
    /// A derived declaration, identified by the marker type generated with it.
    Nominal {
        /// Human-readable identity used in diagnostics.
        label: &'static str,
        /// Nominal identity of the declaration.
        id: TypeId,
    },
    Dynamic(Arc<()>),
}

impl PartialEq for NamedId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NamedId::Static(a), NamedId::Static(b)) => a == b,
            (NamedId::Nominal { id: a, .. }, NamedId::Nominal { id: b, .. }) => a == b,
            (NamedId::Dynamic(a), NamedId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for NamedId {}

impl NamedId {
    /// Best-effort path for diagnostics when the name itself is unavailable.
    pub(crate) fn static_path(&self) -> Cow<'static, str> {
        match self {
            NamedId::Static(label) | NamedId::Nominal { label, .. } => Cow::Borrowed(label),
            NamedId::Dynamic(p) => Cow::Owned(format!("d{:p}", Arc::as_ptr(p))),
        }
    }
}

impl hash::Hash for NamedId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            NamedId::Static(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            NamedId::Nominal { id, .. } => {
                1u8.hash(state);
                id.hash(state);
            }
            NamedId::Dynamic(p) => {
                2u8.hash(state);
                std::ptr::hash(Arc::as_ptr(p), state);
            }
        }
    }
}

/// Removes the internal `#<hash>` declaration suffix from a type identity.
///
/// The suffix only distinguishes declarations that share a module path and
/// name. Diagnostics and recursive-cycle descriptions should show the stable
/// Rust path instead.
fn display_type_identity(identity: &str) -> &str {
    identity.split_once('#').map_or(identity, |(path, _)| path)
}

impl fmt::Debug for NamedTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            NamedId::Nominal { label, .. } | NamedId::Static(label) => {
                write!(f, "s:{}", display_type_identity(label))
            }
            NamedId::Dynamic(p) => write!(f, "d{:p}", Arc::as_ptr(p)),
        }
    }
}
