// We generally want `#[serde(...)]` attributes to only be handled by the runtime but,
// we make an exception for `#[serde(skip)]` because it's usually used on fields
// that would fail a `T: Type` so handling it at runtime would prevent your code compiling.

use crate::utils::impl_parse;

#[derive(Default)]
pub struct SerdeFieldAttr {
    pub skip: bool,
}

impl_parse! {
    SerdeFieldAttr(attr, out) {
        "skip" => out.skip = attr.parse_bool().unwrap_or(true),
    }
}
