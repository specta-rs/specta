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
    fn clone_dyn(&self) -> Arc<dyn DynAttributeValue>;
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

    fn clone_dyn(&self) -> Arc<dyn DynAttributeValue> {
        Arc::new(Self(self.0.clone()))
    }

    fn eq_dyn(&self, other: &dyn DynAttributeValue) -> bool {
        other
            .value_any()
            .downcast_ref::<T>()
            .is_some_and(|other| self.0 == *other)
    }

    fn hash_dyn(&self, state: &mut dyn Hasher) {
        let mut state = state;
        self.0.hash(&mut state);
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A named map of type-erased metadata attached to datatype nodes.
#[derive(Default)]
pub struct Attributes(HashMap<Cow<'static, str>, Arc<dyn DynAttributeValue>>);

impl Clone for Attributes {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(key, value)| (key.clone(), value.clone_dyn()))
                .collect(),
        )
    }
}

impl Attributes {
    /// Creates an empty attribute collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of stored attribute entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when the collection has no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes all entries from the collection.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts a named attribute value.
    pub fn insert_named<T>(&mut self, key: impl Into<Cow<'static, str>>, value: T)
    where
        T: Any + Clone + Eq + Hash + fmt::Debug + Send + Sync + 'static,
    {
        self.0
            .insert(key.into(), Arc::new(NamedAttributeValue(value)));
    }

    /// Extends `self` with entries from `other`.
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    /// Returns `true` if a named attribute is present.
    pub fn contains_named(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Returns the raw type-erased value for a named attribute.
    pub fn get_named(&self, key: &str) -> Option<&dyn Any> {
        self.0.get(key).map(|value| value.value_any())
    }

    /// Returns a typed reference to the named attribute value.
    pub fn get_named_as<T: Any + 'static>(&self, key: &str) -> Option<&T> {
        self.get_named(key)
            .and_then(|value| value.downcast_ref::<T>())
    }

    /// Removes the named attribute.
    pub fn remove_named(&mut self, key: &str) -> bool {
        self.0.remove(key).is_some()
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
