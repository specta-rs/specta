#![allow(missing_docs)]

use crate::{Error, inflection::RenameRule};
use specta::datatype::{Attributes, DataType};

const CONTAINER_ATTR_KEYS: &[&str] = &[
    "serde:container:rename_serialize",
    "serde:container:rename_deserialize",
    "serde:container:rename_all_serialize",
    "serde:container:rename_all_deserialize",
    "serde:container:rename_all_fields_serialize",
    "serde:container:rename_all_fields_deserialize",
    "serde:container:tag",
    "serde:container:content",
    "serde:container:untagged",
    "serde:container:default",
    "serde:container:transparent",
    "serde:container:deny_unknown_fields",
    "serde:container:from_type_src",
    "serde:container:from_resolved",
    "serde:container:try_from_type_src",
    "serde:container:try_from_resolved",
    "serde:container:into_type_src",
    "serde:container:into_resolved",
    "serde:container:variant_identifier",
    "serde:container:field_identifier",
];

const VARIANT_ATTR_KEYS: &[&str] = &[
    "serde:variant:rename_serialize",
    "serde:variant:rename_deserialize",
    "serde:variant:aliases",
    "serde:variant:rename_all_serialize",
    "serde:variant:rename_all_deserialize",
    "serde:variant:skip_serializing",
    "serde:variant:skip_deserializing",
    "serde:variant:has_serialize_with",
    "serde:variant:has_deserialize_with",
    "serde:variant:has_with",
    "serde:variant:other",
    "serde:variant:untagged",
];

const FIELD_ATTR_KEYS: &[&str] = &[
    "serde:field:rename_serialize",
    "serde:field:rename_deserialize",
    "serde:field:aliases",
    "serde:field:default",
    "serde:field:flatten",
    "serde:field:skip_serializing",
    "serde:field:skip_deserializing",
    "serde:field:skip_serializing_if",
    "serde:field:has_serialize_with",
    "serde:field:has_deserialize_with",
    "serde:field:has_with",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConversionType {
    pub type_src: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeContainerAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    pub rename_all_fields_serialize: Option<RenameRule>,
    pub rename_all_fields_deserialize: Option<RenameRule>,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub default: bool,
    pub transparent: bool,
    pub deny_unknown_fields: bool,
    pub from: Option<ConversionType>,
    pub try_from: Option<ConversionType>,
    pub into: Option<ConversionType>,
    pub resolved_from: Option<DataType>,
    pub resolved_try_from: Option<DataType>,
    pub resolved_into: Option<DataType>,
    pub variant_identifier: bool,
    pub field_identifier: bool,
}

impl SerdeContainerAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Result<Option<Self>, Error> {
        has_any_attr(attributes, CONTAINER_ATTR_KEYS)
            .then(|| {
                Ok(Self {
                    rename_serialize: get_string(attributes, "serde:container:rename_serialize"),
                    rename_deserialize: get_string(
                        attributes,
                        "serde:container:rename_deserialize",
                    ),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        "serde:container:rename_all_serialize",
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        "serde:container:rename_all_deserialize",
                    )?,
                    rename_all_fields_serialize: get_rename_rule(
                        attributes,
                        "serde:container:rename_all_fields_serialize",
                    )?,
                    rename_all_fields_deserialize: get_rename_rule(
                        attributes,
                        "serde:container:rename_all_fields_deserialize",
                    )?,
                    tag: get_string(attributes, "serde:container:tag"),
                    content: get_string(attributes, "serde:container:content"),
                    untagged: has_attr(attributes, "serde:container:untagged"),
                    default: has_attr(attributes, "serde:container:default"),
                    transparent: has_attr(attributes, "serde:container:transparent"),
                    deny_unknown_fields: has_attr(
                        attributes,
                        "serde:container:deny_unknown_fields",
                    ),
                    from: get_string(attributes, "serde:container:from_type_src")
                        .map(|type_src| ConversionType { type_src }),
                    try_from: get_string(attributes, "serde:container:try_from_type_src")
                        .map(|type_src| ConversionType { type_src }),
                    into: get_string(attributes, "serde:container:into_type_src")
                        .map(|type_src| ConversionType { type_src }),
                    resolved_from: get_datatype(attributes, "serde:container:from_resolved"),
                    resolved_try_from: get_datatype(
                        attributes,
                        "serde:container:try_from_resolved",
                    ),
                    resolved_into: get_datatype(attributes, "serde:container:into_resolved"),
                    variant_identifier: has_attr(attributes, "serde:container:variant_identifier"),
                    field_identifier: has_attr(attributes, "serde:container:field_identifier"),
                })
            })
            .transpose()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeVariantAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub has_serialize_with: bool,
    pub has_deserialize_with: bool,
    pub has_with: bool,
    pub other: bool,
    pub untagged: bool,
}

