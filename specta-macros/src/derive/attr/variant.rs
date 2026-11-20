use syn::{Result, Type};

use super::{AttrExtract, Attribute};

use super::RustCAttr;

#[derive(Default)]
pub(crate) struct VariantAttr {
    pub(crate) r#type: Option<Type>,
    pub(crate) skip: bool,
    pub(crate) inline: bool,
    pub(crate) common: RustCAttr,
}

impl VariantAttr {
    pub(crate) fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self {
            common: RustCAttr::from_attrs(attrs)?,
            ..Default::default()
        };

        if let Some(attr) = attrs.extract("specta", "skip") {
            result.skip = attr.parse_bool_or_true()?;
        } else if let Some(attr) = attrs.extract("serde", "skip") {
            // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
            // we make an exception for `#[serde(skip)]` because it's usually used on fields
            // that would fail a `T: Type` so handling it at runtime would prevent your code from compiling.
            result.skip = attr.parse_bool_or_true()?;
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool_or_true()?;
        }

        if let Some(attr) = attrs
            .extract("specta", "type")
            .or_else(|| attrs.extract("specta", "r#type"))
        {
            result.r#type = result.r#type.take().or(Some(attr.parse_type()?));
        }

        Ok(result)
    }
}
