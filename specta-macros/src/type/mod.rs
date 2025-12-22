use attr::*;
use r#enum::parse_enum;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse};

use crate::utils::{AttributeValue, parse_attrs, unraw_raw_ident};

use self::generics::{
    add_type_to_where_clause, generics_with_ident_and_bounds_only, generics_with_ident_only,
};

pub(crate) mod attr;
mod r#enum;
mod field;
mod generics;
mod r#struct;

// TODO: MOVE THIS

// Runtime attribute representation for syn 2.0 compatibility
// These types mirror the runtime attribute types in specta/src/datatype/struct.rs
// but use owned String data instead of static str references for macro parsing
struct RuntimeAttributeIR {
    path: String,
    kind: RuntimeMetaIR,
}

enum RuntimeMetaIR {
    Path,
    NameValue {
        key: String,
        value: RuntimeLiteralIR,
    },
    List(Vec<RuntimeNestedMetaIR>),
}

enum RuntimeNestedMetaIR {
    Meta(RuntimeMetaIR),
    Literal(RuntimeLiteralIR),
}

enum RuntimeLiteralIR {
    Str(String),
    Int(i64),
    Bool(bool),
    Float(f64),
}

// impl RuntimeAttributeIR {
//     fn to_tokens(&self) -> proc_macro2::TokenStream {
//         let path = &self.path;
//         let kind = self.kind.to_tokens();

//         quote::quote! {
//             RuntimeAttribute {
//                 path: #path,
//                 kind: #kind,
//             }
//         }
//     }
// }

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
            items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::Path));
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

                items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::NameValue {
                    key: meta.path.to_token_stream().to_string(),
                    value: runtime_lit,
                }));
            } else {
                // Fall back to parsing as token stream and converting to string
                // This handles complex expressions like `remote = Value` or `crate = crate`
                let tokens: proc_macro2::TokenStream = value_stream.parse()?;
                let value_str = tokens.to_string();

                items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::NameValue {
                    key: meta.path.to_token_stream().to_string(),
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
        items.push(RuntimeNestedMetaIR::Meta(RuntimeMetaIR::Path));
        Ok(())
    })?;

    Ok(items)
}

