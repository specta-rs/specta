use std::{
    any::TypeId,
    borrow::Cow,
    mem,
    panic::{self, AssertUnwindSafe, Location},
    sync::Arc,
};

use crate::{
    Types,
    datatype::{
        Attributes, DataType, Generic, NamedReference, NamedReferenceType, NamedTypeId,
        RecursiveInlineFrame, RecursiveInlineType, Reference, generic::GenericDefinition,
        reference::NamedId,
    },
};

/// Resolves any named types created by `func` as inline references.
/// This is emitted when `#[specta(inline)]` is used on a field so the inner fields `Type` implementation knows to inline.
pub fn inline<R>(types: &mut Types, func: impl FnOnce(&mut Types) -> R) -> R {
    let prev = mem::replace(&mut types.should_inline, true);
    let result = panic::catch_unwind(AssertUnwindSafe(|| func(types)));
    types.should_inline = prev;
    match result {
        Ok(result) => result,
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Named datatype with its own export identity.
///
/// Exporters commonly render these as top-level declarations, such as
/// `export type MyType = ...` in TypeScript. Other datatypes refer back to a
/// named datatype through [`Reference::Named`].
///
/// # Invariants
///
/// The `id` is the stable identity used by [`Types`] and [`NamedReference`]. The
/// human-readable `name` alone is not guaranteed to be globally unique.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NamedDataType {
    /// Stable identity for resolving references to this datatype.
    pub(crate) id: NamedTypeId,
    /// Exported type name.
    pub name: Cow<'static, str>,
    /// Documentation comments attached to the source type.
    pub docs: Cow<'static, str>,
    /// Deprecation metadata attached to the source type.
    pub deprecated: Option<Deprecated>,
    /// Rust module path where the source type was defined.
    pub module_path: Cow<'static, str>,
    /// Source location where this named datatype was created.
    pub location: Location<'static>,
    /// Generic parameters declared by this named datatype.
    pub generics: Cow<'static, [GenericDefinition]>,
    /// Metadata attached to the declaration itself rather than to its shape.
    ///
    /// The derive records container-level attributes here as well, so they
    /// survive when the shape is replaced by an override such as
    /// `#[specta(type = ...)]` and is therefore no longer a struct or an enum.
    pub attributes: Attributes,
    /// The generalised datatype of this specific named data type.
    /// This is what will be used for creating `export Type = ...;` statements.
    ///
    /// This will be `None` for types which are container inlined as they aren't exported.
    pub ty: Option<DataType>,
}

/// Identity a named declaration is registered under.
///
/// Built-in implementations pass their stable Rust path; generated code passes
/// a key built from the marker type emitted next to the derive.
#[doc(hidden)]
#[derive(Clone)]
pub struct NamedKey {
    label: &'static str,
    nominal: Option<TypeId>,
}

impl NamedKey {
    /// A key identified by its stable `module_path::TypeName`.
    pub const fn label(label: &'static str) -> Self {
        Self {
            label,
            nominal: None,
        }
    }

    /// A key identified by the `TypeId` of the declaration's marker type.
    pub const fn nominal(label: &'static str, id: TypeId) -> Self {
        Self {
            label,
            nominal: Some(id),
        }
    }

    /// Human-readable identity used in diagnostics.
    pub(crate) const fn label_str(&self) -> &'static str {
        self.label
    }

    pub(crate) fn id(&self) -> NamedTypeId {
        NamedTypeId(match self.nominal {
            Some(id) => NamedId::Nominal {
                label: self.label,
                id,
            },
            None => NamedId::Static(self.label),
        })
    }
}

