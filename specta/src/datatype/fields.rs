//! Field types are used by both enums and structs.

use crate::datatype::Struct;

use super::{Attributes, DataType, Deprecated};
use std::borrow::Cow;

/// Data stored within an enum variant or struct.
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

/// Field metadata for a struct field or enum variant field.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    /// Did the user apply a `#[specta(optional)]` attribute.
    pub(crate) optional: bool,
    /// Did the user apply a `#[serde(flatten)]` attribute.
    pub(crate) flatten: bool,
    /// Deprecated attribute for the field.
    pub(crate) deprecated: Option<Deprecated>,
    /// Documentation comments for the field.
    pub(crate) docs: Cow<'static, str>,
    /// Should we inline the definition of this type.
    pub(crate) inline: bool,
    /// Did the user apply a `#[specta(type = ...)]` or `#[specta(r#type = ...)]` attribute.
    pub(crate) type_overridden: bool,
    /// Runtime attributes for this field.
    pub(crate) attributes: Attributes,
    /// Type for the field. Is optional if `#[serde(skip)]` or `#[specta(skip)]` was applied.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub(crate) ty: Option<DataType>,
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
            type_overridden: false,
            ty: Some(ty),
            attributes: Attributes::default(),
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

    /// Has the Serde flatten attribute been applied to this field?
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

    /// Has the Specta type override attribute been applied to this field?
    pub fn type_overridden(&self) -> bool {
        self.type_overridden
    }

    /// Set whether a Specta type override attribute was applied to this field.
    pub fn set_type_overridden(&mut self, type_overridden: bool) {
        self.type_overridden = type_overridden;
    }

    /// Has the Rust deprecated attribute been applied to this field?
    pub fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }

    /// Has the Rust deprecated attribute been applied to this field?
    pub fn deprecated_mut(&mut self) -> Option<&mut Deprecated> {
        self.deprecated.as_mut()
    }

    /// Set the deprecated attribute for this field.
    pub fn set_deprecated(&mut self, deprecated: Option<Deprecated>) {
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
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// Mutable reference to the runtime attributes for this field.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    /// Set the runtime attributes for this field.
    pub fn set_attributes(&mut self, attrs: Attributes) {
        self.attributes = attrs;
    }
}

/// An iterator over the fields of a [`Fields`] value.
///
/// Yields `(Option<name>, &Field)` pairs — `None` for unnamed fields, `Some(name)` for named.
/// Unit fields produce an empty iterator.
pub enum FieldsIter<'a> {
    Named(std::slice::Iter<'a, (Cow<'static, str>, Field)>),
    Unnamed(std::slice::Iter<'a, Field>),
    Unit,
}

impl<'a> Iterator for FieldsIter<'a> {
    type Item = (Option<&'a Cow<'static, str>>, &'a Field);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Named(it) => it.next().map(|(name, f)| (Some(name), f)),
            Self::Unnamed(it) => it.next().map(|f| (None, f)),
            Self::Unit => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Named(it) => it.size_hint(),
            Self::Unnamed(it) => it.size_hint(),
            Self::Unit => (0, Some(0)),
        }
    }
}

impl ExactSizeIterator for FieldsIter<'_> {}

impl Fields {
    /// Iterate over all fields as `(Optional name, &Field)` pairs.
    pub fn iter(&self) -> FieldsIter<'_> {
        match self {
            Fields::Named(nf) => FieldsIter::Named(nf.fields.iter()),
            Fields::Unnamed(uf) => FieldsIter::Unnamed(uf.fields.iter()),
            Fields::Unit => FieldsIter::Unit,
        }
    }

    /// Iterate over field names. `None` for unnamed fields, `Some(name)` for named.
    pub fn keys(&self) -> impl Iterator<Item = Option<&Cow<'static, str>>> {
        self.iter().map(|(k, _)| k)
    }

    /// Iterate over fields, ignoring names.
    pub fn values(&self) -> impl Iterator<Item = &Field> {
        self.iter().map(|(_, v)| v)
    }
}

/// The fields of an unnamed enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnnamedFields {
    pub(crate) fields: Vec<Field>,
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
}

/// The fields of an named enum variant or a struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedFields {
    pub(crate) fields: Vec<(Cow<'static, str>, Field)>,
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
}

#[derive(Debug, Clone)]
/// Builder for constructing [`DataType::Struct`] values.
pub struct StructBuilder<F = ()> {
    pub(crate) fields: F,
}

impl StructBuilder<NamedFields> {
    /// Add a named field.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.fields.fields.push((name.into(), field));
        self
    }

    /// Add a named field in-place.
    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: Field) {
        self.fields.fields.push((name.into(), field));
    }

    /// Finalize this builder into a [`DataType`].
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Named(self.fields),
            attributes: Default::default(),
        })
    }
}

impl StructBuilder<UnnamedFields> {
    /// Add an unnamed field.
    pub fn field(mut self, field: Field) -> Self {
        self.fields.fields.push(field);
        self
    }

    /// Add an unnamed field in-place.
    pub fn field_mut(&mut self, field: Field) {
        self.fields.fields.push(field);
    }

    /// Finalize this builder into a [`DataType`].
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatype::Primitive;

    fn named_fields(pairs: Vec<(&'static str, DataType)>) -> Fields {
        Fields::Named(NamedFields {
            fields: pairs.into_iter().map(|(k, ty)| (k.into(), Field::new(ty))).collect(),
            attributes: vec![],
        })
    }

    fn unnamed_fields(tys: Vec<DataType>) -> Fields {
        Fields::Unnamed(UnnamedFields {
            fields: tys.into_iter().map(Field::new).collect(),
            attributes: vec![],
        })
    }

    #[test]
    fn unit_iter_is_empty() {
        assert_eq!(Fields::Unit.iter().count(), 0);
        assert_eq!(Fields::Unit.keys().count(), 0);
        assert_eq!(Fields::Unit.values().count(), 0);
    }

    #[test]
    fn unnamed_keys_are_none() {
        let fields = unnamed_fields(vec![DataType::Primitive(Primitive::String)]);
        assert!(fields.keys().all(|k| k.is_none()));
    }

    #[test]
    fn named_keys_are_some() {
        let fields = named_fields(vec![("foo", DataType::Primitive(Primitive::String))]);
        assert!(fields.keys().all(|k| k.is_some()));
        assert_eq!(fields.keys().next().unwrap().unwrap().as_ref(), "foo");
    }

    #[test]
    fn values_match_field_types() {
        let ty = DataType::Primitive(Primitive::u32);
        let fields = unnamed_fields(vec![ty.clone()]);
        assert_eq!(fields.values().next().unwrap().ty(), Some(&ty));
    }

    #[test]
    fn iter_exact_size() {
        let fields = named_fields(vec![
            ("a", DataType::Primitive(Primitive::String)),
            ("b", DataType::Primitive(Primitive::bool)),
        ]);
        let it = fields.iter();
        assert_eq!(it.len(), 2);
    }
}
