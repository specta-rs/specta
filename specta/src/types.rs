use std::{
    collections::hash_map,
    fmt,
    ops::Range,
    panic::{self, AssertUnwindSafe},
};

use crate::{
    Type,
    datatype::{DataType, Generic, NamedDataType, NamedId, NamedReference, RecursiveInlineFrame},
    hash::FastHashMap,
};

/// Collection of named datatypes that can be exported together.
///
/// Resolving a [`Type`] adds every named type it depends on to this collection.
/// Exporters usually receive a completed `Types` value and iterate over the
/// collected [`NamedDataType`] entries.
///
/// # Invariants
///
/// Internally, entries may temporarily be placeholders while recursive types are
/// resolving. Public iterators and [`Types::len`] expose only completed
/// [`NamedDataType`] values.
#[derive(Default, Clone)]
pub struct Types {
    /// Concrete datatypes returned by explicit top-level registrations.
    roots: Vec<DataType>,

    /// Registered named datatypes keyed by their stable identity.
    ///
    /// A `None` value is a placeholder for a datatype whose definition is
    /// currently being resolved. This lets recursive definitions refer to the
    /// in-progress type without re-entering resolution indefinitely.
    pub(crate) types: FastHashMap<NamedId, Option<NamedDataType>>,

    /// Cached count of completed entries in [`Self::types`].
    ///
    /// Placeholders are excluded. Keeping this count avoids repeatedly walking
    /// the full map when exporters ask for iterator lengths which we need for `ExactSizeIterator`.
    pub(crate) len: usize,

    /// Stack of inline named-type expansions currently being resolved.
    ///
    /// Each entry is a hash of the named type sentinel and concrete generic
    /// arguments for that inline use site. Seeing the same entry twice means an
    /// inline definition has recursively reached itself, so resolution can emit
    /// a recursive reference instead of expanding forever.
    pub(crate) stack: Vec<InlineResolutionFrame>,

    /// Generic-argument identities of the frames in [`Self::stack`].
    ///
    /// All frames share this buffer so that a frame push does not need an
    /// allocation of its own. Each frame records the range that belongs to it.
    stack_identities: Vec<&'static str>,

    /// Generic arguments of the frames in [`Self::stack`], shared the same way.
    stack_generics: Vec<(Generic, DataType)>,

    /// Whether named types discovered in the current context should be inlined.
    ///
    /// This is set while resolving fields annotated with `#[specta(inline)]` and
    /// similar container/wrapper contexts. It is temporarily cleared when
    /// building canonical named definitions so top-level registrations are not
    /// accidentally affected by a use-site inline request.
    pub(crate) should_inline: bool,

    /// Whether the current named-type definition is being built with const parameters.
    ///
    /// This remains `false` unless Specta is exporting the canonical definition
    /// for a `#[derive(Type)]` type that declares one or more const-generic
    /// parameters.
    ///
    /// Consider a type like this:
    ///
    /// ```rs
    /// #[derive(Type)]
    /// struct Demo<const N: usize> {
    ///     data: [u32; N],
    /// }
    /// ```
    ///
    /// If `impl Type for [T; N]` always exported the concrete array length, the
    /// first encountered value of `N` would be baked into the shared global
    /// definition for `Demo`, which is wrong. For example:
    ///
    /// ```rs
    /// pub struct A {
    ///     a: Demo<1>,
    ///     b: Demo<2>,
    /// }
    /// // becomes:
    /// // export type A = { a: Demo, b: Demo }
    /// // export type Demo = { [number] }; // This is invalid for the `b` field.
    ///
    /// // and if we encounter the fields in the opposite order it changes:
    ///
    /// pub struct B {
    ///     // we flipped field definition
    ///     b: Demo<2>,
    ///     a: Demo<1>,
    /// }
    /// // becomes:
    /// // export type A = { a: Demo, b: Demo }
    /// // export type Demo = { [number, number] }; // This is invalid for the `a` field.
    /// ```
    ///
    /// For a length to differ across two instantiations of the same type, the
    /// type must either have a const parameter or have a generic parameter whose
    /// type uses a trait associated constant.
    ///
    /// Specta does not support the trait-associated-constant case here because
    /// generic `T` parameters are shadowed by virtual structs that return generic
    /// references instead of flat datatypes.
    ///
    /// Therefore, including the fixed array length is safe as long as the
    /// current resolving context has no const parameters. This is tracked at
    /// runtime, avoiding brittle scans of the user's `TokenStream` in the derive
    /// macro.
    ///
    /// `specta_util::FixedArray<N, T>` can be used to force Specta to export a
    /// fixed-length array instead of a generic `number[]` when the user knows
    /// the length is safe to include. This does not fix the core issue, but it
    /// gives the user a way to assert the specific use site is correct.
    pub(crate) has_const_params: bool,