impl NamedDataType {
    /// Constructs a new named datatype and register it into the [`Types`] collection.
    #[track_caller]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        types: &mut Types,
        build: impl FnOnce(&mut Types, &mut NamedDataType),
    ) -> Self {
        let location = Location::caller();
        let id = NamedTypeId(NamedId::Dynamic(Arc::new(())));
        let mut ndt = Self {
            id: id.clone(),
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual")),
            location: location.to_owned(),
            generics: Cow::Borrowed(&[]),
            attributes: Attributes::default(),
            ty: None,
        };
        build(types, &mut ndt);
        // The closure receives the whole value, so it can replace it with a
        // clone of another named type. Restore the identity the closure may
        // have used for self-references before inserting.
        ndt.id = id.clone();
        types.types.insert(id, Some(ndt.clone()));
        types.len += 1;
        ndt
    }

    /// Stable identity of this declaration within the type graph.
    ///
    /// Two declarations compare equal only when they are the same declaration;
    /// a copy of the graph keeps the same identities. The value is process-local
    /// and must not be used as a persistence key or as a stable sort tiebreaker.
    pub fn id(&self) -> &NamedTypeId {
        &self.id
    }

    /// Initializes a named type using a stable key as its identity.
    ///
    /// This is used by `#[derive(Type)]` and the built-in `Type` implementation macros and must be used carefully.
    ///
    /// <div class="warning">
    ///
    /// **WARNING:** Do not call this outside of `specta` as its signature and behavior may change in minor releases!!!!
    ///
    /// </div>
    ///
    /// This registers the canonical [`NamedDataType`] for `sentinel` at most once, then returns a
    /// use-site [`Reference`]. During first registration, `None` is inserted into [`Types`] before
    /// `build_ndt` runs so recursive named lookups can observe that the type is already being
    /// resolved instead of re-entering `build_ndt` (which would stack overflow).
    ///
    /// The returned reference depends on the current inline context:
    ///
    /// - When not inlining, this returns [`NamedReferenceType::Reference`] with
    ///   `instantiation_generics` as the concrete generic arguments for this use site.
    /// - When inlining, this calls `build_ty` and returns [`NamedReferenceType::Inline`] containing
    ///   the resulting datatype.
    /// - If inline expansion recursively reaches the same sentinel, generic arguments, and
    ///   supplemental generic identities, this returns [`NamedReferenceType::Recursive`] so
    ///   exporters can avoid infinite expansion.
    ///
    /// `has_const_param` only affects the temporary resolution context used while `build_ndt`
    /// builds the canonical named type. That context controls implementations such as fixed-size
    /// arrays, so they intentionally don't become part of the global type identity
    /// (We don't want one call-sites const generic in the shared datatype on the `NamedDataType`).
    ///
    /// `passthrough` is for wrapper/container types whose own definition is inline but whose inner
    /// type should only see the caller's inline context. When passthrough expansion recursively
    /// reaches the same wrapper, it clears `Types::should_inline` before calling `build_ty` so the
    /// inner named type can break the cycle with a reference.
    ///
    /// `build_ndt` fills metadata and, for exported named types, `NamedDataType::ty`. `build_ty`
    /// builds the datatype used by inline references. If `build_ndt` panics, this removes the
    /// placeholder entry and restores the previous resolution context before resuming the panic.
    ///
    /// `generic_substitution_base` is the position the caller's generic-substitution scope held
    /// before it resolved this type's generic arguments. Both `build_ndt` and `build_ty` re-resolve
    /// those arguments, so this rewinds the scope to that position first, keeping the two
    /// traversals in lockstep instead of letting one consume the other's slots.
    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        key: NamedKey,
        instantiation_generics: &[(Generic, DataType)],
        instantiation_generic_identities: &[&'static str],
        has_const_param: bool,
        passthrough: bool,
        generic_substitution_base: usize,
        types: &mut Types,
        build_ndt: fn(&mut Types, &mut NamedDataType),
        build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        Self::init(
            key,
            instantiation_generics.to_vec(),
            instantiation_generic_identities,
            has_const_param,
            passthrough,
            generic_substitution_base,
            Location::caller().to_owned(),
            types,
            &build_ndt,
            build_ty,
        )
    }

    /// The shared body of [`NamedDataType::init_with_sentinel`].
    ///
    /// `build_ndt` is a `&dyn Fn` rather than a `fn` pointer so
    /// [`Types::define_named`] can pass a closure that captures the metadata it
    /// was given. Taking it by reference (instead of `impl Fn`) keeps this
    /// function non-generic, so it is compiled once in this crate instead of
    /// once per distinct closure type in every crate that derives `Type`.
    ///
    /// `location` is passed in rather than read here because callers such as
    /// [`Types::define_instantiation_params`] run this from inside a closure,
    /// which `#[track_caller]` cannot see through.
    #[track_caller]
    pub(crate) fn init(
        key: NamedKey,
        instantiation_generics: Vec<(Generic, DataType)>,
        instantiation_generic_identities: &[&'static str],
        has_const_param: bool,
        passthrough: bool,
        generic_substitution_base: usize,
        location: Location<'static>,
        types: &mut Types,
        build_ndt: &dyn Fn(&mut Types, &mut NamedDataType),
        mut build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        types.set_generic_substitution_cursor(generic_substitution_base);
        let id = key.id();
        let caller_inline = types.should_inline;
        let mut inline = caller_inline || passthrough;

        // If we have never encountered this type, register it to type map.
        //
        // A single `entry` probe both answers "is this type known?" and installs
        // the in-progress placeholder, which keeps the common repeat use site to
        // one hash lookup instead of a `contains_key` followed by an `insert`.
        let vacant = match types.types.entry(id.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(None);
                true
            }
        };

        if vacant {
            let mut ndt = NamedDataType {
                id: id.clone(),
                location,
                // `build_ndt` will just override all of this.
                generics: Cow::Borrowed(&[]),
                attributes: Default::default(),
                ty: None,
                name: Cow::Borrowed(""),
                docs: Cow::Borrowed(""),
                deprecated: None,
                module_path: Cow::Borrowed(""),
            };

            let prev_inline = mem::replace(&mut types.should_inline, false);
            let prev_has_const_params = mem::replace(&mut types.has_const_params, has_const_param);

            let result = panic::catch_unwind(AssertUnwindSafe(|| build_ndt(types, &mut ndt)));
            types.should_inline = prev_inline;
            types.has_const_params = prev_has_const_params;
            if let Err(payload) = result {
                if types.types.contains_key(&id) {
                    types.types.remove(&id);
                }
                panic::resume_unwind(payload);
            };
            // `build_ndt` receives the whole value and may have replaced it
            // with a clone of another type. The key is already registered as
            // this sentinel's identity, so the stored value must match it.
            ndt.id = id.clone();

            // Patch legacy Tauri channels, but preserve explicit opaque definitions.
            let is_tauri_type = ndt.name == "TAURI_CHANNEL"
                && ndt.module_path.starts_with("tauri::")
                && !matches!(ndt.ty, Some(DataType::Reference(Reference::Opaque(_))));
            if is_tauri_type {
                ndt.ty = None;
                inline = true;
                build_ty = |_| {
                    unreachable!("Specta `build_ty` shouldn't be callable with `tauri::Channel`")
                }
            }

            // `build_ndt` may already have completed this entry, for example by
            // merging a collection that contains it through `Types::extend`.
            // Only count the entry when the placeholder we installed is still a
            // placeholder, otherwise `Types::named_len` would report more entries than
            // the map holds.
            let already_complete = types
                .types
                .insert(id.clone(), Some(ndt))
                .is_some_and(|slot| slot.is_some());
            types.len += usize::from(!already_complete);

            // We patch the Tauri `Type` implementation.
            if is_tauri_type {
                return Reference::Named(NamedReference {
                    id,
                    inner: NamedReferenceType::Reference {
                        generics: instantiation_generics,
                    },
                });
            }
        }

        if inline {
            if let Some(cycle_start) = types.inline_cycle_start(
                &id,
                instantiation_generic_identities,
                &instantiation_generics,
            ) {
                // For container inline types we wanna passthrough instead of rejecting on the container.
                if passthrough {
                    let prev_inline = mem::replace(&mut types.should_inline, false);
                    let result = panic::catch_unwind(AssertUnwindSafe(|| build_ty(types)));
                    types.should_inline = prev_inline;

                    match result {
                        Ok(DataType::Reference(reference)) => return reference,
                        Ok(_) => {}
                        Err(payload) => panic::resume_unwind(payload),
                    }
                }

                let mut cycle = types.stack[cycle_start..]
                    .iter()
                    .map(|frame| frame.describe(types))
                    .collect::<Vec<_>>();
                cycle.push(RecursiveInlineFrame::describe(
                    types,
                    &id,
                    &instantiation_generics,
                ));

                return Reference::Named(NamedReference {
                    id,
                    inner: NamedReferenceType::Recursive(RecursiveInlineType::from_cycle(
                        cycle,
                        instantiation_generics.to_vec(),
                    )),
                });
            }

            // Say for `Box<T>` if we put `#[specta(inline)]` on it we will,
            // naively inline the `Box` instead of `T`.
            //
            // "wrapper" types enable this to properly to passthrough inline to the inner type's resolution.
            let child_inline = passthrough && caller_inline;
            let prev_inline = (types.should_inline != child_inline)
                .then(|| mem::replace(&mut types.should_inline, child_inline));
            types.push_inline_frame(
                id.clone(),
                instantiation_generic_identities,
                &instantiation_generics,
            );
            let result = panic::catch_unwind(AssertUnwindSafe(|| build_ty(types)));
            if let Some(prev_inline) = prev_inline {
                types.should_inline = prev_inline;
            };
            types.pop_inline_frame();
            let dt = match result {
                Ok(DataType::Reference(reference)) if passthrough && !caller_inline => {
                    return reference;
                }
                Ok(dt) => Box::new(dt),
                Err(payload) => panic::resume_unwind(payload),
            };

            Reference::Named(NamedReference {
                id,
                inner: NamedReferenceType::Inline { dt },
            })
        } else {
            Reference::Named(NamedReference {
                id,
                inner: NamedReferenceType::Reference {
                    generics: instantiation_generics,
                },
            })
        }
    }

    /// Constructs a [`Reference`] to this named datatype.
    /// The reference returned by this will error in the language exporter if `Self.ty` is `None` as the type can't generate a named export.
    pub fn reference(&self, generics: Vec<(Generic, DataType)>) -> Reference {
        Reference::Named(NamedReference {
            id: self.id.clone(),
            inner: NamedReferenceType::Reference { generics },
        })
    }
}

