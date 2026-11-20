use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Expr, Ident, Lit, Meta, Path, Result, Token, Type, TypePath,
    ext::IdentExt,
    parse::{Parse, ParseStream, Parser},
    spanned::Spanned,
    token::Paren,
};

/// Wrap a fragment in an invisible group.
///
/// The tokens are unchanged — rustc's parser treats an invisible group as if it
/// were not there, so the expanded AST and its printed form are identical — but
/// the fragment becomes a single token. `quote!` interpolates a stream by
/// reference and that clones one `Ident` per identifier (their text is owned),
/// while cloning a group is a reference-count bump; the derive nests its body
/// several levels deep on the way to the output, so wrapping the large
/// fragments before they are interpolated avoids most of that copying.
pub fn invisible(tokens: TokenStream) -> TokenStream {
    TokenStream::from(TokenTree::Group(Group::new(Delimiter::None, tokens)))
}

#[derive(Clone)]
pub enum AttributeValue {
    /// Literal value. Eg. `#[specta(name = "hello")]` or `#[specta(name = u32)]`
    Lit(Lit),
    /// Path value. Eg. `#[specta(type = String)]` or `#[specta(type = ::std::string::String)]`
    /// Path doesn't follow the Rust spec hence the need for this custom parser. We are doing this anyway for backwards compatibility.
    Path(Path),
    /// Expression value for values that are not valid paths.
    /// This allows us to later parse richer forms such as tuple/array/reference types.
    Expr(Expr),
    /// A Rust type parsed where the value grammar is ambiguous, such as a
    /// reference with an explicit lifetime.
    Type(Type),
    /// A nested attribute. Eg. the `deprecated(note = "some note") in `#[specta(deprecated(note = "some note"))]`
    Attribute { span: Span, attr: Vec<Attribute> },
}

impl AttributeValue {
    fn span(&self) -> Span {
        match self {
            Self::Lit(lit) => lit.span(),
            Self::Path(path) => path.span(),
            Self::Expr(expr) => expr.span(),
            Self::Type(ty) => ty.span(),
            Self::Attribute { span, .. } => *span,
        }
    }
}

impl Parse for AttributeValue {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(match input.peek(Lit) {
            true => Self::Lit(input.parse()?),
            false => {
                if input.fork().parse::<Path>().is_ok() {
                    Self::Path(input.parse()?)
                } else {
                    Self::Expr(input.parse()?)
                }
            }
        })
    }
}

fn attribute_value_from_expr(expr: &Expr) -> AttributeValue {
    match expr {
        Expr::Lit(expr_lit) => AttributeValue::Lit(expr_lit.lit.clone()),
        Expr::Path(expr_path) => AttributeValue::Path(expr_path.path.clone()),
        expr => AttributeValue::Expr(expr.clone()),
    }
}

#[derive(Clone)]
pub struct Attribute {
    /// Source of the attribute. Eg. `specta`, `serde`, `repr`, `deprecated`, etc.
    pub source: AttributeSource,
    /// Key of the current item. Eg. `specta` or `type`in `#[specta(type = String)]`
    pub key: Ident,
    /// Value of the item. Eg. `String` in `#[specta(type = String)]`
    pub value: Option<AttributeValue>,
}

/// The attribute an [`Attribute`] was parsed from.
///
/// The names Specta looks at are compared as a small enum, so the common case
/// costs no allocation and no proc-macro bridge round-trip. Every other
/// attribute keeps the `Ident` it was parsed from, so extraction by a name this
/// list doesn't know about still works: the `#[specta]` function macro reads
/// `#[command(rename_all = ...)]`.
#[derive(Clone, PartialEq, Eq)]
pub enum AttributeSource {
    Specta,
    Serde,
    Doc,
    Deprecated,
    Repr,
    Other(Ident),
}

