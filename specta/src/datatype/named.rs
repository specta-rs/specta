use std::{borrow::Cow, convert::Infallible, panic::Location, sync::Arc};

use crate::{
    Type, TypeCollection,
    datatype::{
        DataType, Generic, NamedDataTypeBuilder, NamedReference, Reference, reference::NamedId,
    },
};

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedDataType {
    pub(crate) id: NamedId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) module_path: Cow<'static, str>,
    pub(crate) location: Location<'static>,
    pub(crate) generics: Vec<Generic>,
    pub(crate) inline: bool,
    pub(crate) inner: DataType,
}

impl NamedDataType {
    // ## Sentinel
    //
    // MUST point to a `static ...: () = ();`. This is used as a unique identifier for the type and `const` or `Box::leak` SHOULD NOT be used.
    //
    // If this invariant is violated you will see unexpected behavior.
    //
    // ## Why return a reference?
    //
    // If a recursive type is being resolved it's possible the `init_with_sentinel` function will be called recursively.
    // To avoid this we avoid resolving a type that's already marked as being resolved but this means the [NamedDataType]'s [DataType] is unknown at this stage so we can't return it. Instead we always return [Reference]'s as they are always valid.
    #[doc(hidden)] // This should not be used outside of `specta_macros` as it may have breaking changes.
    #[track_caller]
    pub fn init_with_sentinel(
        generics: Vec<(Generic, DataType)>,
        mut inline: bool,
        types: &mut TypeCollection,
        sentinel: &'static (),
        build_ndt: fn(&mut TypeCollection, &mut NamedDataType),
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();

        // We have never encountered this type. Start resolving it!
        if let Some(ndt) = types.0.get(&id) {
            if let Some(ndt) = ndt {
                inline = inline || ndt.inline;
            }
        } else {
            types.0.insert(id.clone(), None);
            let mut ndt = NamedDataType {
                id: id.clone(),
                location,
                // `build_ndt` will just override all of this
                name: Cow::Borrowed(""),
                docs: Cow::Borrowed(""),
                deprecated: None,
                module_path: Cow::Borrowed(""),
                generics: vec![],
                inline,
                inner: DataType::Primitive(super::Primitive::i8),
            };
            build_ndt(types, &mut ndt);

            // We patch the Tauri `Type` implementation.
            // TODO: Can we upstream these without backwards compatibility issues???
            if ndt.name() == "TAURI_CHANNEL" && ndt.module_path().starts_with("tauri::") {
                // This produces `never`.
                // It's expected a framework replaces this with it's own setup.
                ndt.inner = Infallible::definition(types);

                // This ensures that we never create a `export type Channel`,
                // instead the definition gets inlined into each callsite.
                inline = true;
                ndt.inline = true;
            }

            types.0.insert(id.clone(), Some(ndt));
            types.1 += 1;
        }

        Reference::Named(NamedReference {
            id,
            generics,
            inline,
        })
    }

    /// Register a runtime named datatype.
    /// This is exposed via [NamedDataTypeBuilder::build].
    #[track_caller]
    pub(crate) fn register(builder: NamedDataTypeBuilder, types: &mut TypeCollection) -> Self {
        let location = Location::caller();

        let module_path = builder.module_path.unwrap_or_else(|| {
            file_path_to_module_path(location.file())
                .map(Into::into)
                .unwrap_or(Cow::Borrowed("virtual"))
        });

        let ndt = Self {
            id: NamedId::Dynamic(Arc::new(())),
            name: builder.name,
            docs: builder.docs,
            deprecated: builder.deprecated,
            module_path,
            location: location.to_owned(),
            generics: builder.generics,
            inline: builder.inline,
            inner: builder.inner,
        };

        types.0.insert(ndt.id.clone(), Some(ndt.clone()));
        types.1 += 1;
        ndt
    }

