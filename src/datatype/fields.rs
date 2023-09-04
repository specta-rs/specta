//! Field types are used by both enums and structs.

use std::borrow::Cow;

use crate::DataType;

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub(crate) optional: bool,
    pub(crate) flatten: bool,
    pub(crate) ty: DataType,
}

impl Field {
    pub fn optional(&self) -> bool {
        self.optional
    }

    pub fn flatten(&self) -> bool {
        self.flatten
    }

    pub fn ty(&self) -> &DataType {
        &self.ty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnnamedFields {
    pub(crate) fields: Vec<Field>,
}

impl UnnamedFields {
    /// A list of fields for the current type.
    pub fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

/// The fields for a [StructType] or the anonymous struct declaration in an [EnumVariant].
///
/// Eg.
/// ```rust
/// // This whole thing is a [StructFields::Named]
/// pub struct Demo {
///     a: String
/// }
///
/// pub enum Demo2 {
///     A { a: String } // This variant is a [EnumVariant::Named]
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NamedFields {
    pub(crate) fields: Vec<(Cow<'static, str>, Field)>,
    pub(crate) tag: Option<Cow<'static, str>>,
}

impl NamedFields {
    /// A list of fields in the format (name, [StructField]).
    pub fn fields(&self) -> &Vec<(Cow<'static, str>, Field)> {
        &self.fields
    }

    /// Serde tag for the current field.
    pub fn tag(&self) -> &Option<Cow<'static, str>> {
        &self.tag
    }
}
