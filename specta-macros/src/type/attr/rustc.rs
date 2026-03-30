use std::borrow::Cow;

use quote::quote;
use syn::{Lit, Result};

use crate::utils::{Attribute, AttributeValue};

// Copy of `specta/src/datatype/named.rs`
pub struct Deprecated {
    note: Option<Cow<'static, str>>,
    since: Option<Cow<'static, str>>,
}

impl Deprecated {
    pub const fn new() -> Self {
        Self {
            note: None,
            since: None,
        }
    }

    pub fn with_note(note: Cow<'static, str>) -> Self {
        Self {
            note: Some(note),
            since: None,
        }
    }

    pub fn with_since_note(since: Option<Cow<'static, str>>, note: Cow<'static, str>) -> Self {
        Self {
            note: Some(note),
            since,
        }
    }

    pub fn note(&self) -> Option<&Cow<'static, str>> {
        self.note.as_ref()
    }

    pub fn since(&self) -> Option<&Cow<'static, str>> {
        self.since.as_ref()
    }
}

#[derive(Default)]
pub struct RustCAttr {
    pub doc: String,
    pub deprecated: Option<Deprecated>,
}

impl RustCAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let doc = attrs.extract_if(.., |attr| attr.key == "doc").try_fold(
            String::new(),
            |mut s, doc| {
                let doc = doc.parse_string()?;
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(&doc);
                Ok(s) as syn::Result<_>
            },
        )?;

        let mut deprecated = None;
        if let Some(pos) = attrs.iter().position(|attr| attr.key == "deprecated") {
            let attr_value = attrs[pos].clone();

            match &attr_value.value {
                Some(AttributeValue::Lit(lit)) => {
                    deprecated = Some(Deprecated::with_note(match lit {
                        Lit::Str(s) => s.value().into(),
                        _ => return Err(syn::Error::new_spanned(lit, "expected string")),
                    }));
                }
                Some(AttributeValue::Path(_)) => {
                    unreachable!("deprecated attribute can't be a path!")
                }
                Some(AttributeValue::Expr(_)) => {
                    unreachable!("deprecated attribute can't be an expression!")
                }
                Some(AttributeValue::Attribute { attr, .. }) => {
                    let since = attr
                        .iter()
                        .find(|attr| attr.key == "since")
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
                        .find(|attr| attr.key == "note")
                        .and_then(|v| match v.value.as_ref() {
                            Some(AttributeValue::Lit(lit)) => Some(lit),
                            _ => None, // TODO: This should probs be an error
                        })
                        .and_then(|lit| match lit {
                            syn::Lit::Str(s) => Some(s.value()),
                            _ => None, // TODO: This should probs be an error
                        })
                        .unwrap_or_default();

                    deprecated = Some(Deprecated::with_since_note(
                        // TODO: Use Cow's earlier rather than later
                        since.map(Into::into),
                        note.into(),
                    ));
                }
                None => deprecated = Some(Deprecated::new()),
            }

            attrs.swap_remove(pos);
        };

        Ok(RustCAttr { doc, deprecated })
    }

    pub fn deprecated_as_tokens(&self) -> proc_macro2::TokenStream {
        match &self.deprecated {
            Some(deprecated) => {
                let since = deprecated
                    .since()
                    .map(|v| quote!(#v.into()))
                    .unwrap_or(quote!(None));

                let note = match deprecated.note() {
                    Some(note) => quote!(Some(#note.into())),
                    None => quote!(None),
                };

                quote!({
                    let mut deprecated = datatype::Deprecated::new();
                    deprecated.set_since(#since);
                    deprecated.set_note(#note);
                    Some(deprecated)
                })
            }
            None => quote!(None),
        }
    }
}
