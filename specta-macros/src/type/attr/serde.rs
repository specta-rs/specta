// We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
// we make an exception for `#[serde(skip)]` because it's usually used on fields
// that would fail a `T: Type` so handling it at runtime would prevent your code compiling.

use crate::utils::{AttrExtract, Attribute};

#[derive(Default)]
pub struct SerdeFieldAttr {
    pub skip: bool,
}

impl SerdeFieldAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Self {
        let mut result = Self::default();
        if let Some(attr) = attrs.extract("serde", "skip") {
            result.skip = attr.parse_bool().unwrap_or(true);
        }
        result
    }
}
