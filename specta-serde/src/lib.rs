//! [Serde](https://serde.rs) support for Specta
//!
//! This crate parses `#[serde(...)]` attributes and applies the necessary transformations to your types.
//! This is possible because the Specta macro crate stores discovered macro attributes in the [specta::DataType] definition of your type.
//!
//! For specific attributes, refer to Serde's [official documentation](https://serde.rs/attributes.html).
//!
//! # Usage
//!
//! ## Transform a TypeCollection in-place
//!
//! ```ignore
//! use specta::TypeCollection;
//! use specta_serde::{apply, SerdeMode};
//!
//! let mut types = TypeCollection::default();
//! // Add your types...
//!
//! // For serialization only
//! apply(&mut types, SerdeMode::Serialize)?;
//!
//! // For deserialization only
//! apply(&mut types, SerdeMode::Deserialize)?;
//!
//! // For both (uses common attributes, skips mode-specific ones)
//! apply(&mut types, SerdeMode::Both)?;
//! ```
//!
//! ## Transform a single DataType
//!
//! ```ignore
//! use specta::DataType;
//! use specta_serde::{apply_to_dt, SerdeMode};
//!
//! let dt = DataType::Primitive(specta::datatype::Primitive::String);
//! let transformed = apply_to_dt(dt, SerdeMode::Serialize)?;
//! ```
//!
//! ## Understanding SerdeMode
//!
//! - `SerdeMode::Serialize`: Apply transformations for serialization (Rust → JSON/etc).
//!   Respects `skip_serializing`, `rename_serialize`, etc.
//!
//! - `SerdeMode::Deserialize`: Apply transformations for deserialization (JSON/etc → Rust).
//!   Respects `skip_deserializing`, `rename_deserialize`, etc.
//!
//! - `SerdeMode::Both`: Apply transformations that work for both directions.
//!   - Uses common attributes like `rename`, `rename_all`, `skip`
//!   - Only skips fields/types that are skipped in BOTH modes
//!   - Ignores mode-specific attributes unless they match in both modes
//!   - Useful when you want a single type definition for bidirectional APIs
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod inflection;
mod repr;
mod serde_attrs;

pub use error::Error;
pub use repr::EnumRepr;
pub use serde_attrs::{SerdeMode, apply_serde_transformations};

use specta::TypeCollection;
use specta::datatype::{
    DataType, Enum, Fields, Generic, NamedReference, Primitive, Reference, Attribute,
    AttributeMeta, AttributeNestedMeta, skip_fields, skip_fields_named,
};
use std::collections::HashSet;

/// Apply Serde attributes to a [TypeCollection] in-place.
///
/// This function validates all types in the collection, then applies serde transformations
/// according to the specified mode.
///
/// # Modes
///
/// - [`SerdeMode::Serialize`]: Apply transformations for serialization (Rust → JSON/etc)
/// - [`SerdeMode::Deserialize`]: Apply transformations for deserialization (JSON/etc → Rust)
/// - [`SerdeMode::Both`]: Apply common transformations (useful for bidirectional APIs)
///
/// The validation ensures:
/// - Map keys are valid types (string/number types)
/// - Internally tagged enums are properly structured
/// - Skip attributes don't result in empty enums
///
/// # Example
/// ```ignore
/// use specta_serde::{apply, SerdeMode};
///
/// let mut types = specta::TypeCollection::default();
/// // For serialization only
/// apply(&mut types, SerdeMode::Serialize)?;
///
/// // For both serialization and deserialization
/// apply(&mut types, SerdeMode::Both)?;
/// ```
pub fn apply(types: &mut TypeCollection, mode: SerdeMode) -> Result<(), Error> {
    // First validate all types before transformation
    for ndt in types.into_unsorted_iter() {
        validate_type(ndt.ty(), types, &[], &mut Default::default())?;
    }

    // Apply transformations to each type in the collection
    let mut transform_error = None;
    let transformed = types.clone().map(|mut ndt| {
        // Apply serde transformations - we validated above so this should succeed
        // Pass the type name for struct tagging
        match serde_attrs::apply_serde_transformations_with_name(ndt.ty(), ndt.name(), mode) {
            Ok(transformed_dt) => {
                ndt.set_ty(transformed_dt);
                ndt
            }
            Err(err) => {
                if transform_error.is_none() {
                    transform_error = Some(err);
                }
                ndt
            }
        }
    });

    if let Some(err) = transform_error {
        return Err(err);
    }

    // Validate transformed types
    for ndt in transformed.into_unsorted_iter() {
        validate_type(ndt.ty(), &transformed, &[], &mut Default::default())?;
    }

    // Replace the original collection with the transformed one
    *types = transformed;

    Ok(())
}

