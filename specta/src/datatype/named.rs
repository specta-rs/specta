use std::{borrow::Cow, panic::Location, sync::Arc};

use crate::{
    Types,
    datatype::{
        DataType, NamedReference, Reference,
        reference::{self, GenericReference, NamedId},
    },
};

/// Named type represents any type with it's own unique name and identity.
///
/// These can become `export MyNamedType = ...` in Typescript can we be referenced in types like `{ field: MyNamedType }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedDataType {
    pub(crate) id: NamedId,
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<Deprecated>,
    pub(crate) module_path: Cow<'static, str>,
    pub(crate) location: Location<'static>,
    pub(crate) generics: Cow<'static, [(GenericReference, Cow<'static, str>)]>,
    pub(crate) inline: bool,
    pub(crate) inner: DataType,
}

impl NamedDataType {
    /// Construct a new named datatype.
    ///
    /// Note: Ensure you call `Self::register` to register the type.
    #[track_caller]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        generics: Vec<(GenericReference, Cow<'static, str>)>,
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
            inline: false,
            inner: dt,
        }
    }

    /// Construct a new inlined named datatype.
    ///
    /// Note: Ensure you call `Self::register` to register the type.
    #[track_caller]
    pub fn new_inline(
        name: impl Into<Cow<'static, str>>,
        generics: Vec<(GenericReference, Cow<'static, str>)>,
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
            inner: dt,
        }
    }

    /// Register the type into a [Types].
    pub fn register(&self, types: &mut Types) {
        types.0.insert(self.id.clone(), Some(self.clone()));
        types.1 += 1;
    }

    // ## Why return a reference?
    //
    // If a recursive type is being resolved it's possible the `init_with_sentinel` function will be called recursively.
    // To avoid this we avoid resolving a type that's already marked as being resolved but this means the [NamedDataType]'s [DataType] is unknown at this stage so we can't return it. Instead we always return [Reference]'s as they are always valid.
    // WARNING: This should not be used outside of `specta_macros` as it may have breaking changes in minor releases
    #[doc(hidden)]
    #[track_caller]
    pub fn init_with_sentinel(
        generics_for_ndt: &'static [(GenericReference, Cow<'static, str>)],
        generics_for_ref: Vec<(GenericReference, DataType)>,
        mut inline: bool,
        types: &mut Types,
        sentinel: &'static str,
        build_ndt: fn(&mut Types, &mut NamedDataType),
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
                generics: Cow::Borrowed(generics_for_ndt),
                inline,
                inner: DataType::Primitive(super::Primitive::i8),
            };
            build_ndt(types, &mut ndt);

            // We patch the Tauri `Type` implementation.
            if ndt.name() == "TAURI_CHANNEL" && ndt.module_path().starts_with("tauri::") {
                // This causes an exporter that isn't aware of Tauri's channel to error.
                // This is effectively `Reference::opaque(TauriChannel)` but we do some hackery for better errors.
                ndt.inner = reference::tauri().into();

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
            generics: generics_for_ref,
            inline,
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
    pub fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }

    /// Get a mutable reference to the Rust deprecated comment if the type is deprecated.
    pub fn deprecated_mut(&mut self) -> Option<&mut Deprecated> {
        self.deprecated.as_mut()
    }

    /// Set the Rust deprecated comment if the type is deprecated.
    pub fn set_deprecated(&mut self, deprecated: Option<Deprecated>) {
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
    pub fn generics(&self) -> &[(GenericReference, Cow<'static, str>)] {
        &self.generics
    }

    /// Get a mutable reference to the generics that are defined on this type
    pub fn generics_mut(&mut self) -> &mut Cow<'static, [(GenericReference, Cow<'static, str>)]> {
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
/// Runtime representation of Rust's `#[deprecated]` metadata.
pub struct Deprecated {
    note: Option<Cow<'static, str>>,
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

    /// Optional deprecation note/message.
    pub fn note(&self) -> Option<&Cow<'static, str>> {
        self.note.as_ref()
    }

    /// Mutable optional deprecation note/message.
    pub fn note_mut(&mut self) -> Option<&mut Cow<'static, str>> {
        self.note.as_mut()
    }

    /// Set the optional deprecation note/message.
    pub fn set_note(&mut self, note: Option<Cow<'static, str>>) {
        self.note = note;
    }

    /// Optional version string from `since = "..."`.
    pub fn since(&self) -> Option<&Cow<'static, str>> {
        self.since.as_ref()
    }

    /// Mutable optional version string from `since = "..."`.
    pub fn since_mut(&mut self) -> Option<&mut Cow<'static, str>> {
        self.since.as_mut()
    }

    /// Set the optional version string from `since = "..."`.
    pub fn set_since(&mut self, since: Option<Cow<'static, str>>) {
        self.since = since;
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
