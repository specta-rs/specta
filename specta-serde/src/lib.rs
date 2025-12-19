//! [Serde](https://serde.rs) support for Specta
//!
//! This crates takes the types with there attribute metadata returned from the macros and validates that the type is a valid Serde type and then applies any transformations needed.
//!
//! # Usage
//!
//! ```
//! let types = specta::TypeCollection::default();
//! specta_serde::validate(&types).unwrap();
//! // Use your `types` as normal with a language exporter
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod inflection;
mod repr;
pub mod serde_attrs;
mod validate;

pub use error::Error;
pub use serde_attrs::{SerdeMode, apply_serde_transformations};
pub use validate::{validate, validate_with_serde_deserialize, validate_with_serde_serialize};

use specta::TypeCollection;
use specta::builder::NamedDataTypeBuilder;

/// Process a TypeCollection and return transformed types for serialization
///
/// This function takes a TypeCollection, applies serde transformations for serialization,
/// and returns a new TypeCollection with the transformed types.
pub fn process_for_serialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut new_types = TypeCollection::default();

    for ndt in types.into_unsorted_iter() {
        let transformed_dt = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize)?;

        // Create a new NamedDataType with the transformed DataType using the builder
        let builder =
            NamedDataTypeBuilder::new(ndt.name().clone(), ndt.generics().to_vec(), transformed_dt)
                .docs(ndt.docs().clone())
                .module_path(ndt.module_path().clone());

        // Set deprecated if present
        let builder = if let Some(deprecated) = ndt.deprecated() {
            builder.deprecated(deprecated.clone())
        } else {
            builder
        };

        builder.build(&mut new_types);
    }

    Ok(new_types)
}

/// Process a TypeCollection and return transformed types for deserialization
///
/// This function takes a TypeCollection, applies serde transformations for deserialization,
/// and returns a new TypeCollection with the transformed types.
pub fn process_for_deserialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut new_types = TypeCollection::default();

    for ndt in types.into_unsorted_iter() {
        let transformed_dt = apply_serde_transformations(ndt.ty(), SerdeMode::Deserialize)?;

        // Create a new NamedDataType with the transformed DataType using the builder
        let builder =
            NamedDataTypeBuilder::new(ndt.name().clone(), ndt.generics().to_vec(), transformed_dt)
                .docs(ndt.docs().clone())
                .module_path(ndt.module_path().clone());

        // Set deprecated if present
        let builder = if let Some(deprecated) = ndt.deprecated() {
            builder.deprecated(deprecated.clone())
        } else {
            builder
        };

        builder.build(&mut new_types);
    }

    Ok(new_types)
}

/// Convenience function to process types for both serialization and deserialization
///
/// Returns a tuple of (serialization_types, deserialization_types)
pub fn process_for_both(types: &TypeCollection) -> Result<(TypeCollection, TypeCollection), Error> {
    let ser_types = process_for_serialization(types)?;
    let de_types = process_for_deserialization(types)?;
    Ok((ser_types, de_types))
}
