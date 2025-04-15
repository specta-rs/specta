use std::{borrow::Cow, cmp::Ordering, panic::Location};

/// The unique Specta ID for the type.
///
/// Be aware type aliases don't exist as far as Specta is concerned as they are flattened into their inner type by Rust's trait system.
/// The Specta Type ID holds for the given properties:
///  - `T::SID == T::SID`
///  - `T::SID != S::SID`
///  - `Type<T>::SID == Type<S>::SID` (unlike std::any::TypeId)
///  - `&'a T::SID == &'b T::SID` (unlike std::any::TypeId which forces a static lifetime)
///  - `Box<T> == Arc<T> == Rc<T>` (unlike std::any::TypeId)
///
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Debug, Clone, Copy, Hash)]
pub struct SpectaID {
    pub(crate) type_name: &'static str,
    pub(crate) hash: u64,
}

impl SpectaID {
    /// Construct a new unique identifier for a type.
    ///
    /// It's up to you to ensure the type produced is consistent for each identifier.
    ///
    #[doc(hidden)] // TODO: Should we stablise this?
    #[track_caller]
    pub const fn new(type_name: &'static str) -> Self {
        let caller = Location::caller();
        let mut hash = 0xcbf29ce484222325;
        let prime = 0x00000100000001B3;

        let mut bytes = type_name.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(prime);
            i += 1;
        }

        bytes = caller.file().as_bytes();
        i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(prime);
            i += 1;
        }

        hash ^= ':' as u64;
        hash = hash.wrapping_mul(prime);

        hash ^= caller.line() as u64;
        hash = hash.wrapping_mul(prime);

        hash ^= ':' as u64;
        hash = hash.wrapping_mul(prime);

        hash ^= caller.column() as u64;
        hash = hash.wrapping_mul(prime);

        SpectaID { type_name, hash }
    }

    pub fn type_name(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.type_name)
    }
}

// We do custom impls so the order prefers type_name over hash.
impl Ord for SpectaID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.type_name
            .cmp(other.type_name)
            .then(self.hash.cmp(&other.hash))
    }
}

// We do custom impls so the order prefers type_name over hash.
impl PartialOrd<Self> for SpectaID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// We do custom impls so equals is by SID exclusively.
impl Eq for SpectaID {}

// We do custom impls so equals is by SID exclusively.
impl PartialEq<Self> for SpectaID {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}