/// Convert a syn::Meta to RuntimeMetaIR.
/// Updated for syn 2.0 - Meta structure is the same but NestedMeta parsing changed.
fn lower_meta(meta: &syn::Meta) -> syn::Result<RuntimeMetaIR> {
    Ok(match meta {
        syn::Meta::Path(_) => RuntimeMetaIR::Path,

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
fn lower_attribute(attr: &syn::Attribute) -> syn::Result<RuntimeAttributeIR> {
    Ok(RuntimeAttributeIR {
        path: attr.path().to_token_stream().to_string(),
        kind: lower_meta(&attr.meta)?,
    })
}

impl RuntimeLiteralIR {
    fn to_tokens(&self) -> proc_macro2::TokenStream {
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
    fn to_tokens(&self) -> proc_macro2::TokenStream {
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
    fn to_tokens(&self) -> proc_macro2::TokenStream {
        match self {
            RuntimeMetaIR::Path => quote::quote!(datatype::RuntimeMeta::Path),

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
    fn to_tokens(&self) -> proc_macro2::TokenStream {
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

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = &parse::<DeriveInput>(input)?;

    // for attr in attrs {
    //     println!(
    //         "{:?} {:?}",
    //         attr.to_token_stream().to_string(),
    //         lower_attribute(attr)?.to_tokens().to_string()
    //     );
    // }

    // TODO: Only produce this on attribute that aren't consumed by the Specta macro parser.
    let lowered_attrs = attrs
        .iter()
        .map(|attr| lower_attribute(attr).map(|attr| attr.to_tokens()))
        .collect::<Result<Vec<_>, _>>()?;

    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let mut attrs = parse_attrs(attrs)?;
    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;

    let raw_ident = ident;
    let ident = container_attrs
        .remote
        .clone()
        .unwrap_or_else(|| ident.to_token_stream());

    let crate_ref: TokenStream = container_attrs.crate_name.clone().unwrap_or(quote!(specta));

    let name = container_attrs.rename.clone().unwrap_or_else(|| {
        unraw_raw_ident(&format_ident!("{}", raw_ident.to_string())).to_token_stream()
    });

    let (inlines, can_flatten) = match data {
        Data::Struct(data) => parse_struct(&container_attrs, &crate_ref, data, &lowered_attrs),
        Data::Enum(data) => parse_enum(
            &EnumAttr::from_attrs(&container_attrs, &mut attrs)?,
            &container_attrs,
            &crate_ref,
            data,
            &lowered_attrs,
        ),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "specta: Union types are not supported by Specta yet!",
        )),
    }?;

    // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
    // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
    if let Some(attrs) = attrs.iter().find(|attr| attr.key == "specta") {
        match &attrs.value {
            Some(AttributeValue::Attribute { attr, .. }) => {
                if let Some(attr) = attr.first() {
                    return Err(syn::Error::new(
                        attr.key.span(),
                        format!(
                            "specta: Found unsupported container attribute '{}'",
                            attr.key
                        ),
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new(
                    attrs.key.span(),
                    "specta: invalid formatted attribute",
                ));
            }
        }
    }

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let where_bound = add_type_to_where_clause(&quote!(#crate_ref::Type), generics);

    let flatten_impl = can_flatten.then(|| {
        quote! {
            #[automatically_derived]
            impl #bounds #crate_ref::Flatten for #ident #type_args #where_bound {}
        }
    });

    let shadow_generics = {
        let g = generics.params.iter().map(|param| match param {
            // Pulled from outside
            GenericParam::Lifetime(_) | GenericParam::Const(_) => quote!(),
            // We shadow the generics to replace them.
            GenericParam::Type(t) => {
                let ident = &t.ident;
                let placeholder_ident = format_ident!("PLACEHOLDER_{}", t.ident);
                quote!(type #ident = datatype::GenericPlaceholder<#placeholder_ident>;)
            }
        });

        quote!(#(#g)*)
    };

    let generic_placeholders = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let ident = format_ident!("PLACEHOLDER_{}", t.ident);
            let ident_str = t.ident.to_string();
            Some(quote!(
                pub struct #ident;
                impl datatype::ConstGenericPlaceholder for #ident {
                    const PLACEHOLDER: &'static str = #ident_str;
                }
            ))
        }
    });

    let collect = (cfg!(feature = "DO_NOT_USE_collect") && container_attrs.collect.unwrap_or(true))
        .then(|| {
            let export_fn_name = format_ident!("__push_specta_type_{}", raw_ident);

            let generic_params = generics
                .params
                .iter()
                .filter(|param| matches!(param, syn::GenericParam::Type(_)))
                .map(|_| quote! { () });

            quote! {
                #[allow(non_snake_case)]
                #[#crate_ref::collect::internal::ctor::ctor(anonymous, crate_path = #crate_ref::collect::internal::ctor)]
                unsafe fn #export_fn_name() {
                    #crate_ref::collect::internal::register::<#ident<#(#generic_params),*>>();
                }
            }
        });

    let comments = &container_attrs.common.doc;
    let inline = container_attrs.inline;
    let deprecated = container_attrs.common.deprecated_as_tokens();

    let reference_generics = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let i = &t.ident;
            let i_str = i.to_string();
            Some(quote!((internal::construct::generic_data_type(#i_str), <#i as #crate_ref::Type>::definition(types))))
        }
    });

    let definition_generics = generics.params.iter().filter_map(|p| match p {
        GenericParam::Type(t) => {
            let ident = t.ident.to_string();
            Some(quote!(std::borrow::Cow::Borrowed(#ident).into()))
        }
        _ => None,
    });

    Ok(quote! {
        #[allow(non_camel_case_types)]
        const _: () = {
            use std::borrow::Cow;
            use #crate_ref::{datatype, internal};

            #[automatically_derived]
            impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
                fn definition(types: &mut #crate_ref::TypeCollection) -> datatype::DataType {
                    #(#generic_placeholders)*

                    static SENTINEL: () = ();
                    datatype::DataType::Reference(
                        datatype::NamedDataType::init_with_sentinel(
                            vec![#(#reference_generics),*],
                            #inline,
                            types,
                            &SENTINEL,
                            |types, ndt| {
                                ndt.set_name(Cow::Borrowed(#name));
                                ndt.set_docs(Cow::Borrowed(#comments));
                                ndt.set_deprecated(#deprecated);
                                ndt.set_module_path(Cow::Borrowed(module_path!()));
                                *ndt.generics_mut() = vec![#(#definition_generics),*];
                                ndt.set_ty({
                                    #shadow_generics
                                    #inlines
                                });
                            }
                        )
                    )
                }
            }

            #flatten_impl

            #collect
        };

    }
    .into())
}
