//! Runtime representation of Rust attributes for type metadata.
//!
//! This module provides types that represent Rust attributes (like `#[serde(...)]` or
//! `#[specta(...)]`) in a runtime-accessible format.

use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// A complete runtime representation of a Rust attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    /// The attribute path (e.g., `"serde"`, `"specta"`, `"doc"`).
    pub path: String,
    /// The kind of metadata this attribute contains.
    pub kind: AttributeMeta,
}

/// The kind of metadata contained in an attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeMeta {
    /// A simple path identifier (e.g., `untagged`, `skip`, `flatten`).
    Path(String),
    /// A key-value pair (e.g., `rename = "value"`, `default = 42`).
    NameValue {
        /// The option key (for example `rename` or `default`).
        key: String,
        /// The option value associated with [`Self::NameValue::key`].
        value: AttributeValue,
    },
    /// A list of nested metadata items (e.g., the contents of `#[serde(...)]`).
    List(Vec<AttributeNestedMeta>),
}

/// Nested metadata within a list-style attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeNestedMeta {
    /// Structured metadata (path, name-value, or list).
    Meta(AttributeMeta),
    /// A direct literal value.
    Literal(AttributeLiteral),
    /// A non-literal expression captured from attribute syntax.
    Expr(String),
}

/// A value in a name-value attribute pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeValue {
    /// A literal value (e.g., `rename = "value"`, `default = true`).
    Literal(AttributeLiteral),
    /// A non-literal expression (e.g., `with = module::path`).
    Expr(String),
}

/// A literal value that can appear in an attribute.
#[derive(Debug, Clone)]
pub enum AttributeLiteral {
    /// A string literal.
    Str(String),
    /// An integer literal.
    Int(i64),
    /// A boolean literal.
    Bool(bool),
    /// A floating-point literal.
    Float(f64),
    /// A byte literal.
    Byte(u8),
    /// A character literal.
    Char(char),
    /// A byte string literal.
    ByteStr(Vec<u8>),
    /// A C-string literal.
    CStr(Vec<u8>),
}

impl PartialEq for AttributeLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Byte(a), Self::Byte(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::ByteStr(a), Self::ByteStr(b)) => a == b,
            (Self::CStr(a), Self::CStr(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for AttributeLiteral {}

impl Hash for AttributeLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Str(v) => {
                0_u8.hash(state);
                v.hash(state);
            }
            Self::Int(v) => {
                1_u8.hash(state);
                v.hash(state);
            }
            Self::Bool(v) => {
                2_u8.hash(state);
                v.hash(state);
            }
            Self::Float(v) => {
                3_u8.hash(state);
                v.to_bits().hash(state);
            }
            Self::Byte(v) => {
                4_u8.hash(state);
                v.hash(state);
            }
            Self::Char(v) => {
                5_u8.hash(state);
                v.hash(state);
            }
            Self::ByteStr(v) => {
                6_u8.hash(state);
                v.hash(state);
            }
            Self::CStr(v) => {
                7_u8.hash(state);
                v.hash(state);
            }
        }
    }
}

trait DynAttributeValue: Any {
    fn as_any(&self) -> &dyn Any;
    fn value_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool;
    fn hash_dyn(&self, state: &mut dyn Hasher);
    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

#[derive(Clone)]
struct TypedAttributeValue<T>(Arc<T>);

impl<T> DynAttributeValue for TypedAttributeValue<T>
where
    T: Any + Eq + Hash + fmt::Debug + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn value_any(&self) -> &dyn Any {
        self.0.as_ref()
    }

    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool {
        other
            .value_any()
            .downcast_ref::<T>()
            .map(|other| self.0.as_ref() == other)
            .unwrap_or_default()
    }

    fn hash_dyn(&self, state: &mut dyn Hasher) {
        let mut state = state;
        self.0.hash(&mut state);
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
struct AnyAttributeValue(Arc<dyn Any>);

impl DynAttributeValue for AnyAttributeValue {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn value_any(&self) -> &dyn Any {
        self.0.as_ref()
    }

    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map(|other| Arc::ptr_eq(&self.0, &other.0))
            .unwrap_or_default()
    }

    fn hash_dyn(&self, state: &mut dyn Hasher) {
        state.write_usize(Arc::as_ptr(&self.0) as *const () as usize);
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Arc<dyn Any>(type_id={:?})", self.0.type_id())
    }
}

/// Runtime key/value attributes attached to a datatype.
///
/// Values are stored by `TypeId` so each concrete type can have at most one value.
#[derive(Clone, Default)]
pub struct Attributes(HashMap<TypeId, Arc<dyn DynAttributeValue>>);

impl Attributes {
    /// Construct an empty attribute store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored values.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when no values are stored.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Remove all values.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Insert a typed value.
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Eq + Hash + fmt::Debug + 'static,
    {
        self.insert_arc(Arc::new(value));
    }

    /// Insert a typed value already wrapped in [`Arc`].
    pub fn insert_arc<T>(&mut self, value: Arc<T>)
    where
        T: Any + Eq + Hash + fmt::Debug + 'static,
    {
        self.0
            .insert(TypeId::of::<T>(), Arc::new(TypedAttributeValue(value)));
    }

    /// Insert a value as `Arc<dyn Any>`.
    ///
    /// Values inserted with this method use pointer identity for equality and hashing.
    pub fn insert_any(&mut self, value: Arc<dyn Any>) {
        self.0
            .insert(value.type_id(), Arc::new(AnyAttributeValue(value)));
    }

    /// Returns `true` if a value of type `T` is present.
    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<T>())
    }

    /// Get a typed value by reference.
    pub fn get<T: Any + 'static>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|value| value.value_any().downcast_ref::<T>())
    }

    /// Remove a typed value, returning whether it existed.
    pub fn remove<T: Any + 'static>(&mut self) -> bool {
        self.0.remove(&TypeId::of::<T>()).is_some()
    }
}

impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(k, v)| {
                other
                    .0
                    .get(k)
                    .map(|other| v.eq_dyn(other.as_ref()))
                    .unwrap_or_default()
            })
    }
}

impl Eq for Attributes {}

impl Hash for Attributes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self
            .0
            .iter()
            .map(|(k, v)| {
                let mut hasher = DefaultHasher::new();
                k.hash(&mut hasher);
                v.hash_dyn(&mut hasher);
                hasher.finish()
            })
            .collect::<Vec<_>>();

        entries.sort_unstable();

        self.0.len().hash(state);
        entries.hash(state);
    }
}

impl fmt::Debug for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries = self.0.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(k, _)| {
            let mut hasher = DefaultHasher::new();
            k.hash(&mut hasher);
            hasher.finish()
        });

        let mut map = f.debug_map();
        for (type_id, value) in entries {
            map.entry(&type_id, &fmt::from_fn(|f| value.fmt_dyn(f)));
        }
        map.finish()
    }
}
