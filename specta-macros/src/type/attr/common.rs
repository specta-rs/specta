use std::borrow::Cow;

use quote::quote;
use syn::{Lit, Result};

use crate::utils::{Attribute, AttributeValue};

#[derive(Clone)]
#[non_exhaustive]
pub enum DeprecatedType {
    /// A type that has been deprecated without a message.
    ///
    /// Eg. `#[deprecated]`
    Deprecated,
    /// A type that has been deprecated with a message and an optional `since` version.
    ///
    /// Eg. `#[deprecated = "Use something else"]` or `#[deprecated(since = "1.0.0", message = "Use something else")]`
    #[non_exhaustive]
    DeprecatedWithSince {
        since: Option<Cow<'static, str>>,
        note: Cow<'static, str>,
    },
}

#[derive(Default, Clone)]
pub struct CommonAttr {
    pub doc: String,
    pub deprecated: Option<DeprecatedType>,
}

impl CommonAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let doc = attrs.iter().filter(|attr| attr.key == "doc").try_fold(
            String::new(),
            |mut s, doc| {
                let doc = doc.parse_string()?;
                if !s.is_empty() {
                    s.push_str("\n");
                }
                s.push_str(&doc);
                Ok(s) as syn::Result<_>
            },
        )?;

        let mut deprecated = None;
        if let Some(attr_value) = attrs.iter().filter(|attr| attr.key == "deprecated").next() {
            match &attr_value.value {
                Some(AttributeValue::Lit(lit)) => {
                    deprecated = Some(DeprecatedType::DeprecatedWithSince {
                        since: None,
                        note: match lit {
                            Lit::Str(s) => s.value().into(),
                            _ => return Err(syn::Error::new_spanned(lit, "expected string")),
                        },
                    });
                }
                Some(AttributeValue::Path(_)) => {
                    unreachable!("deprecated attribute can't be a path!")
                }
                Some(AttributeValue::Attribute { attr, .. }) => {
                    let since = attr
                        .iter()
                        .filter(|attr| attr.key == "since")
                        .next()
                        .and_then(|v| v.value.as_ref())
                        .and_then(|v| match v {
                            AttributeValue::Lit(lit) => Some(lit),
                            _ => None, // TODO: This should probs be an error
                        })
                        .and_then(|lit| match lit {
                            syn::Lit::Str(s) => Some(s.value()),
                            _ => None, // TODO: This should probs be an error
                        });

                    let note = attr
                        .iter()
                        .filter(|attr| attr.key == "note")
                        .next()
                        .and_then(|v| match v.value.as_ref() {
                            Some(AttributeValue::Lit(lit)) => Some(lit),
                            _ => None, // TODO: This should probs be an error
                        })
                        .and_then(|lit| match lit {
                            syn::Lit::Str(s) => Some(s.value()),
                            _ => None, // TODO: This should probs be an error
                        })
                        .unwrap_or_default();

                    deprecated = Some(DeprecatedType::DeprecatedWithSince {
                        // TODO: Use Cow's earlier rather than later
                        since: since.map(Into::into),
                        note: note.into(),
                    });
                }
                None => deprecated = Some(DeprecatedType::Deprecated),
            }
        };

        Ok(CommonAttr { doc, deprecated })
    }

    pub fn deprecated_as_tokens(&self) -> proc_macro2::TokenStream {
        match &self.deprecated {
            Some(DeprecatedType::Deprecated) => {
                quote!(Some(datatype::DeprecatedType::Deprecated))
            }
            Some(DeprecatedType::DeprecatedWithSince { since, note }) => {
                let since = since
                    .as_ref()
                    .map(|v| quote!(#v.into()))
                    .unwrap_or(quote!(None));

                quote!(Some(datatype::DeprecatedType::DeprecatedWithSince {
                    since: #since,
                    note: #note.into(),
                }))
            }
            None => quote!(None),
        }
    }
}