    /// Active generic-parameter substitutions, innermost scope last.
    ///
    /// While `#[derive(Type)]` builds the canonical declaration for a generic
    /// type it records each parameter here so that resolving the parameter's
    /// concrete type through [`Types::define`] yields a `Generic` placeholder
    /// instead. This lets the declaration be built from the *concrete* generic
    /// arguments (which satisfies any user trait bounds a field's own type
    /// declaration requires) without baking those arguments in.
    pub(crate) generic_substitutions: Vec<GenericSubstitution>,
}

/// One active generic-substitution scope.
///
/// `slots` holds one entry per type argument of the field type being resolved,
/// in source order, and `cursor` is the next slot [`Types::define_param`] will
/// consume positionally. `consumed` records slots taken by name when a type
/// alias reorders or fixes arguments. Duplicate concrete types stay positional
/// because their names cannot distinguish them; see
/// <https://github.com/specta-rs/specta/issues/609>.
#[derive(Clone)]
pub(crate) struct GenericSubstitution {
    pub(crate) slots: Vec<GenericSubstitutionSlot>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) cursor: usize,
}

impl GenericSubstitution {
    fn skip_consumed(&mut self) {
        while self
            .consumed
            .get(self.cursor)
            .is_some_and(|consumed| *consumed)
        {
            self.cursor += 1;
        }
    }

    /// Takes the first unconsumed slot at or after the cursor.
    fn take_slot(&mut self) -> Option<GenericSubstitutionSlot> {
        self.skip_consumed();
        let slot = self.slots.get(self.cursor)?.clone();
        self.consumed[self.cursor] = true;
        self.cursor += 1;
        Some(slot)
    }

    /// Takes a specific unconsumed slot.
    fn take_index(&mut self, index: usize) -> Option<GenericSubstitutionSlot> {
        if self.consumed.get(index).copied().unwrap_or(true) {
            return None;
        }

        self.consumed[index] = true;
        if index >= self.cursor {
            self.cursor = index + 1;
        }
        self.slots.get(index).cloned()
    }
}

/// One type-argument slot of a generic-substitution scope.
///
/// The derive builds these from the field type's argument list: a bare
/// enclosing parameter becomes [`GenericSubstitutionSlot::Generic`], a compound
/// argument that mentions enclosing parameters becomes
/// [`GenericSubstitutionSlot::Nested`], and anything else is
/// [`GenericSubstitutionSlot::Skip`].
#[doc(hidden)]
#[derive(Clone)]
pub enum GenericSubstitutionSlot {
    /// The argument is an enclosing parameter's concrete type; rewrite it.
    Generic {
        /// [`std::any::type_name`] of the enclosing parameter's concrete type.
        name: &'static str,
        /// The `Generic` to rewrite the argument to.
        generic: Generic,
    },
    /// The argument is a compound type; resolve it with these nested slots.
    Nested(Vec<GenericSubstitutionSlot>),
    /// The argument mentions no enclosing parameter.
    Skip,
}

