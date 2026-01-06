use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Result;

use crate::utils::{AttrExtract, Attribute};

use super::RustCAttr;

#[derive(Default)]
pub struct ContainerAttr {
    pub crate_name: Option<TokenStream>,
    pub inline: bool,
    pub remote: Option<TokenStream>,
    pub collect: Option<bool>,
    pub skip_attrs: Vec<String>,
    pub common: RustCAttr,

    // Struct only (we pass it anyway so enums get nice errors)
    pub transparent: bool,
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;

        if let Some(attr) = attrs.extract("specta", "crate") {
            result.crate_name = result
                .crate_name
                .take()
                .or(Some(attr.parse_path()?.to_token_stream()));
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "remote") {
            result.remote = result
                .remote
                .take()
                .or(Some(attr.parse_path()?.to_token_stream()));
        }

        if let Some(attr) = attrs.extract("specta", "collect") {
            result.collect = result
                .collect
                .take()
                .or(Some(attr.parse_bool().unwrap_or(true)));
        }

        for attr in attrs.extract_all("specta", "skip_attr") {
            result.skip_attrs.push(attr.parse_string()?);
        }

        if let Some(attr) = attrs.extract("specta", "transparent") {
            result.transparent = attr.parse_bool().unwrap_or(true);
        }

        // Handle `#[repr(transparent)]`
        if let Some(attr) = attrs.extract("repr", "transparent") {
            result.transparent = attr.parse_bool().unwrap_or(true);
        }

        Ok(result)
    }
}
