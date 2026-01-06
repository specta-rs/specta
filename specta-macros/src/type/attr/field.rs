use syn::{Result, Type, TypePath};

use crate::utils::{Attribute, impl_parse};

use super::RustCAttr;

#[derive(Default)]
pub struct FieldAttr {
    pub r#type: Option<Type>,
    pub inline: bool,
    pub skip: bool,
    pub optional: bool,
    pub common: RustCAttr,
}

impl_parse! {
    FieldAttr(attr, out) {
        "type" => out.r#type = out.r#type.take().or(Some(Type::Path(TypePath {
            qself: None,
            path: attr.parse_path()?,
        }))),
        "inline" => out.inline = attr.parse_bool().unwrap_or(true),
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
        "optional" => out.optional = attr.parse_bool().unwrap_or(true),
        "default" => out.optional = attr.parse_bool().unwrap_or(true),
    }
}

impl FieldAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;
        Self::try_from_attrs("specta", attrs, &mut result)?;

        // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
        // we make an exception for `#[serde(skip)]` because it's usually used on fields
        // that would fail a `T: Type` so handling it at runtime would prevent your code compiling.
        result.skip = result.skip || {
            let mut result = SerdeFieldAttr::default();
            SerdeFieldAttr::try_from_attrs("serde", attrs, &mut result)?;
            result.skip
        };

        Ok(result)
    }
}
