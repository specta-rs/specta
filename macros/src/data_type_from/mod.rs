mod attr;

use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use attr::*;

use crate::utils::parse_attrs;

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident, data, attrs, ..
    } = &parse_macro_input::parse::<DeriveInput>(input)?;

    let mut attrs = parse_attrs(attrs)?;
    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;

    let crate_ref = container_attrs.crate_name.unwrap_or_else(|| quote!(specta));

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
                let fields = fields.iter().filter_map(|(field, attrs)| {
                    if attrs.skip {
                        return None;
                    }

                    let ident = field
                        .ident
                        .as_ref()
                        // TODO: Proper syn error would be nice
                        .expect("'specta::DataTypeFrom' requires named fields.");
                    let ident_str = ident.to_string();

                    Some(quote! {
                        #crate_ref::internal::construct::struct_field(
                            #ident_str.into(),
                            false,
                            false,
                            t.#ident.into(),
                        )
                    })
                });

                quote! {
                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::StructType {
                        fn from(t: #ident) -> #crate_ref::StructType {
                            #crate_ref::internal::construct::r#struct(vec![], vec![#(#fields),*], None)
                        }
                    }

                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::DataType {
                        fn from(t: #ident) -> #crate_ref::DataType {
                            #crate_ref::DataType::Struct(t.into())
                        }
                    }
                }
            }
            Fields::Unnamed(_) => {
                let fields = data.fields.iter().enumerate().map(|(i, _)| {
                    let i = proc_macro2::Literal::usize_unsuffixed(i);
                    quote!(t.#i.into())
                });

                quote! {
                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::TupleType {
                        fn from(t: #ident) -> #crate_ref::TupleType {
                            #crate_ref::TupleType::Named {
                                generics: vec![],
                                fields: vec![#(#fields),*]
                            }
                        }
                    }

                    #[automatically_derived]
                    impl From<#ident> for #crate_ref::DataType {
                        fn from(t: #ident) -> #crate_ref::DataType {
                            #crate_ref::DataType::Tuple(t.into())
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
