// Runtime attribute representation for syn 2.0 compatibility
// These types mirror the runtime attribute types in specta/src/datatype/struct.rs
// but use owned String data instead of static str references for macro parsing

use quote::ToTokens;

pub struct RuntimeAttributeIR {
    path: String,
    kind: RuntimeMetaIR,
}

pub enum RuntimeMetaIR {
    Path(String),
    NameValue {
        key: String,
        value: RuntimeLiteralIR,
    },
    List(Vec<RuntimeNestedMetaIR>),
}

pub enum RuntimeNestedMetaIR {
    Meta(RuntimeMetaIR),
    Literal(RuntimeLiteralIR),
}

pub enum RuntimeLiteralIR {
    Str(String),
    Int(i64),
    Bool(bool),
    Float(f64),
}

fn lower_lit(expr: &syn::Expr) -> syn::Result<RuntimeLiteralIR> {
    match expr {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => match lit {
            syn::Lit::Str(s) => Ok(RuntimeLiteralIR::Str(s.value())),
            syn::Lit::Int(i) => Ok(RuntimeLiteralIR::Int(i.base10_parse()?)),
            syn::Lit::Bool(b) => Ok(RuntimeLiteralIR::Bool(b.value)),
            syn::Lit::Float(f) => Ok(RuntimeLiteralIR::Float(f.base10_parse()?)),
            _ => Err(syn::Error::new_spanned(lit, "unsupported literal")),
        },
        _ => Err(syn::Error::new_spanned(expr, "expected literal")),
    }
}

/// Parse nested meta items from a MetaList using syn 2.0's parse_nested_meta API.
/// This replaces the old syn 1.0 NestedMeta parsing which was removed.
///
/// Handles four types of nested items:
/// - Path-only items (e.g., `untagged`)
/// - Name-value pairs with literals (e.g., `rename = "value"`)
/// - Name-value pairs with complex expressions (e.g., `remote = Value`)
/// - Nested lists (e.g., `key(...)`) - recursive parsing
/// - Direct literals (e.g., `"bruh"` in `#[test("bruh")]`)
fn parse_nested_meta_items(list: &syn::MetaList) -> syn::Result<Vec<RuntimeNestedMetaIR>> {
    // For simple function-like attributes like #[test("bruh")], we need to parse the tokens directly
    // because parse_nested_meta doesn't handle bare literals well
    let tokens = &list.tokens;

    // Try to parse as a single literal first (handles #[test("bruh")] case)
    if let Ok(lit) = syn::parse2::<syn::Lit>(tokens.clone()) {
        let runtime_lit = match lit {
            syn::Lit::Str(s) => RuntimeLiteralIR::Str(s.value()),
            syn::Lit::Int(i) => RuntimeLiteralIR::Int(i.base10_parse()?),
            syn::Lit::Bool(b) => RuntimeLiteralIR::Bool(b.value),
            syn::Lit::Float(f) => RuntimeLiteralIR::Float(f.base10_parse()?),
            _ => return Err(syn::Error::new_spanned(lit, "unsupported literal")),
        };
        return Ok(vec![RuntimeNestedMetaIR::Literal(runtime_lit)]);
    }

    // Fall back to the standard parse_nested_meta API
    let mut items = Vec::new();

    list.parse_nested_meta(|meta| {
        // Handle different types of nested meta items

        // Check if it's a path-only meta (like `untagged`)
        if meta.input.is_empty() {
            let path_str = meta.path.to_token_stream().to_string();
            items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::Path(path_str)));
            return Ok(());
        }

        // Check if it's a name-value pair (like `rename = "value"`)
        if meta.input.peek(syn::Token![=]) {
            let value_stream = meta.value()?;

            // Try to parse as a literal first
            if let Ok(lit) = value_stream.parse::<syn::Lit>() {
                let runtime_lit = match lit {
                    syn::Lit::Str(s) => RuntimeLiteralIR::Str(s.value()),
                    syn::Lit::Int(i) => RuntimeLiteralIR::Int(i.base10_parse()?),
                    syn::Lit::Bool(b) => RuntimeLiteralIR::Bool(b.value),
                    syn::Lit::Float(f) => RuntimeLiteralIR::Float(f.base10_parse()?),
                    _ => return Err(syn::Error::new_spanned(lit, "unsupported literal")),
                };

                let path_str = meta.path.to_token_stream().to_string();
                items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::NameValue {
                    key: path_str,
                    value: runtime_lit,
                }));
            } else {
                // Fall back to parsing as token stream and converting to string
                // This handles complex expressions like `remote = Value` or `crate = crate`
                let tokens: proc_macro2::TokenStream = value_stream.parse()?;
                let value_str = tokens.to_string();
                let path_str = meta.path.to_token_stream().to_string();

                items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::NameValue {
                    key: path_str,
                    value: RuntimeLiteralIR::Str(value_str),
                }));
            }

            return Ok(());
        }

        // Handle nested lists (like `key(...)`) by parsing recursively
        if meta.input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in meta.input);

            // Create a synthetic MetaList for recursive parsing
            let nested_list = syn::MetaList {
                path: meta.path.clone(),
                delimiter: syn::MacroDelimiter::Paren(Default::default()),
                tokens: content.parse()?,
            };

            let nested_items = parse_nested_meta_items(&nested_list)?;
            items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::List(nested_items)));
            return Ok(());
        }

        // Default case: treat as path
        let path_str = meta.path.to_token_stream().to_string();
        items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::Path(path_str)));
        Ok(())
    })?;

    Ok(items)
}