/// Runtime representation of Rust's `#[deprecated]` metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Deprecated {
    /// Optional deprecation note or replacement guidance.
    pub note: Option<Cow<'static, str>>,
    /// Optional version where the item became deprecated.
    pub since: Option<Cow<'static, str>>,
}

impl Deprecated {
    /// Constructs deprecation metadata without details.
    ///
    /// Corresponds to `#[deprecated]`.
    pub const fn new() -> Self {
        Self {
            note: None,
            since: None,
        }
    }

    /// Constructs deprecation metadata with a note.
    ///
    /// Corresponds to `#[deprecated = "Use something else"]`.
    ///
    /// `const` so a generated implementation can keep the metadata in a
    /// `static` and pass a reference to it, instead of building a temporary in
    /// every monomorphised `definition`.
    pub const fn with_note(note: Cow<'static, str>) -> Self {
        Self {
            note: Some(note),
            since: None,
        }
    }

    /// Constructs deprecation metadata with a note and optional `since` version.
    ///
    /// Corresponds to `#[deprecated(since = "1.0.0", note = "Use something else")]`.
    ///
    /// `const` for the same reason as [`Deprecated::with_note`].
    pub const fn with_since_note(
        since: Option<Cow<'static, str>>,
        note: Cow<'static, str>,
    ) -> Self {
        Self {
            note: Some(note),
            since,
        }
    }
}

