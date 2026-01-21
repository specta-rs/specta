use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    panic::Location,
    sync::Arc,
};

use crate::{
    datatype::{
        reference::NamedId, DataType, Generic, NamedDataTypeBuilder, NamedReference, Reference,
    },
    TypeCollection,
};

thread_local! {
    static COLLECTED_TYPES: RefCell<Option<Vec<HashMap<NamedId, NamedDataType>>>> = const { RefCell::new(None) };
}

/// Collects all named data types constructed within the provided closure.
///
/// This is useful for collecting up the required imports when generating an output file.
pub fn collect(func: impl FnOnce()) -> impl Iterator<Item = NamedDataType> {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            COLLECTED_TYPES.with_borrow_mut(|types| {
                if let Some(v) = types {
                    // Last collection means we can drop all memory
                    if v.len() == 1 {
                        *types = None;
                    } else {
                        // Otherwise just remove the current collection.
                        v.pop();
                    }
                }
            })
        }
    }

    // If we have no collection, register one
    // If we already have one create a new context.
    COLLECTED_TYPES.with_borrow_mut(|v| {
        if let Some(v) = v {
            v.push(Default::default());
        } else {
            *v = Some(vec![Default::default()]);
        }
    });

    let guard = Guard;
    func();
    // We only use the guard when unwinding
    std::mem::forget(guard);

    COLLECTED_TYPES.with_borrow_mut(|types| {
        types
            .as_mut()
            .expect("COLLECTED_TYPES is unset but it should be set")
            .pop()
            .expect("COLLECTED_TYPES is missing a valid collection context")
            .into_iter()
            .map(|v| v.1)
    })
}

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
    pub(crate) tags: HashSet<TypeTag>,
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
        inline: bool,
        types: &mut TypeCollection,
        sentinel: &'static (),
        build_ndt: fn(&mut TypeCollection, &mut NamedDataType),
    ) -> Reference {
        let id = NamedId::Static(sentinel);
        let location = Location::caller().to_owned();

        if let Some(ndt) = types.0.get(&id) {
            // If this is `None` we will add into the `COLLECTED_TYPES`,
            // when resolution is finished.
            if let Some(ndt) = ndt {
                let needs_references = COLLECTED_TYPES.with_borrow_mut(|ctxs| {
                    if let Some(ctxs) = ctxs {
                        for ctx in ctxs {
                            ctx.insert(ndt.id.clone(), ndt.clone());
                        }

                        true
                    } else {
                        false
                    }
                });

                // If a type has already been resolved we can just pull it's `NamedDataType` from the `TypeCollection`.
                // However this means any dependent types are never registered as references are not created in this scope.
                // We run the `build_ndt` function to resolve these dependent, even though we do just throw out the result.
                if needs_references {
                    build_ndt(
                        types,
                        &mut NamedDataType {
                            id: id.clone(),
                            location,
                            // `build_ndt` will just override all of this
                            name: Cow::Borrowed(""),
                            docs: Cow::Borrowed(""),
                            deprecated: None,
                            module_path: Cow::Borrowed(""),
                            generics: vec![],
                            inner: DataType::Primitive(super::Primitive::i8),
                        },
                    );
                }
            }
        } else {
            // We have never encountered this type. Start resolving it!

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
                inner: DataType::Primitive(super::Primitive::i8),
            };
            build_ndt(types, &mut ndt);
            COLLECTED_TYPES.with_borrow_mut(|ctxs| {
                if let Some(ctxs) = ctxs {
                    for ctx in ctxs {
                        ctx.insert(ndt.id.clone(), ndt.clone());
                    }
                }
            });
            types.0.insert(id.clone(), Some(ndt));
        }

        Reference::Named(NamedReference {
            id,
            generics,
            inline,
        })
    }

    /// Register a runtime named datatype.
    /// This is exposed via [NamedDataTypeBuilder::build].
    pub(crate) fn register(
        builder: NamedDataTypeBuilder,
        types: &mut TypeCollection,
    ) -> NamedDataType {
        let ndt = NamedDataType {
            id: NamedId::Dynamic(Arc::new(())),
            name: builder.name,
            docs: builder.docs,
            deprecated: builder.deprecated,
            module_path: builder.module_path,
            location: Location::caller().to_owned(),
            generics: builder.generics,
            inner: builder.inner,
        };

        types.0.insert(ndt.id.clone(), Some(ndt.clone()));
        COLLECTED_TYPES.with_borrow_mut(|ctxs| {
            if let Some(ctxs) = ctxs {
                for ctx in ctxs {
                    ctx.insert(ndt.id.clone(), ndt.clone());
                }
            }
        });
        ndt
    }

    /// TODO
    // TODO: Problematic to seal + allow generics to be `Cow`
    // TODO: HashMap instead of array for better typesafety??
    pub fn reference(&self, generics: Vec<(Generic, DataType)>, inline: bool) -> Reference {
        Reference::Named(NamedReference {
            id: self.id.clone(),
            generics,
            inline,
        })
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

    /// Get the tags associated with this type
    pub fn tags(&self) -> &HashSet<TypeTag> {
        &self.tags
    }

    /// Get a mutable reference to the tags associated with this type
    pub fn tags_mut(&mut self) -> &mut HashSet<TypeTag> {
        &mut self.tags
    }

    /// Set the tags associated with this type
    pub fn set_tags(&mut self, tags: HashSet<TypeTag>) {
        self.tags = tags;
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TypeTag {
    Date,
    Custom(Cow<'static, str>),
}
