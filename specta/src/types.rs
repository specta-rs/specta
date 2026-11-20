use std::{
    borrow::Cow,
    collections::hash_map,
    fmt,
    ops::Range,
    panic::{self, AssertUnwindSafe, Location},
};

use crate::{
    Type,
    datatype::{
        self, DataType, Deprecated, Generic, GenericDefinition, NamedDataType, NamedId,
        NamedReference, RecursiveInlineFrame,
    },
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
/// consume. Ordering by *position* (rather than by `type_name`) is what fixes
/// <https://github.com/specta-rs/specta/issues/609>: a concrete type that
/// appears twice cannot say which parameter it came from, but the nested type
/// resolves its arguments in the same order they appear in the field type.
#[derive(Clone)]
pub(crate) struct GenericSubstitution {
    pub(crate) slots: Vec<GenericSubstitutionSlot>,
    /// Which slots have been taken, so a slot can be taken by name out of
    /// order (a type alias may reorder or fix arguments) without being handed
    /// out twice. Rewinding the cursor clears it from there onwards.
    pub(crate) consumed: Vec<bool>,
    pub(crate) cursor: usize,
}

impl GenericSubstitution {
    /// Takes the slot at the cursor, advancing past it.
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

/// One type parameter of a generic `Type` implementation, in declaration order.
///
/// A derived `definition` builds an array of these and hands it to
/// [`Types::define_instantiation_params`], which resolves each parameter through
/// the active generic substitution and builds the instantiation metadata. That
/// keeps the per-instantiation scaffolding compiled once in this crate instead
/// of once per monomorphised `definition`.
#[doc(hidden)]
pub enum InstantiationParam {
    /// A parameter this implementation resolves.
    Resolved {
        /// The parameter's name, as written.
        name: &'static str,
        /// [`std::any::type_name`] of the concrete argument.
        ///
        /// Resolution matches a substitution slot by concrete type when exactly
        /// one slot names it, which is what keeps a type alias that reorders or
        /// fixes its arguments aligned (see [`Types::define_param`]).
        identity: &'static str,
        /// `<T as Type>::definition`.
        definition: fn(&mut Types) -> DataType,
    },
    /// A parameter this implementation never resolves: it is always reported as
    /// its own `Generic` placeholder.
    ///
    /// The built-in `impl_ndt!` containers use this for an inline definition
    /// whose `where` clause keeps the parameter opaque, so its reference carries
    /// the declaration's `Generic` rather than the concrete argument.
    Generic {
        /// The parameter's name, as written.
        name: &'static str,
    },
    /// A parameter this implementation does not resolve. Its substitution slot
    /// is stepped over so the parameters after it stay aligned (see issue
    /// <https://github.com/specta-rs/specta/issues/609>).
    Skipped,
}

/// Everything one instantiation of a named datatype fixes, gathered so
/// [`Types::instantiate`] can build it without being generic.
struct Instantiation<'a> {
    sentinel: &'static str,
    name: &'static str,
    module_path: &'static str,
    generics: Cow<'static, [GenericDefinition]>,
    docs: &'static str,
    /// Borrowed rather than owned so a `static` path can pass a reference to
    /// its metadata: an owned `Option<Deprecated>` is a temporary that every
    /// monomorphised `definition` would need a drop flag, drop glue and a
    /// cleanup path for.
    deprecated: Option<&'a Deprecated>,
    instantiation_generics: Vec<(Generic, DataType)>,
    instantiation_generic_identities: &'a [&'static str],
    has_const_param: bool,
    inline: bool,
    passthrough: bool,
    generic_substitution_base: usize,
    generic_substitution_after: usize,
    location: Location<'static>,
    build_ty: fn(&mut Types) -> DataType,
    build_reference_ty: fn(&mut Types) -> DataType,
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
    /// Call this from a hand-written generic [`Type`](crate::Type) implementation
    /// instead of `<T as Type>::definition(types)`. When the caller is building a
    /// type's canonical *declaration*, an active scope maps the argument to a
    /// [`Generic`] node rather than one concrete definition; otherwise this
    /// simply calls `T`'s definition. A compound argument that mentions
    /// enclosing parameters resolves through the nested scope the caller paired
    /// with its slot, so a field type keeps its generic parameters instead of
    /// baking in the first instantiation.
    ///
    /// Generated implementations resolve *positionally* with
    /// [`Types::define_param`] instead, because their parameters are known at
    /// expansion time and two of them may share a concrete type. This method
    /// looks `T` up by [`std::any::type_name`] in the **innermost** scope only —
    /// it must not fall through to an enclosing one, or the argument would leak
    /// into a nested declaration.
    ///
    /// # Contract
    ///
    /// Resolve each argument **once**, in declaration order: the scope hands out
    /// its slots positionally, so a skipped or reordered argument misaligns
    /// every later one. That produces a silently wrong export rather than a
    /// compile error.
    ///
    /// # Example
    ///
    /// ```
    /// use specta::{datatype::DataType, Type, Types};
    ///
    /// # #[allow(dead_code)]
    /// struct Wrapper<T>(T);
    ///
    /// impl<T: Type> Type for Wrapper<T> {
    ///     fn definition(types: &mut Types) -> DataType {
    ///         // Not `<T as Type>::definition(types)`: resolving through
    ///         // `define` records `T` as a generic parameter of the
    ///         // declaration instead of one concrete argument.
    ///         types.define::<T>()
    ///     }
    /// }
    ///
    /// let mut types = Types::default();
    /// let _: DataType = <Wrapper<u8> as Type>::definition(&mut types);
    /// ```
    pub fn define<T: Type + ?Sized>(&mut self) -> DataType {
        self.define_with(std::any::type_name::<T>(), T::definition)
    }

    /// The non-generic body of [`Types::define`].
    ///
    /// Keeping it out of the generic method means a crate with many
    /// instantiations compiles this logic once instead of once per `T`: each
    /// `define::<T>` call site is only `type_name` plus one indirect call.
    fn define_with(&mut self, name: &'static str, f: fn(&mut Types) -> DataType) -> DataType {
        if let Some(generic) = self.take_substitution(name) {
            return DataType::Generic(generic);
        }

        if let Some(slots) = self.take_nested_slot() {
            return self.with_generic_substitution_fn(&slots, f);
        }

        f(self)
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
        self.define_root_with(std::any::type_name::<T>(), T::definition)
    }

    /// The non-generic body of [`Types::define_root`].
    fn define_root_with(&mut self, name: &'static str, f: fn(&mut Types) -> DataType) -> DataType {
        match self.take_substitution(name) {
            Some(generic) => DataType::Generic(generic),
            None => f(self),
        }
    }

    /// Rejects two different types claiming the same registration sentinel.
    ///
    /// The sentinel is a type's identity in [`Types`]. Two types sharing one
    /// would silently export as a single named type — the second registration
    /// sees the first and returns a reference to it — so this turns a
    /// near-undebuggable export bug into a failing assertion. Only debug builds
    /// pay for the lookup; a generated or hand-written implementation that
    /// re-registers the same type (every generic instantiation does) passes,
    /// because its name is unchanged.
    #[cfg(debug_assertions)]
    fn assert_sentinel_owner(&self, sentinel: &'static str, name: &str) {
        let Some(Some(existing)) = self.types.get(&NamedId::Static(sentinel)) else {
            return;
        };
        assert!(
            existing.name.as_ref() == name,
            "specta: two types are registered under the same sentinel `{sentinel}` \
             (`{}` and `{name}`); a sentinel must identify exactly one type",
            existing.name,
        );
    }

    /// Registers a named datatype that declares no generic parameters.
    ///
    /// This is [`NamedDataType::init_with_sentinel`] with every argument that
    /// the `#[derive(Type)]` output would fix up front: no instantiation
    /// generics or identities, no const parameters and no passthrough, with the
    /// caller's cursor as the substitution base. The build function both fills
    /// `NamedDataType::ty` and serves as the inline definition.
    ///
    /// It exists so the derive — for which this is the common case — emits one
    /// call instead of the full argument list plus a closure that only assigns
    /// the metadata it was given. It is `#[track_caller]` so the recorded
    /// [`NamedDataType::location`] stays the derive's own.
    ///
    /// `build_ty` runs inside one empty generic-substitution scope, which keeps
    /// the fields of a non-generic container from seeing an enclosing
    /// declaration's scope (issue #609). Doing it here means the derive does not
    /// emit a fresh `with_generic_substitution(&[], ..)` closure — and so a
    /// second copy of the whole body — for every type it derives.
    #[doc(hidden)]
    #[track_caller]
    pub fn define_named(
        &mut self,
        sentinel: &'static str,
        name: &'static str,
        module_path: &'static str,
        docs: &'static str,
        deprecated: Option<Deprecated>,
        build_ty: fn(&mut Types) -> DataType,
    ) -> DataType {
        #[cfg(debug_assertions)]
        self.assert_sentinel_owner(sentinel, name);

        let generic_substitution_base = self.generic_substitution_cursor();
        let location = Location::caller().to_owned();

        self.with_generic_substitution(&[], |types| {
            NamedDataType::init(
                sentinel,
                Vec::new(),
                &[],
                false,
                false,
                generic_substitution_base,
                location,
                types,
                &|types, ndt| {
                    ndt.name = Cow::Borrowed(name);
                    ndt.module_path = Cow::Borrowed(module_path);
                    ndt.docs = Cow::Borrowed(docs);
                    ndt.deprecated = deprecated.clone();
                    ndt.ty = Some(build_ty(types));
                },
                build_ty,
            )
            .into()
        })
    }

    /// Registers a named datatype with the arguments a generic `Type`
    /// implementation fixes per instantiation.
    ///
    /// This is the non-generic half of [`NamedDataType::init_with_sentinel`]:
    /// the derived code and the built-in container implementations in this
    /// crate pass everything that only depends on the concrete instantiation —
    /// the name, module path, generic declarations and the two datatype builders
    /// — as arguments, and both the metadata builder and the inline wrapping are
    /// compiled once in this crate. Previously each concrete `Vec<X>`,
    /// `Option<X>`, ... instantiation compiled its own copy of those closure
    /// bodies.
    ///
    /// `generics` and `deprecated` are borrowed, so a `definition` body passes
    /// promoted constants and a reference to a `static` instead of building a
    /// `Cow` and an `Option<Deprecated>` temporary. Those are the difference
    /// between the body being the parameter table plus one call and it also
    /// carrying two drop flags, two drop-glue calls and a cleanup path — once
    /// per monomorphisation.
    ///
    /// `params` holds one entry per type parameter of the implementation, in
    /// declaration order. This resolves them exactly like the generated code
    /// used to — [`Types::define_param`] for a parameter the implementation
    /// resolves, [`Types::skip_param_slot`] for one it does not — and reads the
    /// substitution cursor around them itself.
    ///
    /// The point is code size: a crate with many instantiations otherwise
    /// compiles the array construction and drop glue of every `definition` body
    /// once per concrete argument.
    #[doc(hidden)]
    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub fn define_instantiation_params(
        &mut self,
        sentinel: &'static str,
        name: &'static str,
        module_path: &'static str,
        generics: &'static [GenericDefinition],
        docs: &'static str,
        deprecated: Option<&'static Deprecated>,
        params: &[InstantiationParam],
        instantiation_generic_identities: &[&'static str],
        has_const_param: bool,
        inline: bool,
        passthrough: bool,
        build_ty: fn(&mut Types) -> DataType,
        build_reference_ty: fn(&mut Types) -> DataType,
    ) -> DataType {
        let location = Location::caller().to_owned();
        self.instantiate_params(
            sentinel,
            name,
            module_path,
            Cow::Borrowed(generics),
            docs,
            deprecated,
            params,
            instantiation_generic_identities,
            has_const_param,
            inline,
            passthrough,
            location,
            build_ty,
            build_reference_ty,
        )
    }

    /// [`Types::define_instantiation_params`] with owned declarations.
    ///
    /// Only the derived implementation's generic-default path needs this: a
    /// parameter default can name another parameter (`U = T`), so its
    /// `GenericDefinition` list is resolved at runtime and cannot be a
    /// `&'static` slice.
    #[doc(hidden)]
    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub fn define_instantiation_params_owned(
        &mut self,
        sentinel: &'static str,
        name: &'static str,
        module_path: &'static str,
        generics: Cow<'static, [GenericDefinition]>,
        docs: &'static str,
        deprecated: Option<Deprecated>,
        params: &[InstantiationParam],
        instantiation_generic_identities: &[&'static str],
        has_const_param: bool,
        inline: bool,
        passthrough: bool,
        build_ty: fn(&mut Types) -> DataType,
        build_reference_ty: fn(&mut Types) -> DataType,
    ) -> DataType {
        let location = Location::caller().to_owned();
        self.instantiate_params(
            sentinel,
            name,
            module_path,
            generics,
            docs,
            deprecated.as_ref(),
            params,
            instantiation_generic_identities,
            has_const_param,
            inline,
            passthrough,
            location,
            build_ty,
            build_reference_ty,
        )
    }

    /// The shared body of the two instantiation entry points.
    #[allow(clippy::too_many_arguments)]
    fn instantiate_params(
        &mut self,
        sentinel: &'static str,
        name: &'static str,
        module_path: &'static str,
        generics: Cow<'static, [GenericDefinition]>,
        docs: &'static str,
        deprecated: Option<&Deprecated>,
        params: &[InstantiationParam],
        instantiation_generic_identities: &[&'static str],
        has_const_param: bool,
        inline: bool,
        passthrough: bool,
        location: Location<'static>,
        build_ty: fn(&mut Types) -> DataType,
        build_reference_ty: fn(&mut Types) -> DataType,
    ) -> DataType {
        #[cfg(debug_assertions)]
        self.assert_sentinel_owner(sentinel, name);

        let generic_substitution_base = self.generic_substitution_cursor();
        let mut instantiation_generics = Vec::with_capacity(params.len());

        for param in params {
            match param {
                InstantiationParam::Resolved {
                    name,
                    identity,
                    definition,
                } => {
                    instantiation_generics.push((
                        Generic::new((*name).into()),
                        self.define_param_with(identity, *definition),
                    ));
                }
                InstantiationParam::Generic { name } => {
                    let generic = Generic::new((*name).into());
                    instantiation_generics.push((generic.clone(), DataType::Generic(generic)));
                }
                InstantiationParam::Skipped => self.skip_param_slot(),
            }
        }

        let instantiation = Instantiation {
            sentinel,
            name,
            module_path,
            generics,
            docs,
            deprecated,
            has_const_param,
            inline,
            passthrough,
            instantiation_generics,
            instantiation_generic_identities,
            generic_substitution_base,
            generic_substitution_after: self.generic_substitution_cursor(),
            location,
            build_ty,
            build_reference_ty,
        };
        self.instantiate(instantiation)
    }

    /// Builds the named datatype one instantiation describes.
    ///
    /// Every argument is fixed by the concrete arguments, so this — the metadata
    /// builder and the inline wrapping — is compiled once here instead of once
    /// per monomorphised `definition`.
    fn instantiate(&mut self, instantiation: Instantiation<'_>) -> DataType {
        if instantiation.inline {
            return datatype::inline(self, |types| types.init_instantiation(instantiation));
        }

        self.init_instantiation(instantiation)
    }

    /// [`Types::instantiate`] with the inline context already entered.
    fn init_instantiation(&mut self, instantiation: Instantiation<'_>) -> DataType {
        let Instantiation {
            sentinel,
            name,
            module_path,
            generics,
            docs,
            deprecated,
            has_const_param,
            inline,
            passthrough,
            instantiation_generics,
            instantiation_generic_identities,
            generic_substitution_base,
            generic_substitution_after,
            location,
            build_ty,
            build_reference_ty,
        } = instantiation;

        let build_ndt = |types: &mut Types, ndt: &mut NamedDataType| {
            ndt.name = Cow::Borrowed(name);
            ndt.module_path = Cow::Borrowed(module_path);
            ndt.generics = generics.clone();
            ndt.docs = Cow::Borrowed(docs);
            ndt.deprecated = deprecated.cloned();
            if !inline && !passthrough {
                ndt.ty = Some(build_reference_ty(types));
            }
        };

        let reference = NamedDataType::init(
            sentinel,
            instantiation_generics,
            instantiation_generic_identities,
            has_const_param,
            passthrough,
            generic_substitution_base,
            location,
            self,
            &build_ndt,
            build_ty,
        );

        self.set_generic_substitution_cursor(generic_substitution_after);

        DataType::Reference(reference)
    }

    /// Resolves the next positional slot of the innermost substitution scope,
    /// like [`Types::define`].
    ///
    /// The derive and the built-in impl macros resolve each of their own type
    /// parameters through this method, in parameter order, so a parameter that
    /// is instantiated with the same concrete type as a sibling still maps to
    /// its own slot. Parameters that are not resolved at all (`#[specta(skip)]`,
    /// unused) are stepped over with [`Types::skip_param_slot`] so the remaining
    /// parameters stay aligned.
    #[doc(hidden)]
    pub fn define_param<T: Type + ?Sized>(&mut self) -> DataType {
        self.define_param_with(std::any::type_name::<T>(), T::definition)
    }

    /// The non-generic body of [`Types::define_param`].
    fn define_param_with(&mut self, name: &'static str, f: fn(&mut Types) -> DataType) -> DataType {
        let nested = {
            let Some(scope) = self.generic_substitutions.last_mut() else {
                return f(self);
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
            Some(slots) => self.with_generic_substitution_fn(&slots, f),
            None => f(self),
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
    /// `name`. Distinct concrete types stay order-independent, and a slot
    /// consumed out of order is never handed out twice.
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
    /// [`Types::define_param`] consumes them positionally and [`Types::define`]
    /// matches them by [`std::any::type_name`] in the **innermost** scope only,
    /// so a nested declaration cannot pick up an enclosing parameter. Scopes
    /// nest: the innermost scope is consulted first.
    ///
    /// A `Type` implementation must therefore resolve its generic arguments in
    /// the order the slots were built in, so that the argument the caller passes
    /// at a given position lines up with the slot for that position. The derive
    /// builds the slots in the field type's argument order (or, for a field type
    /// it cannot analyse, in the container's parameter order); generated
    /// implementations get the alignment for free by resolving each parameter
    /// with [`Types::define_param`], stepping over unresolved ones with
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
            slots: substitutions.to_vec(),
            consumed: vec![false; substitutions.len()],
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

    /// [`Types::with_generic_substitution`] taking a function pointer.
    ///
    /// The generic form monomorphises its whole body — `catch_unwind` machinery
    /// included — once per closure type. Generated implementations call it once
    /// per field, so a crate with many instantiations pays for thousands of
    /// near-identical copies. A `fn` pointer is a single type, so this keeps one
    /// copy of the body in this crate; non-capturing closures coerce to it and
    /// each call site pays one indirect call instead.
    #[doc(hidden)]
    pub fn with_generic_substitution_fn(
        &mut self,
        substitutions: &[GenericSubstitutionSlot],
        f: fn(&mut Types) -> DataType,
    ) -> DataType {
        self.with_generic_substitution(substitutions, f)
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

    /// Returns the concrete datatypes explicitly registered as top-level roots.
    ///
    /// Named dependencies discovered while resolving a root are available
    /// through the normal type iterators and are not repeated here.
    pub fn roots(&self) -> impl ExactSizeIterator<Item = &'_ DataType> {
        self.roots.iter()
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
    pub fn iter_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        for ndt in self.types.values_mut().flatten() {
            f(ndt);
        }
    }

    /// Transforms each [`NamedDataType`] in the collection.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for slot in self.types.values_mut() {
            if let Some(ndt) = slot.take() {
                *slot = Some(f(ndt));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatype::{Generic, Primitive};

    fn u32_slot() -> GenericSubstitutionSlot {
        GenericSubstitutionSlot::Generic {
            name: std::any::type_name::<u32>(),
            generic: Generic::new("T".into()),
        }
    }

    #[test]
    fn empty_substitution_scope_still_hides_enclosing_scope() {
        let mut types = Types::default();

        // An empty scope is a barrier: a parameter of the enclosing scope must
        // not leak into a nested declaration that does not mention it.
        types.with_generic_substitution(&[u32_slot()], |types| {
            assert_eq!(
                types.with_generic_substitution(&[], |types| types.define::<u32>()),
                DataType::Primitive(Primitive::u32)
            );
        });

        // A nested scope that does hold the parameter still substitutes.
        types.with_generic_substitution(&[u32_slot()], |types| {
            assert_eq!(
                types.with_generic_substitution(&[u32_slot()], |types| types.define::<u32>()),
                DataType::Generic(Generic::new("T".into()))
            );
        });

        // Outside any scope the parameter resolves concretely again.
        assert_eq!(types.define::<u32>(), DataType::Primitive(Primitive::u32));
    }

    #[test]
    fn inline_frames_share_storage_and_release_lifo() {
        let mut types = Types::default();
        let outer = NamedId::Static("test::Outer");
        let inner = NamedId::Static("test::Inner");
        let arguments = [(
            Generic::new("T".into()),
            DataType::Primitive(Primitive::u32),
        )];

        types.push_inline_frame(outer.clone(), &["u32"], &arguments);

        // A use site is identified by its identity and its concrete arguments,
        // so a different instantiation of the same type is not a cycle.
        assert_eq!(
            types.inline_cycle_start(&outer, &["u32"], &arguments),
            Some(0)
        );
        assert_eq!(
            types.inline_cycle_start(&outer, &["alloc::string::String"], &arguments),
            None
        );

        types.push_inline_frame(inner.clone(), &["u32"], &arguments);
        assert_eq!(
            types.inline_cycle_start(&inner, &["u32"], &arguments),
            Some(1)
        );

        types.pop_inline_frame();
        types.pop_inline_frame();

        // The shared storage is released with the frames that own it.
        assert!(types.stack.is_empty());
        assert!(types.stack_identities.is_empty());
        assert!(types.stack_generics.is_empty());
        assert_eq!(types.inline_cycle_start(&outer, &["u32"], &arguments), None);
    }

    fn primitive(_: &mut Types) -> DataType {
        DataType::Primitive(Primitive::i32)
    }

    /// A sentinel identifies exactly one type; two types sharing one would
    /// silently collapse into a single export.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "same sentinel")]
    fn two_types_cannot_share_a_sentinel() {
        let mut types = Types::default();
        types.define_named("crate::A", "A", "crate", "", None, primitive);
        types.define_named("crate::A", "B", "crate", "", None, primitive);
    }

    /// Every instantiation of a generic type re-registers under the same
    /// sentinel and name, so that must stay accepted.
    #[test]
    fn re_registering_the_same_type_is_allowed() {
        let mut types = Types::default();
        types.define_named("crate::A", "A", "crate", "", None, primitive);
        types.define_named("crate::A", "A", "crate", "", None, primitive);
    }
}
