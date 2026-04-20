#![allow(missing_docs)]

use crate::{Error, inflection::RenameRule};
use specta::datatype::{Attributes, DataType};

const CONTAINER_RENAME_SERIALIZE: &str = "serde:container:rename_serialize";
const CONTAINER_RENAME_DESERIALIZE: &str = "serde:container:rename_deserialize";
const CONTAINER_RENAME_ALL_SERIALIZE: &str = "serde:container:rename_all_serialize";
const CONTAINER_RENAME_ALL_DESERIALIZE: &str = "serde:container:rename_all_deserialize";
const CONTAINER_RENAME_ALL_FIELDS_SERIALIZE: &str = "serde:container:rename_all_fields_serialize";
const CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE: &str =
    "serde:container:rename_all_fields_deserialize";
const CONTAINER_TAG: &str = "serde:container:tag";
const CONTAINER_CONTENT: &str = "serde:container:content";
const CONTAINER_UNTAGGED: &str = "serde:container:untagged";
const CONTAINER_DEFAULT: &str = "serde:container:default";
const CONTAINER_TRANSPARENT: &str = "serde:container:transparent";
const CONTAINER_FROM_TYPE_SRC: &str = "serde:container:from_type_src";
const CONTAINER_FROM_RESOLVED: &str = "serde:container:from_resolved";
const CONTAINER_TRY_FROM_TYPE_SRC: &str = "serde:container:try_from_type_src";
const CONTAINER_TRY_FROM_RESOLVED: &str = "serde:container:try_from_resolved";
const CONTAINER_INTO_TYPE_SRC: &str = "serde:container:into_type_src";
const CONTAINER_INTO_RESOLVED: &str = "serde:container:into_resolved";
const CONTAINER_VARIANT_IDENTIFIER: &str = "serde:container:variant_identifier";
const CONTAINER_FIELD_IDENTIFIER: &str = "serde:container:field_identifier";

const VARIANT_RENAME_SERIALIZE: &str = "serde:variant:rename_serialize";
const VARIANT_RENAME_DESERIALIZE: &str = "serde:variant:rename_deserialize";
const VARIANT_ALIASES: &str = "serde:variant:aliases";
const VARIANT_RENAME_ALL_SERIALIZE: &str = "serde:variant:rename_all_serialize";
const VARIANT_RENAME_ALL_DESERIALIZE: &str = "serde:variant:rename_all_deserialize";
const VARIANT_SKIP_SERIALIZING: &str = "serde:variant:skip_serializing";
const VARIANT_SKIP_DESERIALIZING: &str = "serde:variant:skip_deserializing";
const VARIANT_HAS_SERIALIZE_WITH: &str = "serde:variant:has_serialize_with";
const VARIANT_HAS_DESERIALIZE_WITH: &str = "serde:variant:has_deserialize_with";
const VARIANT_HAS_WITH: &str = "serde:variant:has_with";
const VARIANT_OTHER: &str = "serde:variant:other";
const VARIANT_UNTAGGED: &str = "serde:variant:untagged";

const FIELD_RENAME_SERIALIZE: &str = "serde:field:rename_serialize";
const FIELD_RENAME_DESERIALIZE: &str = "serde:field:rename_deserialize";
const FIELD_ALIASES: &str = "serde:field:aliases";
const FIELD_DEFAULT: &str = "serde:field:default";
const FIELD_FLATTEN: &str = "serde:field:flatten";
const FIELD_SKIP_SERIALIZING: &str = "serde:field:skip_serializing";
const FIELD_SKIP_DESERIALIZING: &str = "serde:field:skip_deserializing";
const FIELD_SKIP_SERIALIZING_IF: &str = "serde:field:skip_serializing_if";
const FIELD_HAS_SERIALIZE_WITH: &str = "serde:field:has_serialize_with";
const FIELD_HAS_DESERIALIZE_WITH: &str = "serde:field:has_deserialize_with";
const FIELD_HAS_WITH: &str = "serde:field:has_with";

const CONTAINER_ATTR_KEYS: &[&str] = &[
    CONTAINER_RENAME_SERIALIZE,
    CONTAINER_RENAME_DESERIALIZE,
    CONTAINER_RENAME_ALL_SERIALIZE,
    CONTAINER_RENAME_ALL_DESERIALIZE,
    CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
    CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
    CONTAINER_TAG,
    CONTAINER_CONTENT,
    CONTAINER_UNTAGGED,
    CONTAINER_DEFAULT,
    CONTAINER_TRANSPARENT,
    CONTAINER_FROM_TYPE_SRC,
    CONTAINER_FROM_RESOLVED,
    CONTAINER_TRY_FROM_TYPE_SRC,
    CONTAINER_TRY_FROM_RESOLVED,
    CONTAINER_INTO_TYPE_SRC,
    CONTAINER_INTO_RESOLVED,
    CONTAINER_VARIANT_IDENTIFIER,
    CONTAINER_FIELD_IDENTIFIER,
];

