use syn::Result;

use crate::utils::{Attribute, Inflection};

#[derive(Default)]
pub struct VariantAttr {
    pub rename_all: Option<Inflection>,
    pub rename: Option<String>,
    pub skip: bool,
}

impl_parse! {
    VariantAttr(attr, out) {
        "rename_all" => out.rename_all = out.rename_all.take().or(Some(attr.parse_inflection()?)),
        "rename" => out.rename = out.rename.take().or(Some(attr.parse_string()?)),
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
        "skip_serializing" => out.skip = true,
        "skip_deserializing" => out.skip = true,
    }
}

impl VariantAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        Self::try_from_attrs("specta", attrs, &mut result)?;
        #[cfg(feature = "serde")]
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Ok(result)
    }
}
