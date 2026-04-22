use std::{
    borrow::Cow,
    cell::Cell,
    hash::{DefaultHasher, Hash, Hasher},
    panic::{self, AssertUnwindSafe, Location},
    ptr,
    sync::Arc,
};

use crate::{
    Types,
    datatype::{
        DataType, Generic, NamedReference, NamedReferenceInner, Reference,
        reference::{self, GenericReference, NamedId},
    },
};

thread_local! {
    /// This variables remains false unless your exporting in the context of `#[derive(Type)]` on a type which contains one or more const-generic parameters.
    ///
    /// Say for a type like this
    /// ```rs
    /// #[derive(Type)]
    /// struct Demo<const N: usize> {
    ///     data: [u32; N],
    /// }
    /// ```
    ///
    /// If we always set the length in the `impl Type for [T; N]`, the implementation will "bake" whatever the first encountered value of `N` is into the global type definition which is wrong. For example:
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
    /// One observation is that for a length to differ across two instantiations of the same type it must either:
    ///  - Have a const parameter
    ///  - Have a generic which uses a trait associated constant
    ///
    /// Now Specta doesn't and can't support a generic with a trait associated constant as the generic `T` is shadowed by a virtual struct which is used to alter the type to return a generic reference, instead of a flat datatype.
    ///
    /// So for DX we know including length is safe as long as the resolving context doesn't have any const parameters. We track this using a thread local so it's entirely runtime meaning the solution doesn't require brittle scanning of the user's `TokenStream` in the derive macro.
    ///
    /// We provide `specta_util::FixedArray<N, T>` as a helper type to force Specta to export a fixed-length array instead of a generic `number[]` if you know what your doing.
    /// This doesn't fix the core issue but it does allow the user to assert they are correct.
    ///
    static HAS_CONST_PARAMS: Cell<bool> = const { Cell::new(false) };

    /// TODO
    static SHOULD_INLINE: Cell<bool> = const { Cell::new(false) };
}

