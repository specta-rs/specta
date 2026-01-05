use syn::Result;

use crate::utils::{Attribute, impl_parse};

use super::RustCAttr;

#[derive(Default)]
pub struct VariantAttr {
    pub skip: bool,
    pub inline: bool,
    pub common: RustCAttr,
}

impl_parse! {
    VariantAttr(attr, out) {
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
        "inline" => out.inline = attr.parse_bool().unwrap_or(true),
    }
}

impl VariantAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Ok(result)
    }
}
