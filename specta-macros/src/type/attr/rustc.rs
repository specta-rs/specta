use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Lit, Result};

use crate::utils::{Attribute, AttributeValue};

// Copy of `specta/src/datatype/named.rs`
#[derive(Clone)]
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
}

#[derive(Default)]
pub struct RustCAttr {
    pub doc: String,
    pub deprecated: Option<Deprecated>,
}

impl RustCAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
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
                    let since = parse_deprecated_string_attr(attr, "since")?;
                    let note = parse_deprecated_string_attr(attr, "note")?.unwrap_or_default();

                    deprecated = Some(Deprecated::with_since_note(since, note));
                }
                None => deprecated = Some(Deprecated::new()),
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

/// `dt` is the path the generated code uses for the `datatype` module.
pub(crate) fn deprecated_as_tokens(
    dt: &syn::Ident,
    Deprecated { note, since }: Deprecated,
) -> TokenStream {
    let since = since
        .map(|v| quote!(Some(#v.into())))
        .unwrap_or(quote!(None));

    // `with_since_note` takes a `Cow`, so an empty note is an empty `&str`
    // converted into one, without needing `Cow` in the generated scope. The
    // empty case interpolates a `&str` rather than writing a literal in the
    // template, which `quote!` would have to lex.
    let empty = "";
    let note = note
        .map(|note| quote!(#note.into()))
        .unwrap_or(quote!(#empty.into()));

    quote!(Some(#dt::Deprecated::with_since_note(#since, #note)))
}

/// The same value as [`deprecated_as_tokens`], as a `const` expression.
///
/// The generated code keeps it in a `static` and passes a reference to it, so a
/// `definition` body does not build an owned `Option<Deprecated>` temporary (and
/// the drop flag, drop glue and cleanup path that come with it) once per
/// monomorphisation. `Into::into` is not callable in a const context, so the
/// `Cow` is built directly.
pub(crate) fn deprecated_as_const_tokens(
    dt: &syn::Ident,
    Deprecated { note, since }: Deprecated,
) -> TokenStream {
    let since = since
        .map(|v| quote!(::core::option::Option::Some(::std::borrow::Cow::Borrowed(#v))))
        .unwrap_or(quote!(::core::option::Option::None));

    let empty = "";
    let note = note
        .map(|note| quote!(::std::borrow::Cow::Borrowed(#note)))
        .unwrap_or(quote!(::std::borrow::Cow::Borrowed(#empty)));

    quote!(#dt::Deprecated::with_since_note(#since, #note))
}
