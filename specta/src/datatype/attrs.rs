use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

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

#[derive(Clone, Default)]
pub struct Attributes(HashMap<TypeId, Arc<dyn DynAttributeValue>>);

impl Attributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Eq + Hash + fmt::Debug + 'static,
    {
        self.insert_arc(Arc::new(value));
    }

    pub fn insert_arc<T>(&mut self, value: Arc<T>)
    where
        T: Any + Eq + Hash + fmt::Debug + 'static,
    {
        self.0
            .insert(TypeId::of::<T>(), Arc::new(TypedAttributeValue(value)));
    }

    pub fn insert_any(&mut self, value: Arc<dyn Any>) {
        self.0
            .insert(value.type_id(), Arc::new(AnyAttributeValue(value)));
    }

    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.0.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: Any + 'static>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|value| value.value_any().downcast_ref::<T>())
    }

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
