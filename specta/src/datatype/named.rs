use std::{borrow::Cow, panic::Location};

use crate::{
    DataType, TypeCollection,
    datatype::{Generic, Reference, reference::ArcId},
};

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    pub(crate) id: ArcId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) module_path: Cow<'static, str>,
    pub(crate) location: Location<'static>,
    pub(crate) generics: Vec<Generic>,
    pub(crate) inner: DataType,
}

impl NamedDataType {
    // TODO: Explain invariants on sentinel
    #[doc(hidden)] // This should not be used outside of `specta_macros` as it may have breaking changes.
    pub fn init_with_sentinel(
        types: &mut TypeCollection,
        sentinel: &'static (),
        build_dt: fn(&mut TypeCollection) -> DataType,
    ) -> Self {
        // types.0

        todo!();
        // Self {
        //     id: ArcId::Static(sentinel),
        //     name: Cow::Borrowed(""),
        //     docs: Cow::Borrowed(""),
        //     deprecated: None,
        //     module_path: Cow::Borrowed(""),
        //     location: Location::caller().to_owned(),
        //     generics: Vec::new(),
        //     inner: dt,
        // }
    }

    /// TODO
    // TODO: Should this take `&mut TypeCollection` to maintain invariants???
    #[track_caller]
    pub fn new(types: &mut TypeCollection, dt: DataType) -> Self {
        // TODO: Ensure this type is registered into the type collection

        Self {
            id: ArcId::Dynamic(Default::default()),
            name: Cow::Borrowed(""),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: Cow::Borrowed(""),
            location: Location::caller().to_owned(),
            generics: Vec::new(),
            inner: dt,
        }
    }

    /// TODO
    // TODO: Problematic to seal + allow generics to be `Cow`
    // TODO: HashMap instead of array for better typesafety??
    pub fn reference(&self, generics: Vec<(Generic, DataType)>, inline: bool) -> Reference {
        Reference {
            id: self.id.clone(),
            generics,
            inline,
        }
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
