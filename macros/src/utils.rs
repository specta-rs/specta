use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
    Ident, Lit, Path, Result, Token,
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

    pub fn parse_inflection(&self) -> Result<Inflection> {
        match &self.value {
            Some(AttributeValue::Lit(Lit::Str(lit))) => {
                Ok(match lit.value().to_lowercase().replace('_', "").as_str() {
                    "lowercase" => Inflection::Lower,
                    "uppercase" => Inflection::Upper,
                    "camelcase" => Inflection::Camel,
                    "snakecase" => Inflection::Snake,
                    "pascalcase" => Inflection::Pascal,
                    "screamingsnakecase" => Inflection::ScreamingSnake,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            lit,
                            "specta: found string literal containing an unsupported inflection",
                        ))
                    }
                })
            }
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected string literal containing an inflection",
            )),
        }
    }
}

fn parse_attribute(input: ParseStream) -> Result<Vec<Attribute>> {
    // (demo = "hello")
    // ^              ^
    let content_owned;
    let content;
    if input.peek(Paren) {
        parenthesized!(content_owned in input);
        content = &content_owned;
    } else {
        content = input;
    }

    let mut result = Vec::new();
    while !content.is_empty() {
        // (demo = "hello")
        //  ^^^^
        let key = content.call(Ident::parse_any)?;
        let key_span = key.span();

        result.push(Attribute {
            key,
            value: match false {
                // `(demo(...))`
                //       ^^^^^
                _ if content.peek(Paren) => Some(AttributeValue::Attribute {
                    span: key_span,
                    attr: parse_attribute(content)?,
                }),
                // `(demo = "hello")`
                //        ^^^^^^^^^
                _ if content.peek(Token![=]) => {
                    content.parse::<Token![=]>()?;
                    Some(AttributeValue::parse(content)?)
                }
                // `(demo)`
                _ => None,
            },
        });

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    Ok(result)
}

/// pass all of the attributes into a single structure.
/// We can then remove them from the struct while passing an any left over must be invalid and an error can be thrown.
pub fn parse_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<Attribute>> {
    let mut result = Vec::new();

    for attr in attrs {
        let ident = attr
            .path
            .segments
            .last()
            .expect("Attribute path must have at least one segment")
            .clone()
            .ident;

        // TODO: We should somehow build this up from the macro output automatically -> if not our attribute parser is applied to stuff like `allow` and that's bad.
        if !(ident == "specta"
            || ident == "serde"
            || ident == "doc"
            || ident == "repr"
            || ident == "deprecated")
        {
            continue;
        }

        let parser = |input: ParseStream| {
            Ok(Attribute {
                key: ident.clone(),
                value: match false {
                    // `#[demo]`
                    _ if input.is_empty() => None,
                    // `#[demo = "todo"]`
                    _ if input.peek(Token![=]) => {
                        input.parse::<Token![=]>()?;
                        Some(AttributeValue::parse(input)?)
                    }
                    // `#[demo(...)]`
                    _ => Some(AttributeValue::Attribute {
                        span: ident.span(),
                        attr: parse_attribute(input)?,
                    }),
                },
            })
        };
        let attr = syn::parse::Parser::parse2(parser, attr.tokens.clone().into())?;
        result.push(attr);
    }

    Ok(result)
}

macro_rules! impl_parse {
    ($i:ident ($attr_parser:ident, $out:ident) { $($k:pat => $e:expr),* $(,)? }) => {
        impl $i {
            fn try_from_attrs(
                ident: &'static str,
                attrs: &mut Vec<crate::utils::Attribute>,
                $out: &mut Self,
            ) -> syn::Result<()> {
                // Technically we can have multiple root-level attributes
                // Eg. `#[specta(...)]` can exist multiple times on a single type
                for attr in attrs.iter_mut().filter(|attr| attr.key == ident) {
                    match &mut attr.value {
                        Some($crate::utils::AttributeValue::Attribute { attr, .. }) => {
                            *attr = std::mem::take(attr)
                                .into_iter()
                                .map(|$attr_parser| {
                                    let mut was_passed_by_user = true;

                                    match $attr_parser.key.to_string().as_str() {
                                        $($k => $e,)*
                                        #[allow(unreachable_patterns)]
                                        _ => {
                                            was_passed_by_user = false;
                                        }
                                    }

                                    Ok(($attr_parser, was_passed_by_user))
                                })
                                .collect::<syn::Result<Vec<(Attribute, bool)>>>()?
                                .into_iter()
                                .filter_map(
                                    |(attr, was_passed_by_user)| {
                                        if was_passed_by_user {
                                            None
                                        } else {
                                            Some(attr)
                                        }
                                    },
                                )
                                .collect();
                        }
                        _ => {}
                    }
                }

                Ok(())
            }
        }
    };
}

pub fn unraw_raw_ident(ident: &Ident) -> String {
    let ident = ident.to_string();
    if ident.starts_with("r#") {
        ident.trim_start_matches("r#").to_owned()
    } else {
        ident
    }
}

#[derive(Copy, Clone)]
pub enum Inflection {
    Lower,
    Upper,
    Camel,
    Snake,
    Pascal,
    ScreamingSnake,
}

impl Inflection {
    pub fn apply(self, string: &str) -> String {
        use inflector::Inflector;

        match self {
            Inflection::Lower => string.to_lowercase(),
            Inflection::Upper => string.to_uppercase(),
            Inflection::Camel => string.to_camel_case(),
            Inflection::Snake => string.to_snake_case(),
            Inflection::Pascal => string.to_pascal_case(),
            Inflection::ScreamingSnake => string.to_screaming_snake_case(),
        }
    }
}

#[cfg(feature = "function")]
pub fn format_fn_wrapper(function: &Ident) -> Ident {
    quote::format_ident!("__specta__fn__{}", function)
}

pub fn then_option(condition: bool, inner: TokenStream) -> TokenStream {
    condition
        .then(|| quote!(None))
        .unwrap_or_else(|| quote!(Some(#inner)))
}
