//! Field types are used by both enums and structs.

use std::borrow::Cow;

use super::{DataType, DeprecatedType};

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
}

impl Field {
    pub fn optional(&self) -> bool {
        self.optional
    }

    pub fn flatten(&self) -> bool {
        self.flatten
    }

    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    pub fn inline(&self) -> bool {
        self.inline
    }

    pub fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
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
