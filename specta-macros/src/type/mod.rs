use attr::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use r#enum::parse_enum;
use r#struct::parse_struct;
use syn::{parse, Data, DeriveInput, GenericParam};

use crate::utils::{parse_attrs, unraw_raw_ident, AttributeValue};

use self::generics::{
    add_type_to_where_clause, generics_with_ident_and_bounds_only, generics_with_ident_only,
};

pub(crate) mod attr;
mod r#enum;
mod field;
mod generics;
mod r#struct;

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = &parse::<DeriveInput>(input)?;

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
        Data::Struct(data) => parse_struct(&name, &container_attrs, generics, &crate_ref, data),
        Data::Enum(data) => parse_enum(
            &name,
            &EnumAttr::from_attrs(&container_attrs, &mut attrs)?,
            &container_attrs,
            generics,
            &crate_ref,
            data,
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
                ))
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

    //     if container_attrs.inline || container_attrs.transparent {
    //     let generics = &generics.params;
    //     quote!(#generics)
    // } else {
    let inline_generics_def = {
        let g = generics.params.iter().map(|param| match param {
            GenericParam::Lifetime(lt) => {
                let lt = &lt.lifetime;
                quote!(#lt)
            }
            GenericParam::Type(t) => {
                let ident = format_ident!("PLACEHOLDER_{}", t.ident);
                quote!(#crate_ref::datatype::Generic<#ident>)
            }
            GenericParam::Const(c) => {
                let ident = &c.ident;
                quote!(#ident)
            }
        });

        quote!(#(#g),*)
    };

    let generic_placeholders = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let ident = format_ident!("PLACEHOLDER_{}", t.ident);
            let ident_str = t.ident.to_string();
            Some(quote!(
                pub struct #ident;
                impl #crate_ref::datatype::GenericPlaceholder for #ident {
                    const PLACEHOLDER: &'static str = #ident_str;
                }
            ))
        }
    });

    let export = (cfg!(feature = "DO_NOT_USE_export") && container_attrs.export.unwrap_or(true))
        .then(|| {
            let export_fn_name = format_ident!("__push_specta_type_{}", raw_ident);

            let generic_params = generics
                .params
                .iter()
                .filter(|param| matches!(param, syn::GenericParam::Type(_)))
                .map(|_| quote! { () });

            quote! {
                #[allow(non_snake_case)]
                #[#crate_ref::export::internal::ctor]
                fn #export_fn_name() {
                    #crate_ref::export::internal::register::<#ident<#(#generic_params),*>>();
                }
            }
        });

    let comments = &container_attrs.common.doc;
    let inline = container_attrs.inline;
    let deprecated = container_attrs.common.deprecated_as_tokens(&crate_ref);
    let impl_location = quote!(#crate_ref::internal::construct::impl_location(concat!(file!(), ":", line!(), ":", column!())));

    let reference_generics = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let i = &t.ident;
            let i_str = i.to_string();
            Some(quote!((#crate_ref::internal::construct::generic_data_type(#i_str), <#i as #crate_ref::Type>::definition(types))))
        }
    });

    Ok(quote! {
        #[allow(non_camel_case_types)]
        const _: () = {
            pub use #crate_ref::Type;

            // This is equivalent to `<Self as #crate_ref::NamedType>::ID` but it's shorter so we use it instead.
            const SID: #crate_ref::SpectaID = #crate_ref::internal::construct::sid(#name, concat!("::", module_path!(), ":", line!(), ":", column!()));

            #(#generic_placeholders)*

            // TODO: We should make this a standalone function but that caused issues resolving lifetimes.
            #[automatically_derived]
            impl #bounds #ident #type_args #where_bound {
                fn ___specta_definition___(types: &mut #crate_ref::TypeCollection) -> #crate_ref::datatype::DataType {
                    #inlines
                }
            }

            #[automatically_derived]
            impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
                fn definition(types: &mut #crate_ref::TypeCollection) -> #crate_ref::datatype::DataType {
                    #crate_ref::internal::register(
                        types,
                        #name.into(),
                        #comments.into(),
                        #deprecated,
                        SID,
                        #impl_location,
                        |types| #ident::<#inline_generics_def>::___specta_definition___(types),
                    );

                    #crate_ref::datatype::reference::Reference::construct(SID, [#(#reference_generics),*], #inline).into()
                }
            }

            #[automatically_derived]
            impl #bounds #crate_ref::NamedType for #ident #type_args #where_bound {
                const ID: #crate_ref::SpectaID = SID;
            }

            #flatten_impl

            #export
        };

    }.into())
}
