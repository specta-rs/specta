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

    // Custom where clause bounds (None = automatic, Some(vec) = custom)
    pub bound: Option<Vec<syn::WherePredicate>>,
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self {
            common: RustCAttr::from_attrs(attrs)?,
            ..Default::default()
        };

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
        } else if let Some(attr) = attrs.extract("repr", "transparent") {
            result.transparent = attr.parse_bool().unwrap_or(true);
        } else if let Some(attr) = attrs.extract("serde", "transparent") {
            // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
            // we make an exception for `#[serde(transparent)]`.
            result.transparent = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "bound") {
            let bound_str = attr.parse_string()?;
            if bound_str.is_empty() {
                // Empty string means explicitly no automatic bounds
                result.bound = Some(Vec::new());
            } else {
                // Parse where predicates from string
                let where_clause_str = format!("where {}", bound_str);
                match syn::parse_str::<syn::WhereClause>(&where_clause_str) {
                    Ok(where_clause) => {
                        result.bound = Some(where_clause.predicates.into_iter().collect());
                    }
                    Err(e) => {
                        return Err(syn::Error::new(
                            attr.value_span(),
                            format!("Failed to parse bound attribute: {}", e),
                        ));
                    }
                }
            }
        }

        Ok(result)
    }
}