impl AttributeSource {
    /// The source an attribute's outermost identifier names.
    fn from_ident(ident: &Ident) -> Self {
        if ident == "specta" {
            Self::Specta
        } else if ident == "serde" {
            Self::Serde
        } else if ident == "doc" {
            Self::Doc
        } else if ident == "deprecated" {
            Self::Deprecated
        } else if ident == "repr" {
            Self::Repr
        } else {
            Self::Other(ident.clone())
        }
    }

    /// Whether this source was the attribute named `name`.
    fn is(&self, name: &str) -> bool {
        match self {
            Self::Specta => name == "specta",
            Self::Serde => name == "serde",
            Self::Doc => name == "doc",
            Self::Deprecated => name == "deprecated",
            Self::Repr => name == "repr",
            Self::Other(ident) => ident == name,
        }
    }
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

    pub fn parse_bool_or_true(&self) -> Result<bool> {
        match &self.value {
            None => Ok(true),
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
            Some(AttributeValue::Expr(Expr::Path(path))) => Ok(path.path.clone()),
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected path. Eg. `String` or `std::string::String`",
            )),
        }
    }

    pub fn parse_type(&self) -> Result<Type> {
        match &self.value {
            Some(AttributeValue::Type(ty)) => Ok(ty.clone()),
            Some(AttributeValue::Path(path)) => Ok(Type::Path(TypePath {
                qself: None,
                path: path.clone(),
            })),
            Some(AttributeValue::Expr(expr)) => syn::parse2(expr.to_token_stream()).map_err(|_| {
                syn::Error::new(
                    self.value_span(),
                    "specta: expected type. Eg. `String`, `(String, i32)` or `Vec<T>`",
                )
            }),
            _ => Err(syn::Error::new(
                self.value_span(),
                "specta: expected type. Eg. `String`, `(String, i32)` or `Vec<T>`",
            )),
        }
    }
}

pub trait AttrExtract {
    fn extract(&mut self, source: &str, key: &str) -> Option<Attribute>;
    fn extract_all(&mut self, source: &str, key: &str) -> Vec<Attribute>;
}

impl AttrExtract for Vec<Attribute> {
    fn extract(&mut self, source: &str, key: &str) -> Option<Attribute> {
        // 1. Check for top-level match (e.g., #[deprecated])
        if let Some(pos) = self
            .iter()
            .position(|attr| attr.source.is(source) && attr.key == key)
        {
            return Some(self.swap_remove(pos));
        }

        // 2. Check nested attributes within parent matching source
        //    e.g., extract("specta", "inline") from #[specta(inline)]
        //    Structure: Attribute { source: "specta", key: "specta", value: Attribute { key: "inline" } }
        for i in 0..self.len() {
            if self[i].source.is(source)
                && self[i].key == source
                && let Some(AttributeValue::Attribute {
                    attr: nested_attrs, ..
                }) = &mut self[i].value
                && let Some(nested_pos) = nested_attrs.iter().position(|a| a.key == key)
            {
                let result = nested_attrs.swap_remove(nested_pos);

                // If parent attribute is now empty, remove it too
                if nested_attrs.is_empty() {
                    self.swap_remove(i);
                }

                return Some(result);
            }
        }

        None
    }

    fn extract_all(&mut self, source: &str, key: &str) -> Vec<Attribute> {
        let mut result = Vec::new();

        // 1. Extract all top-level matches
        let mut i = 0;
        while i < self.len() {
            if self[i].source.is(source) && self[i].key == key {
                result.push(self.swap_remove(i));
                // Don't increment i, as swap_remove moved a new element to position i
            } else {
                i += 1;
            }
        }

        // 2. Extract all nested matches
        let mut i = 0;
        while i < self.len() {
            let mut should_remove_parent = false;

            if self[i].source.is(source)
                && self[i].key == source
                && let Some(AttributeValue::Attribute {
                    attr: nested_attrs, ..
                }) = &mut self[i].value
            {
                let mut j = 0;
                while j < nested_attrs.len() {
                    if nested_attrs[j].key == key {
                        result.push(nested_attrs.swap_remove(j));
                    } else {
                        j += 1;
                    }
                }

                should_remove_parent = nested_attrs.is_empty();
            }

            if should_remove_parent {
                self.swap_remove(i);
            } else {
                i += 1;
            }
        }

        result
    }
}

