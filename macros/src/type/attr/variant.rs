use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{Attribute, Inflection};

use super::CommonAttr;

#[derive(Default)]
pub struct VariantAttr {
    pub rename_all: Option<Inflection>,
    pub rename: Option<TokenStream>,
    pub skip: bool,
    pub inline: bool,
    pub common: CommonAttr,
}

impl_parse! {
    VariantAttr(attr, out) {
        "rename_all" => out.rename_all = out.rename_all.take().or(Some(attr.parse_inflection()?)),
        "rename" => out.rename = out.rename.take().or(Some(attr.parse_string()?.to_token_stream())),
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
        "skip_serializing" => out.skip = true,
        "skip_deserializing" => out.skip = true,
        "inline" => out.inline = attr.parse_bool().unwrap_or(true),
    }
}

impl VariantAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = CommonAttr::from_attrs(attrs)?;
        Self::try_from_attrs("specta", attrs, &mut result)?;
        #[cfg(feature = "serde")]
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Ok(result)
    }
}
