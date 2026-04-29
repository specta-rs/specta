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
    pub(crate) id: NamedId,
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
    /// Constructs a new named datatype and register it into the [`Types`] collection.
    #[track_caller]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        types: &mut Types,
        build: impl FnOnce(&mut Types, &mut NamedDataType),
    ) -> Self {
        let location = Location::caller();
        let mut ndt = Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual")),
            location: location.to_owned(),
            generics: Cow::Borrowed(&[]),
            ty: None,
        };
        build(types, &mut ndt);
        types.types.insert(ndt.id.clone(), Some(ndt.clone()));
        types.len += 1;
        ndt
    }

    /// Initializes a named type using a static sentinel as its identity.
    ///
    /// This is used by `#[derive(Type)]` and the built-in `Type` implementation macros and must be used carefully.
    ///
    /// WARNING: Do not call this outside of `specta` as its signature and behavior may change in minor releases!!!!
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
    /// - If inline expansion recursively reaches the same sentinel and generic arguments, this
    ///   returns [`NamedReferenceType::Recursive`] so exporters can avoid infinite expansion.
    ///
    /// `has_const_param` only affects the temporary resolution context used while `build_ndt`
    /// builds the canonical named type. That context controls implementations such as fixed-size
    /// arrays, so they intentionally don't become part of the global type identity
    /// (We don't want one call-sites const generic in the shared datatype on the `NamedDataType`).
    ///
    /// `passthrough_inline` is for wrapper/container types whose own definition is inline but whose
    /// inner type should still see the caller's inline context. When it is `false`, inline expansion
    /// temporarily clears `Types::should_inline` before calling `build_ty`.
    ///
    /// `build_ndt` fills metadata and, for exported named types, `NamedDataType::ty`. `build_ty`
    /// builds the datatype used by inline references. If `build_ndt` panics, this removes the
    /// placeholder entry and restores the previous resolution context before resuming the panic.
    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        sentinel: &'static str,
        instantiation_generics: &[(Generic, DataType)],
        has_const_param: bool,
        passthrough_inline: bool,
        types: &mut Types,
        build_ndt: fn(&mut Types, &mut NamedDataType),
        mut build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();
        let mut inline = types.should_inline;

        // If we have never encountered this type, register it to type map
        if !types.types.contains_key(&id) {
            let mut ndt = NamedDataType {
                id: id.clone(),
                location,
                // `build_ndt` will just override all of this.
                generics: Cow::Borrowed(&[]),
                ty: None,
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

            if types.stack.contains(&hash) {
                // For container inline types we wanna passthrough instead of rejecting on the container.
                if passthrough_inline {
                    let prev_inline = mem::replace(&mut types.should_inline, false);
                    let result = panic::catch_unwind(AssertUnwindSafe(|| build_ty(types)));
                    types.should_inline = prev_inline;

                    return match result {
                        Ok(DataType::Reference(reference)) => reference,
                        Ok(_) => Reference::Named(NamedReference {
                            id,
                            inner: NamedReferenceType::Recursive,
                        }),
                        Err(payload) => panic::resume_unwind(payload),
                    };
                }

                return Reference::Named(NamedReference {
                    id,
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
