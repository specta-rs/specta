use syn::{Result, Type};

use super::{AttrExtract, Attribute};

use super::RustCAttr;

#[derive(Default)]
pub(crate) struct FieldAttr {
    pub(crate) r#type: Option<Type>,
    pub(crate) inline: bool,
    pub(crate) skip: bool,
    pub(crate) serde_skip: bool,
    pub(crate) serde_newtype_skip_ignored: bool,
    pub(crate) optional: bool,
    pub(crate) common: RustCAttr,
}

impl FieldAttr {
    pub(crate) fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self {
            common: RustCAttr::from_attrs(attrs)?,
            ..Default::default()
        };

        if let Some(attr) = attrs.extract("specta", "type") {
            result.r#type = result.r#type.take().or(Some(attr.parse_type()?));
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool_or_true()?;
        }

        if let Some(attr) = attrs.extract("specta", "skip") {
            result.skip = attr.parse_bool_or_true()?;
        } else if let Some(attr) = attrs.extract("serde", "skip") {
            // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
            // we make an exception for `#[serde(skip)]` because it's usually used on fields
            // that would fail a `T: Type` so handling it at runtime would prevent your code from compiling.
            result.skip = attr.parse_bool_or_true()?;
            result.serde_skip = result.skip;
        }

        for attr in ["skip_serializing", "skip_deserializing"] {
            if let Some(value) = attrs.extract("serde", attr) {
                result.serde_skip |= value.parse_bool_or_true()?;
            }
        }

        if let Some(attr) = attrs.extract("specta", "optional") {
            result.optional = attr.parse_bool_or_true()?;
        }

        if let Some(attr) = attrs.extract("specta", "default") {
            result.optional = attr.parse_bool_or_true()?;
        }

        Ok(result)
    }
}