impl SerdeVariantAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Result<Option<Self>, Error> {
        has_any_attr(attributes, VARIANT_ATTR_KEYS)
            .then(|| {
                Ok(Self {
                    rename_serialize: get_string(attributes, "serde:variant:rename_serialize"),
                    rename_deserialize: get_string(attributes, "serde:variant:rename_deserialize"),
                    aliases: get_strings(attributes, "serde:variant:aliases"),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        "serde:variant:rename_all_serialize",
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        "serde:variant:rename_all_deserialize",
                    )?,
                    skip_serializing: has_attr(attributes, "serde:variant:skip_serializing"),
                    skip_deserializing: has_attr(attributes, "serde:variant:skip_deserializing"),
                    has_serialize_with: has_attr(attributes, "serde:variant:has_serialize_with"),
                    has_deserialize_with: has_attr(
                        attributes,
                        "serde:variant:has_deserialize_with",
                    ),
                    has_with: has_attr(attributes, "serde:variant:has_with"),
                    other: has_attr(attributes, "serde:variant:other"),
                    untagged: has_attr(attributes, "serde:variant:untagged"),
                })
            })
            .transpose()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeFieldAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub default: bool,
    pub flatten: bool,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub skip_serializing_if: Option<String>,
    pub has_serialize_with: bool,
    pub has_deserialize_with: bool,
    pub has_with: bool,
}

impl SerdeFieldAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Result<Option<Self>, Error> {
        has_any_attr(attributes, FIELD_ATTR_KEYS)
            .then(|| {
                Ok(Self {
                    rename_serialize: get_string(attributes, "serde:field:rename_serialize"),
                    rename_deserialize: get_string(attributes, "serde:field:rename_deserialize"),
                    aliases: get_strings(attributes, "serde:field:aliases"),
                    default: has_attr(attributes, "serde:field:default"),
                    flatten: has_attr(attributes, "serde:field:flatten"),
                    skip_serializing: has_attr(attributes, "serde:field:skip_serializing"),
                    skip_deserializing: has_attr(attributes, "serde:field:skip_deserializing"),
                    skip_serializing_if: get_string(attributes, "serde:field:skip_serializing_if"),
                    has_serialize_with: has_attr(attributes, "serde:field:has_serialize_with"),
                    has_deserialize_with: has_attr(attributes, "serde:field:has_deserialize_with"),
                    has_with: has_attr(attributes, "serde:field:has_with"),
                })
            })
            .transpose()
    }
}

fn get_string(attributes: &Attributes, key: &str) -> Option<String> {
    attributes.get_named_as::<String>(key).cloned()
}

fn get_strings(attributes: &Attributes, key: &str) -> Vec<String> {
    attributes
        .get_named_as::<Vec<String>>(key)
        .cloned()
        .unwrap_or_default()
}

fn has_attr(attributes: &Attributes, key: &str) -> bool {
    attributes.contains_key(key)
}

fn get_datatype(attributes: &Attributes, key: &str) -> Option<DataType> {
    attributes.get_named_as::<DataType>(key).cloned()
}

fn get_rename_rule(attributes: &Attributes, key: &str) -> Result<Option<RenameRule>, Error> {
    match get_string(attributes, key) {
        Some(value) => RenameRule::from_str(&value)
            .map(Some)
            .map_err(|_| Error::invalid_rename_rule(key.to_string(), value.clone())),
        None => Ok(None),
    }
}

fn has_any_attr(attributes: &Attributes, keys: &[&str]) -> bool {
    // Most datatype nodes carry no serde metadata at all; the empty check is a
    // single length test, while the key scan below hashes every known key.
    !attributes.is_empty() && keys.iter().copied().any(|key| attributes.contains_key(key))
}
