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
    ArcId,
};
use std::borrow::Cow;
use std::collections::{HashSet, HashMap};

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

/// Transform a TypeCollection into phases with separate types for serialization and deserialization
///
/// This function takes a `TypeCollection` and returns a new collection containing:
/// - All original types (unchanged)
/// - A `{OriginalName}_Serialize` version of each type with serialization transformations applied
/// - A `{OriginalName}_Deserialize` version of each type with deserialization transformations applied
///
/// References between types are automatically updated so that types in the serialize phase
/// reference other serialize-phase types, and likewise for deserialize-phase types.
///
/// This is useful when you need separate type definitions for input and output in an API,
/// or when serialization and deserialization representations differ significantly.
///
/// # Example
/// ```ignore
/// use specta::TypeCollection;
/// use specta_serde::into_phases;
///
/// let mut types = TypeCollection::default();
/// types.register::<User>();
/// types.register::<Post>();
///
/// // Returns collection with: User, User_Serialize, User_Deserialize, Post, Post_Serialize, Post_Deserialize
/// let phased = into_phases(types)?;
/// ```
///
/// # Behavior
/// - Each type gets two new versions with `_Serialize` and `_Deserialize` suffixes
/// - Original types remain unchanged in the returned collection
/// - References are updated: `User_Serialize` referencing `Post` becomes a reference to `Post_Serialize`
/// - Serde transformations are applied according to the phase (serialize or deserialize)
/// - Generic parameters are preserved in the phase-specific versions
pub fn into_phases(types: TypeCollection) -> Result<TypeCollection, Error> {
    // Step 1: Build mapping from original ArcId to phase-specific ArcIds
    let mut id_mapping: HashMap<ArcId, (ArcId, ArcId)> = HashMap::new();
    let mut serialize_types = TypeCollection::default();
    let mut deserialize_types = TypeCollection::default();
    
    // Step 2: Create phase-specific versions of each type
    for ndt in types.into_unsorted_iter() {
        let original_id = ndt.id().clone();
        
        // Create new ArcIds for phase-specific versions
        let ser_id = ArcId::new_dynamic();
        let de_id = ArcId::new_dynamic();
        id_mapping.insert(original_id, (ser_id.clone(), de_id.clone()));
        
        // Clone for serialize version
        let ser_name = format!("{}_Serialize", ndt.name());
        let ser_ndt = ndt.clone_with_id(ser_id, Cow::Owned(ser_name));
        serialize_types = serialize_types.insert(ser_ndt);
        
        // Clone for deserialize version
        let de_name = format!("{}_Deserialize", ndt.name());
        let de_ndt = ndt.clone_with_id(de_id, Cow::Owned(de_name));
        deserialize_types = deserialize_types.insert(de_ndt);
    }

    // Step 3: Update references in phase-specific types
    serialize_types = serialize_types.map(|mut ndt| {
        let transformed_dt = update_references_in_datatype(
            ndt.ty().clone(),
            SerdeMode::Serialize,
            &id_mapping,
        );
        ndt.set_ty(transformed_dt);
        ndt
    });
    
    deserialize_types = deserialize_types.map(|mut ndt| {
        let transformed_dt = update_references_in_datatype(
            ndt.ty().clone(),
            SerdeMode::Deserialize,
            &id_mapping,
        );
        ndt.set_ty(transformed_dt);
        ndt
    });

    // Step 4: Apply serde transformations to each phase
    apply(&mut serialize_types, SerdeMode::Serialize)?;
    apply(&mut deserialize_types, SerdeMode::Deserialize)?;

    // Step 5: Merge all collections (original + serialize + deserialize)
    let mut result = types;
    for ndt in serialize_types.into_sorted_iter() {
        result = result.insert(ndt);
    }
    for ndt in deserialize_types.into_sorted_iter() {
        result = result.insert(ndt);
    }
    
    Ok(result)
}

/// Helper function to recursively update references in a DataType
fn update_references_in_datatype(
    dt: DataType,
    phase: SerdeMode,
    mapping: &HashMap<ArcId, (ArcId, ArcId)>,
) -> DataType {
    match dt {
        DataType::Nullable(inner) => {
            DataType::Nullable(Box::new(update_references_in_datatype(*inner, phase, mapping)))
        }
        DataType::Map(mut map) => {
            let key = update_references_in_datatype(map.key_ty().clone(), phase, mapping);
            let value = update_references_in_datatype(map.value_ty().clone(), phase, mapping);
            *map.key_ty_mut() = key;
            *map.value_ty_mut() = value;
            DataType::Map(map)
        }
        DataType::Struct(mut s) => {
            let fields = match s.fields().clone() {
                Fields::Unit => Fields::Unit,
                Fields::Unnamed(mut fields) => {
                    for field in fields.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = update_references_in_datatype(ty.clone(), phase, mapping);
                        }
                    }
                    Fields::Unnamed(fields)
                }
                Fields::Named(mut fields) => {
                    for (_, field) in fields.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = update_references_in_datatype(ty.clone(), phase, mapping);
                        }
                    }
                    Fields::Named(fields)
                }
            };
            s.set_fields(fields);
            DataType::Struct(s)
        }
        DataType::Enum(mut e) => {
            for (_, variant) in e.variants_mut() {
                let fields = match variant.fields().clone() {
                    Fields::Unit => Fields::Unit,
                    Fields::Unnamed(mut fields) => {
                        for field in fields.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = update_references_in_datatype(ty.clone(), phase, mapping);
                            }
                        }
                        Fields::Unnamed(fields)
                    }
                    Fields::Named(mut fields) => {
                        for (_, field) in fields.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = update_references_in_datatype(ty.clone(), phase, mapping);
                            }
                        }
                        Fields::Named(fields)
                    }
                };
                variant.set_fields(fields);
            }
            DataType::Enum(e)
        }
        DataType::Tuple(mut tuple) => {
            let elements = tuple.elements().iter()
                .map(|elem| update_references_in_datatype(elem.clone(), phase, mapping))
                .collect();
            *tuple.elements_mut() = elements;
            DataType::Tuple(tuple)
        }
        DataType::List(mut list) => {
            let ty = update_references_in_datatype(list.ty().clone(), phase, mapping);
            *list.ty_mut() = ty;
            DataType::List(list)
        }
        DataType::Reference(mut reference) => {
            // Update generic parameters
            let updated_generics: Vec<(Generic, DataType)> = reference.generics().iter()
                .map(|(g, dt)| (g.clone(), update_references_in_datatype(dt.clone(), phase, mapping)))
                .collect();
            *reference.generics_mut() = updated_generics;
            
            // Check if we need to update this reference to a phase-specific version
            if let Some((ser_id, de_id)) = mapping.get(reference.id()) {
                let target_id = match phase {
                    SerdeMode::Serialize => ser_id,
                    SerdeMode::Deserialize => de_id,
                    SerdeMode::Both => reference.id(), // Keep original for Both mode
                };
                
                // Reconstruct the reference with updated id
                Reference::from_parts(
                    target_id.clone(),
                    reference.generics().to_vec(),
                    reference.inline(),
                ).into()
            } else {
                // No mapping found, return updated reference
                DataType::Reference(reference)
            }
        }
        // Primitives and other types don't contain references
        other => other,
    }
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