fn file_path_to_module_path(file_path: &str) -> Option<String> {
    let normalized = file_path.replace('\\', "/");

    // Try different prefixes
    let (prefix, path) = if let Some(p) = normalized.strip_prefix("src/") {
        ("crate", p)
    } else {
        ("tests", normalized.strip_prefix("tests/")?)
    };

    let path = path.strip_suffix(".rs")?;
    let path = path.strip_suffix("/mod").unwrap_or(path);
    let module_path = path.replace('/', "::");

    if module_path.is_empty() {
        Some(prefix.to_string())
    } else {
        Some(format!("{}::{}", prefix, module_path))
    }
}

#[cfg(test)]
mod tests {
    use super::file_path_to_module_path;
    use crate::Types;
    use crate::datatype::DataType;

    #[test]
    fn duplicate_sentinel_reuses_the_first_definition() {
        fn first(_: &mut Types, ndt: &mut super::NamedDataType) {
            ndt.name = std::borrow::Cow::Borrowed("Dup");
            ndt.module_path = std::borrow::Cow::Borrowed("test");
        }

        fn duplicate(_: &mut Types, _: &mut super::NamedDataType) {
            panic!("a duplicate sentinel must not rebuild the definition");
        }

        fn inline(_: &mut Types) -> DataType {
            unreachable!("inline definitions are not built for a non-inline type")
        }

        let mut types = Types::default();
        super::NamedDataType::init_with_sentinel(
            super::NamedKey::label("test::Dup"),
            &[],
            &[],
            false,
            false,
            0,
            &mut types,
            first,
            inline,
        );
        // The second registration has the same sentinel but a different builder.
        // It must reuse the first definition instead of exporting another type
        // with the same name.
        super::NamedDataType::init_with_sentinel(
            super::NamedKey::label("test::Dup"),
            &[],
            &[],
            false,
            false,
            0,
            &mut types,
            duplicate,
            inline,
        );

        assert_eq!(types.named_len(), 1);
        let ndt = types
            .iter_sorted()
            .next()
            .expect("the definition is registered");
        assert_eq!(ndt.name, "Dup");
    }

