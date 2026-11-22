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
///
/// This is a declaration's identity: registration returns it, other datatypes
/// refer to the declaration through it, and equality means the same
/// declaration. Generic arguments are part of the use site, not the identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedReference {
    pub(crate) id: NamedTypeId,
    /// How this named type should be referenced at the use site.
    pub inner: NamedReferenceType,
}

impl NamedReference {
    /// Constructs a use-site [`Reference`] to this declaration.
    ///
    /// The reference will error in the language exporter if the declaration's
    /// `ty` is `None`, as the type can't generate a named export.
    pub fn reference(&self, generics: Vec<(Generic, DataType)>) -> Reference {
        Reference::Named(self.with_generics(generics))
    }

    /// Re-mints this reference at the same declaration with different generic
    /// arguments.
    pub fn with_generics(&self, generics: Vec<(Generic, DataType)>) -> Self {
        Self {
            id: self.id.clone(),
            inner: NamedReferenceType::Reference { generics },
        }
    }
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
            None => Self::new_inner(types, id.static_path(), generics),
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

/// Identity of a named declaration within a [`Types`](crate::Types) graph.
///
/// Two declarations are equal only when they are the same declaration in the
/// same graph; a copy of the graph keeps the same identities. The identity is
/// process-local and must not be used as a persistence key or as a stable sort
/// tiebreaker.
///
/// Identity is the one fact a declaration's producer cannot misstate by
/// accident, which differs by producer:
///
/// `#[derive(Type)]` expands at sites nobody controls - a user's
/// `macro_rules!` macro can expand the same derive twice, and the two
/// declarations then share every path-shaped fact: module path, name,
/// source span, `type_name`. Only a marker the compiler mints per
/// expansion tells them apart, so a derived declaration is identified by
/// [`NamedTypeId::nominal`] and its path stays a readable label.
///
/// A hand-written implementation is identified by [`NamedTypeId::label`]:
/// its canonical path, stringified from the tokens as written. Coherence
/// allows one `impl Type` per concrete type, so a path can only repeat when
/// a second dependency is deliberately aliased to the same crate name - and
/// that repeat is exactly how two majors of one dependency become one
/// declaration without further action.
#[doc(hidden)]
#[derive(Clone)]
pub enum NamedTypeId {
    /// A declaration known at compile time.
    Named {
        /// The declaration's path: used in diagnostics and, when no marker
        /// was minted, compared as identity.
        label: &'static str,
        /// The marker a derive minted for its expansion; when present, it
        /// is the identity and the path is only a label.
        nominal: Option<TypeId>,
    },
    /// A declaration created at runtime, identified by its `Arc` allocation.
    Dynamic(Arc<()>),
}

impl NamedTypeId {
    /// Identity of a hand-written implementation: its canonical path,
    /// stringified from the tokens as written. Aliasing a second major of
    /// the same dependency to this path makes both implementations
    /// register as one declaration.
    pub const fn label(label: &'static str) -> Self {
        Self::Named {
            label,
            nominal: None,
        }
    }

    /// Identity of a derived declaration: its path plus the marker's `TypeId`.
    pub const fn nominal(label: &'static str, id: TypeId) -> Self {
        Self::Named {
            label,
            nominal: Some(id),
        }
    }

    /// Human-readable identity used in diagnostics.
    pub(crate) const fn label_str(&self) -> &'static str {
        match self {
            Self::Named { label, .. } => label,
            Self::Dynamic(_) => "",
        }
    }
}

impl PartialEq for NamedTypeId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                NamedTypeId::Named {
                    label: a_label,
                    nominal: a,
                },
                NamedTypeId::Named {
                    label: b_label,
                    nominal: b,
                },
            ) => a_label == b_label && a == b,
            (NamedTypeId::Dynamic(a), NamedTypeId::Dynamic(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for NamedTypeId {}

impl NamedTypeId {
    /// Best-effort path for diagnostics when the name itself is unavailable.
    pub(crate) fn static_path(&self) -> Cow<'static, str> {
        match self {
            NamedTypeId::Named { label, .. } => Cow::Borrowed(label),
            NamedTypeId::Dynamic(p) => Cow::Owned(format!("d{:p}", Arc::as_ptr(p))),
        }
    }
}

impl hash::Hash for NamedTypeId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            NamedTypeId::Named { label, nominal } => {
                0u8.hash(state);
                label.hash(state);
                nominal.hash(state);
            }
            NamedTypeId::Dynamic(p) => {
                1u8.hash(state);
                std::ptr::hash(Arc::as_ptr(p), state);
            }
        }
    }
}

impl fmt::Debug for NamedTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedTypeId::Named { label, .. } => write!(f, "s:{label}"),
            NamedTypeId::Dynamic(p) => write!(f, "d{:p}", Arc::as_ptr(p)),
        }
    }
}
