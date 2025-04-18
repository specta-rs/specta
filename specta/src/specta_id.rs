/// The unique Specta ID for the type.
///
/// Be aware type aliases don't exist as far as Specta is concerned as they are flattened into their inner type by Rust's trait system.
/// The Specta Type ID holds for the given properties:
///  - `T::SID == T::SID`
///  - `T::SID != S::SID`
///  - `Type<T>::SID == Type<S>::SID` (unlike std::any::TypeId)
///  - `&'a T::SID == &'b T::SID` (unlike std::any::TypeId which forces a static lifetime)
///  - `Box<T> == Arc<T> == Rc<T>` (unlike std::any::TypeId)
///  - `crate_a@v1::T::SID == crate_a@v2::T::SID` (unlike std::any::TypeId)
///
// TODO: Encode the properties above into unit tests.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SpectaID(pub(crate) SpectaIDInner);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SpectaIDInner {
    /// A statically defined hash
    /// This will be consistent across `TypeCollection`'s.
    Static(u64),
    /// An identifier issued by a specific `TypeCollection` for a runtime-defined type.
    Virtual(u64),
}

impl SpectaID {
    /// is this identifier valid for any `TypeCollection`.
    /// This is true for types that were declared with `#[derive(Type)]`.
    pub fn is_static(&self) -> bool {
        matches!(self.0, SpectaIDInner::Static(_))
    }

    /// is this identifier tied to the `TypeCollection` it was defined with.
    /// This is true for types that were defined with `TypeCollection::declare`.
    pub fn is_virtual(&self) -> bool {
        matches!(self.0, SpectaIDInner::Virtual(_))
    }
}

pub(crate) fn r#virtual(id: u64) -> SpectaID {
    SpectaID(SpectaIDInner::Virtual(id))
}

/// Compute an SID hash for a given type.
/// This will produce a type hash from the arguments.
/// This hashing function was derived from <https://stackoverflow.com/a/71464396>
// Exposed as `specta::internal::construct::sid`
pub const fn sid(type_name: &'static str, type_identifier: &'static str) -> SpectaID {
    let mut hash = 0xcbf29ce484222325;
    let prime = 0x00000100000001B3;

    let mut bytes = type_name.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    bytes = type_identifier.as_bytes();
    i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    SpectaID(crate::specta_id::SpectaIDInner::Static(hash))
}