    #[test]
    fn extending_during_resolution_counts_the_entry_once() {
        use std::cell::RefCell;

        thread_local! {
            static OTHER: RefCell<Types> = RefCell::new(Types::default());
        }

        const SENTINEL: &str = "test::ExtendDuringResolution";

        fn metadata(ndt: &mut super::NamedDataType) {
            ndt.name = std::borrow::Cow::Borrowed("ExtendDuringResolution");
            ndt.module_path = std::borrow::Cow::Borrowed("test");
            ndt.ty = Some(crate::datatype::Primitive::str.into());
        }

        fn build(_: &mut Types, ndt: &mut super::NamedDataType) {
            metadata(ndt);
        }

        // A definition that merges another collection into the one being built,
        // as an exporter that has already collected this type would.
        fn build_while_extending(types: &mut Types, ndt: &mut super::NamedDataType) {
            metadata(ndt);
            OTHER.with(|other| types.extend(&other.borrow()));
        }

        fn inline(_: &mut Types) -> DataType {
            unreachable!("the definition is registered up front")
        }

        OTHER.with(|other| {
            super::NamedDataType::init_with_sentinel(
                super::NamedKey::label(SENTINEL),
                &[],
                &[],
                false,
                false,
                0,
                &mut other.borrow_mut(),
                build,
                inline,
            );
        });

        let mut types = Types::default();
        super::NamedDataType::init_with_sentinel(
            super::NamedKey::label(SENTINEL),
            &[],
            &[],
            false,
            false,
            0,
            &mut types,
            build_while_extending,
            inline,
        );

        // `Types::named_len` checks its count against the map, so a double count
        // shows up here rather than as a wrong length in an exporter.
        assert_eq!(types.named_len(), 1);
    }

    #[test]
    fn file_path_to_module_path_supports_unix_and_windows_separators() {
        assert_eq!(
            file_path_to_module_path("src/datatype/named.rs"),
            Some("crate::datatype::named".to_string())
        );
        assert_eq!(
            file_path_to_module_path("src\\datatype\\named.rs"),
            Some("crate::datatype::named".to_string())
        );
        assert_eq!(
            file_path_to_module_path("tests/tests/types.rs"),
            Some("tests::tests::types".to_string())
        );
        assert_eq!(
            file_path_to_module_path("tests\\tests\\types.rs"),
            Some("tests::tests::types".to_string())
        );
    }
}
