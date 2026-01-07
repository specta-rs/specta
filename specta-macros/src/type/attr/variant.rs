use syn::Result;

use crate::utils::{AttrExtract, Attribute};

use super::RustCAttr;

#[derive(Default)]
pub struct VariantAttr {
    pub skip: bool,
    pub inline: bool,
    pub common: RustCAttr,
}

impl VariantAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self {
            common: RustCAttr::from_attrs(attrs)?,
            ..Default::default()
        };

        if let Some(attr) = attrs.extract("specta", "skip") {
            result.skip = attr.parse_bool().unwrap_or(true);
        } else if let Some(attr) = attrs.extract("serde", "skip") {
            // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
            // we make an exception for `#[serde(skip)]` because it's usually used on fields
            // that would fail a `T: Type` so handling it at runtime would prevent your code from compiling.
            result.skip = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool().unwrap_or(true);
        }

        Ok(result)
    }
}