pub fn inline<R>(func: impl FnOnce() -> R) -> R {
    println!("USED INLINE");
    let prev = SHOULD_INLINE.replace(true);
    let result = panic::catch_unwind(AssertUnwindSafe(func));
    SHOULD_INLINE.set(prev);
    match result {
        Ok(result) => result,
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub(crate) fn context_has_const_params() -> bool {
    HAS_CONST_PARAMS.with(|c| c.get())
}

/// Named type represents any type with it's own unique name and identity.
///
/// These can become `export MyNamedType = ...` in Typescript can we be referenced in types like `{ field: MyNamedType }`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NamedDataType {
    pub id: NamedId,
    pub name: Cow<'static, str>,
    pub docs: Cow<'static, str>,
    pub deprecated: Option<Deprecated>,
    pub module_path: Cow<'static, str>,
    pub location: Location<'static>,
    pub generics: Cow<'static, [Generic]>,
    /// The generalised datatype of this specific named data type.
    /// This is what will be used for creating `export Type = ...;` statements.
    ///
    /// This will be `None` for types which are container inlined as they aren't exported.
    pub ty: Option<DataType>,
}

impl NamedDataType {
    /// Construct a new named datatype.
    ///
    /// Note: Ensure you call `Self::register` to register the type.
    #[track_caller]
    pub fn new(name: impl Into<Cow<'static, str>>, generics: Vec<Generic>, dt: DataType) -> Self {
        let location = Location::caller();
        // Self {
        //     id: NamedId::Dynamic(Arc::new(())),
        //     name: name.into(),
        //     docs: Cow::Borrowed(""),
        //     deprecated: None,
        //     module_path: file_path_to_module_path(location.file())
        //         .map(Into::into)
        //         .unwrap_or(Cow::Borrowed("virtual")),
        //     location: location.to_owned(),
        //     generics: Cow::Owned(generics),
        //     inline: false,
        //     ty: dt,
        //     instances: Vec::new(),
        // }
        todo!();
    }

    /// Construct a new inlined named datatype.
    ///
    /// Note: Ensure you call `Self::register` to register the type.
    #[track_caller]
    pub fn new_inline(
        name: impl Into<Cow<'static, str>>,
        generics: Vec<Generic>,
        dt: DataType,
    ) -> Self {
        let location = Location::caller();
        // Self {
        //     id: NamedId::Dynamic(Arc::new(())),
        //     name: name.into(),
        //     docs: Cow::Borrowed(""),
        //     deprecated: None,
        //     module_path: file_path_to_module_path(location.file())
        //         .map(Into::into)
        //         .unwrap_or(Cow::Borrowed("virtual")),
        //     location: location.to_owned(),
        //     generics: Cow::Owned(generics),
        //     inline: true,
        //     ty: dt,
        //     instances: Vec::new(),
        // }
        todo!();
    }

    /// Register the type into a [Types].
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
    pub fn init_with_sentinel_inline(
        types: &mut Types,
        sentinel: &'static str,
        generics: &'static [Generic],
        instantiation_generics: &[(GenericReference, DataType)],
        has_const_param: bool,
        container_inline: bool,
        passthrough_inline: bool,
        build_ndt: fn(&mut Types, &mut NamedDataType),
        mut build_ty: fn(&mut Types) -> DataType,
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();
        let mut inline = container_inline || SHOULD_INLINE.get();

        println!("init_with_sentinel_inline {sentinel} {inline}");

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

            // TODO: Do we need panic catcher still?
            // TODO: Adjust typemap length properly
            types.types.insert(id.clone(), None);
            let prev_inline = SHOULD_INLINE.replace(false);
            let prev_has_const_params = HAS_CONST_PARAMS.replace(has_const_param);
            let result = panic::catch_unwind(AssertUnwindSafe(|| build_ndt(types, &mut ndt)));
            SHOULD_INLINE.set(prev_inline);
            HAS_CONST_PARAMS.set(prev_has_const_params);
            if let Err(payload) = result {
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

        // // TODO: Is this good?
        // if types.types.get(&id).cloned().is_none() {
        //     return Reference::Named(NamedReference {
        //         id,
        //         inner: NamedReferenceInner::Reference {
        //             generics: Vec::new(), // TODO: Hook this up
        //         },
        //     });
        // }

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
                    inner: NamedReferenceInner::Recursive,
                });
            }

            let prev_inline = SHOULD_INLINE.replace(
                // Say for `Box<T>` if we put `#[specta(inline)]` on it we will,
                // naively inline the `Box` instead of `T`.
                //
                // "wrapper" types enable this to properly passthrough inline.
                if passthrough_inline {
                    SHOULD_INLINE.get()
                } else {
                    false
                },
            );
            types.stack.push(hash);
            let result = panic::catch_unwind(AssertUnwindSafe(|| build_ty(types)));
            SHOULD_INLINE.set(prev_inline);
            types.stack.pop();
            let dt = match result {
                Ok(dt) => Box::new(dt),
                Err(payload) => panic::resume_unwind(payload),
            };

            Reference::Named(NamedReference {
                id,
                inner: NamedReferenceInner::Inline { dt },
            })
        } else {
            Reference::Named(NamedReference {
                id,
                inner: NamedReferenceInner::Reference {
                    generics: instantiation_generics.to_owned(),
                },
            })
        }
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        generics_for_ndt: &'static [Generic],
        generics_for_ref: Vec<(GenericReference, DataType)>,
        has_const_param: bool,
        inline: bool,
        types: &mut Types,
        sentinel: &'static str,
        build_ndt: fn(&mut Types, &mut NamedDataType),
    ) -> Reference {
        todo!();
    }

    /// Construct a [Reference] to a [NamedDataType].
    /// This can be included in a `DataType::Reference` within another type.
    ///
    /// This reference will be inlined if the type is inlined, otherwise you can inline it with [Reference::inline].
    pub fn reference(&self, generics: Vec<(GenericReference, DataType)>) -> Reference {
        // TODO: allow generics to be `Cow`
        // TODO: HashMap instead of array for better typesafety??

        // Reference::Named(NamedReference {
        //     id: self.id.clone(),
        //     generics,
        //     inline: self.inline,
        //     instance: None,
        //     dt: None, // TODO
        // })

        todo!();
    }

    /// Check whether a type requires a reference to be generated.
    ///
    /// This if `false` is all [Reference]'s created for the type are inlined,
    /// in that case it doesn't need to be exported because it will never be
    /// referenced.
    pub fn requires_reference(&self, _types: &Types) -> bool {
        // `Types` is unused but I wanna keep it for future flexibility.

        // If a type is inlined, all it's references are,
        // therefor we don't need to export a named version of it.
        // !self.inline

        todo!();
    }

    /// Construct a [Reference] to this [NamedDataType] using another reference as a template.
    ///
    /// This preserves hidden per-reference state such as inline and instance information while
    /// retargeting the reference to this named type.
    pub fn reference_from(&self, template: &NamedReference) -> Reference {
        // Reference::Named(NamedReference {
        //     id: self.id.clone(),
        //     generics: template.generics.clone(),
        //     inline: template.inline,
        //     instance: template.instance,
        //     dt: None, // TODO
        // })

        todo!();
    }

    /// Allows you to map over the inner [`DataType`] and all instances.
    /// Sometimes a [`NamedDataType`] will be represented as multiple [`DataType`]'s so inlining can be more accurate so you should prefer this over [`NamedDataType::ty_*`] helpers.
    pub fn map_ty_mut(&mut self, mut f: impl FnMut(&mut DataType)) {
        // f(&mut self.ty);
        // for instance in self.instances.iter_mut() {
        //     f(instance);
        // }
        todo!();
    }

    pub(crate) fn register_instance(&mut self, ty: DataType) -> Option<usize> {
        // if self.ty == ty {
        //     return None;
        // }

        // if let Some(index) = self.instances.iter().position(|existing| existing == &ty) {
        //     return Some(index);
        // }

        // let index = self.instances.len();
        // self.instances.push(ty);
        // Some(index)
        todo!();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
/// Runtime representation of Rust's `#[deprecated]` metadata.
pub struct Deprecated {
    pub note: Option<Cow<'static, str>>,
    since: Option<Cow<'static, str>>,
}

impl Deprecated {
    /// Construct deprecation metadata without details.
    ///
    /// Eg. `#[deprecated]`
    pub const fn new() -> Self {
        Self {
            note: None,
            since: None,
        }
    }

    /// Construct deprecation metadata with a note/message.
    ///
    /// Eg. `#[deprecated = "Use something else"]`
    pub fn with_note(note: Cow<'static, str>) -> Self {
        Self {
            note: Some(note),
            since: None,
        }
    }

    /// Construct deprecation metadata with a note/message and an optional `since` version.
    ///
    /// Eg. `#[deprecated(since = "1.0.0", note = "Use something else")]`
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
