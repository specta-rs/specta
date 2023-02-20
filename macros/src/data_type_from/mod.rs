mod attr;

use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

use attr::*;

use crate::utils::parse_attrs;

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident, data, attrs, ..
    } = &parse_macro_input::parse::<DeriveInput>(input)?;

    let mut attrs = parse_attrs(attrs)?;
    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;

    let crate_ref = format_ident!(
        "{}",
        container_attrs
            .crate_name
            .unwrap_or_else(|| "specta".into())
    );

    let (body, impl_typ) = match data {
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

                    let ident = &field.ident;

                    Some(quote! {
                        #crate_ref::ObjectField {
                            key: stringify!(#ident),
                            optional: false,
                            flatten: false
                            ty: t.#ident,
                        }
                    })
                });

                let impl_typ = quote!(#crate_ref::ObjectType);

                (
                    quote! {
                        #impl_typ {
                            name: stringify!(#ident),
                            generics: vec![],
                            fields: vec![#(#fields),*],
                            tag: None,
                        }
                    },
                    impl_typ,
                )
            }
            Fields::Unnamed(_) => {
                let fields = data.fields.iter().enumerate().map(|(i, _)| {
                    let i = proc_macro2::Literal::usize_unsuffixed(i);
                    quote!(t.#i.into()) // TODO: Replace `into` with inline impl
                });

                let impl_typ = quote!(#crate_ref::TupleType);

                (
                    quote! {
                        #impl_typ {
                            generics: vec![],
                            fields: vec![#(#fields),*]
                        }
                    },
                    impl_typ,
                )
            }
            _ => todo!("ToDataType only supports named structs"),
        },
        _ => todo!("ToDataType only supports named structs"),
    };

    Ok(quote! {
        #[automatically_derived]
        impl From<#ident> for #impl_typ {
            fn from(t: #ident) -> #impl_typ {
                #body
            }
        }
    }
    .into())
}
