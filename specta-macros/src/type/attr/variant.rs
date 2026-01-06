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
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;

        if let Some(attr) = attrs.extract("specta", "skip") {
            result.skip = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool().unwrap_or(true);
        }

        Ok(result)
    }
}
