// inspired by https://github.com/tauri-apps/tauri/blob/2901145c497299f033ba7120af5f2e7ead16c75a/core/tauri-macros/src/command/handler.rs

use std::str::FromStr;

use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{FnArg, ItemFn, Pat, parse};

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

/// Builds an identifier suffix unique to this function's source position.
///
/// `macro_export` puts every exported macro in the crate root, so two public
/// functions with the same name in different modules would otherwise define the
/// same root macro. The source position distinguishes them while the module
/// still re-exports the stable `__specta__fn__<name>` alias that `fn_datatype!`
/// resolves.
fn source_position_suffix(item: &proc_macro::TokenStream) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let span = item
        .clone()
        .into_iter()
        .next()
        .map_or_else(proc_macro::Span::call_site, |token| token.span());
    let mut hasher = DefaultHasher::new();
    span.file().hash(&mut hasher);
    span.line().hash(&mut hasher);
    span.column().hash(&mut hasher);

    format!("{:x}", hasher.finish())
}

pub fn attribute(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let crate_ref = quote!(specta);
    let source_suffix = source_position_suffix(&item);
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
    let function_name = &function.sig.ident;
    let unique_wrapper = format_ident!("{}_{}", wrapper, source_suffix);
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
                Pat::Slice(s) => {
                    s.attrs
                        .first()
                        .map(ToTokens::to_token_stream)
                        .ok_or_else(|| {
                            syn::Error::new_spanned(
                                input,
                                "functions with `#[specta]` must take named arguments",
                            )
                        })?
                }
                Pat::Tuple(s) => {
                    s.elems
                        .first()
                        .map(ToTokens::to_token_stream)
                        .ok_or_else(|| {
                            syn::Error::new_spanned(
                                input,
                                "functions with `#[specta]` must take named arguments",
                            )
                        })?
                }
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

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #unique_wrapper {
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

        // Allow the macro to be resolved with the same path as the function.
        // The exported root macro is unique, while this alias is the stable
        // name `fn_datatype!` derives from the function path.
        #[allow(unused_imports)]
        #visibility use #unique_wrapper as #wrapper;
    }
    .into())
}
