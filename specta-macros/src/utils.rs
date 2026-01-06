use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Ident, Lit, Meta, Path, Result, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
};

#[derive(Clone)]
pub enum AttributeValue {
    /// Literal value. Eg. `#[specta(name = "hello")]` or `#[specta(name = u32)]`
    Lit(Lit),
    /// Path value. Eg. `#[specta(type = String)]` or `#[specta(type = ::std::string::String)]`
    /// Path doesn't follow the Rust spec hence the need for this custom parser. We are doing this anyway for backwards compatibility.
    Path(Path),
    /// A nested attribute. Eg. the `deprecated(note = "some note") in `#[specta(deprecated(note = "some note"))]`
    Attribute { span: Span, attr: Vec<Attribute> },
}

impl AttributeValue {
    fn span(&self) -> Span {
        match self {
            Self::Lit(lit) => lit.span(),
            Self::Path(path) => path.span(),
            Self::Attribute { span, .. } => *span,
        }
    }
}

impl Parse for AttributeValue {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(match input.peek(Lit) {
            true => Self::Lit(input.parse()?),
            false => Self::Path(input.parse()?),
        })
    }
}

#[derive(Clone)]
pub struct Attribute {
    /// Source of the attribute. Eg. `specta`, `serde`, `repr`, `deprecated`, etc.
    pub source: String,
    /// Key of the current item. Eg. `specta` or `type`in `#[specta(type = String)]`
    pub key: Ident,
    /// Value of the item. Eg. `String` in `#[specta(type = String)]`
    pub value: Option<AttributeValue>,
}

impl Attribute {
    /// Span of they value. Eg. `String` in `#[specta(type = String)]`
    /// Will fallback to the key span if no value is present.
    pub fn value_span(&self) -> Span {
        match &self.value {
            Some(v) => v.span(),
            None => self.key.span(),
        }
    }

    pub fn parse_string(&self) -> Result<String> {
        match &self.value {
            Some(AttributeValue::Lit(Lit::Str(string))) => Ok(string.value()),
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected string literal. Eg. `\"somestring\"`",
            )),
        }
    }

    pub fn parse_bool(&self) -> Result<bool> {
        match &self.value {
            Some(AttributeValue::Lit(Lit::Bool(b))) => Ok(b.value()),
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected boolean literal. Eg. `true` or `false`",
            )),
        }
    }

    pub fn parse_path(&self) -> Result<Path> {
        match &self.value {
            Some(AttributeValue::Path(path)) => Ok(path.clone()),
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected path. Eg. `String` or `std::string::String`",
            )),
        }
    }
}

pub trait AttrExtract {
    fn extract(&self, source: &str, key: &str) -> Option<&Attribute>;
    fn extract_all(&self, source: &str, key: &str) -> Vec<&Attribute>;
}

impl AttrExtract for Vec<Attribute> {
    fn extract(&self, source: &str, key: &str) -> Option<&Attribute> {
        self.iter()
            .find(|attr| attr.source == source && attr.key == key)
    }

    fn extract_all(&self, source: &str, key: &str) -> Vec<&Attribute> {
        self.iter()
            .filter(|attr| attr.source == source && attr.key == key)
            .collect()
    }
}

struct NestedAttributeList {
    attrs: Vec<Attribute>,
}

impl Parse for NestedAttributeList {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Vec::new();
        while !input.is_empty() {
            let key = input.call(Ident::parse_any)?;
            let key_span = key.span();

            attrs.push(Attribute {
                source: String::new(), // Will be updated by caller
                key,
                value: match false {
                    _ if input.peek(Paren) => Some(AttributeValue::Attribute {
                        span: key_span,
                        attr: input.parse::<NestedAttributeList>()?.attrs,
                    }),
                    _ if input.peek(Token![=]) => {
                        input.parse::<Token![=]>()?;
                        Some(input.parse()?)
                    }
                    _ => None,
                },
            });

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(NestedAttributeList { attrs })
    }
}

/// pass all of the attributes into a single structure.
/// We can then remove them from the struct while passing an any left over must be invalid and an error can be thrown.
pub fn parse_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<Attribute>> {
    parse_attrs_with_filter(attrs, &[])
}

/// Same as `parse_attrs` but allows skipping attributes by name.
/// This is useful for skipping attributes that may have non-standard syntax that we can't parse.
pub fn parse_attrs_with_filter(
    attrs: &[syn::Attribute],
    skip_attrs: &[String],
) -> syn::Result<Vec<Attribute>> {
    let mut result = Vec::new();

    for attr in attrs {
        let ident = attr
            .path()
            .segments
            .last()
            .expect("Attribute path must have at least one segment")
            .clone()
            .ident;

        // Skip attributes that are in the skip list
        let attr_name = ident.to_string();
        if skip_attrs.contains(&attr_name) {
            continue;
        }

        result.append(&mut match &attr.meta {
            Meta::Path(_) => vec![Attribute {
                source: attr_name.clone(),
                key: ident.clone(),
                value: None,
            }],
            Meta::List(meta) => {
                let source = attr_name.clone();
                let mut parsed: Vec<Attribute> =
                    syn::parse2::<NestedAttributeList>(meta.tokens.clone())?.attrs;
                for a in &mut parsed {
                    a.source = source.clone();
                }
                vec![Attribute {
                    source,
                    key: ident.clone(),
                    value: Some(AttributeValue::Attribute {
                        span: ident.span(),
                        attr: parsed,
                    }),
                }]
            }
            Meta::NameValue(meta) => {
                let source = attr_name.clone();
                let mut parsed: Vec<Attribute> =
                    syn::parse2::<NestedAttributeList>(meta.to_token_stream().clone())?.attrs;
                for a in &mut parsed {
                    a.source = source.clone();
                }
                parsed
            }
        });
    }

    Ok(result)
}

pub fn unraw_raw_ident(ident: &Ident) -> String {
    let ident = ident.to_string();
    if ident.starts_with("r#") {
        ident.trim_start_matches("r#").to_owned()
    } else {
        ident
    }
}

#[cfg(feature = "DO_NOT_USE_function")]
pub fn format_fn_wrapper(function: &Ident) -> Ident {
    quote::format_ident!("__specta__fn__{}", function)
}