impl GenericSubstitutionSlot {
    /// The concrete type name this slot rewrites, if any.
    fn name(&self) -> Option<&'static str> {
        match self {
            Self::Generic { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// One entry on the inline resolution stack.
///
/// Only the identity of the use site is stored. The human-readable description
/// (module path, type name, rendered generic arguments) is built on demand when
/// a cycle is reported, because frames are pushed for every inline and
/// passthrough use site while cycles are rare.
#[derive(Debug, Clone)]
pub(crate) struct InlineResolutionFrame {
    pub(crate) id: NamedId,
    /// Range of this frame's generic-argument identities in
    /// [`Types::stack_identities`].
    ///
    /// Kept as a range rather than a `Vec` so pushing a frame does not need its
    /// own allocation: frames are pushed and popped for every inline use site
    /// and the arguments are tiny.
    identities: Range<usize>,
    /// Range of this frame's generic arguments in [`Types::stack_generics`].
    ///
    /// Needed because an inlined generic type records its arguments as
    /// placeholders, so two different instantiations can carry identical
    /// [`DataType`] arguments.
    generics: Range<usize>,
}

impl InlineResolutionFrame {
    /// Whether this frame is the same use site as `id` with `generics`.
    ///
    /// This replaces the previous hashed comparison: comparing the identity and
    /// arguments directly is both cheaper (no hashing of whole `DataType`
    /// trees) and free of hash collisions, which could previously report a
    /// cycle that did not exist.
    pub(crate) fn is_use_site(
        &self,
        types: &Types,
        id: &NamedId,
        generic_identities: &[&'static str],
        generics: &[(Generic, DataType)],
    ) -> bool {
        // Cheapest comparisons first; the datatypes are the expensive part.
        self.id == *id
            && types.stack_identities[self.identities.clone()] == *generic_identities
            && types.stack_generics[self.generics.clone()] == *generics
    }

    /// Renders this frame's description for a reported cycle.
    pub(crate) fn describe(&self, types: &Types) -> RecursiveInlineFrame {
        RecursiveInlineFrame::describe(
            types,
            &self.id,
            &types.stack_generics[self.generics.clone()],
        )
    }
}

impl fmt::Debug for Types {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Types")
            .field("types", &self.types)
            .field("roots", &self.roots)
            .finish()
    }
}

impl Types {
    /// Registers `T` and its named dependencies with the collection.
    ///
    /// This consumes and returns `self`, making it convenient to chain multiple
    /// registrations.
    pub fn register<T: Type>(mut self) -> Self {
        self.register_mut::<T>();
        self
    }

    /// Registers `T` and its named dependencies with the collection in-place.
    pub fn register_mut<T: Type>(&mut self) -> &mut Self {
        let root = T::definition(self);
        self.roots.push(root);
        self
    }

    /// Resolves `T` like [`Type::definition`], honouring any active generic
    /// substitution.
    ///
    /// This exists so the `#[derive(Type)]` generated code can resolve a field
    /// type through the concrete generic arguments (satisfying any bounds the
    /// field's own type declaration requires) while still emitting `Generic`
    /// placeholders into the canonical declaration. See
    /// [`Types::with_generic_substitution`].
    ///
    /// # Contract
    ///
    /// Every generic `Type` implementation — the derive, the built-in impls in
    /// this crate, `specta_util`, exporters, and hand-written impls — **must**
    /// resolve its generic arguments through this method (or
    /// [`Types::define_param`]) rather than `<T as Type>::definition(types)`.
    /// Otherwise a canonical declaration can bake in the first instantiation's
    /// concrete argument.
    ///
    /// Generated implementations resolve *positionally* with
    /// [`Types::define_param`]. Hand-written ones call this method, which looks
    /// the argument up by [`std::any::type_name`] in the **innermost** scope
    /// only. It must not fall through to an enclosing scope: the argument may
    /// happen to match an enclosing parameter's concrete type and would leak
    /// into this declaration.
    ///
    /// A compound argument that mentions enclosing parameters is resolved with
    /// the nested scope the derive paired with its slot.
    #[doc(hidden)]
    pub fn define<T: Type + ?Sized>(&mut self) -> DataType {
        let name = std::any::type_name::<T>();

        if let Some(generic) = self.take_substitution(name) {
            return DataType::Generic(generic);
        }

        if let Some(slots) = self.take_nested_slot() {
            return self.with_generic_substitution(&slots, |types| T::definition(types));
        }

        T::definition(self)
    }

    /// Resolves the field type itself, like [`Types::define`].
    ///
    /// Unlike [`Types::define`], this never consumes a
    /// [`GenericSubstitutionSlot::Nested`] slot: the scope's slots describe the
    /// field type's *arguments*, and a compound argument belongs to the field
    /// type's own resolution rather than to the field type. It only rewrites a
    /// bare-parameter field (`struct A<T> { a: T }`), consuming that parameter's
    /// slot.
    #[doc(hidden)]
    pub fn define_root<T: Type + ?Sized>(&mut self) -> DataType {
        let name = std::any::type_name::<T>();

        if let Some(generic) = self.take_substitution(name) {
            return DataType::Generic(generic);
        }

        T::definition(self)
    }

    /// Resolves the next positional slot of the innermost substitution scope,
    /// like [`Types::define`].
    ///
    /// The derive and the built-in impl macros resolve each of their own type
    /// parameters through this method. A parameter whose concrete type names
    /// exactly one unconsumed slot takes that slot, which keeps aliases that
    /// reorder or fix arguments correct. Duplicate concrete types fall back to
    /// positional order so siblings with the same concrete type stay distinct.
    /// Parameters that are not resolved at all (`#[specta(skip)]`, unused) are
    /// stepped over with [`Types::skip_param_slot`] so the remaining parameters
    /// stay aligned.
    #[doc(hidden)]
    pub fn define_param<T: Type + ?Sized>(&mut self) -> DataType {
        let name = std::any::type_name::<T>();
        let nested = {
            let Some(scope) = self.generic_substitutions.last_mut() else {
                return T::definition(self);
            };

            // A type alias can reorder or fix arguments without changing the
            // field's source syntax. Match a concrete parameter by name when
            // exactly one unconsumed slot names it; duplicate concrete types
            // stay positional because their names cannot distinguish them.
            let matches = scope
                .slots
                .iter()
                .enumerate()
                .filter(|(index, slot)| {
                    !scope.consumed.get(*index).copied().unwrap_or(true)
                        && slot.name() == Some(name)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();

            if let [index] = matches.as_slice() {
                return match scope.take_index(*index) {
                    Some(GenericSubstitutionSlot::Generic { generic, .. }) => {
                        DataType::Generic(generic)
                    }
                    _ => unreachable!("a name-matched slot is always a generic"),
                };
            }

            scope.skip_consumed();
            let next = scope.slots.get(scope.cursor).cloned();
            match next {
                Some(GenericSubstitutionSlot::Generic {
                    name: slot_name, ..
                }) if matches.is_empty() && slot_name != name => {
                    // This slot belongs to a different enclosing parameter.
                    // An alias may have inserted an argument that has no slot.
                    None
                }
                Some(GenericSubstitutionSlot::Generic { .. }) => match scope.take_slot() {
                    Some(GenericSubstitutionSlot::Generic { generic, .. }) => {
                        return DataType::Generic(generic);
                    }
                    _ => unreachable!("selected a generic slot"),
                },
                Some(GenericSubstitutionSlot::Nested(_)) => match scope.take_slot() {
                    Some(GenericSubstitutionSlot::Nested(slots)) => Some(slots),
                    _ => unreachable!("selected a nested slot"),
                },
                Some(GenericSubstitutionSlot::Skip) | None => {
                    scope.take_slot();
                    None
                }
            }
        };

        match nested {
            Some(slots) => self.with_generic_substitution(&slots, |types| T::definition(types)),
            None => T::definition(self),
        }
    }

    /// Steps over one positional slot of the innermost substitution scope.
    ///
    /// Used for a type parameter a generated implementation does not resolve,
    /// so the parameters after it still line up with their slots.
    #[doc(hidden)]
    pub fn skip_param_slot(&mut self) {
        if let Some(scope) = self.generic_substitutions.last_mut() {
            scope.take_slot();
        }
    }

    /// Takes the slot at the cursor if it is a compound argument's nested scope.
    fn take_nested_slot(&mut self) -> Option<Vec<GenericSubstitutionSlot>> {
        let scope = self.generic_substitutions.last_mut()?;
        scope.skip_consumed();

        let Some(GenericSubstitutionSlot::Nested(_)) = scope.slots.get(scope.cursor) else {
            return None;
        };

        match scope.take_slot() {
            Some(GenericSubstitutionSlot::Nested(slots)) => Some(slots),
            _ => unreachable!("checked the current slot is nested"),
        }
    }

    /// Takes the first unconsumed innermost slot whose concrete type name is
    /// `name`. Distinct concrete types stay order-independent.
    fn take_substitution(&mut self, name: &'static str) -> Option<Generic> {
        let scope = self.generic_substitutions.last_mut()?;

        let index = scope.slots.iter().enumerate().find_map(|(index, slot)| {
            (!scope.consumed.get(index).copied().unwrap_or(true) && slot.name() == Some(name))
                .then_some(index)
        })?;

        match scope.take_index(index)? {
            GenericSubstitutionSlot::Generic { generic, .. } => Some(generic),
            _ => None,
        }
    }

    /// Whether any active scope holds a slot that could be matched.
    fn has_possible_substitution(&self) -> bool {
        self.generic_substitutions
            .iter()
            .any(|scope| !scope.slots.is_empty())
    }

    /// Returns the cursor of the innermost active substitution scope.
    ///
    /// Used by the derive and the built-in impl macros to rewind a scope to the
    /// position it held before a type's generic arguments were resolved. The
    /// passthrough impls resolve those arguments more than once (once for the
    /// use-site reference and once while inlining), and each traversal must
    /// start from the same slot or they would desynchronise.
    #[doc(hidden)]
    pub fn generic_substitution_cursor(&self) -> usize {
        self.generic_substitutions
            .last()
            .map_or(0, |scope| scope.cursor)
    }

    /// Rewinds the innermost active substitution scope to `cursor`.
    #[doc(hidden)]
    pub fn set_generic_substitution_cursor(&mut self, cursor: usize) {
        if let Some(scope) = self.generic_substitutions.last_mut() {
            scope.cursor = cursor.min(scope.slots.len());
            // Re-resolving from an earlier point must make those slots
            // available again, including slots consumed out of order by a
            // name match.
            for consumed in &mut scope.consumed[scope.cursor..] {
                *consumed = false;
            }
        }
    }

    /// Runs `f` with `substitutions` active for the duration of the call.
    ///
    /// `substitutions` holds one [`GenericSubstitutionSlot`] per type argument
    /// of the type being resolved, in source order. While active,
    /// [`Types::define_param`] takes a uniquely named slot wherever it appears
    /// and otherwise consumes slots positionally. [`Types::define`] matches by
    /// [`std::any::type_name`] in the **innermost** scope only, so a nested
    /// declaration cannot pick up an enclosing parameter. Scopes nest: the
    /// innermost scope is consulted first.
    ///
    /// The derive builds the slots in the field type's argument order (or, for
    /// a field type it cannot analyse, in the container's parameter order);
    /// generated implementations resolve each parameter with
    /// [`Types::define_param`], stepping over unresolved ones with
    /// [`Types::skip_param_slot`]. Hand-written implementations resolve each
    /// argument with [`Types::define`], which falls back to matching by name.
    ///
    /// The scope is restored on unwind (the panic is then re-raised) so a panic
    /// in a nested resolution cannot leak substitutions into the surrounding
    /// scope.
    #[doc(hidden)]
    pub fn with_generic_substitution<R>(
        &mut self,
        substitutions: &[GenericSubstitutionSlot],
        f: impl FnOnce(&mut Types) -> R,
    ) -> R {
        // An empty scope exists only to hide enclosing scopes. When no enclosing
        // scope can match anything the scope is a no-op, which is the common case
        // for field types that do not mention a parameter.
        if substitutions.is_empty() && !self.has_possible_substitution() {
            return f(self);
        }

        self.generic_substitutions.push(GenericSubstitution {
            consumed: vec![false; substitutions.len()],
            slots: substitutions.to_vec(),
            cursor: 0,
        });
        let result = panic::catch_unwind(AssertUnwindSafe(|| f(self)));
        self.generic_substitutions.pop();
        match result {
            Ok(result) => result,
            Err(payload) => panic::resume_unwind(payload),
        }
    }

    /// Records `id` with its generic arguments as an inline use site that is
    /// currently being expanded.
    pub(crate) fn push_inline_frame(
        &mut self,
        id: NamedId,
        generic_identities: &[&'static str],
        generics: &[(Generic, DataType)],
    ) {
        let identities = self.stack_identities.len();
        self.stack_identities.extend_from_slice(generic_identities);
        let arguments = self.stack_generics.len();
        self.stack_generics.extend_from_slice(generics);

        self.stack.push(InlineResolutionFrame {
            id,
            identities: identities..self.stack_identities.len(),
            generics: arguments..self.stack_generics.len(),
        });
    }

    /// Removes the most recently pushed inline use site.
    pub(crate) fn pop_inline_frame(&mut self) {
        let Some(frame) = self.stack.pop() else {
            return;
        };

        self.stack_identities.truncate(frame.identities.start);
        self.stack_generics.truncate(frame.generics.start);
    }

    /// Returns the oldest frame that is the same use site as `id` with
    /// `generic_identities` and `generics`, if the current expansion is
    /// recursively reaching itself.
    pub(crate) fn inline_cycle_start(
        &self,
        id: &NamedId,
        generic_identities: &[&'static str],
        generics: &[(Generic, DataType)],
    ) -> Option<usize> {
        self.stack
            .iter()
            .position(|frame| frame.is_use_site(self, id, generic_identities, generics))
    }

    /// Returns the concrete datatypes explicitly registered as top-level roots.
    ///
    /// Named dependencies discovered while resolving a root are available
    /// through the normal type iterators and are not repeated here.
    pub fn roots(&self) -> impl ExactSizeIterator<Item = &'_ DataType> {
        self.roots.iter()
    }

    /// Transforms the explicitly registered top-level roots.
    ///
    /// Graph transformations that change datatypes must use this in addition
    /// to [`Types::map`], otherwise a root can keep referring to the
    /// pre-transformation shape.
    pub fn map_roots<F>(mut self, f: F) -> Self
    where
        F: FnMut(DataType) -> DataType,
    {
        self.roots = std::mem::take(&mut self.roots).into_iter().map(f).collect();
        self
    }

    /// Gets the named datatype targeted by a [`NamedReference`].
    ///
    /// Returns `None` if the reference is unknown or currently only has an
    /// internal placeholder entry.
    pub fn get(&self, r: &NamedReference) -> Option<&NamedDataType> {
        self.types.get(&r.id)?.as_ref()
    }

    /// Gets the named datatype targeted by a [`NamedReference`] mutably.
    ///
    /// The entry's identity cannot change; mutating the returned value updates
    /// the same graph node that the reference resolves to.
    pub fn get_mut(&mut self, r: &NamedReference) -> Option<&mut NamedDataType> {
        self.types.get_mut(&r.id)?.as_mut()
    }

    /// Returns the number of completed named datatypes in the collection.
    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.len,
            self.types.values().filter_map(|ndt| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.len
    }

    /// Returns `true` when the typemap has no entries at all.
    pub fn is_empty(&self) -> bool {
        debug_assert_eq!(
            self.len,
            self.types.values().filter_map(|ndt| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.len == 0
    }

    /// Merges types from another collection into this one.
    ///
    /// Existing completed entries in `self` are kept. A placeholder in `self` is
    /// replaced by a completed entry from `other` when available.
    pub fn extend(&mut self, other: &Self) {
        self.roots.extend(other.roots.iter().cloned());
        for (id, other) in &other.types {
            match self.types.get(id) {
                // Key doesn't exist - insert from other
                None => {
                    if other.is_some() {
                        self.len += 1;
                    }
                    self.types.insert(id.clone(), other.clone());
                }
                // Key exists with Some - keep self (prefer self over other)
                Some(Some(_)) => {}
                // Key exists with None, but other has Some - use other (prefer Some over None)
                Some(None) if other.is_some() => {
                    self.len += 1;
                    self.types.insert(id.clone(), other.clone());
                }
                // Key exists with None, other also None - do nothing
                Some(None) => {}
            }
        }
    }

    /// Sorts completed named datatypes into a consistent order and returns an iterator.
    ///
    /// The sort order is not guaranteed to remain identical between releases but is designed to stay stable,
    /// so that between multiple runs of the exporter you get the same type in the same order in the file.
    ///
    /// This method allocates a temporary vector to sort the collection. Prefer
    /// [`Types::into_unsorted_iter`] if the order does not matter.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        let mut v = self
            .types
            .values()
            .filter_map(|ndt| ndt.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(v.len(), self.len, "Types count logic mismatch");
        v.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        v.into_iter()
    }

    /// Returns an unsorted iterator over completed named datatypes.
    pub fn into_unsorted_iter(&self) -> impl ExactSizeIterator<Item = &NamedDataType> {
        UnsortedIter {
            iter: self.types.iter(),
            count: self.len,
        }
    }

    /// Mutably modifies each [`NamedDataType`] in the collection.
    ///
    /// The named type's identity is preserved across the callback, including
    /// when the callback replaces the whole value. References to the type
    /// therefore keep resolving to this entry.
    pub fn iter_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        for (id, slot) in &mut self.types {
            if let Some(ndt) = slot {
                f(ndt);
                ndt.id = id.clone();
            }
        }
    }

    /// Transforms each [`NamedDataType`] in the collection.
    ///
    /// The named type's identity is preserved across the callback, including
    /// when the callback returns a clone of a different named type.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for (id, slot) in &mut self.types {
            if let Some(ndt) = slot.take() {
                let mut mapped = f(ndt);
                mapped.id = id.clone();
                *slot = Some(mapped);
            }
        }
        self
    }
}

struct UnsortedIter<'a> {
    iter: hash_map::Iter<'a, NamedId, Option<NamedDataType>>,
    count: usize,
}

impl<'a> Iterator for UnsortedIter<'a> {
    type Item = &'a NamedDataType;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, ndt)| ndt.as_ref()).inspect(|_| {
            self.count -= 1;
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl ExactSizeIterator for UnsortedIter<'_> {}
