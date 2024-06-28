use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{impl_parse, Attribute};

#[derive(Default)]
pub struct ContainerAttr {
    pub crate_name: Option<TokenStream>,
}

impl_parse! {
    ContainerAttr(attr, out) {
        "crate" => out.crate_name = out.crate_name.take().or(Some(attr.parse_path()?.into_token_stream())),
    }
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Ok(result)
    }
}
