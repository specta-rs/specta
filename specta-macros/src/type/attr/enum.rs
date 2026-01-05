use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, Result};

use crate::utils::{Attribute, impl_parse};

use super::ContainerAttr;

#[derive(Default)]
pub struct EnumAttr {
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: Option<bool>,
}

impl_parse! {
    EnumAttr(attr, out) {
        "tag" => out.tag = out.tag.take().or(Some(attr.parse_string()?)),
        "content" => out.content = out.content.take().or(Some(attr.parse_string()?)),
        "untagged" => out.untagged = Some(attr.parse_bool().unwrap_or(true)),
    }
}

impl EnumAttr {
    pub fn from_attrs(
        _container_attrs: &ContainerAttr,
        attrs: &mut Vec<Attribute>,
    ) -> Result<Self> {
        let mut result = Self::default();

        Self::try_from_attrs("specta", attrs, &mut result)?;
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Ok(result)
    }
}