struct NestedAttributeList {
    attrs: Vec<Attribute>,
    unstructured_argument: Option<Span>,
}

impl NestedAttributeList {
    /// Parse a list whose attributes all belong to `source`.
    ///
    /// The source is passed down rather than recovered from each key: a nested
    /// attribute is part of the attribute it sits in, and cloning the key to
    /// guess that would cost an `Ident` per nested attribute.
    fn parse_with_source(input: ParseStream, source: &AttributeSource) -> Result<Self> {
        let mut attrs = Vec::new();
        let mut unstructured_argument = None;
        while !input.is_empty() {
            if input.peek(Ident::peek_any) {
                let fork = input.fork();
                let _ = fork.call(Ident::parse_any)?;

                if fork.peek(Token![::])
                    || !(fork.is_empty()
                        || fork.peek(Paren)
                        || fork.peek(Token![=])
                        || fork.peek(Token![,]))
                {
                    unstructured_argument.get_or_insert(input.span());
                    skip_unstructured_argument(input)?;
                } else {
                    let key = input.call(Ident::parse_any)?;
                    let key_span = key.span();
                    let value_is_type = key == "type";

                    attrs.push(Attribute {
                        source: source.clone(),
                        key,
                        value: match false {
                            _ if input.peek(Paren) => {
                                let content;
                                syn::parenthesized!(content in input);
                                let nested = Self::parse_with_source(&content, source)?;
                                if unstructured_argument.is_none() {
                                    unstructured_argument = nested.unstructured_argument;
                                }
                                Some(AttributeValue::Attribute {
                                    span: key_span,
                                    attr: nested.attrs,
                                })
                            }
                            _ if input.peek(Token![=]) => {
                                input.parse::<Token![=]>()?;
                                if value_is_type {
                                    Some(AttributeValue::Type(input.parse()?))
                                } else {
                                    Some(input.parse()?)
                                }
                            }
                            _ => None,
                        },
                    });
                }
            } else {
                unstructured_argument.get_or_insert(input.span());
                skip_unstructured_argument(input)?;
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(NestedAttributeList {
            attrs,
            unstructured_argument,
        })
    }
}

fn skip_unstructured_argument(input: ParseStream) -> Result<()> {
    while !input.is_empty() && !input.peek(Token![,]) {
        let _ = input.parse::<TokenTree>()?;
    }
    Ok(())
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
    // One `Attribute` per input attribute at most, so the common case (no
    // nesting surprises) is a single allocation.
    let mut result = Vec::with_capacity(attrs.len());

    for attr in attrs {
        let ident = &attr
            .path()
            .segments
            .last()
            .expect("Attribute path must have at least one segment")
            .ident;

        // Skip attributes that are in the skip list. `#[tracing::instrument]`
        // accepts shorthand syntax such as `%value` that is not valid for
        // Specta's generic metadata parser and should never be interpreted as
        // Specta configuration.
        //
        // The `Ident` is compared directly: it has a `str` comparison that
        // spells raw identifiers the same way `Ident::to_string` does, so this
        // needs no allocation per attribute.
        if *ident == "instrument" || skip_attrs.iter().any(|skip| ident == skip.as_str()) {
            continue;
        }

        let source = AttributeSource::from_ident(ident);
        let ident = ident.clone();

        match &attr.meta {
            Meta::Path(_) => result.push(Attribute {
                source,
                key: ident,
                value: None,
            }),
            Meta::List(meta) => {
                let span = ident.span();
                let tokens = meta.tokens.clone();
                let parsed =
                    (|input: ParseStream| NestedAttributeList::parse_with_source(input, &source))
                        .parse2(tokens)?;
                if source.is("specta")
                    && let Some(span) = parsed.unstructured_argument
                {
                    return Err(syn::Error::new(
                        span,
                        "specta: unsupported attribute syntax",
                    ));
                }
                result.push(Attribute {
                    source,
                    key: ident,
                    value: Some(AttributeValue::Attribute {
                        span,
                        attr: parsed.attrs,
                    }),
                });
            }
            Meta::NameValue(meta) => result.push(Attribute {
                source,
                key: ident,
                value: Some(attribute_value_from_expr(&meta.value)),
            }),
        }
    }

    Ok(result)
}

/// The name an identifier is exported under: the identifier without its raw
/// prefix (`r#type` is exported as `type`).
///
/// This is an owned `String` because that is what the exported name becomes: a
/// string literal in the generated code. `proc_macro2` has no way to ask an
/// `Ident` whether it is raw without formatting it first, so the common case
/// cannot avoid the allocation — but the raw prefix is drained from the same
/// buffer instead of copying the remaining name into a second one.
pub fn unraw_raw_ident(ident: &Ident) -> String {
    let mut name = ident.to_string();
    if name.starts_with("r#") {
        name.drain(..2);
    }
    name
}

#[cfg(feature = "DO_NOT_USE_function")]
pub fn format_fn_wrapper(function: &Ident) -> Ident {
    quote::format_ident!("__specta__fn__{}", function)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_method_calls_in_unrelated_attributes() {
        let input = syn::parse_str::<syn::DeriveInput>(
            r#"
                #[error("Circular dependency detected: {}", chain.join(" -> "))]
                enum InheritanceError { CircularDependency { chain: Vec<String> } }
            "#,
        )
        .expect("test input should parse");

        let attrs = parse_attrs(&input.attrs).expect("unrelated attribute should remain parseable");
        let argument_count = match attrs[0].value.as_ref() {
            Some(AttributeValue::Attribute { attr, .. }) => attr.len(),
            _ => usize::MAX,
        };
        assert_eq!(argument_count, 0);
    }

    #[test]
    fn parses_structured_metadata_alongside_unstructured_arguments() {
        let input = syn::parse_str::<syn::DeriveInput>(
            r#"
                #[error("Circular dependency detected: {}", chain.join(" -> "))]
                #[serde(rename_all = "camelCase", untagged)]
                enum InheritanceError { CircularDependency { chain: Vec<String> } }
            "#,
        )
        .expect("test input should parse");

        let mut attrs = parse_attrs(&input.attrs).expect("attributes should parse");
        assert!(attrs.extract("serde", "untagged").is_some());
        assert_eq!(
            attrs
                .extract("serde", "rename_all")
                .expect("rename_all should be captured")
                .parse_string()
                .expect("rename_all should be a string"),
            "camelCase"
        );
    }

    /// The `#[specta]` function macro reads `#[command(rename_all = ...)]`, a
    /// source name the `AttributeSource` enum doesn't name. Extraction has to
    /// still match it.
    #[test]
    fn extracts_attributes_whose_source_is_not_a_known_name() {
        let input = syn::parse_str::<syn::DeriveInput>(
            r#"
                #[command(rename_all = "kebab-case")]
                struct Func;
            "#,
        )
        .expect("test input should parse");

        let mut attrs = parse_attrs(&input.attrs).expect("attributes should parse");
        assert_eq!(
            attrs
                .extract("command", "rename_all")
                .expect("rename_all should be captured")
                .parse_string()
                .expect("rename_all should be a string"),
            "kebab-case"
        );
    }

    #[test]
    fn rejects_unstructured_specta_arguments() {
        let input = syn::parse_str::<syn::DeriveInput>(
            r#"
                #[specta(collect = false, typo.missing)]
                struct Invalid;
            "#,
        )
        .expect("test input should parse");

        assert!(parse_attrs(&input.attrs).is_err());
    }
}
