pub(crate) use container::*;
pub(crate) use field::*;
pub(crate) use parse::*;
pub(crate) use rustc::*;
pub(crate) use variant::*;

use parse::{Attribute, AttributeValue};

mod container;
mod field;
mod parse;
mod rustc;
mod variant;

/// Where an attribute was written, used to word `#[specta(...)]` errors.
#[derive(Clone, Copy)]
pub(crate) enum Scope {
    Container,
    Field,
    Variant,
}

impl Scope {
    fn as_str(self) -> &'static str {
        match self {
            Scope::Container => "container",
            Scope::Field => "field",
            Scope::Variant => "variant",
        }
    }
}

/// The `#[serde(...)]` replacement for a removed `#[specta(...)]` attribute.
fn migration_hint(scope: Scope, key: &str) -> Option<String> {
    let replacement = match (scope, key) {
        (Scope::Container, "rename") | (Scope::Field, "rename") | (Scope::Variant, "rename") => {
            Some("#[serde(rename = \"...\")]")
        }
        (Scope::Container, "rename_all") | (Scope::Variant, "rename_all") => {
            Some("#[serde(rename_all = \"...\")]")
        }
        (Scope::Container, "tag") => Some("#[serde(tag = \"...\")]"),
        (Scope::Container, "content") => Some("#[serde(content = \"...\")]"),
        (Scope::Container, "untagged") => Some("#[serde(untagged)]"),
        (Scope::Field, "flatten") => Some("#[serde(flatten)]"),
        (Scope::Field, "skip_serializing") | (Scope::Variant, "skip_serializing") => {
            Some("#[serde(skip_serializing)]")
        }
        (Scope::Field, "skip_deserializing") | (Scope::Variant, "skip_deserializing") => {
            Some("#[serde(skip_deserializing)]")
        }
        (Scope::Field, "skip_serializing_if") => Some("#[serde(skip_serializing_if = \"...\")]"),
        _ => None,
    }?;

    Some(format!(
        "specta: `#[specta({key} ...)]` is no longer supported on {scope}s. Use `{replacement}` instead.",
        scope = scope.as_str()
    ))
}

pub(crate) fn reject_unknown_specta_attrs(attrs: &[Attribute], scope: Scope) -> syn::Result<()> {
    let Some(attr) = attrs.iter().find(|attr| attr.source == "specta") else {
        return Ok(());
    };

    match &attr.value {
        None
        | Some(AttributeValue::Lit(_))
        | Some(AttributeValue::Path(_))
        | Some(AttributeValue::Expr(_))
        | Some(AttributeValue::Type(_)) => Err(syn::Error::new(
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