/// Apply Serde attributes to a single [DataType].
///
/// This function takes a DataType, applies serde transformations according to the
/// specified mode, and returns the transformed DataType.
///
/// # Example
/// ```ignore
/// let dt = DataType::Primitive(Primitive::String);
/// let transformed = specta_serde::apply_to_dt(dt, SerdeMode::Serialize)?;
/// ```
pub fn apply_to_dt(dt: DataType, mode: SerdeMode) -> Result<DataType, Error> {
    serde_attrs::apply_serde_transformations(&dt, mode)
}

/// Process a TypeCollection and return transformed types for serialization
///
/// This is a convenience function that creates a new TypeCollection with serde transformations
/// applied for serialization. For in-place transformation, use [`apply`] instead.
///
/// # Example
/// ```ignore
/// let types = specta::TypeCollection::default();
/// let ser_types = specta_serde::process_for_serialization(&types)?;
/// ```
#[doc(hidden)]
pub fn process_for_serialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut cloned = types.clone();
    apply(&mut cloned, SerdeMode::Serialize)?;
    Ok(cloned)
}

/// Process a TypeCollection and return transformed types for deserialization
///
/// This is a convenience function that creates a new TypeCollection with serde transformations
/// applied for deserialization. For in-place transformation, use [`apply`] instead.
///
/// # Example
/// ```ignore
/// let types = specta::TypeCollection::default();
/// let de_types = specta_serde::process_for_deserialization(&types)?;
/// ```
#[doc(hidden)]
pub fn process_for_deserialization(types: &TypeCollection) -> Result<TypeCollection, Error> {
    let mut cloned = types.clone();
    apply(&mut cloned, SerdeMode::Deserialize)?;
    Ok(cloned)
}

/// Process types for both serialization and deserialization
///
/// This is a convenience function that returns separate TypeCollections for serialization
/// and deserialization. For in-place transformation, use [`apply`] instead.
///
/// Returns a tuple of (serialization_types, deserialization_types)
///
/// # Example
/// ```ignore
/// let types = specta::TypeCollection::default();
/// let (ser_types, de_types) = specta_serde::process_for_both(&types)?;
/// ```
#[doc(hidden)]
pub fn process_for_both(types: &TypeCollection) -> Result<(TypeCollection, TypeCollection), Error> {
    let ser_types = process_for_serialization(types)?;
    let de_types = process_for_deserialization(types)?;
    Ok((ser_types, de_types))
}

