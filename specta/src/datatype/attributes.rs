use std::{
    any::Any,
    borrow::Cow,
    collections::{HashMap, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

trait DynAttributeValue: Send + Sync {
    fn value_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool;
    fn hash_dyn(&self, state: &mut dyn Hasher);
    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

struct NamedAttributeValue<T>(T);

impl<T> DynAttributeValue for NamedAttributeValue<T>
where
    T: Any + Clone + Eq + Hash + fmt::Debug + Send + Sync + 'static,
{
    fn value_any(&self) -> &dyn Any {
        &self.0
    }

    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool {
        other
            .value_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self.0 == *other)
    }

    fn hash_dyn(&self, state: &mut dyn Hasher) {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        state.write_u64(hasher.finish());
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A named map of type-erased metadata attached to datatype nodes.
///
/// `Attributes` is primarily used by advanced consumers that need to inspect
/// metadata recorded on [`DataType`](super::DataType) nodes at runtime. Each
/// entry is stored under a string key and can later be retrieved either as a
/// raw [`Any`] value or by downcasting to the original type.
///
/// Stored values must be owned and implement [`Clone`], [`Eq`], [`Hash`], and
/// [`fmt::Debug`] so attributes remain comparable, hashable, and printable as
/// part of the surrounding datatype graph.
///
/// # Examples
///
/// ```rust
/// use specta::datatype::Attributes;
///
/// let mut attrs = Attributes::default();
/// attrs.insert("serde:rename", String::from("user_name"));
/// attrs.insert("serde:skip", true);
///
/// assert_eq!(attrs.len(), 2);
/// assert!(attrs.contains_key("serde:rename"));
/// assert_eq!(
///     attrs.get_named_as::<String>("serde:rename"),
///     Some(&String::from("user_name"))
/// );
/// assert_eq!(attrs.get_named_as::<bool>("serde:skip"), Some(&true));
/// assert_eq!(attrs.get_named_as::<u32>("serde:skip"), None);
/// ```
#[derive(Default)]
pub struct Attributes(HashMap<Cow<'static, str>, Arc<dyn DynAttributeValue>>);

impl Clone for Attributes {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }
}

impl Attributes {
    /// Returns the number of stored attribute entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when the collection has no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Inserts or replaces an attribute value.
    ///
    /// Values are stored in a type-erased form, but they must still implement
    /// [`Clone`], [`Eq`], [`Hash`], and [`fmt::Debug`] so the containing
    /// [`Attributes`] remains cloneable, comparable, hashable, and printable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::default();
    /// attrs.insert("serde:default", true);
    ///
    /// assert_eq!(attrs.get_named_as::<bool>("serde:default"), Some(&true));
    /// ```
    pub fn insert<T>(&mut self, key: impl Into<Cow<'static, str>>, value: T)
    where
        T: Any + Clone + Eq + Hash + fmt::Debug + Send + Sync + 'static,
    {
        self.0
            .insert(key.into(), Arc::new(NamedAttributeValue(value)));
    }

    /// Extends `self` with entries from `other`.
    ///
    /// If both collections contain the same key, the value from `other`
    /// replaces the existing entry in `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut base = Attributes::default();
    /// base.insert("serde:rename", String::from("first_name"));
    ///
    /// let mut extra = Attributes::default();
    /// extra.insert("serde:skip", true);
    ///
    /// base.extend(extra);
    ///
    /// assert_eq!(base.get_named_as::<bool>("serde:skip"), Some(&true));
    /// ```
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// Returns `true` if an attribute entry is present for `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns the raw type-erased value for a named attribute.
    ///
    /// This is useful when the expected type is not known until runtime.
    /// Prefer [`Attributes::get_named_as`] when you know the concrete type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::default();
    /// attrs.insert("serde:rename", String::from("user_name"));
    ///
    /// let value = attrs.get_named("serde:rename").unwrap();
    /// assert_eq!(value.downcast_ref::<String>(), Some(&String::from("user_name")));
    /// ```
    pub fn get_named(&self, key: &str) -> Option<&dyn Any> {
        self.0.get(key).map(|value| value.value_any())
    }

    /// Returns a typed reference to the named attribute value.
    ///
    /// Returns `None` when the key is missing or when the stored value has a
    /// different type than `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::default();
    /// attrs.insert("serde:skip", true);
    ///
    /// assert_eq!(attrs.get_named_as::<bool>("serde:skip"), Some(&true));
    /// assert_eq!(attrs.get_named_as::<String>("serde:skip"), None);
    /// ```
    pub fn get_named_as<T: Any + 'static>(&self, key: &str) -> Option<&T> {
        self.0
            .get(key)
            .and_then(|value| value.value_any().downcast_ref::<T>())
    }
}

impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self.0.iter().all(|(key, value)| {
                other
                    .0
                    .get(key)
                    .is_some_and(|other| value.eq_dyn(other.as_ref()))
            })
    }
}

impl Eq for Attributes {}

impl Hash for Attributes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self
            .0
            .iter()
            .map(|(key, value)| {
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                value.hash_dyn(&mut hasher);
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
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut map = f.debug_map();
        for (key, value) in entries {
            map.entry(key, &fmt::from_fn(|f| value.fmt_dyn(f)));
        }
        map.finish()
    }
}
