use syn::{Result, Type, TypePath};

use crate::utils::{AttrExtract, Attribute};

use super::RustCAttr;

#[derive(Default)]
pub struct FieldAttr {
    pub r#type: Option<Type>,
    pub inline: bool,
    pub skip: bool,
    pub optional: bool,
    pub common: RustCAttr,
}

impl FieldAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        result.common = RustCAttr::from_attrs(attrs)?;

        if let Some(attr) = attrs.extract("specta", "type") {
            result.r#type = result.r#type.take().or(Some(Type::Path(TypePath {
                qself: None,
                path: attr.parse_path()?,
            })));
        }

        if let Some(attr) = attrs.extract("specta", "inline") {
            result.inline = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "skip") {
            result.skip = attr.parse_bool().unwrap_or(true);
        } else if let Some(attr) = attrs.extract("serde", "skip") {
            // We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
            // we make an exception for `#[serde(skip)]` because it's usually used on fields
            // that would fail a `T: Type` so handling it at runtime would prevent your code from compiling.
            result.skip = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "optional") {
            result.optional = attr.parse_bool().unwrap_or(true);
        }

        if let Some(attr) = attrs.extract("specta", "default") {
            result.optional = attr.parse_bool().unwrap_or(true);
        }

        Ok(result)
    }
}
