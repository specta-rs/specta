use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{impl_parse, Attribute};

#[derive(Default)]
pub struct FieldAttr {
    pub skip: bool,
    pub rename: Option<TokenStream>,
}

impl_parse! {
    FieldAttr(attr, out) {
        "skip" => out.skip = true,
        "rename" => {
            let attr = attr.parse_string()?;
            out.rename = out.rename.take().or_else(|| Some(
                attr.to_token_stream()
            ))
        },
    }
}

impl FieldAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Ok(result)
    }
}
