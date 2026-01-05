//! Field types are used by both enums and structs.

use super::{
    DataType, DeprecatedType, RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta,
};
use std::borrow::Cow;

/// Data stored within an enum variant or struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fields {
    /// A unit struct.
    ///
    /// Represented in Rust as `pub struct Unit;` and in TypeScript as `null`.
    Unit,
    /// A struct with unnamed fields.
    ///
    /// Represented in Rust as `pub struct Unit();` and in TypeScript as `[]`.
    Unnamed(UnnamedFields),
    /// A struct with named fields.
    ///
    /// Represented in Rust as `pub struct Unit {}` and in TypeScript as `{}`.
    Named(NamedFields),
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    /// Did the user apply a `#[specta(optional)]` attribute.
    pub(crate) optional: bool,
    /// Did the user apply a `#[serde(flatten)]` or `#[specta(flatten)]` attribute.
    pub(crate) flatten: bool,
    /// Deprecated attribute for the field.
    pub(crate) deprecated: Option<DeprecatedType>,
    /// Documentation comments for the field.
    pub(crate) docs: Cow<'static, str>,
    /// Type for the field. Is optional if `#[serde(skip)]` or `#[specta(skip)]` was applied.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub(crate) ty: Option<DataType>,
    // TODO: This is a Typescript-specific thing
    pub(crate) inline: bool,
    /// Runtime attributes for this field (e.g., serde attributes)
    pub(crate) attributes: Vec<RuntimeAttribute>,
}

impl Field {
    /// Construct a new field with the given type.
    ///
    /// You can skip the requirement on providing a [`DataType`] by using [`Field::default`]
    pub fn new(ty: DataType) -> Self {
        Field {
            optional: false,
            flatten: false,
            deprecated: None,
            docs: "".into(),
            inline: false,
            ty: Some(ty),
            attributes: Vec::new(),
        }
    }

    /// Has the Serde or Specta optional attribute been applied to this field?
    pub fn optional(&self) -> bool {
        self.optional
    }

    /// Set the optional attribute for this field.
    pub fn set_optional(&mut self, optional: bool) {
        self.optional = optional;
    }

    /// Has the Serde or Specta flatten attribute been applied to this field?
    pub fn flatten(&self) -> bool {
        self.flatten
    }

    /// Set the flatten attribute for this field.
    pub fn set_flatten(&mut self, flatten: bool) {
        self.flatten = flatten;
    }

    /// Has the Serde inline attribute been applied to this field?
    pub fn inline(&self) -> bool {
        self.inline
    }

    /// Set the inline attribute for this field.
    pub fn set_inline(&mut self, inline: bool) {
        self.inline = inline;
    }

    /// Has the Rust deprecated attribute been applied to this field?
    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    /// Has the Rust deprecated attribute been applied to this field?
    pub fn deprecated_mut(&mut self) -> Option<&mut DeprecatedType> {
        self.deprecated.as_mut()
    }

    /// Set the deprecated attribute for this field.
    pub fn set_deprecated(&mut self, deprecated: Option<DeprecatedType>) {
        self.deprecated = deprecated;
    }

    /// Get an immutable reference to the documentation attribute for this field.
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// Mutable reference to the documentation attribute for this field.
    pub fn docs_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.docs
    }

    /// Set the documentation attribute for this field.
    pub fn set_docs(&mut self, docs: Cow<'static, str>) {
        self.docs = docs;
    }

    /// Get an immutable reference to the type of this field.
    pub fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
    }

    /// Mutable reference to the type of this field.
    pub fn ty_mut(&mut self) -> Option<&mut DataType> {
        self.ty.as_mut()
    }

    /// Set the type of this field.
    pub fn set_ty(&mut self, ty: DataType) {
        self.ty = Some(ty);
    }

    /// Get an immutable reference to the runtime attributes for this field.
    pub fn attributes(&self) -> &Vec<RuntimeAttribute> {
        &self.attributes
    }

    /// Mutable reference to the runtime attributes for this field.
    pub fn attributes_mut(&mut self) -> &mut Vec<RuntimeAttribute> {
        &mut self.attributes
    }

    /// Set the runtime attributes for this field.
    pub fn set_attributes(&mut self, attrs: Vec<RuntimeAttribute>) {
        self.attributes = attrs;
    }
}

/// The fields of an unnamed enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnnamedFields {
    pub(crate) fields: Vec<Field>,
    pub(crate) attributes: Vec<RuntimeAttribute>,
}

impl UnnamedFields {
    /// Get an immutable reference to the fields of this unnamed enum variant.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Mutable reference to the fields of this unnamed enum variant.
    pub fn fields_mut(&mut self) -> &mut Vec<Field> {
        &mut self.fields
    }

    /// Get an immutable reference to the runtime attributes for this unnamed fields.
    pub fn attributes(&self) -> &Vec<RuntimeAttribute> {
        &self.attributes
    }

    /// Mutable reference to the runtime attributes for this unnamed fields.
    pub fn attributes_mut(&mut self) -> &mut Vec<RuntimeAttribute> {
        &mut self.attributes
    }

    /// Set the runtime attributes for this unnamed fields.
    pub fn set_attributes(&mut self, attrs: Vec<RuntimeAttribute>) {
        self.attributes = attrs;
    }
}

/// The fields of an named enum variant or a struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedFields {
    pub(crate) fields: Vec<(Cow<'static, str>, Field)>,
    pub(crate) attributes: Vec<RuntimeAttribute>,
}

impl NamedFields {
    /// Get an immutable reference to the fields.
    pub fn fields(&self) -> &[(Cow<'static, str>, Field)] {
        &self.fields
    }

    /// Mutable reference to the fields.
    pub fn fields_mut(&mut self) -> &mut Vec<(Cow<'static, str>, Field)> {
        &mut self.fields
    }

    /// Get an immutable reference to the runtime attributes for this named fields.
    pub fn attributes(&self) -> &Vec<RuntimeAttribute> {
        &self.attributes
    }

    /// Mutable reference to the runtime attributes for this named fields.
    pub fn attributes_mut(&mut self) -> &mut Vec<RuntimeAttribute> {
        &mut self.attributes
    }

    /// Set the runtime attributes for this named fields.
    pub fn set_attributes(&mut self, attrs: Vec<RuntimeAttribute>) {
        self.attributes = attrs;
    }
}
