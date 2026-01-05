use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{Attribute, impl_parse};

use super::RustCAttr;

#[derive(Default)]
pub struct ContainerAttr {
    pub crate_name: Option<TokenStream>,
    pub inline: bool,
    pub remote: Option<TokenStream>,
    pub collect: Option<bool>,
    pub common: RustCAttr,

    // Struct only (we pass it anyway so enums get nice errors)
    pub transparent: bool,
}

impl_parse! {
    ContainerAttr(attr, out) {
        "crate" => {
            out.crate_name = out.crate_name.take().or(Some(attr.parse_path()?.to_token_stream()));
        },
        "inline" => out.inline = attr.parse_bool().unwrap_or(true),
        "remote" => out.remote = out.remote.take().or(Some(attr.parse_path()?.to_token_stream())),
        "collect" => out.collect = out.collect.take().or(Some(attr.parse_bool().unwrap_or(true))),
        "transparent" => out.transparent = attr.parse_bool().unwrap_or(true),
    }
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Self::try_from_attrs("repr", attrs, &mut result)?; // To handle `#[repr(transparent)]`
        Ok(result)
    }
}
