use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Result, Type, TypePath};

use crate::utils::Attribute;

#[derive(Default)]
pub struct FieldAttr {
    pub rename: Option<TokenStream>,
    pub r#type: Option<Type>,
    pub inline: bool,
    pub skip: bool,
    pub optional: bool,
    pub flatten: bool,
    pub doc: String,
}

impl_parse! {
    FieldAttr(attr, out) {
        "rename" => {
            let attr = attr.parse_string()?;
            out.rename = out.rename.take().or_else(|| Some(
                attr.to_token_stream()
            ))
        },
        "rename_from_path" => {
            let attr = attr.parse_path()?;
            out.rename = out.rename.take().or_else(|| Some({
                let expr = attr.to_token_stream();
                quote::quote!( #expr )
            }))
        },
        "type" => out.r#type = out.r#type.take().or(Some(Type::Path(TypePath {
            qself: None,
            path: attr.parse_path()?,
        }))),
        "inline" => out.inline = attr.parse_bool().unwrap_or(true),
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
        "skip_serializing" => out.skip = true,
        "skip_deserializing" => out.skip = true,
        "skip_serializing_if" => out.optional = attr.parse_string()? == *"Option::is_none",
        // Specta only attribute
        "optional" => out.optional = attr.parse_bool().unwrap_or(true),
        "default" => out.optional = attr.parse_bool().unwrap_or(true),
        "flatten" => out.flatten = attr.parse_bool().unwrap_or(true),
        "doc" => {
            if attr.key == "doc" {
                if !out.doc.is_empty() {
                    out.doc.push_str("\n");
                }

                out.doc.push_str(&attr.parse_string()?);
            }
        },
    }
}

impl FieldAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        Self::try_from_attrs("specta", attrs, &mut result)?;
        #[cfg(feature = "serde")]
        Self::try_from_attrs("serde", attrs, &mut result)?;
        Self::try_from_attrs("doc", attrs, &mut result)?;
        Ok(result)
    }
}
