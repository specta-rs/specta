// inspired by https://github.com/tauri-apps/tauri/blob/2901145c497299f033ba7120af5f2e7ead16c75a/core/tauri-macros/src/command/handler.rs

use std::str::FromStr;

use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, ItemFn, Pat, Visibility, parse};

use crate::{
    r#type::attr::deprecated_as_tokens,
    utils::{AttrExtract, format_fn_wrapper, parse_attrs},
};

fn unraw(s: &str) -> &str {
    if s.starts_with("r#") {
        s.split_at(2).1
    } else {
        s
    }
}

#[derive(Clone, Copy)]
enum RenameAllRule {
    Lowercase,
    Uppercase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameAllRule {
    fn parse(value: &str, span: proc_macro2::Span) -> syn::Result<Self> {
        match value {
            "lowercase" => Ok(Self::Lowercase),
            "UPPERCASE" => Ok(Self::Uppercase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            _ => Err(syn::Error::new(
                span,
                "specta: unsupported rename rule. Expected one of lowercase, UPPERCASE, PascalCase, camelCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, SCREAMING-KEBAB-CASE",
            )),
        }
    }

    fn apply(self, input: &str) -> String {
        match self {
            Self::Lowercase => input.to_lowercase(),
            Self::Uppercase => input.to_uppercase(),
            Self::PascalCase => input.to_pascal_case(),
            Self::CamelCase => input.to_camel_case(),
            Self::SnakeCase => input.to_snake_case(),
            Self::ScreamingSnakeCase => input.to_screaming_snake_case(),
            Self::KebabCase => input.to_kebab_case(),
            Self::ScreamingKebabCase => input.to_kebab_case().to_uppercase(),
        }
    }
}

struct FunctionNameAttrs {
    rename: Option<String>,
    rename_all: Option<RenameAllRule>,
}

fn parse_name_attrs(
    attr: proc_macro::TokenStream,
    function: &ItemFn,
) -> syn::Result<FunctionNameAttrs> {
    let mut attrs = function.attrs.clone();
    let attr = proc_macro2::TokenStream::from(attr);
    if !attr.is_empty() {
        let specta_attr: syn::Attribute = syn::parse_quote!(#[specta(#attr)]);
        attrs.push(specta_attr);
    }

    let mut attrs = parse_attrs(&attrs)?;

    let rename = attrs
        .extract("specta", "rename")
        .map(|attr| attr.parse_string())
        .transpose()?;

    let rename_all = attrs
        .extract("specta", "rename_all")
        .or_else(|| attrs.extract("command", "rename_all"))
        .map(|attr| RenameAllRule::parse(&attr.parse_string()?, attr.value_span()))
        .transpose()?;

    Ok(FunctionNameAttrs { rename, rename_all })
}

pub fn attribute(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let crate_ref = quote!(specta);
    let function = parse::<ItemFn>(item)?;
    let name_attrs = parse_name_attrs(attr, &function)?;
    let wrapper = format_fn_wrapper(&function.sig.ident);

    // While using wasm_bindgen and Specta is rare, this should make the DX nicer.
    if function.sig.unsafety.is_some()
        && function
            .sig
            .ident
            .to_string()
            .starts_with("__wasm_bindgen_generated")
    {
        return Err(syn::Error::new_spanned(
            function.sig.ident,
            "specta: You must apply the #[specta] macro before the #[wasm_bindgen] macro",
        ));
    }

    let visibility = &function.vis;
    let maybe_macro_export = match &visibility {
        Visibility::Public(_) => {
            quote!(#[macro_export])
        }
        _ => Default::default(),
    };

    let function_name = &function.sig.ident;
    let mut function_name_str = unraw(&function_name.to_string()).to_string();
    if let Some(rule) = name_attrs.rename_all {
        function_name_str = rule.apply(&function_name_str);
    }
    if let Some(rename) = name_attrs.rename {
        function_name_str = rename;
    }
    let function_asyncness = function.sig.asyncness.is_some();

    let mut arg_names = Vec::new();
    for input in function.sig.inputs.iter() {
        let arg = match input {
            FnArg::Receiver(_) => {
                return Err(syn::Error::new_spanned(
                    input,
                    "functions with `#[specta]` cannot take 'self'",
                ));
            }
            FnArg::Typed(arg) => match &*arg.pat {
                Pat::Ident(ident) => ident.ident.to_token_stream(),
                Pat::Macro(m) => m.mac.tokens.to_token_stream(),
                Pat::Struct(s) => s.path.to_token_stream(),
                Pat::Slice(s) => s.attrs[0].to_token_stream(),
                Pat::Tuple(s) => s.elems[0].to_token_stream(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        input,
                        "functions with `#[specta]` must take named arguments",
                    ));
                }
            },
        };

        let mut s = arg.to_string();

        let s = if s.starts_with("r#") {
            s.split_off(2)
        } else {
            s
        };

        let arg_name = TokenStream::from_str(&s).map_err(|err| {
            syn::Error::new_spanned(input, format!("invalid token stream for argument: {err}"))
        })?;

        let mut arg_name_str = arg_name.to_string();
        if let Some(rule) = name_attrs.rename_all {
            arg_name_str = rule.apply(&arg_name_str);
        }

        arg_names.push(arg_name_str);
    }

    let arg_signatures = function.sig.inputs.iter().map(|_| quote!(_));

    let mut attrs = parse_attrs(&function.attrs)?;
    let common = crate::r#type::attr::RustCAttr::from_attrs(&mut attrs)?;

    let deprecated = if let Some(deprecated) = common.deprecated {
        let tokens = deprecated_as_tokens(deprecated);
        quote!(#tokens)
    } else {
        quote!(None)
    };
    let docs = common.doc;

    let no_return_type = match function.sig.output {
        syn::ReturnType::Default => true,
        syn::ReturnType::Type(_, _) => false,
    };

    Ok(quote! {
        #function

        #maybe_macro_export
        #[doc(hidden)]
        macro_rules! #wrapper {
            // We take in `$function` from the invocation so we have `fn_name::<concrete_generics_types>`
            (@export_fn; $function:path) => {{
                use #crate_ref::datatype;
                fn export(types: &mut #crate_ref::Types) -> datatype::Function {
                    #crate_ref::internal::get_fn_datatype(
                        $function as fn(#(#arg_signatures),*) -> _,
                        #function_asyncness,
                        #function_name_str.into(),
                        types,
                        &[#(#arg_names.into()),* ],
                        std::borrow::Cow::Borrowed(#docs),
                        #deprecated,
                        #no_return_type,
                    )
                }

                export
            }}
        }

        // allow the macro to be resolved with the same path as the function
        #[allow(unused_imports)]
        #visibility use #wrapper;
    }
    .into())
}
