use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

trait DynAttributeValue {
    fn value_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool;
    fn hash_dyn(&self, state: &mut dyn Hasher);
    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

#[derive(Clone)]
struct TypedAttributeValue<T>(T);

impl<T> DynAttributeValue for TypedAttributeValue<T>
where
    T: Any + Eq + Hash + fmt::Debug + 'static,
{
    fn value_any(&self) -> &dyn Any {
        &self.0
    }

    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool {
        other
            .value_any()
            .downcast_ref::<T>()
            .map(|other| &self.0 == other)
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

#[derive(Clone, Default)]
/// A type-indexed map for attaching metadata to datatype nodes.
///
/// `Attributes` stores at most one value per concrete Rust type. This makes it
/// useful for format-specific metadata where each parser contributes its own
/// strongly typed attribute payload.
///
/// Values must implement `Any + Eq + Hash + Debug + 'static` so the collection
/// can support type-safe retrieval, equality, hashing, and debug output.
///
/// # Examples
///
/// ```rust
/// use specta::datatype::Attributes;
///
/// let mut attrs = Attributes::new();
/// attrs.insert::<String>("serde".to_owned());
///
/// assert_eq!(attrs.get::<String>().map(String::as_str), Some("serde"));
/// assert!(attrs.get::<u32>().is_none());
/// ```
pub struct Attributes(HashMap<TypeId, Arc<dyn DynAttributeValue>>);

impl Attributes {
    /// Creates an empty attribute collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let attrs = Attributes::new();
    /// assert!(attrs.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of stored attribute entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<u8>(1);
    /// attrs.insert::<u16>(2);
    ///
    /// assert_eq!(attrs.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when the collection has no entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// assert!(attrs.is_empty());
    ///
    /// attrs.insert::<u8>(1);
    /// assert!(!attrs.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes all entries from the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<u8>(1);
    /// attrs.clear();
    ///
    /// assert!(attrs.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts an attribute value keyed by its concrete type.
    ///
    /// If a value of the same type already exists, it is replaced.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<String>("left".to_owned());
    /// attrs.insert::<String>("right".to_owned());
    ///
    /// assert_eq!(attrs.len(), 1);
    /// assert_eq!(attrs.get::<String>().map(String::as_str), Some("right"));
    /// ```
    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Eq + Hash + fmt::Debug + 'static,
    {
        self.0
            .insert(TypeId::of::<T>(), Arc::new(TypedAttributeValue(value)));
    }

    /// Extends `self` with entries from `other`.
    ///
    /// Like [`HashMap::extend`], entries from `other` overwrite entries in
    /// `self` when they share the same key (attribute type).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut left = Attributes::new();
    /// left.insert::<u8>(1);
    /// left.insert::<String>("left".to_owned());
    ///
    /// let mut right = Attributes::new();
    /// right.insert::<u16>(2);
    /// right.insert::<String>("right".to_owned());
    ///
    /// left.extend(right);
    ///
    /// assert_eq!(left.get::<u8>(), Some(&1));
    /// assert_eq!(left.get::<u16>(), Some(&2));
    /// assert_eq!(left.get::<String>().map(String::as_str), Some("right"));
    /// ```
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// Returns `true` if a value for type `T` is present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<u8>(1);
    ///
    /// assert!(attrs.contains::<u8>());
    /// assert!(!attrs.contains::<u16>());
    /// ```
    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<T>())
    }

    /// Returns a shared reference to the value stored for type `T`.
    ///
    /// Returns `None` when no value exists for `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<String>("value".to_owned());
    ///
    /// assert_eq!(attrs.get::<String>().map(String::as_str), Some("value"));
    /// assert!(attrs.get::<u8>().is_none());
    /// ```
    pub fn get<T: Any + 'static>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|value| value.value_any().downcast_ref::<T>())
    }

    /// Removes the value for type `T`.
    ///
    /// Returns `true` when a value was present and removed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use specta::datatype::Attributes;
    ///
    /// let mut attrs = Attributes::new();
    /// attrs.insert::<u8>(1);
    ///
    /// assert!(attrs.remove::<u8>());
    /// assert!(!attrs.remove::<u8>());
    /// ```
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
