// Runtime attribute representation for syn 2.0 compatibility
// These types mirror the runtime attribute types in specta/src/datatype/struct.rs
// but use owned String data instead of static str references for macro parsing

use quote::ToTokens;
use syn::parse::discouraged::Speculative;

pub struct RuntimeAttributeIR {
    path: String,
    kind: RuntimeMetaIR,
}

pub enum RuntimeMetaIR {
    Path(String),
    NameValue {
        key: String,
        value: RuntimeValueIR,
    },
    List(Vec<RuntimeNestedMetaIR>),
}

pub enum RuntimeNestedMetaIR {
    Meta(RuntimeMetaIR),
    Literal(RuntimeLiteralIR),
    Expr(String),
}

pub enum RuntimeValueIR {
    Literal(RuntimeLiteralIR),
    Expr(String),
}

pub enum RuntimeLiteralIR {
    Str(String),
    Int(i64),
    Bool(bool),
    Float(f64),
    Byte(u8),
    Char(char),
    ByteStr(Vec<u8>),
    CStr(Vec<u8>),
}

fn lower_lit(lit: &syn::Lit) -> syn::Result<RuntimeLiteralIR> {
    match lit {
        syn::Lit::Str(s) => Ok(RuntimeLiteralIR::Str(s.value())),
        syn::Lit::Int(i) => Ok(RuntimeLiteralIR::Int(i.base10_parse()?)),
        syn::Lit::Bool(b) => Ok(RuntimeLiteralIR::Bool(b.value)),
        syn::Lit::Float(f) => Ok(RuntimeLiteralIR::Float(f.base10_parse()?)),
        syn::Lit::Byte(b) => Ok(RuntimeLiteralIR::Byte(b.value())),
        syn::Lit::Char(c) => Ok(RuntimeLiteralIR::Char(c.value())),
        syn::Lit::ByteStr(bs) => Ok(RuntimeLiteralIR::ByteStr(bs.value())),
        syn::Lit::CStr(cs) => Ok(RuntimeLiteralIR::CStr(cs.value().to_bytes().to_vec())),
        _ => Err(syn::Error::new_spanned(lit, "unsupported literal")),
    }
}

fn lower_value_expr(expr: &syn::Expr) -> syn::Result<RuntimeValueIR> {
    match expr {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => Ok(RuntimeValueIR::Literal(lower_lit(lit)?)),
        _ => Ok(RuntimeValueIR::Expr(expr.to_token_stream().to_string())),
    }
}

enum NestedItem {
    Meta(syn::Meta),
    Lit(syn::Lit),
    Expr(syn::Expr),
}

impl syn::parse::Parse for NestedItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        if let Ok(meta) = fork.parse::<syn::Meta>() {
            input.advance_to(&fork);
            return Ok(Self::Meta(meta));
        }

        let fork = input.fork();
        if let Ok(lit) = fork.parse::<syn::Lit>() {
            input.advance_to(&fork);
            return Ok(Self::Lit(lit));
        }

        Ok(Self::Expr(input.parse()?))
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
    use syn::parse::Parser;

    let parser = syn::punctuated::Punctuated::<NestedItem, syn::Token![,]>::parse_terminated;
    let parsed = parser.parse2(list.tokens.clone())?;

    parsed
        .into_iter()
        .map(|item| match item {
            NestedItem::Meta(meta) => Ok(RuntimeNestedMetaIR::Meta(lower_meta(&meta)?)),
            NestedItem::Lit(lit) => Ok(RuntimeNestedMetaIR::Literal(lower_lit(&lit)?)),
            NestedItem::Expr(expr) => {
                Ok(RuntimeNestedMetaIR::Expr(expr.to_token_stream().to_string()))
            }
        })
        .collect()
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
            value: lower_value_expr(&nv.value)?,
        },

        syn::Meta::List(list) => {
            let items = parse_nested_meta_items(list)?;
            RuntimeMetaIR::List(items)
        }
    })
}

/// Convert a syn::Attribute to RuntimeAttributeIR.
/// Updated for syn 2.0 - uses attr.meta instead of attr.parse_meta().
/// Returns None for #[specta(...)] attributes as they are handled by the macro.
pub fn lower_attribute(attr: &syn::Attribute) -> syn::Result<Option<RuntimeAttributeIR>> {
    let path = attr.path().to_token_stream().to_string();

    // Skip #[specta(...)] attributes as they are handled by the macro
    if path == "specta" {
        return Ok(None);
    }

    Ok(Some(RuntimeAttributeIR {
        path,
        kind: lower_meta(&attr.meta)?,
    }))
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
            RuntimeLiteralIR::Byte(b) => quote::quote!(datatype::RuntimeLiteral::Byte(#b)),
            RuntimeLiteralIR::Char(c) => quote::quote!(datatype::RuntimeLiteral::Char(#c)),
            RuntimeLiteralIR::ByteStr(bs) => {
                quote::quote!(datatype::RuntimeLiteral::ByteStr(vec![#(#bs),*]))
            }
            RuntimeLiteralIR::CStr(cs) => {
                quote::quote!(datatype::RuntimeLiteral::CStr(vec![#(#cs),*]))
            }
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
            RuntimeNestedMetaIR::Expr(expr) => {
                quote::quote!(datatype::RuntimeNestedMeta::Expr(String::from(#expr)))
            }
        }
    }
}

impl RuntimeValueIR {
    pub fn to_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            RuntimeValueIR::Literal(lit) => {
                let lit = lit.to_tokens();
                quote::quote!(datatype::RuntimeValue::Literal(#lit))
            }
            RuntimeValueIR::Expr(expr) => {
                quote::quote!(datatype::RuntimeValue::Expr(String::from(#expr)))
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
