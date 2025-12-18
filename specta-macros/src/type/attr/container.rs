use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{Attribute, Inflection, impl_parse};

use super::CommonAttr;

#[derive(Default, Clone)]
pub struct ContainerAttr {
    pub rename_all: Option<Inflection>,
    pub rename: Option<TokenStream>,
    pub tag: Option<String>,
    pub crate_name: Option<TokenStream>,
    pub inline: bool,
    pub remote: Option<TokenStream>,
    pub collect: Option<bool>,
    pub common: CommonAttr,

    // Struct ony (we pass it anyway so enums get nice errors)
    pub transparent: bool,
}

impl_parse! {
    ContainerAttr(attr, out) {
        "rename_all" => out.rename_all = out.rename_all.take().or(Some(attr.parse_inflection()?)),
        "rename" => {
            let attr = attr.parse_string()?;
            out.rename = out.rename.take().or_else(|| Some(attr.to_token_stream()))
        },
        "rename_from_path" => {
            let attr = attr.parse_path()?;
            out.rename = out.rename.take().or_else(|| Some({
                let expr = attr.to_token_stream();
                quote::quote!( #expr )
            }))
        },
        "tag" => out.tag = out.tag.take().or(Some(attr.parse_string()?)),
        "crate" => {
            // if attr.key == "specta" { // TODO: Fix this check
                out.crate_name = out.crate_name.take().or(Some(attr.parse_path()?.to_token_stream()));
            // }
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
        result.common = CommonAttr::from_attrs(attrs)?;
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Self::try_from_attrs("repr", attrs, &mut result)?; // To handle `#[repr(transparent)]`
        Ok(result)
    }
}
