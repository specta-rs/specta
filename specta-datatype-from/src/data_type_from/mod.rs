mod attr;

use quote::{quote, ToTokens};
use syn::{parse, Data, DeriveInput, Fields};

use attr::*;

use crate::utils::{parse_attrs, then_option};

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident,
        data,
        attrs,
        generics,
        ..
    } = &parse::<DeriveInput>(input)?;

    let mut attrs = parse_attrs(attrs)?;
    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;

    let crate_ref = container_attrs.crate_name.unwrap_or_else(|| quote!(specta));

    if generics.params.len() > 0 {
        return Err(syn::Error::new_spanned(
            generics,
            "DataTypeFrom does not support generics",
        ));
    }

    Ok(match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| {
                        let mut attrs = parse_attrs(&field.attrs)?;
                        let field_attrs = FieldAttr::from_attrs(&mut attrs)?;

                        Ok((field, field_attrs))
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                let fields = fields.iter().map(|(field, attrs)| {
                    let ident = field
                        .ident
                        .as_ref()
                        // TODO: Proper syn error would be nice
                        .expect("'specta::DataTypeFrom' requires named fields.");

                    let ident_str = match &attrs.rename {
                        Some(rename) => rename.to_token_stream(),
                        None => ident.to_string().to_token_stream(),
                    };

                    let ty = then_option(attrs.skip, quote!(t.#ident.into()));

                    quote!((#ident_str.into(), #crate_ref::internal::construct::field(
                        false,
                        false,
                        None,
                        std::borrow::Cow::Borrowed(""),
                        #ty
                    )))
                });

                let struct_name = ident.to_string();
                quote! {
                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::StructType {
                        fn from(t: #ident) -> #crate_ref::StructType {
                            #crate_ref::internal::construct::r#struct(#struct_name.into(), None, vec![], #crate_ref::internal::construct::struct_named(vec![#(#fields),*], None))
                        }
                    }

                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::DataType {
                        fn from(t: #ident) -> #crate_ref::DataType {
                           Self::Struct(t.into())
                        }
                    }
                }
            }
            Fields::Unnamed(_) => {
                let fields = data
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let i = proc_macro2::Literal::usize_unsuffixed(i);
                        quote!(t.#i.into())
                    })
                    .collect::<Vec<_>>();

                let implementation = if fields.len() == 1 {
                    fields[0].clone()
                } else {
                    quote! {
                        #crate_ref::DataType::Tuple(#crate_ref::internal::construct::tuple(
                            vec![#(#fields),*]
                        ))
                    }
                };

                quote! {
                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::DataType {
                        fn from(t: #ident) -> #crate_ref::DataType {
                            #implementation
                        }
                    }
                }
            }
            Fields::Unit => {
                syn::Error::new_spanned(ident, "DataTypeFrom does not support unit structs")
                    .into_compile_error()
            }
        },
        _ => syn::Error::new_spanned(ident, "DataTypeFrom only supports structs")
            .into_compile_error(),
    }
    .into())
}
