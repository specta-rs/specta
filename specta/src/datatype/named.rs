use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    panic::{self, AssertUnwindSafe, Location},
    ptr,
    sync::Arc,
};

use crate::{
    Types,
    datatype::{
        DataType, Generic, NamedReference, NamedReferenceType, Reference,
        generic::GenericDefinition,
        reference::{self, NamedId},
    },
};

/// Resolves any named types created by `func` as inline references.
///
/// This temporarily enables inline resolution on the provided [`Types`]
/// collection and restores the previous setting even if `func` panics.
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
    pub id: NamedId,
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
    /// The generalised datatype of this specific named data type.
    /// This is what will be used for creating `export Type = ...;` statements.
    ///
    /// This will be `None` for types which are container inlined as they aren't exported.
    pub ty: Option<DataType>,
}

impl NamedDataType {
    /// Returns whether this named datatype should be emitted as a named
    /// declaration instead of only being inlined at use sites.
    ///
    /// A datatype with no canonical `ty` has no standalone definition to export.
    pub fn requires_reference(&self, _types: &Types) -> bool {
        self.ty.is_some()
    }

    /// Returns the generic parameters declared by this named datatype.
    pub fn generics(&self) -> &[GenericDefinition] {
        &self.generics
    }

    /// Returns the canonical datatype definition, if this named datatype has one.
    ///
    /// `None` means the type is intended to be represented only by inline or
    /// opaque references.
    pub fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
    }

    /// Constructs a new named datatype.
    ///
    /// Call [`NamedDataType::register`] to make the type available through a
    /// [`Types`] collection.
    #[track_caller]
    pub fn new(name: impl Into<Cow<'static, str>>, generics: Vec<Generic>, dt: DataType) -> Self {
        let location = Location::caller();
        Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual")),
            location: location.to_owned(),
            generics: Cow::Owned(
                generics
                    .into_iter()
                    .map(|generic| GenericDefinition::new(generic.name().clone(), None))
                    .collect(),
            ),
            ty: Some(dt),
        }
    }

    /// Constructs a new named datatype intended to be inlined at reference sites.
    ///
    /// Call [`NamedDataType::register`] to make the type available through a
    /// [`Types`] collection.
    #[track_caller]
    pub fn new_inline(
        name: impl Into<Cow<'static, str>>,
        generics: Vec<Generic>,
        dt: DataType,
    ) -> Self {
        let location = Location::caller();
        Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual")),
            location: location.to_owned(),
            generics: Cow::Owned(
                generics
                    .into_iter()
                    .map(|generic| GenericDefinition::new(generic.name().clone(), None))
                    .collect(),
            ),
            ty: Some(dt),
        }
    }

    /// Registers this named datatype into a [`Types`] collection.
    ///
    /// If an entry with the same identity already exists, it is replaced and the
    /// completed-entry count is incremented for this registration.
    pub fn register(&self, types: &mut Types) {
        types.types.insert(self.id.clone(), Some(self.clone()));
        types.len += 1;
    }

    // TODO: Rewrite this with the new changes
    /// Initialize a named type using a temporary sentinel as it's identity. The sentinel avoids allocating an ID which is used by `#[derive(Type)]` but is too unsafe as a general public API.
    ///
    /// WARNING: This should not be used outside of `specta_macros` as it may have breaking changes in minor releases
    ///
    /// This always returns a [`Reference`] rather than a [`NamedDataType`]. While a type is being
    /// resolved we insert `None` into [`Types`] for its id. Recursive lookups treat that `None` as
    /// "currently resolving" and immediately emit a placeholder reference instead of re-entering
    /// `build_ndt`.
    ///
    /// The canonical [`NamedDataType::inner`] must stay generic enough to be shared by every
    /// reference to the type. When a particular use-site needs a more specific instantiated shape,
    /// such as const-generic expansion or a post-processing rewrite, that shape is stored in
    /// [`NamedDataType::instances`] and the returned [`NamedReference`] stores only the stable
    /// instance index.
    ///
    /// `has_const_param` only affects the thread-local resolution context used while building the
    /// canonical named type. That context intentionally does not become part of the global type
    /// identity.
    // This is called for a container inlined type.
    // This means we know the type is *always* inlined.
    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        sentinel: &'static str,
        generics: &'static [GenericDefinition],
        instantiation_generics: &[(Generic, DataType)],
        has_const_param: bool,
        container_inline: bool,
        passthrough_inline: bool,
        types: &mut Types,
        build_ndt: fn(&mut Types, &mut NamedDataType),
        mut build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();
        let mut inline = container_inline || types.should_inline;

        println!("init_with_sentinel {sentinel} {inline}");

        // If we have never encountered this type, register it to type map
        if !types.types.contains_key(&id) {
            let mut ndt = NamedDataType {
                id: id.clone(),
                location,
                generics: Cow::Borrowed(generics),
                ty: None,
                // `build_ndt` will just override all of this.
                name: Cow::Borrowed(""),
                docs: Cow::Borrowed(""),
                deprecated: None,
                module_path: Cow::Borrowed(""),
            };

            types.types.insert(id.clone(), None);

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

            // We patch the Tauri `Type` implementation.
            if ndt.name == "TAURI_CHANNEL" && ndt.module_path.starts_with("tauri::") {
                ndt.ty = None;
                inline = true;
                build_ty = |_| reference::tauri().into();

                // // This causes an exporter that isn't aware of Tauri's channel to error.
                // // This is effectively `Reference::opaque(TauriChannel)` but we do some hackery for better errors.

                // TODO: reference::tauri().into();

                // // This ensures that we never create a `export type Channel`,
                // // instead the definition gets inlined into each callsite.
                // inline = true;
                // // ndt.inline = true;
            }

            types.types.insert(id.clone(), Some(ndt));
            types.len += 1;
        }

        if inline {
            let hash = {
                let mut h = DefaultHasher::new();
                sentinel.hash(&mut h);
                ptr::hash(sentinel, &mut h);
                for (generic_r, generic) in instantiation_generics {
                    generic_r.hash(&mut h);
                    generic.hash(&mut h);
                }
                h.finish()
            };
            println!("INLINE {sentinel} {hash:?}");

            if types.stack.contains(&hash) {
                todo!("recursive inline reference detected {:?}", types.stack);
                return Reference::Named(NamedReference {
                    id,
                    // TODO: Include metadata about where the recursive loop is
                    inner: NamedReferenceType::Recursive,
                });
            }

            // Say for `Box<T>` if we put `#[specta(inline)]` on it we will,
            // naively inline the `Box` instead of `T`.
            //
            // "wrapper" types enable this to properly to passthrough inline to the inner type's resolution.
            let prev_inline =
                (!passthrough_inline).then(|| mem::replace(&mut types.should_inline, false));
            types.stack.push(hash);
            let result = panic::catch_unwind(AssertUnwindSafe(|| build_ty(types)));
            if let Some(prev_inline) = prev_inline {
                types.should_inline = prev_inline;
            };
            types.stack.pop();
            let dt = match result {
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
                    generics: instantiation_generics.to_owned(),
                },
            })
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel_inline(
        sentinel: &'static str,
        generics: &'static [GenericDefinition],
        instantiation_generics: &[(Generic, DataType)],
        has_const_param: bool,
        container_inline: bool,
        passthrough_inline: bool,
        types: &mut Types,
        build_ndt: fn(&mut Types, &mut NamedDataType),
        build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        Self::init_with_sentinel(
            sentinel,
            generics,
            instantiation_generics,
            has_const_param,
            container_inline,
            passthrough_inline,
            types,
            build_ndt,
            build_ty,
        )
    }

    /// Constructs a [`Reference`] to this named datatype.
    ///
    /// The returned reference can be embedded in another [`DataType`]. The
    /// `generics` vector provides concrete datatypes for this named type's
    /// declared generic parameters.
    ///
    /// This reference will be inlined if the type is configured for inline
    /// export. Otherwise callers can force an inline reference with
    /// [`Reference::inline`].
    pub fn reference(&self, generics: Vec<(Generic, DataType)>) -> Reference {
        // TODO: allow generics to be `Cow`
        // TODO: HashMap instead of array for better typesafety??

        Reference::Named(NamedReference {
            id: self.id.clone(),
            inner: NamedReferenceType::Reference { generics },
        })
    }
}

/// Runtime representation of Rust's `#[deprecated]` metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Deprecated {
    /// Optional deprecation note or replacement guidance.
    pub note: Option<Cow<'static, str>>,
    /// Optional version where the item became deprecated.
    since: Option<Cow<'static, str>>,
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
    pub fn with_note(note: Cow<'static, str>) -> Self {
        Self {
            note: Some(note),
            since: None,
        }
    }

    /// Constructs deprecation metadata with a note and optional `since` version.
    ///
    /// Corresponds to `#[deprecated(since = "1.0.0", note = "Use something else")]`.
    pub fn with_since_note(since: Option<Cow<'static, str>>, note: Cow<'static, str>) -> Self {
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
    } else if let Some(p) = normalized.strip_prefix("tests/") {
        ("tests", p)
    } else {
        return None;
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
