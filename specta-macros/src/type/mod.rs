use attr::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use r#enum::parse_enum;
use r#struct::parse_struct;
use syn::{parse, Data, DeriveInput};

use crate::utils::{parse_attrs, unraw_raw_ident, AttributeValue};

use self::generics::{
    add_type_to_where_clause, generics_with_ident_and_bounds_only, generics_with_ident_only,
};

pub(crate) mod attr;
mod r#enum;
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
        Data::Struct(data) => {
            parse_struct(&name, &container_attrs, generics, &crate_ref, data)
        }
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

    let definition_generics = generics.type_params().map(|param| {
        let ident = param.ident.to_string();
        quote!(#crate_ref::internal::construct::generic_data_type(#ident))
    });

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let where_bound = add_type_to_where_clause(&quote!(#crate_ref::Type), generics);

    let flatten_impl = can_flatten.then(|| {
        quote! {
            #[automatically_derived]
            impl #bounds #crate_ref::Flatten for #ident #type_args #where_bound {}
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
    let deprecated = container_attrs.common.deprecated_as_tokens(&crate_ref);

    let impl_location = quote!(#crate_ref::internal::construct::impl_location(concat!(file!(), ":", line!(), ":", column!())));

    Ok(quote! {
        const _: () = {
	        const IMPL_LOCATION: #crate_ref::ImplLocation = #impl_location;
			const DEFINITION_GENERICS: &[#crate_ref::datatype::DataType] = &[#(#definition_generics),*];
			const SID: #crate_ref::SpectaID = #crate_ref::internal::construct::sid(#name, concat!("::", module_path!(), ":", line!(), ":", column!()));

            fn internal_inline(type_map: &mut #crate_ref::TypeCollection, generics: &[#crate_ref::datatype::DataType]) -> #crate_ref::datatype::DataType {
                #inlines
            }

            #[automatically_derived]
             impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
                fn definition(type_map: &mut #crate_ref::TypeCollection) -> #crate_ref::datatype::DataType {
                    {
                        let def = #crate_ref::internal::construct::named_data_type(
                            #name.into(),
                            #comments.into(),
                            #deprecated,
                            SID,
                            IMPL_LOCATION,
                            internal_inline(type_map, DEFINITION_GENERICS)
                        );

                        // TODO: We probs need to handle this for recursive types.
                        // if self.map.get(&sid).is_none() {
                        //     self.map.entry(sid).or_insert(None);
                        //     let dt = T::definition_named_data_type(self);
                        //     self.map.insert(sid, Some(dt));
                        // }

                        type_map.insert(SID, def);
                    }

                    // TODO: Fill in the generics
                    specta::datatype::reference::Reference::construct(SID, vec![]).into()
                }
            }

            #[automatically_derived]
            impl #bounds #crate_ref::NamedType for #ident #type_args #where_bound {
                fn reference(type_map: &mut #crate_ref::TypeCollection) -> #crate_ref::datatype::reference::Reference {
                    <Self as #crate_ref::Type>::definition(type_map);

                    // TODO: Fill in the generics
                    #crate_ref::datatype::reference::Reference::construct(SID, vec![])
                }
            }

            #flatten_impl

            #export
        };

    }.into())
}