/// Internal validation function that recursively validates types
fn validate_type(
    dt: &DataType,
    types: &TypeCollection,
    generics: &[(Generic, DataType)],
    checked_references: &mut HashSet<NamedReference>,
) -> Result<(), Error> {
    match dt {
        DataType::Nullable(ty) => validate_type(ty, types, generics, checked_references)?,
        DataType::Map(ty) => {
            is_valid_map_key(ty.key_ty(), types, generics)?;
            validate_type(ty.value_ty(), types, generics, checked_references)?;
        }
        DataType::Struct(ty) => match ty.fields() {
            Fields::Unit => {}
            Fields::Unnamed(ty) => {
                for (_, ty) in skip_fields(ty.fields()) {
                    validate_type(ty, types, generics, checked_references)?;
                }
            }
            Fields::Named(ty) => {
                for (_, (_, ty)) in skip_fields_named(ty.fields()) {
                    validate_type(ty, types, generics, checked_references)?;
                }
            }
        },
        DataType::Enum(ty) => {
            validate_enum(ty, types)?;

            for (_variant_name, variant) in ty.variants().iter() {
                if variant.skip() {
                    continue;
                }

                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Named(variant) => {
                        for (_, (_, ty)) in skip_fields_named(variant.fields()) {
                            validate_type(ty, types, generics, checked_references)?;
                        }
                    }
                    Fields::Unnamed(variant) => {
                        for (_, ty) in skip_fields(variant.fields()) {
                            validate_type(ty, types, generics, checked_references)?;
                        }
                    }
                }
            }
        }
        DataType::Tuple(ty) => {
            for ty in ty.elements() {
                validate_type(ty, types, generics, checked_references)?;
            }
        }
        DataType::List(ty) => {
            validate_type(ty.ty(), types, generics, checked_references)?;
        }
        DataType::Reference(Reference::Named(r)) => {
            for (_, dt) in r.generics() {
                validate_type(dt, types, &[], checked_references)?;
            }

            if !checked_references.contains(r) {
                checked_references.insert(r.clone());
                if let Some(ndt) = r.get(types) {
                    validate_type(ndt.ty(), types, r.generics(), checked_references)?;
                }
            }
        }
        DataType::Reference(Reference::Opaque(_)) => {}
        _ => {}
    }

    Ok(())
}

// Typescript: Must be assignable to `string | number | symbol` says Typescript.
fn is_valid_map_key(
    key_ty: &DataType,
    types: &TypeCollection,
    generics: &[(Generic, DataType)],
) -> Result<(), Error> {
    match key_ty {
        DataType::Primitive(
            Primitive::i8
            | Primitive::i16
            | Primitive::i32
            | Primitive::i64
            | Primitive::i128
            | Primitive::isize
            | Primitive::u8
            | Primitive::u16
            | Primitive::u32
            | Primitive::u64
            | Primitive::u128
            | Primitive::usize
            | Primitive::f32
            | Primitive::f64
            | Primitive::String
            | Primitive::char,
        ) => Ok(()),
        DataType::Primitive(_) => Err(Error::InvalidMapKey),
        // Enum of other valid types are also valid Eg. `"A" | "B"` or `"A" | 5` are valid
        DataType::Enum(ty) => {
            for (_variant_name, variant) in ty.variants() {
                match &variant.fields() {
                    Fields::Unit => {}
                    Fields::Unnamed(item) => {
                        if item.fields().len() > 1 {
                            return Err(Error::InvalidMapKey);
                        }

                        // TODO: Check enum representation for untagged requirement
                        // if *ty.repr().unwrap_or(&EnumRepr::External) != EnumRepr::Untagged {
                        //     return Err(Error::InvalidMapKey);
                        // }
                    }
                    _ => return Err(Error::InvalidMapKey),
                }
            }

            Ok(())
        }
        DataType::Tuple(t) => {
            if t.elements().is_empty() {
                return Err(Error::InvalidMapKey);
            }

            Ok(())
        }
        DataType::Reference(Reference::Named(r)) => {
            if let Some(ndt) = r.get(types) {
                is_valid_map_key(ndt.ty(), types, r.generics())?;
            }
            Ok(())
        }
        DataType::Reference(Reference::Opaque(_)) => Ok(()),
        DataType::Generic(g) => {
            let ty = generics
                .iter()
                .find(|(ge, _)| ge == g)
                .map(|(_, dt)| dt)
                .expect("unable to find expected generic type"); // TODO: Proper error instead of panicking

            is_valid_map_key(ty, types, &[])
        }
        _ => Err(Error::InvalidMapKey),
    }
}

