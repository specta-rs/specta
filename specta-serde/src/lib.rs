//! [Serde](https://serde.rs) support for Specta
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

use specta::TypeCollection;

mod inflection;
mod parser;
mod repr;

pub use inflection::RenameRule;
pub use parser::{
    merge_container_attrs, merge_field_attrs, merge_variant_attrs, ConversionType,
    SerdeContainerAttrs, SerdeFieldAttrs, SerdeVariantAttrs,
};

/// TODO: Documentation
///
// TODO: Change name of result type
pub fn apply(types: TypeCollection) -> TypeCollection {
    // TODO:
    //  - Validate supported types w/ Serde
    //  - Apply attributes
    //  - Apply repr
    //  - Apply flatten

    types
}

/// TODO: Documentation
///
// TODO: Change name of result type
pub fn apply_phases(types: TypeCollection) -> TypeCollection {
    // TODO: Same as `apply` but with phases applied by duplicating types

    types
}
