pub use container::*;
pub use field::*;
pub use legacy::*;
pub use rustc::*;
pub use variant::*;

use crate::utils::{Attribute, AttributeValue};

mod container;
mod field;
mod legacy;
mod rustc;
mod variant;

pub fn reject_unknown_specta_attrs(attrs: &[Attribute], scope: Scope) -> syn::Result<()> {
    let Some(attr) = attrs.iter().find(|attr| attr.source == "specta") else {
        return Ok(());
    };

    match &attr.value {
        None
        | Some(AttributeValue::Lit(_))
        | Some(AttributeValue::Path(_))
        | Some(AttributeValue::Expr(_)) => Err(syn::Error::new(
            attr.key.span(),
            "specta: invalid formatted attribute",
        )),
        Some(AttributeValue::Attribute {
            attr: inner_attrs, ..
        }) => {
            if let Some(inner_attr) = inner_attrs.first() {
                if let Some(message) = migration_hint(scope, &inner_attr.key.to_string()) {
                    return Err(syn::Error::new(inner_attr.key.span(), message));
                }

                return Err(syn::Error::new(
                    inner_attr.key.span(),
                    format!(
                        "specta: Found unsupported {} attribute '{}'",
                        scope.as_str(),
                        inner_attr.key
                    ),
                ));
            }

            Err(syn::Error::new(
                attr.key.span(),
                "specta: invalid formatted attribute",
            ))
        }
    }
}