// Serde does not allow serializing a variant of certain enum shapes.
fn validate_enum(e: &Enum, types: &TypeCollection) -> Result<(), Error> {
    if matches!(get_enum_repr(e.attributes()), EnumRepr::Internal { .. }) {
        validate_internally_tag_enum(e, types, &mut Default::default())?;
    }

    Ok(())
}

fn validate_internally_tag_enum(
    e: &Enum,
    types: &TypeCollection,
    checked_references: &mut HashSet<NamedReference>,
) -> Result<(), Error> {
    for (_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        match &variant.fields() {
            Fields::Unit | Fields::Named(_) => {}
            Fields::Unnamed(item) => {
                let mut fields = skip_fields(item.fields());

                let Some((_, first_field)) = fields.next() else {
                    continue;
                };

                if fields.next().is_some() {
                    return Err(Error::InvalidInternallyTaggedEnum);
                }

                validate_internally_tag_enum_datatype(first_field, types, checked_references)?;
            }
        }
    }

    Ok(())
}

fn validate_internally_tag_enum_datatype(
    ty: &DataType,
    types: &TypeCollection,
    checked_references: &mut HashSet<NamedReference>,
) -> Result<(), Error> {
    match ty {
        DataType::Map(_) => Ok(()),
        DataType::Struct(ty) => match ty.fields() {
            Fields::Unit | Fields::Named(_) => Ok(()),
            Fields::Unnamed(unnamed) => {
                if !is_transparent_struct(ty.attributes()) {
                    return Err(Error::InvalidInternallyTaggedEnum);
                }

                let mut fields = skip_fields(unnamed.fields());

                let Some((_, inner_field)) = fields.next() else {
                    return Ok(());
                };

                if fields.next().is_some() {
                    return Err(Error::InvalidInternallyTaggedEnum);
                }

                validate_internally_tag_enum_datatype(inner_field, types, checked_references)
            }
        },
        DataType::Enum(ty) => match get_enum_repr(ty.attributes()) {
            EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } => Ok(()),
            EnumRepr::Untagged => {
                for (_variant_name, variant) in ty.variants() {
                    match variant.fields() {
                        Fields::Unit | Fields::Named(_) => {}
                        Fields::Unnamed(unnamed) => {
                            let mut fields = skip_fields(unnamed.fields());

                            let Some((_, inner_field)) = fields.next() else {
                                continue;
                            };

                            if fields.next().is_some() {
                                return Err(Error::InvalidInternallyTaggedEnum);
                            }

                            validate_internally_tag_enum_datatype(
                                inner_field,
                                types,
                                checked_references,
                            )?;
                        }
                    }
                }

                Ok(())
            }
            EnumRepr::External | EnumRepr::String { .. } => Err(Error::InvalidInternallyTaggedEnum),
        },
        DataType::Tuple(ty) if ty.elements().is_empty() => Ok(()),
        DataType::Reference(Reference::Named(r)) => {
            if !checked_references.contains(r) {
                checked_references.insert(r.clone());
                if let Some(ndt) = r.get(types) {
                    validate_internally_tag_enum_datatype(ndt.ty(), types, checked_references)?;
                }
            }

            Ok(())
        }
        DataType::Nullable(ty) => {
            validate_internally_tag_enum_datatype(ty, types, checked_references)
        }
        DataType::Reference(Reference::Opaque(_)) | DataType::Generic(_) => Ok(()),
        _ => Err(Error::InvalidInternallyTaggedEnum),
    }
}

fn is_transparent_struct(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attr| {
        if attr.path != "serde" && attr.path != "specta" {
            return false;
        }

        match &attr.kind {
            AttributeMeta::Path(path) => path == "transparent",
            AttributeMeta::List(items) => items.iter().any(|item| {
                matches!(item, AttributeNestedMeta::Meta(AttributeMeta::Path(path)) if path == "transparent")
            }),
            AttributeMeta::NameValue { .. } => false,
        }
    })
}

