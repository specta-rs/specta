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
        DataType, Generic, NamedReference, Reference,
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
    static CONTEXT_HAS_CONST_PARAMS: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn context_has_const_params() -> bool {
    CONTEXT_HAS_CONST_PARAMS.with(|c| c.get())
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
    pub inline: bool,
    pub ty: DataType,
    /// Specialized instantiated shapes for references to this named type.
    ///
    /// The canonical [`NamedDataType::ty`] must stay general so it can be shared by all references.
    /// Some references, such as const-generic instantiations or post-processing rewrites, need a more precise shape than the canonical definition.
    ///
    /// We store those shapes here and let [`NamedReference`] keep only a stable index into this list.
    /// Keeping the instantiated [`DataType`] off the reference avoids recursive type graphs becoming part of the reference's hash/equality semantics.
    ///
    /// This is kept private to ensure we can remove the removal of items as that would change the indexes and could break existing references.
    pub(crate) instances: Vec<DataType>,
}

impl NamedDataType {
    /// Construct a new named datatype.
    ///
    /// Note: Ensure you call `Self::register` to register the type.
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
            generics: Cow::Owned(generics),
            inline: false,
            ty: dt,
            instances: Vec::new(),
        }
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
        Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual")),
            location: location.to_owned(),
            generics: Cow::Owned(generics),
            inline: true,
            ty: dt,
            instances: Vec::new(),
        }
    }

    /// Clones the inner [`DataType`] of this named datatype to one with a unique identifier.
    /// This can be modified before being registered. Not it's name and module path will overlap if not changed,
    /// which will likely cause a duplication type warning in your exporter.
    pub fn clone_ty(&self) -> Self {
        Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: self.name.clone(),
            docs: self.docs.clone(),
            deprecated: self.deprecated.clone(),
            module_path: self.module_path.clone(),
            location: self.location.clone(),
            generics: self.generics.clone(),
            inline: self.inline,
            ty: self.ty.clone(),
            instances: self.instances.clone(),
        }
    }

    /// Register the type into a [Types].
    pub fn register(&self, types: &mut Types) {
        types.types.insert(self.id.clone(), Some(self.clone()));
        types.len += 1;
    }

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
    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        generics_for_ndt: &'static [Generic],
        generics_for_ref: Vec<(GenericReference, DataType)>,
        mut inline: bool,
        has_const_param: bool,
        types: &mut Types,
        sentinel: &'static str,
        build_ndt: fn(&mut Types, &mut NamedDataType),
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();

        let hash = {
            let mut h = DefaultHasher::new();
            sentinel.hash(&mut h);
            ptr::hash(sentinel, &mut h);
            for (generic_r, generic) in &generics_for_ref {
                generic_r.id.hash(&mut h);
                generic.hash(&mut h);
            }
            h.finish()
        };
        println!("{:?} {sentinel} {generics_for_ref:?}", hash);

        // If this named type is already being resolved, emit a reference to the placeholder
        // instead of re-entering resolution which would likely trigger a stack overflow.
        //
        // This stops us resolving the `instances` entry for recursively inlined types but resolving it would just infinitely recurse so we can't.
        if types.types.get(&id).is_some_and(|slot| slot.is_none()) {
            return Reference::Named(NamedReference {
                id: id.clone(),
                generics: generics_for_ref.clone(),
                inline,
                instance: None,
            });
        }

        let mut ndt = NamedDataType {
            id: id.clone(),
            location,
            // `build_ndt` will just override all of this.
            name: Cow::Borrowed(""),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: Cow::Borrowed(""),
            generics: Cow::Borrowed(generics_for_ndt),
            inline,
            ty: DataType::Primitive(super::Primitive::i8),
            instances: Vec::new(),
        };

        if let Some(existing_inline) = types
            .types
            .get(&id)
            .and_then(Option::as_ref)
            .map(|ndt| ndt.inline)
        {
            inline |= existing_inline;
        } else {
            let mut ndt = ndt.clone();

            let previous = CONTEXT_HAS_CONST_PARAMS.replace(has_const_param);
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                let prev = types.types.insert(id.clone(), None);
                let result = panic::catch_unwind(AssertUnwindSafe(|| build_ndt(types, &mut ndt)));
                if let Some(prev) = prev {
                    types.types.insert(id.clone(), prev);
                }
                if let Err(payload) = result {
                    panic::resume_unwind(payload);
                }
            }));
            CONTEXT_HAS_CONST_PARAMS.set(previous);

            match result {
                Ok(value) => value,
                Err(payload) => panic::resume_unwind(payload),
            };

            // We patch the Tauri `Type` implementation.
            if ndt.name == "TAURI_CHANNEL" && ndt.module_path.starts_with("tauri::") {
                // This causes an exporter that isn't aware of Tauri's channel to error.
                // This is effectively `Reference::opaque(TauriChannel)` but we do some hackery for better errors.
                ndt.ty = reference::tauri().into();

                // This ensures that we never create a `export type Channel`,
                // instead the definition gets inlined into each callsite.
                inline = true;
                ndt.inline = true;
            }

            types.types.insert(id.clone(), Some(ndt));
            types.len += 1;
        }

        let prev = types.types.insert(id.clone(), None);
        let result = panic::catch_unwind(AssertUnwindSafe(|| build_ndt(types, &mut ndt)));
        if let Some(prev) = prev {
            types.types.insert(id.clone(), prev);
        }
        if let Err(payload) = result {
            panic::resume_unwind(payload);
        }

        if ndt.name == "TAURI_CHANNEL" && ndt.module_path.starts_with("tauri::") {
            ndt.ty = reference::tauri().into();
            inline = true;
        }

        let instance = types
            .types
            .get_mut(&id)
            .and_then(Option::as_mut)
            .and_then(|existing| existing.register_instance(ndt.ty));

        Reference::Named(NamedReference {
            id: id.clone(),
            generics: generics_for_ref,
            inline,
            instance,
        })
    }

    /// Construct a [Reference] to a [NamedDataType].
    /// This can be included in a `DataType::Reference` within another type.
    ///
    /// This reference will be inlined if the type is inlined, otherwise you can inline it with [Reference::inline].
    pub fn reference(&self, generics: Vec<(GenericReference, DataType)>) -> Reference {
        // TODO: allow generics to be `Cow`
        // TODO: HashMap instead of array for better typesafety??

        Reference::Named(NamedReference {
            id: self.id.clone(),
            generics,
            inline: self.inline,
            instance: None,
        })
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
        !self.inline
    }

    /// Construct a [Reference] to this [NamedDataType] using another reference as a template.
    ///
    /// This preserves hidden per-reference state such as inline and instance information while
    /// retargeting the reference to this named type.
    pub fn reference_from(&self, template: &NamedReference) -> Reference {
        Reference::Named(NamedReference {
            id: self.id.clone(),
            generics: template.generics.clone(),
            inline: template.inline,
            instance: template.instance,
        })
    }

    /// Allows you to map over the inner [`DataType`] and all instances.
    /// Sometimes a [`NamedDataType`] will be represented as multiple [`DataType`]'s so inlining can be more accurate so you should prefer this over [`NamedDataType::ty_*`] helpers.
    pub fn map_ty_mut(&mut self, mut f: impl FnMut(&mut DataType)) {
        f(&mut self.ty);
        for instance in self.instances.iter_mut() {
            f(instance);
        }
    }

    pub(crate) fn register_instance(&mut self, ty: DataType) -> Option<usize> {
        if self.ty == ty {
            return None;
        }

        if let Some(index) = self.instances.iter().position(|existing| existing == &ty) {
            return Some(index);
        }

        let index = self.instances.len();
        self.instances.push(ty);
        Some(index)
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
