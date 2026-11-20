//! Field types are used by both enums and structs.

use std::{any::Any, borrow::Cow, fmt::Debug, hash::Hash};

use crate::datatype::Struct;

use super::{Attributes, DataType, Deprecated};

/// Field layout for a struct or enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fields {
    /// Unit struct.
    ///
    /// Represented in Rust as `pub struct Unit;` and in TypeScript as `null`.
    Unit,
    /// Struct with unnamed fields.
    ///
    /// Represented in Rust as `pub struct Unit();` and in TypeScript as `[]`.
    Unnamed(UnnamedFields),
    /// Struct with named fields.
    ///
    /// Represented in Rust as `pub struct Unit {}` and in TypeScript as `{}`.
    Named(NamedFields),
}

/// Metadata for a struct field or enum variant field.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Field {
    /// Whether the field was marked optional, for example with
    /// `#[specta(optional)]`.
    pub optional: bool,
    /// Deprecated attribute for the field.
    pub deprecated: Option<Deprecated>,
    /// Documentation comments for the field.
    pub docs: Cow<'static, str>,
    /// Runtime attributes for this field.
    pub attributes: Attributes,
    /// Type for the field.
    ///
    /// This is `None` when the field was skipped with an attribute such as
    /// `#[serde(skip)]` or `#[specta(skip)]`. Exporters should preserve enough
    /// information to distinguish skipped fields from absent fields because some
    /// serialization formats still let skipped fields affect layout.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub ty: Option<DataType>,
}

impl Field {
    /// Constructs a new non-optional field with the given type.
    ///
    /// Use [`Field::default`] to construct skipped-field metadata where `ty` is
    /// initially `None`.
    pub fn new(ty: DataType) -> Self {
        Field {
            optional: false,
            deprecated: None,
            docs: "".into(),
            ty: Some(ty),
            attributes: Attributes::default(),
        }
    }

    /// Marks the field as optional.
    ///
    /// This is the consuming form of setting `Field::optional`, so a field can be
    /// built as one expression. Exporters render an optional field as a property
    /// that may be absent; `#[specta(optional)]` and `#[serde(default)]` record it
    /// for you on a derived type.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Sets `Field::deprecated`, the consuming form of the assignment.
    pub fn deprecated(mut self, deprecated: Option<Deprecated>) -> Self {
        self.deprecated = deprecated;
        self
    }

    /// Sets `Field::docs` from a `'static` string, which is what a
    /// `///` doc comment produces.
    pub fn docs(mut self, docs: &'static str) -> Self {
        self.docs = Cow::Borrowed(docs);
        self
    }

    /// Inserts one runtime attribute, like [`Attributes::insert`], and returns
    /// the field.
    ///
    /// Keys beginning with `specta:` or `serde:` are reserved for the derive
    /// and the exporters that read them; give application metadata its own
    /// prefix.
    pub fn attribute<T>(mut self, key: &'static str, value: T) -> Self
    where
        T: Any + Clone + Eq + Hash + Debug + Send + Sync + 'static,
    {
        self.attributes.insert(key, value);
        self
    }

    /// Records that the field was declared as `Option<T>`.
    ///
    /// This is the `specta:nullable` runtime attribute, which consumers such as
    /// `specta-serde` and `specta-swift` read to tell a missing value apart from
    /// an absent field. The derive sets it on every field declared as an `Option`;
    /// a hand-written implementation can set it when the exported datatype alone
    /// doesn't say it (an overridden type, or a `#[specta(skip)]` field).
    ///
    /// The flag is advisory and syntactic: it describes what the source declared,
    /// not what the datatype contains.
    pub fn nullable(mut self) -> Self {
        self.attributes.insert("specta:nullable", true);
        self
    }
}

impl From<DataType> for Field {
    /// Constructs a field with the given type, exactly like [`Field::new`].
    fn from(ty: DataType) -> Self {
        Self::new(ty)
    }
}

/// Fields for an unnamed tuple struct or tuple enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UnnamedFields {
    /// Field metadata in source order.
    pub fields: Vec<Field>,
}

/// Fields for a named struct or struct enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedFields {
    /// Field names and metadata in source order.
    pub fields: Vec<(Cow<'static, str>, Field)>,
}

#[derive(Debug, Clone)]
/// Builder for constructing [`DataType::Struct`] values.
///
/// The type parameter tracks whether the builder is currently collecting named
/// or unnamed fields.
pub struct StructBuilder<F = ()> {
    pub(crate) fields: F,
}

impl StructBuilder<NamedFields> {
    /// Adds a named field and returns the updated builder.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: impl Into<Field>) -> Self {
        self.fields.fields.push((name.into(), field.into()));
        self
    }

    /// Adds a named field in-place.
    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: impl Into<Field>) {
        self.fields.fields.push((name.into(), field.into()));
    }

    /// Finalizes this builder into a [`DataType::Struct`] with named fields.
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Named(self.fields),
            attributes: Default::default(),
        })
    }
}

impl StructBuilder<UnnamedFields> {
    /// Adds an unnamed field and returns the updated builder.
    pub fn field(mut self, field: impl Into<Field>) -> Self {
        self.fields.fields.push(field.into());
        self
    }

    /// Adds an unnamed field in-place.
    pub fn field_mut(&mut self, field: impl Into<Field>) {
        self.fields.fields.push(field.into());
    }

    /// Finalizes this builder into a [`DataType::Struct`] with unnamed fields.
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}
