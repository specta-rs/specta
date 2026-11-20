use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Lit, Result};

use super::{Attribute, AttributeValue};

// Copy of `specta/src/datatype/named.rs`
#[derive(Default)]
pub(crate) struct Deprecated {
    note: Option<Cow<'static, str>>,
    since: Option<Cow<'static, str>>,
}

#[derive(Default)]
pub(crate) struct RustCAttr {
    pub(crate) doc: String,
    pub(crate) deprecated: Option<Deprecated>,
}

impl RustCAttr {
    pub(crate) fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let doc = attrs
            .extract_if(.., |attr| {
                attr.key == "doc" && matches!(attr.value, Some(AttributeValue::Lit(Lit::Str(_))))
            })
            .try_fold(String::new(), |mut s, doc| {
                let doc = doc.parse_string()?;
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(&doc);
                Ok(s) as syn::Result<_>
            })?;

        let mut deprecated = None;
        if let Some(pos) = attrs.iter().position(|attr| attr.key == "deprecated") {
            let attr_value = attrs[pos].clone();

            match &attr_value.value {
                Some(AttributeValue::Lit(lit)) => {
                    deprecated = Some(Deprecated {
                        note: Some(match lit {
                            Lit::Str(s) => s.value().into(),
                            _ => return Err(syn::Error::new_spanned(lit, "expected string")),
                        }),
                        since: None,
                    });
                }
                Some(AttributeValue::Path(_)) => {
                    unreachable!("deprecated attribute can't be a path!")
                }
                Some(AttributeValue::Expr(_)) | Some(AttributeValue::Type(_)) => {
                    unreachable!("deprecated attribute can't be an expression or type!")
                }
                Some(AttributeValue::Attribute { attr, .. }) => {
                    deprecated = Some(Deprecated {
                        note: Some(parse_deprecated_string_attr(attr, "note")?.unwrap_or_default()),
                        since: parse_deprecated_string_attr(attr, "since")?,
                    });
                }
                None => deprecated = Some(Deprecated::default()),
            }

            attrs.swap_remove(pos);
        };

        Ok(RustCAttr { doc, deprecated })
    }
}

fn parse_deprecated_string_attr(
    attrs: &[Attribute],
    key: &str,
) -> Result<Option<Cow<'static, str>>> {
    let Some(attr) = attrs.iter().find(|attr| attr.key == key) else {
        return Ok(None);
    };

    match &attr.value {
        Some(AttributeValue::Lit(syn::Lit::Str(s))) => Ok(Some(Cow::Owned(s.value()))),
        _ => Err(syn::Error::new(
            attr.value_span(),
            format!("specta: deprecated `{key}` must be a string literal"),
        )),
    }
}

pub(crate) fn deprecated_as_tokens(Deprecated { note, since }: Deprecated) -> TokenStream {
    let since = since
        .map(|v| quote!(Some(#v.into())))
        .unwrap_or(quote!(None));

    let note = match note {
        Some(note) => quote!(Some(#note.into())),
        None => quote!(None),
    };

    quote!(Some(datatype::Deprecated::with_since_note(#since, #note.unwrap_or_default())))
}