    /// Construct a [Reference] to a [NamedDataType].
    /// This can be included in a `DataType::Reference` within another type.
    ///
    /// This reference will be inlined if the type is inlined, otherwise you can inline it with [Reference::inline].
    pub fn reference(&self, generics: Vec<(Generic, DataType)>) -> Reference {
        // TODO: allow generics to be `Cow`
        // TODO: HashMap instead of array for better typesafety??

        Reference::Named(NamedReference {
            id: self.id.clone(),
            generics,
            inline: self.inline,
        })
    }

    /// Check whether a type requires a reference to be generated.
    ///
    /// This if `false` is all [Reference]'s created for the type are inlined,
    /// in that case it doesn't need to be exported because it will never be
    /// referenced.
    pub fn requires_reference(&self, _types: &TypeCollection) -> bool {
        // `TypeCollection` is unused but I wanna keep it for future flexibility.

        // If a type is inlined, all it's references are,
        // therefor we don't need to export a named version of it.
        !self.inline
    }

    /// The name of the type
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// Get a mutable reference to the name of the type
    pub fn name_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.name
    }

    /// Set the name of the type
    pub fn set_name(&mut self, name: Cow<'static, str>) {
        self.name = name;
    }

    /// Rust documentation comments on the type
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// Get a mutable reference to the Rust documentation comments on the type
    pub fn docs_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.docs
    }

    /// Set the Rust documentation comments on the type
    pub fn set_docs(&mut self, docs: Cow<'static, str>) {
        self.docs = docs;
    }

    /// The Rust deprecated comment if the type is deprecated.
    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    /// Get a mutable reference to the Rust deprecated comment if the type is deprecated.
    pub fn deprecated_mut(&mut self) -> Option<&mut DeprecatedType> {
        self.deprecated.as_mut()
    }

    /// Set the Rust deprecated comment if the type is deprecated.
    pub fn set_deprecated(&mut self, deprecated: Option<DeprecatedType>) {
        self.deprecated = deprecated;
    }

    /// The code location where this type is implemented
    pub fn location(&self) -> Location<'static> {
        self.location
    }

    /// Set the code location where this type is implemented
    pub fn set_location(&mut self, location: Location<'static>) {
        self.location = location;
    }

    /// The Rust path of the module where this type is defined
    pub fn module_path(&self) -> &Cow<'static, str> {
        &self.module_path
    }

    /// Get a mutable reference to the Rust path of the module where this type is defined
    pub fn module_path_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.module_path
    }

    /// Set the Rust path of the module where this type is defined
    pub fn set_module_path(&mut self, module_path: Cow<'static, str>) {
        self.module_path = module_path;
    }

    /// The generics that are defined on this type
    pub fn generics(&self) -> &[Generic] {
        &self.generics
    }

    /// Get a mutable reference to the generics that are defined on this type
    pub fn generics_mut(&mut self) -> &mut Vec<Generic> {
        &mut self.generics
    }

    /// Get the inner [`DataType`]
    pub fn ty(&self) -> &DataType {
        &self.inner
    }

    /// Get a mutable reference to the inner [`DataType`]
    pub fn ty_mut(&mut self) -> &mut DataType {
        &mut self.inner
    }

    /// Set the inner [`DataType`]
    pub fn set_ty(&mut self, ty: DataType) {
        self.inner = ty;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeprecatedType {
    /// A type that has been deprecated without a message.
    ///
    /// Eg. `#[deprecated]`
    Deprecated,
    /// A type that has been deprecated with a message and an optional `since` version.
    ///
    /// Eg. `#[deprecated = "Use something else"]` or `#[deprecated(since = "1.0.0", message = "Use something else")]`
    DeprecatedWithSince {
        since: Option<Cow<'static, str>>,
        note: Cow<'static, str>,
    },
}

fn file_path_to_module_path(file_path: &str) -> Option<String> {
    // Try different prefixes
    let (prefix, path) = if let Some(p) = file_path.strip_prefix("src/") {
        ("crate", p)
    } else if let Some(p) = file_path.strip_prefix("tests/") {
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
