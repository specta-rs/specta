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
    DataType, Enum, Fields, Generic, Primitive, Reference, skip_fields, skip_fields_named,
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
    let transformed = types.clone().map(|mut ndt| {
        // Apply serde transformations - we validated above so this should succeed
        // Pass the type name for struct tagging
        match serde_attrs::apply_serde_transformations_with_name(ndt.ty(), ndt.name(), mode) {
            Ok(transformed_dt) => {
                ndt.set_ty(transformed_dt);
                ndt
            }
            Err(_) => {
                // This shouldn't happen since we validated, but return unchanged if it does
                ndt
            }
        }
    });

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
    checked_references: &mut HashSet<Reference>,
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
        DataType::Reference(r) => {
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
        DataType::Reference(r) => {
            if let Some(ndt) = r.get(types) {
                is_valid_map_key(ndt.ty(), types, r.generics())?;
            }
            Ok(())
        }
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

// Serde does not allow serializing a variant of certain types of enum's.
fn validate_enum(e: &Enum, _types: &TypeCollection) -> Result<(), Error> {
    // You can't `#[serde(skip)]` your way to an empty enum.
    let valid_variants = e.variants().iter().filter(|(_, v)| !v.skip()).count();
    if valid_variants == 0 && !e.variants().is_empty() {
        return Err(Error::InvalidUsageOfSkip);
    }

    // TODO: Implement internally tagged enum validation
    // Only internally tagged enums can be invalid.
    // if let Some(EnumRepr::Internal { .. }) = get_enum_repr_from_attributes(e.attributes()) {
    //     validate_internally_tag_enum(e, types)?;
    // }

    Ok(())
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
    use specta::datatype::{RuntimeMeta, RuntimeNestedMeta};

    field.attributes().iter().any(|attr| {
        if attr.path == "serde" || attr.path == "specta" {
            match &attr.kind {
                RuntimeMeta::Path(path) => path == "flatten",
                RuntimeMeta::List(items) => items.iter().any(|item| {
                    matches!(item, RuntimeNestedMeta::Meta(RuntimeMeta::Path(path)) if path == "flatten")
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
pub fn get_enum_repr(attributes: &[specta::datatype::RuntimeAttribute]) -> EnumRepr {
    use specta::datatype::{RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta};
    use std::borrow::Cow;

    let mut tag = None;
    let mut content = None;
    let mut untagged = false;

    fn parse_repr_from_meta(
        meta: &RuntimeMeta,
        tag: &mut Option<String>,
        content: &mut Option<String>,
        untagged: &mut bool,
    ) {
        match meta {
            RuntimeMeta::Path(path) => {
                if path == "untagged" {
                    *untagged = true;
                }
            }
            RuntimeMeta::NameValue { key, value } => {
                if key == "tag" {
                    if let RuntimeLiteral::Str(t) = value {
                        *tag = Some(t.clone());
                    }
                } else if key == "content"
                    && let RuntimeLiteral::Str(c) = value
                {
                    *content = Some(c.clone());
                }
            }
            RuntimeMeta::List(list) => {
                for nested in list {
                    if let RuntimeNestedMeta::Meta(nested_meta) = nested {
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

/// Apply enum representation transformations to a DataType recursively.
///
/// This function parses enum repr from serde attributes and transforms
/// the DataType structure to match the serialized JSON structure:
/// - **External**: Wraps variant content with variant name as key (e.g., `{ "VariantName": content }`)
/// - **Untagged**: No transformation - content serialized as-is
/// - **Internal**: Injects tag field into variant fields (e.g., `{ tag: "VariantName", ...fields }`)
/// - **Adjacent**: Wraps in tag + content structure (e.g., `{ tag: "VariantName", content: {...} }`)
/// - **String**: No transformation (unit-only enums)
///
/// The function recursively applies transformations to nested types.
///
/// # Example
/// ```ignore
/// use specta::DataType;
/// use specta_serde::{apply_repr, SerdeMode};
///
/// let dt = DataType::Enum(my_enum);
/// let transformed = apply_repr(dt, SerdeMode::Serialize)?;
/// ```
///
/// # Usage with other transformations
/// ```ignore
/// // Apply renaming first, then repr
/// let dt = apply_serde_transformations(dt, SerdeMode::Serialize)?;
/// let dt = apply_repr(dt, SerdeMode::Serialize)?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - An internally tagged enum has invalid structure (e.g., tuple variant with multiple fields)
/// - All variants in an enum are skipped
pub fn apply_repr(dt: DataType, mode: SerdeMode) -> Result<DataType, Error> {
    use specta::datatype::{List, Map, Struct, Tuple};

    match dt {
        // Primitive types - no transformation
        DataType::Primitive(_) => Ok(dt),

        // Container types - recursively apply to children
        DataType::List(list) => {
            let transformed = apply_repr(list.ty().clone(), mode)?;
            Ok(DataType::List(List::new(transformed)))
        }
        DataType::Map(map) => {
            let key = apply_repr(map.key_ty().clone(), mode)?;
            let value = apply_repr(map.value_ty().clone(), mode)?;
            Ok(DataType::Map(Map::new(key, value)))
        }
        DataType::Nullable(inner) => {
            let transformed = apply_repr(*inner, mode)?;
            Ok(DataType::Nullable(Box::new(transformed)))
        }
        DataType::Tuple(tuple) => {
            let elements: Result<Vec<_>, _> = tuple
                .elements()
                .iter()
                .map(|e| apply_repr(e.clone(), mode))
                .collect();
            Ok(DataType::Tuple(Tuple::new(elements?)))
        }

        // Struct - recursively apply to field types
        DataType::Struct(s) => {
            let transformed_fields = apply_repr_to_fields(s.fields(), mode)?;
            let mut new_struct = Struct::unit();
            new_struct.set_fields(transformed_fields);
            new_struct.set_attributes(s.attributes().clone());
            Ok(DataType::Struct(new_struct))
        }

        // Enum - APPLY REPR TRANSFORMATION + recursively to variants
        DataType::Enum(e) => {
            let transformed_enum = apply_repr_to_enum(&e, mode)?;
            Ok(DataType::Enum(transformed_enum))
        }

        // References and generics - no transformation (resolved elsewhere)
        DataType::Reference(_) | DataType::Generic(_) => Ok(dt),
    }
}

/// Apply repr transformation to an enum
fn apply_repr_to_enum(enum_type: &Enum, mode: SerdeMode) -> Result<Enum, Error> {
    // Parse repr from attributes
    let repr = get_enum_repr(enum_type.attributes());

    let mut new_variants = Vec::new();
    let mut non_skipped_count = 0;

    for (variant_name, variant) in enum_type.variants() {
        // Check if variant should be skipped (both explicit skip flag and attributes)
        let variant_attrs = serde_attrs::parse_serde_attributes(variant.attributes())?;
        let should_skip = variant.skip() || should_skip_variant(&variant_attrs, mode);

        if should_skip {
            // Keep the variant but don't transform it
            new_variants.push((variant_name.clone(), variant.clone()));
            continue;
        }

        non_skipped_count += 1;

        // Apply repr transformation to this variant's fields
        let transformed_fields =
            apply_repr_to_variant_fields(&repr, variant_name, variant.fields(), mode)?;

        // Recursively apply repr to nested types in the fields
        let recursive_fields = apply_repr_to_fields(&transformed_fields, mode)?;

        let mut new_variant = variant.clone();
        new_variant.set_fields(recursive_fields);

        new_variants.push((variant_name.clone(), new_variant));
    }

    // Validate that at least one variant is not skipped
    if non_skipped_count == 0 && !new_variants.is_empty() {
        return Err(Error::InvalidUsageOfSkip);
    }

    let mut new_enum = Enum::new();
    *new_enum.variants_mut() = new_variants;
    *new_enum.attributes_mut() = enum_type.attributes().clone();

    Ok(new_enum)
}

/// Apply repr transformation to variant fields based on repr type
fn apply_repr_to_variant_fields(
    repr: &EnumRepr,
    variant_name: &std::borrow::Cow<'static, str>,
    fields: &Fields,
    _mode: SerdeMode,
) -> Result<Fields, Error> {
    match repr {
        EnumRepr::External => apply_external_repr(variant_name, fields),
        EnumRepr::Untagged => Ok(fields.clone()), // No transformation
        EnumRepr::Internal { tag } => apply_internal_repr(tag, variant_name, fields),
        EnumRepr::Adjacent { tag, content } => {
            apply_adjacent_repr(tag, content, variant_name, fields)
        }
        EnumRepr::String { .. } => Ok(fields.clone()), // No transformation
    }
}

/// Helper to recursively apply repr to fields within a Fields structure
fn apply_repr_to_fields(fields: &Fields, mode: SerdeMode) -> Result<Fields, Error> {
    use specta::internal;

    match fields {
        Fields::Unit => Ok(Fields::Unit),
        Fields::Unnamed(unnamed) => {
            let transformed: Result<Vec<_>, _> = unnamed
                .fields()
                .iter()
                .map(|field| {
                    if let Some(ty) = field.ty() {
                        let transformed_ty = apply_repr(ty.clone(), mode)?;
                        let mut new_field = field.clone();
                        new_field.set_ty(transformed_ty);
                        Ok(new_field)
                    } else {
                        Ok(field.clone())
                    }
                })
                .collect();
            Ok(internal::construct::fields_unnamed(transformed?, vec![]))
        }
        Fields::Named(named) => {
            let transformed: Result<Vec<_>, _> = named
                .fields()
                .iter()
                .map(|(name, field)| {
                    if let Some(ty) = field.ty() {
                        let transformed_ty = apply_repr(ty.clone(), mode)?;
                        let mut new_field = field.clone();
                        new_field.set_ty(transformed_ty);
                        Ok((name.clone(), new_field))
                    } else {
                        Ok((name.clone(), field.clone()))
                    }
                })
                .collect();
            Ok(internal::construct::fields_named(transformed?, vec![]))
        }
    }
}

/// Transform External repr: wrap content with variant name
fn apply_external_repr(
    variant_name: &std::borrow::Cow<'static, str>,
    fields: &Fields,
) -> Result<Fields, Error> {
    use specta::datatype::{Field, Struct, Tuple};
    use specta::internal;

    match fields {
        Fields::Unit => {
            // Unit stays as Unit - serializes as string "VariantName"
            Ok(Fields::Unit)
        }
        Fields::Named(named) => {
            // Wrap in { "VariantName": { ...fields } }
            let mut wrapper_struct = Struct::unit();
            wrapper_struct.set_fields(Fields::Named(named.clone()));

            let wrapper_field = Field::new(DataType::Struct(wrapper_struct));
            Ok(internal::construct::fields_named(
                vec![(variant_name.clone(), wrapper_field)],
                vec![],
            ))
        }
        Fields::Unnamed(unnamed) => {
            // Wrap in { "VariantName": [...fields] }
            let tuple_types: Vec<DataType> = unnamed
                .fields()
                .iter()
                .filter_map(|f| f.ty().cloned())
                .collect();

            let content_type = if tuple_types.len() == 1 {
                // Single field: unwrap tuple
                tuple_types.into_iter().next().unwrap()
            } else {
                // Multiple fields: keep as tuple
                DataType::Tuple(Tuple::new(tuple_types))
            };

            let wrapper_field = Field::new(content_type);
            Ok(internal::construct::fields_named(
                vec![(variant_name.clone(), wrapper_field)],
                vec![],
            ))
        }
    }
}

/// Transform Internal repr: inject tag field
fn apply_internal_repr(
    tag_name: &str,
    variant_name: &std::borrow::Cow<'static, str>,
    fields: &Fields,
) -> Result<Fields, Error> {
    use specta::datatype::Field;
    use specta::internal;
    use std::borrow::Cow;

    let tag_type = create_literal_string_type(variant_name);

    match fields {
        Fields::Unit => {
            // Unit → { tag: "VariantName" }
            let tag_field = Field::new(tag_type);
            Ok(internal::construct::fields_named(
                vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                vec![],
            ))
        }
        Fields::Named(named) => {
            // Add tag field to existing named fields
            let mut new_fields = vec![];
            let tag_field = Field::new(tag_type);
            new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

            for (name, field) in named.fields() {
                new_fields.push((name.clone(), field.clone()));
            }

            Ok(internal::construct::fields_named(new_fields, vec![]))
        }
        Fields::Unnamed(unnamed) => {
            // Check if single field that's a struct/map (unwrappable)
            if unnamed.fields().len() == 1 {
                if let Some(field_ty) = unnamed.fields()[0].ty() {
                    match field_ty {
                        DataType::Struct(s) => {
                            // Unwrap: merge tag into the struct
                            match s.fields() {
                                Fields::Named(inner_named) => {
                                    let mut new_fields = vec![];
                                    let tag_field = Field::new(tag_type);
                                    new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

                                    for (name, field) in inner_named.fields() {
                                        new_fields.push((name.clone(), field.clone()));
                                    }

                                    return Ok(internal::construct::fields_named(
                                        new_fields,
                                        vec![],
                                    ));
                                }
                                _ => {} // Fall through to error
                            }
                        }
                        DataType::Map(_) => {
                            // Maps can be internally tagged (tag is added to map)
                            // For now, treat as valid - actual serialization handles this
                            let tag_field = Field::new(tag_type);
                            return Ok(internal::construct::fields_named(
                                vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                                vec![],
                            ));
                        }
                        _ => {} // Fall through to error
                    }
                }
            }

            // Multiple fields or non-struct/map single field - error
            Err(Error::InvalidInternallyTaggedEnum)
        }
    }
}

/// Transform Adjacent repr: wrap in tag + content
fn apply_adjacent_repr(
    tag_name: &str,
    content_name: &str,
    variant_name: &std::borrow::Cow<'static, str>,
    fields: &Fields,
) -> Result<Fields, Error> {
    use specta::datatype::{Field, Struct, Tuple};
    use specta::internal;
    use std::borrow::Cow;

    let tag_type = create_literal_string_type(variant_name);

    match fields {
        Fields::Unit => {
            // Unit → { tag: "VariantName" } (no content)
            let tag_field = Field::new(tag_type);
            Ok(internal::construct::fields_named(
                vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                vec![],
            ))
        }
        Fields::Named(named) => {
            // { tag: "VariantName", content: { ...fields } }
            let mut new_fields = vec![];

            let tag_field = Field::new(tag_type);
            new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

            let mut content_struct = Struct::unit();
            content_struct.set_fields(Fields::Named(named.clone()));
            let content_field = Field::new(DataType::Struct(content_struct));
            new_fields.push((Cow::Owned(content_name.to_string()), content_field));

            Ok(internal::construct::fields_named(new_fields, vec![]))
        }
        Fields::Unnamed(unnamed) => {
            // { tag: "VariantName", content: tuple }
            let mut new_fields = vec![];

            let tag_field = Field::new(tag_type);
            new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

            let tuple_types: Vec<DataType> = unnamed
                .fields()
                .iter()
                .filter_map(|f| f.ty().cloned())
                .collect();

            let content_type = if tuple_types.len() == 1 {
                tuple_types.into_iter().next().unwrap()
            } else {
                DataType::Tuple(Tuple::new(tuple_types))
            };

            let content_field = Field::new(content_type);
            new_fields.push((Cow::Owned(content_name.to_string()), content_field));

            Ok(internal::construct::fields_named(new_fields, vec![]))
        }
    }
}

/// Create a literal string type (single-variant enum)
fn create_literal_string_type(value: &str) -> DataType {
    use specta::datatype::EnumVariant;
    use std::borrow::Cow;

    let mut literal_enum = Enum::new();
    let unit_variant = EnumVariant::unit();
    *literal_enum.variants_mut() = vec![(Cow::Owned(value.to_string()), unit_variant)];
    DataType::Enum(literal_enum)
}

/// Check if a variant should be skipped based on the current mode
fn should_skip_variant(attrs: &serde_attrs::SerdeAttributes, mode: SerdeMode) -> bool {
    if attrs.skip {
        return true;
    }

    match mode {
        SerdeMode::Serialize => attrs.skip_serializing,
        SerdeMode::Deserialize => attrs.skip_deserializing,
        // For Both mode, only skip if skipped in both directions
        SerdeMode::Both => attrs.skip_serializing && attrs.skip_deserializing,
    }
}

// TODO: Implement these validation functions once enum representation parsing is complete
// fn validate_internally_tag_enum(e: &Enum, types: &TypeCollection) -> Result<(), Error> {
//     for (_variant_name, variant) in e.variants() {
//         match &variant.fields() {
//             Fields::Unit => {}
//             Fields::Named(_) => {}
//             Fields::Unnamed(item) => {
//                 let mut fields = skip_fields(item.fields());
//
//                 let Some(first_field) = fields.next() else {
//                     continue;
//                 };
//
//                 if fields.next().is_some() {
//                     return Err(Error::InvalidInternallyTaggedEnum);
//                 }
//
//                 validate_internally_tag_enum_datatype(first_field.1, types)?;
//             }
//         }
//     }
//
//     Ok(())
// }

// fn validate_internally_tag_enum_datatype(
//     ty: &DataType,
//     types: &TypeCollection,
// ) -> Result<(), Error> {
//     match ty {
//         DataType::Map(_) => {}
//         DataType::Struct(_) => {}
//         DataType::Enum(ty) => {
//             // TODO: Check enum representation
//         }
//         DataType::Tuple(ty) if ty.elements().is_empty() => {}
//         DataType::Reference(r) => {
//             if let Some(ndt) = r.get(types) {
//                 validate_internally_tag_enum_datatype(ndt.ty(), types)?;
//             }
//         }
//         _ => return Err(Error::InvalidInternallyTaggedEnum),
//     }
//
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;
    use specta::datatype::{
        EnumVariant, Field, Primitive, RuntimeAttribute, RuntimeLiteral, RuntimeMeta,
    };
    use std::borrow::Cow;

    /// Helper to create a simple enum with variants
    fn create_test_enum(variants: Vec<(String, Fields)>) -> Enum {
        let mut e = Enum::new();
        *e.variants_mut() = variants
            .into_iter()
            .map(|(name, fields)| {
                let mut variant = EnumVariant::unit();
                variant.set_fields(fields);
                (Cow::Owned(name), variant)
            })
            .collect();
        e
    }

    /// Helper to create enum with serde attributes
    fn create_enum_with_attrs(
        variants: Vec<(String, Fields)>,
        attrs: Vec<RuntimeAttribute>,
    ) -> Enum {
        let mut e = create_test_enum(variants);
        *e.attributes_mut() = attrs;
        e
    }

    /// Helper to create a tag attribute
    fn tag_attr(tag: &str) -> RuntimeAttribute {
        RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::NameValue {
                key: "tag".to_string(),
                value: RuntimeLiteral::Str(tag.to_string()),
            },
        }
    }

    /// Helper to create a content attribute
    fn content_attr(content: &str) -> RuntimeAttribute {
        RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::NameValue {
                key: "content".to_string(),
                value: RuntimeLiteral::Str(content.to_string()),
            },
        }
    }

    /// Helper to create an untagged attribute
    fn untagged_attr() -> RuntimeAttribute {
        RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::Path("untagged".to_string()),
        }
    }

    #[test]
    fn test_external_repr_unit_variant() {
        // Unit variant should remain unchanged for External
        let e = create_test_enum(vec![("Active".to_string(), Fields::Unit)]);
        let dt = DataType::Enum(e);

        let result = apply_repr(dt.clone(), SerdeMode::Serialize).unwrap();

        // Should still be Unit for External
        if let DataType::Enum(transformed) = result {
            assert_eq!(transformed.variants().len(), 1);
            let (name, variant) = &transformed.variants()[0];
            assert_eq!(name.as_ref(), "Active");
            assert!(matches!(variant.fields(), Fields::Unit));
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_external_repr_named_variant() {
        // Named variant should be wrapped
        let named_fields = specta::internal::construct::fields_named(
            vec![(
                Cow::Borrowed("code"),
                Field::new(DataType::Primitive(Primitive::u32)),
            )],
            vec![],
        );

        let e = create_test_enum(vec![("Error".to_string(), named_fields)]);
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (name, variant) = &transformed.variants()[0];
            assert_eq!(name.as_ref(), "Error");

            // Should be wrapped in Named with "Error" field
            if let Fields::Named(fields) = variant.fields() {
                assert_eq!(fields.fields().len(), 1);
                assert_eq!(fields.fields()[0].0.as_ref(), "Error");
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_untagged_repr_no_transformation() {
        // Untagged should not transform
        let named_fields = specta::internal::construct::fields_named(
            vec![(
                Cow::Borrowed("value"),
                Field::new(DataType::Primitive(Primitive::String)),
            )],
            vec![],
        );

        let e = create_enum_with_attrs(
            vec![("Foo".to_string(), named_fields.clone())],
            vec![untagged_attr()],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (_, variant) = &transformed.variants()[0];

            // Should remain unchanged
            if let Fields::Named(fields) = variant.fields() {
                assert_eq!(fields.fields().len(), 1);
                assert_eq!(fields.fields()[0].0.as_ref(), "value");
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_internal_repr_unit_variant() {
        // Unit variant should become Named with tag field
        let e = create_enum_with_attrs(
            vec![("Active".to_string(), Fields::Unit)],
            vec![tag_attr("type")],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (_, variant) = &transformed.variants()[0];

            // Should have tag field
            if let Fields::Named(fields) = variant.fields() {
                assert_eq!(fields.fields().len(), 1);
                assert_eq!(fields.fields()[0].0.as_ref(), "type");

                // Tag field should be a literal string enum
                if let Some(DataType::Enum(tag_enum)) = fields.fields()[0].1.ty() {
                    assert_eq!(tag_enum.variants().len(), 1);
                    assert_eq!(tag_enum.variants()[0].0.as_ref(), "Active");
                } else {
                    panic!("Expected enum literal for tag");
                }
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_internal_repr_named_variant() {
        // Named variant should have tag field added
        let named_fields = specta::internal::construct::fields_named(
            vec![(
                Cow::Borrowed("message"),
                Field::new(DataType::Primitive(Primitive::String)),
            )],
            vec![],
        );

        let e = create_enum_with_attrs(
            vec![("Error".to_string(), named_fields)],
            vec![tag_attr("type")],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (_, variant) = &transformed.variants()[0];

            if let Fields::Named(fields) = variant.fields() {
                assert_eq!(fields.fields().len(), 2);
                assert_eq!(fields.fields()[0].0.as_ref(), "type");
                assert_eq!(fields.fields()[1].0.as_ref(), "message");
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_internal_repr_invalid_tuple_multiple_fields() {
        // Tuple variant with multiple fields should error
        let unnamed_fields = specta::internal::construct::fields_unnamed(
            vec![
                Field::new(DataType::Primitive(Primitive::String)),
                Field::new(DataType::Primitive(Primitive::i32)),
            ],
            vec![],
        );

        let e = create_enum_with_attrs(
            vec![("Data".to_string(), unnamed_fields)],
            vec![tag_attr("type")],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::InvalidInternallyTaggedEnum
        ));
    }

    #[test]
    fn test_adjacent_repr_unit_variant() {
        // Unit variant should only have tag, no content
        let e = create_enum_with_attrs(
            vec![("None".to_string(), Fields::Unit)],
            vec![tag_attr("type"), content_attr("data")],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (_, variant) = &transformed.variants()[0];

            if let Fields::Named(fields) = variant.fields() {
                // Only tag, no content for unit
                assert_eq!(fields.fields().len(), 1);
                assert_eq!(fields.fields()[0].0.as_ref(), "type");
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_adjacent_repr_named_variant() {
        // Named variant should have tag + content
        let named_fields = specta::internal::construct::fields_named(
            vec![(
                Cow::Borrowed("value"),
                Field::new(DataType::Primitive(Primitive::i32)),
            )],
            vec![],
        );

        let e = create_enum_with_attrs(
            vec![("Some".to_string(), named_fields)],
            vec![tag_attr("type"), content_attr("data")],
        );
        let dt = DataType::Enum(e);

        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            let (_, variant) = &transformed.variants()[0];

            if let Fields::Named(fields) = variant.fields() {
                assert_eq!(fields.fields().len(), 2);
                assert_eq!(fields.fields()[0].0.as_ref(), "type");
                assert_eq!(fields.fields()[1].0.as_ref(), "data");

                // Content should be a struct
                if let Some(DataType::Struct(_)) = fields.fields()[1].1.ty() {
                    // Good
                } else {
                    panic!("Expected struct for content");
                }
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_recursive_application() {
        // Nested enum in struct
        let inner_enum = create_test_enum(vec![("Active".to_string(), Fields::Unit)]);

        let struct_fields = specta::internal::construct::fields_named(
            vec![(
                Cow::Borrowed("status"),
                Field::new(DataType::Enum(inner_enum)),
            )],
            vec![],
        );

        let mut outer_struct = specta::datatype::Struct::unit();
        outer_struct.set_fields(struct_fields);
        let dt = DataType::Struct(outer_struct);

        // Should recursively apply to inner enum
        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Struct(s) = result {
            if let Fields::Named(fields) = s.fields() {
                assert_eq!(fields.fields().len(), 1);
                // Inner enum should have been processed
                assert!(matches!(fields.fields()[0].1.ty(), Some(DataType::Enum(_))));
            } else {
                panic!("Expected Named fields");
            }
        } else {
            panic!("Expected struct");
        }
    }

    #[test]
    fn test_skip_handling() {
        // Create variant with skip attribute
        let mut variant1 = EnumVariant::unit();
        variant1.set_skip(true);

        let variant2 = EnumVariant::unit();

        let mut e = Enum::new();
        *e.variants_mut() = vec![
            (Cow::Borrowed("Skipped"), variant1),
            (Cow::Borrowed("Active"), variant2),
        ];

        let dt = DataType::Enum(e);
        let result = apply_repr(dt, SerdeMode::Serialize).unwrap();

        if let DataType::Enum(transformed) = result {
            // Both variants should be present
            assert_eq!(transformed.variants().len(), 2);
            // But skipped one should be unchanged (not transformed)
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_all_variants_skipped_error() {
        let mut variant = EnumVariant::unit();
        variant.set_skip(true);

        let mut e = Enum::new();
        *e.variants_mut() = vec![(Cow::Borrowed("Skipped"), variant)];

        let dt = DataType::Enum(e);
        let result = apply_repr(dt, SerdeMode::Serialize);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidUsageOfSkip));
    }
}