/// Check if a field has the `#[serde(flatten)]` attribute
///
/// This is a utility function for exporters that need to handle flattened fields.
/// It checks both `#[serde(flatten)]` and `#[specta(flatten)]` attributes.
///
/// # Example
/// ```ignore
/// use specta::datatype::Field;
/// use specta_serde::is_field_flattened;
///
/// fn process_field(field: &Field) {
///     if is_field_flattened(field) {
///         // Handle flattened field
///     } else {
///         // Handle regular field
///     }
/// }
/// ```
pub fn is_field_flattened(field: &specta::datatype::Field) -> bool {
    use specta::datatype::{AttributeMeta, AttributeNestedMeta};

    field.attributes().iter().any(|attr| {
        if attr.path == "serde" || attr.path == "specta" {
            match &attr.kind {
                AttributeMeta::Path(path) => path == "flatten",
                AttributeMeta::List(items) => items.iter().any(|item| {
                    matches!(item, AttributeNestedMeta::Meta(AttributeMeta::Path(path)) if path == "flatten")
                }),
                _ => false,
            }
        } else {
            false
        }
    })
}

/// Get the enum representation from serde attributes
///
/// This function parses `#[serde(tag = "...")]`, `#[serde(content = "...")]`,
/// and `#[serde(untagged)]` attributes to determine the enum representation.
///
/// Returns `EnumRepr::External` by default if no representation attributes are found.
///
/// # Example
/// ```ignore
/// use specta::datatype::Enum;
/// use specta_serde::{get_enum_repr, EnumRepr};
///
/// fn process_enum(e: &Enum) {
///     let repr = get_enum_repr(e.attributes());
///     match repr {
///         EnumRepr::External => { /* handle external */ },
///         EnumRepr::Internal { tag } => { /* handle internal */ },
///         EnumRepr::Adjacent { tag, content } => { /* handle adjacent */ },
///         EnumRepr::Untagged => { /* handle untagged */ },
///         _ => {}
///     }
/// }
/// ```
pub fn get_enum_repr(attributes: &[specta::datatype::Attribute]) -> EnumRepr {
    use specta::datatype::{AttributeLiteral, AttributeMeta, AttributeNestedMeta, AttributeValue};
    use std::borrow::Cow;

    let mut tag = None;
    let mut content = None;
    let mut untagged = false;

    fn parse_repr_from_meta(
        meta: &AttributeMeta,
        tag: &mut Option<String>,
        content: &mut Option<String>,
        untagged: &mut bool,
    ) {
        match meta {
            AttributeMeta::Path(path) => {
                if path == "untagged" {
                    *untagged = true;
                }
            }
            AttributeMeta::NameValue { key, value } => {
                if key == "tag" {
                    if let AttributeValue::Literal(AttributeLiteral::Str(t)) = value {
                        *tag = Some(t.clone());
                    }
                } else if key == "content"
                    && let AttributeValue::Literal(AttributeLiteral::Str(c)) = value
                {
                    *content = Some(c.clone());
                }
            }
            AttributeMeta::List(list) => {
                for nested in list {
                    if let AttributeNestedMeta::Meta(nested_meta) = nested {
                        parse_repr_from_meta(nested_meta, tag, content, untagged);
                    }
                }
            }
        }
    }

    for attr in attributes {
        if attr.path == "serde" {
            parse_repr_from_meta(&attr.kind, &mut tag, &mut content, &mut untagged);
        }
    }

    if let (Some(tag_name), Some(content_name)) = (tag.clone(), content.clone()) {
        EnumRepr::Adjacent {
            tag: Cow::Owned(tag_name),
            content: Cow::Owned(content_name),
        }
    } else if let Some(tag_name) = tag {
        EnumRepr::Internal {
            tag: Cow::Owned(tag_name),
        }
    } else if untagged {
        EnumRepr::Untagged
    } else {
        EnumRepr::External
    }
}