const VARIANT_ATTR_KEYS: &[&str] = &[
    VARIANT_RENAME_SERIALIZE,
    VARIANT_RENAME_DESERIALIZE,
    VARIANT_ALIASES,
    VARIANT_RENAME_ALL_SERIALIZE,
    VARIANT_RENAME_ALL_DESERIALIZE,
    VARIANT_SKIP_SERIALIZING,
    VARIANT_SKIP_DESERIALIZING,
    VARIANT_HAS_SERIALIZE_WITH,
    VARIANT_HAS_DESERIALIZE_WITH,
    VARIANT_HAS_WITH,
    VARIANT_OTHER,
    VARIANT_UNTAGGED,
];

const FIELD_ATTR_KEYS: &[&str] = &[
    FIELD_RENAME_SERIALIZE,
    FIELD_RENAME_DESERIALIZE,
    FIELD_ALIASES,
    FIELD_DEFAULT,
    FIELD_FLATTEN,
    FIELD_SKIP_SERIALIZING,
    FIELD_SKIP_DESERIALIZING,
    FIELD_SKIP_SERIALIZING_IF,
    FIELD_HAS_SERIALIZE_WITH,
    FIELD_HAS_DESERIALIZE_WITH,
    FIELD_HAS_WITH,
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
                    rename_serialize: get_string(attributes, CONTAINER_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, CONTAINER_RENAME_DESERIALIZE),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        CONTAINER_RENAME_ALL_SERIALIZE,
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        CONTAINER_RENAME_ALL_DESERIALIZE,
                    )?,
                    rename_all_fields_serialize: get_rename_rule(
                        attributes,
                        CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
                    )?,
                    rename_all_fields_deserialize: get_rename_rule(
                        attributes,
                        CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
                    )?,
                    tag: get_string(attributes, CONTAINER_TAG),
                    content: get_string(attributes, CONTAINER_CONTENT),
                    untagged: has_attr(attributes, CONTAINER_UNTAGGED),
                    default: has_attr(attributes, CONTAINER_DEFAULT),
                    transparent: has_attr(attributes, CONTAINER_TRANSPARENT),
                    from: get_string(attributes, CONTAINER_FROM_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    try_from: get_string(attributes, CONTAINER_TRY_FROM_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    into: get_string(attributes, CONTAINER_INTO_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    resolved_from: get_datatype(attributes, CONTAINER_FROM_RESOLVED),
                    resolved_try_from: get_datatype(attributes, CONTAINER_TRY_FROM_RESOLVED),
                    resolved_into: get_datatype(attributes, CONTAINER_INTO_RESOLVED),
                    variant_identifier: has_attr(attributes, CONTAINER_VARIANT_IDENTIFIER),
                    field_identifier: has_attr(attributes, CONTAINER_FIELD_IDENTIFIER),
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
                    rename_serialize: get_string(attributes, VARIANT_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, VARIANT_RENAME_DESERIALIZE),
                    aliases: get_strings(attributes, VARIANT_ALIASES),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        VARIANT_RENAME_ALL_SERIALIZE,
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        VARIANT_RENAME_ALL_DESERIALIZE,
                    )?,
                    skip_serializing: has_attr(attributes, VARIANT_SKIP_SERIALIZING),
                    skip_deserializing: has_attr(attributes, VARIANT_SKIP_DESERIALIZING),
                    has_serialize_with: has_attr(attributes, VARIANT_HAS_SERIALIZE_WITH),
                    has_deserialize_with: has_attr(attributes, VARIANT_HAS_DESERIALIZE_WITH),
                    has_with: has_attr(attributes, VARIANT_HAS_WITH),
                    other: has_attr(attributes, VARIANT_OTHER),
                    untagged: has_attr(attributes, VARIANT_UNTAGGED),
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
                    rename_serialize: get_string(attributes, FIELD_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, FIELD_RENAME_DESERIALIZE),
                    aliases: get_strings(attributes, FIELD_ALIASES),
                    default: has_attr(attributes, FIELD_DEFAULT),
                    flatten: has_attr(attributes, FIELD_FLATTEN),
                    skip_serializing: has_attr(attributes, FIELD_SKIP_SERIALIZING),
                    skip_deserializing: has_attr(attributes, FIELD_SKIP_DESERIALIZING),
                    skip_serializing_if: get_string(attributes, FIELD_SKIP_SERIALIZING_IF),
                    has_serialize_with: has_attr(attributes, FIELD_HAS_SERIALIZE_WITH),
                    has_deserialize_with: has_attr(attributes, FIELD_HAS_DESERIALIZE_WITH),
                    has_with: has_attr(attributes, FIELD_HAS_WITH),
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
    keys.iter().copied().any(|key| attributes.contains_key(key))
}
