use proc_macro2::Span;
use quote::format_ident;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Ident, Lit, Path, Result, Token,
};

#[derive(Clone)]
pub enum AttributeValue {
    /// Literal value. Eg. `#[specta(name = "hello")]` or `#[specta(name = u32)]`
    Lit(Lit),
    /// Path value. Eg. `#[specta(type = String)]` or `#[specta(type = ::std::string::String)]`
    /// Path doesn't follow the Rust spec hence the need for this custom parser. We are doing this anyway for backwards compatibility.
    Path(Path),
}

impl AttributeValue {
    fn span(&self) -> Span {
        match self {
            Self::Lit(lit) => lit.span(),
            Self::Path(path) => path.span(),
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
    /// Root ident of the attribute. Eg. `specta` in `#[specta(type = String)]`
    pub root_ident: Ident,
    /// Key of the item. Eg. `type` in `#[specta(type = String)]`
    pub key: Ident,
    /// Value of the item. Eg. `String` in `#[specta(type = String)]`
    value: Option<AttributeValue>,
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

/// This parser is an alternative to `attr.parse_meta()?` from `syn`.
/// We do this to allow `#[specta(type = String)]`.
/// This is technically against the Rust spec,
/// but it's nicer for DX (and the API that we had before these changes).
impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            root_ident: Ident::new("TEMP", input.span()),
            key: input.call(Ident::parse_any)?,
            value: match input.peek(Token![=]) {
                true => {
                    input.parse::<Token![=]>()?;
                    Some(input.parse()?)
                }
                false => None,
            },
        })
    }
}

/// pass all of the attributes into a single structure.
/// We can then remove them from the struct while passing an any left over must be invalid and an error can be thrown.
pub fn parse_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<Attribute>> {
    Ok(attrs
        .iter()
        .map(|attr| {
            let ident = attr
                .path
                .get_ident()
                .expect("Attribute path must be an ident")
                .clone();

            if !(ident == "specta" || ident == "serde" || ident == "doc") {
                return Ok(vec![]);
            }

            if ident == "doc" {
                let meta = attr.parse_meta()?;
                return match meta {
                    syn::Meta::NameValue(value) => Ok(vec![Attribute {
                        root_ident: ident.clone(),
                        key: ident,
                        value: Some(AttributeValue::Lit(value.lit)),
                    }]),
                    _ => Err(syn::Error::new(meta.span(), "specta: invalid doc comment")),
                };
            }

            Ok(attr
                .parse_args_with(Punctuated::<Attribute, Token![,]>::parse_terminated)?
                .into_iter()
                .map(|a| Attribute {
                    root_ident: ident.clone(),
                    ..a
                })
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect())
}

macro_rules! impl_parse {
    ($i:ident ($attr_parser:ident, $out:ident) { $($k:pat => $e:expr),* $(,)? }) => {
        impl $i {
            fn try_from_attrs(
                ident: &'static str,
                attrs: &mut Vec<crate::utils::Attribute>,
                $out: &mut Self,
            ) -> syn::Result<()> {
                use itertools::{Either, Itertools};

                let (filtered_attrs, mut rest): (Vec<_>, Vec<_>) = std::mem::take(attrs)
                    .into_iter()
                    .partition_map(|attr| match attr.root_ident == ident {
                        true => Either::Left(attr),
                        false => Either::Right(attr),
                    });

                let mut new_attrs = filtered_attrs
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
                    .filter_map(|(attr, was_passed_by_user)| {
                        if was_passed_by_user {
                            None
                        } else {
                            Some(attr)
                        }
                    })
                    .collect::<Vec<Attribute>>();
                new_attrs.append(&mut rest);
                let _ = std::mem::replace(attrs, new_attrs);

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

pub fn format_fn_wrapper(function: &Ident) -> Ident {
    format_ident!("__specta__fn__{}", function)
}