/// Convert a syn::Meta to RuntimeMetaIR.
/// Updated for syn 2.0 - Meta structure is the same but NestedMeta parsing changed.
fn lower_meta(meta: &syn::Meta) -> syn::Result<RuntimeMetaIR> {
    Ok(match meta {
        syn::Meta::Path(path) => {
            let path_str = path.to_token_stream().to_string();
            RuntimeMetaIR::Path(path_str)
        }

        syn::Meta::NameValue(nv) => RuntimeMetaIR::NameValue {
            key: nv.path.to_token_stream().to_string(),
            value: lower_lit(&nv.value)?,
        },

        syn::Meta::List(list) => {
            let items = parse_nested_meta_items(list)?;
            RuntimeMetaIR::List(items)
        }
    })
}

/// Convert a syn::Attribute to RuntimeAttributeIR.
/// Updated for syn 2.0 - uses attr.meta instead of attr.parse_meta().
pub fn lower_attribute(attr: &syn::Attribute) -> syn::Result<RuntimeAttributeIR> {
    Ok(RuntimeAttributeIR {
        path: attr.path().to_token_stream().to_string(),
        kind: lower_meta(&attr.meta)?,
    })
}

impl RuntimeLiteralIR {
    pub fn to_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            RuntimeLiteralIR::Str(s) => {
                quote::quote!(datatype::RuntimeLiteral::Str(String::from(#s)))
            }
            RuntimeLiteralIR::Int(i) => quote::quote!(datatype::RuntimeLiteral::Int(#i)),
            RuntimeLiteralIR::Bool(b) => quote::quote!(datatype::RuntimeLiteral::Bool(#b)),
            RuntimeLiteralIR::Float(f) => quote::quote!(datatype::RuntimeLiteral::Float(#f)),
        }
    }
}

impl RuntimeNestedMetaIR {
    pub fn to_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            RuntimeNestedMetaIR::Meta(m) => {
                let m = m.to_tokens();
                quote::quote!(datatype::RuntimeNestedMeta::Meta(#m))
            }
            RuntimeNestedMetaIR::Literal(l) => {
                let l = l.to_tokens();
                quote::quote!(datatype::RuntimeNestedMeta::Literal(#l))
            }
        }
    }
}

impl RuntimeMetaIR {
    pub fn to_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            RuntimeMetaIR::Path(path) => {
                quote::quote!(datatype::RuntimeMeta::Path(String::from(#path)))
            }

            RuntimeMetaIR::NameValue { key, value } => {
                let value = value.to_tokens();
                quote::quote! {
                    datatype::RuntimeMeta::NameValue {
                        key: String::from(#key),
                        value: #value,
                    }
                }
            }

            RuntimeMetaIR::List(items) => {
                let items = items.iter().map(|i| i.to_tokens());
                quote::quote! {
                    datatype::RuntimeMeta::List(vec![ #(#items),* ])
                }
            }
        }
    }
}

impl RuntimeAttributeIR {
    pub fn to_tokens(&self) -> proc_macro2::TokenStream {
        let path = &self.path;
        let kind = self.kind.to_tokens();

        quote::quote! {
            datatype::RuntimeAttribute {
                path: String::from(#path),
                kind: #kind,
            }
        }
    }
}
