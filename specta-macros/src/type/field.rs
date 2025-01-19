use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, Generics, Type};

use super::{ContainerAttr, FieldAttr};

// Construct a field.
pub fn construct_field(crate_ref: &TokenStream, generics: &Generics, container_attrs: &ContainerAttr, attrs: FieldAttr, field_ty: &Type) -> TokenStream {
    let field_ty = attrs.r#type.as_ref().unwrap_or(&field_ty);
    let deprecated = attrs.common.deprecated_as_tokens(crate_ref);
    let optional = attrs.optional;
    let doc = attrs.common.doc;
    let inline = container_attrs.inline || attrs.inline;

    // Skip must be handled by the macro so that we don't try and constrain the inner type to `Type` or `Flatten` traits.
    if attrs.skip {
        return quote!(#crate_ref::internal::construct::skipped_field(
            #optional,
            #deprecated,
            #doc.into()
        ));
    }

    let generics = generics.params.iter().filter_map(|p| match p {
        GenericParam::Const(..) | GenericParam::Lifetime(..) => None,
        GenericParam::Type(p) => {
            let ident = &p.ident;
            let ident_str = p.ident.to_string();

            quote!((std::borrow::Cow::Borrowed(#ident_str).into(), <#ident as #crate_ref::Type>::definition(types))).into()
        }
    });

    let method = attrs.flatten.then(|| quote!(field_flattened)).unwrap_or_else(|| quote!(field));
    let ty = quote!(#crate_ref::internal::construct::#method::<#field_ty>(
        #optional,
        #deprecated,
        #doc.into(),
        #inline,
        &[#(#generics),*],
        types
    ));

    ty
}

/// Given a type `struct S<A, B, C> { ... }` we have two options:
///  - Either we `S<GenericType(A), GenericType(B), GenericType(C)>`.
///    This is probally more reliable but it means bounds on the params cause major problems.
///
///  - We detect anywhere `A`, `B` or `C` are used and replace them.
///    We also need to account for <A as Trait::method` and `A::method`, etc.
///    (this function implements this one)
///
/// With either method using a macro as a type could cause problems but we have to live with that.
pub fn replace_generic(generics: &Generics, ty: &Type) {
    match ty {
        Type::Array(a) => replace_generic(generics, &a.elem),
        // TODO: This finish
        Type::BareFn(type_bare_fn) => todo!("a"),
        Type::Group(type_group) => todo!("b"),
        Type::ImplTrait(type_impl_trait) => todo!("c"),
        Type::Infer(type_infer) => todo!("d"),
        Type::Macro(type_macro) => todo!("e"),
        Type::Never(type_never) => todo!("f"),
        Type::Paren(type_paren) => todo!("g"),
        Type::Path(p) => {
           let mut segments = p.path.segments.iter();



           todo!("i {:?}", segments.next().map(|s| s.ident.to_string()));
        }
        Type::Ptr(type_ptr) => todo!("i"),
        Type::Reference(type_reference) => todo!("j"),
        Type::Slice(type_slice) => todo!("k"),
        Type::TraitObject(type_trait_object) => todo!("l"),
        Type::Tuple(type_tuple) => todo!("m"),
        Type::Verbatim(token_stream) => todo!("n"),
        _ => unreachable!(),
    }
}
