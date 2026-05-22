//! Field types are used by both enums and structs.

use crate::datatype::Struct;

use super::{Attributes, DataType, Deprecated};
use std::borrow::Cow;

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
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.fields.fields.push((name.into(), field));
        self
    }

    /// Adds a named field in-place.
    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: Field) {
        self.fields.fields.push((name.into(), field));
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
    pub fn field(mut self, field: Field) -> Self {
        self.fields.fields.push(field);
        self
    }

    /// Adds an unnamed field in-place.
    pub fn field_mut(&mut self, field: Field) {
        self.fields.fields.push(field);
    }

    /// Finalizes this builder into a [`DataType::Struct`] with unnamed fields.
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
        })
    }

    fn unnamed_fields(tys: Vec<DataType>) -> Fields {
        Fields::Unnamed(UnnamedFields {
            fields: tys.into_iter().map(Field::new).collect(),
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
        let fields = unnamed_fields(vec![DataType::Primitive(Primitive::str)]);
        assert!(fields.keys().all(|k| k.is_none()));
    }

    #[test]
    fn named_keys_are_some() {
        let fields = named_fields(vec![("foo", DataType::Primitive(Primitive::str))]);
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
            ("a", DataType::Primitive(Primitive::str)),
            ("b", DataType::Primitive(Primitive::bool)),
        ]);
        let it = fields.iter();
        assert_eq!(it.len(), 2);
    }
}
