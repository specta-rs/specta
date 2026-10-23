#![allow(missing_docs)]

use crate::{Error, inflection::RenameRule};
use specta::datatype::{Attributes, DataType};

// The wire keys themselves live in `specta::markers`. Re-export the ones this
// crate's other modules reference as `parser::*` so each key still has exactly
// one literal definition across the workspace.
pub(crate) use specta::markers::{
    SERDE_CONTAINER_CONTENT, SERDE_CONTAINER_DEFAULT, SERDE_CONTAINER_DENY_UNKNOWN_FIELDS,
    SERDE_CONTAINER_FIELD_IDENTIFIER, SERDE_CONTAINER_FROM_RESOLVED, SERDE_CONTAINER_FROM_TYPE_SRC,
    SERDE_CONTAINER_INTO_RESOLVED, SERDE_CONTAINER_INTO_TYPE_SRC,
    SERDE_CONTAINER_RENAME_ALL_DESERIALIZE, SERDE_CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
    SERDE_CONTAINER_RENAME_ALL_FIELDS_SERIALIZE, SERDE_CONTAINER_RENAME_ALL_SERIALIZE,
    SERDE_CONTAINER_RENAME_DESERIALIZE, SERDE_CONTAINER_RENAME_SERIALIZE, SERDE_CONTAINER_TAG,
    SERDE_CONTAINER_TRANSPARENT, SERDE_CONTAINER_TRY_FROM_RESOLVED,
    SERDE_CONTAINER_TRY_FROM_TYPE_SRC, SERDE_CONTAINER_UNTAGGED,
    SERDE_CONTAINER_VARIANT_IDENTIFIER, SERDE_FIELD_ALIASES, SERDE_FIELD_DEFAULT,
    SERDE_FIELD_FLATTEN, SERDE_FIELD_HAS_DESERIALIZE_WITH, SERDE_FIELD_HAS_SERIALIZE_WITH,
    SERDE_FIELD_HAS_WITH, SERDE_FIELD_RENAME_DESERIALIZE, SERDE_FIELD_RENAME_SERIALIZE,
    SERDE_FIELD_SKIP_DESERIALIZING, SERDE_FIELD_SKIP_SERIALIZING, SERDE_FIELD_SKIP_SERIALIZING_IF,
    SERDE_VARIANT_ALIASES, SERDE_VARIANT_HAS_DESERIALIZE_WITH, SERDE_VARIANT_HAS_SERIALIZE_WITH,
    SERDE_VARIANT_HAS_WITH, SERDE_VARIANT_OTHER, SERDE_VARIANT_RENAME_ALL_DESERIALIZE,
    SERDE_VARIANT_RENAME_ALL_SERIALIZE, SERDE_VARIANT_RENAME_DESERIALIZE,
    SERDE_VARIANT_RENAME_SERIALIZE, SERDE_VARIANT_SKIP_DESERIALIZING,
    SERDE_VARIANT_SKIP_SERIALIZING, SERDE_VARIANT_UNTAGGED,
};

const CONTAINER_ATTR_KEYS: &[&str] = &[
    SERDE_CONTAINER_RENAME_SERIALIZE,
    SERDE_CONTAINER_RENAME_DESERIALIZE,
    SERDE_CONTAINER_RENAME_ALL_SERIALIZE,
    SERDE_CONTAINER_RENAME_ALL_DESERIALIZE,
    SERDE_CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
    SERDE_CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
    SERDE_CONTAINER_TAG,
    SERDE_CONTAINER_CONTENT,
    SERDE_CONTAINER_UNTAGGED,
    SERDE_CONTAINER_DEFAULT,
    SERDE_CONTAINER_TRANSPARENT,
    SERDE_CONTAINER_DENY_UNKNOWN_FIELDS,
    SERDE_CONTAINER_FROM_TYPE_SRC,
    SERDE_CONTAINER_FROM_RESOLVED,
    SERDE_CONTAINER_TRY_FROM_TYPE_SRC,
    SERDE_CONTAINER_TRY_FROM_RESOLVED,
    SERDE_CONTAINER_INTO_TYPE_SRC,
    SERDE_CONTAINER_INTO_RESOLVED,
    SERDE_CONTAINER_VARIANT_IDENTIFIER,
    SERDE_CONTAINER_FIELD_IDENTIFIER,
];

const VARIANT_ATTR_KEYS: &[&str] = &[
    SERDE_VARIANT_RENAME_SERIALIZE,
    SERDE_VARIANT_RENAME_DESERIALIZE,
    SERDE_VARIANT_ALIASES,
    SERDE_VARIANT_RENAME_ALL_SERIALIZE,
    SERDE_VARIANT_RENAME_ALL_DESERIALIZE,
    SERDE_VARIANT_SKIP_SERIALIZING,
    SERDE_VARIANT_SKIP_DESERIALIZING,
    SERDE_VARIANT_HAS_SERIALIZE_WITH,
    SERDE_VARIANT_HAS_DESERIALIZE_WITH,
    SERDE_VARIANT_HAS_WITH,
    SERDE_VARIANT_OTHER,
    SERDE_VARIANT_UNTAGGED,
];

const FIELD_ATTR_KEYS: &[&str] = &[
    SERDE_FIELD_RENAME_SERIALIZE,
    SERDE_FIELD_RENAME_DESERIALIZE,
    SERDE_FIELD_ALIASES,
    SERDE_FIELD_DEFAULT,
    SERDE_FIELD_FLATTEN,
    SERDE_FIELD_SKIP_SERIALIZING,
    SERDE_FIELD_SKIP_DESERIALIZING,
    SERDE_FIELD_SKIP_SERIALIZING_IF,
    SERDE_FIELD_HAS_SERIALIZE_WITH,
    SERDE_FIELD_HAS_DESERIALIZE_WITH,
    SERDE_FIELD_HAS_WITH,
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
                    rename_serialize: get_string(attributes, SERDE_CONTAINER_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, SERDE_CONTAINER_RENAME_DESERIALIZE),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        SERDE_CONTAINER_RENAME_ALL_SERIALIZE,
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        SERDE_CONTAINER_RENAME_ALL_DESERIALIZE,
                    )?,
                    rename_all_fields_serialize: get_rename_rule(
                        attributes,
                        SERDE_CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
                    )?,
                    rename_all_fields_deserialize: get_rename_rule(
                        attributes,
                        SERDE_CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
                    )?,
                    tag: get_string(attributes, SERDE_CONTAINER_TAG),
                    content: get_string(attributes, SERDE_CONTAINER_CONTENT),
                    untagged: has_attr(attributes, SERDE_CONTAINER_UNTAGGED),
                    default: has_attr(attributes, SERDE_CONTAINER_DEFAULT),
                    transparent: has_attr(attributes, SERDE_CONTAINER_TRANSPARENT),
                    deny_unknown_fields: has_attr(attributes, SERDE_CONTAINER_DENY_UNKNOWN_FIELDS),
                    from: get_string(attributes, SERDE_CONTAINER_FROM_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    try_from: get_string(attributes, SERDE_CONTAINER_TRY_FROM_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    into: get_string(attributes, SERDE_CONTAINER_INTO_TYPE_SRC)
                        .map(|type_src| ConversionType { type_src }),
                    resolved_from: get_datatype(attributes, SERDE_CONTAINER_FROM_RESOLVED),
                    resolved_try_from: get_datatype(attributes, SERDE_CONTAINER_TRY_FROM_RESOLVED),
                    resolved_into: get_datatype(attributes, SERDE_CONTAINER_INTO_RESOLVED),
                    variant_identifier: has_attr(attributes, SERDE_CONTAINER_VARIANT_IDENTIFIER),
                    field_identifier: has_attr(attributes, SERDE_CONTAINER_FIELD_IDENTIFIER),
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
                    rename_serialize: get_string(attributes, SERDE_VARIANT_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, SERDE_VARIANT_RENAME_DESERIALIZE),
                    aliases: get_strings(attributes, SERDE_VARIANT_ALIASES),
                    rename_all_serialize: get_rename_rule(
                        attributes,
                        SERDE_VARIANT_RENAME_ALL_SERIALIZE,
                    )?,
                    rename_all_deserialize: get_rename_rule(
                        attributes,
                        SERDE_VARIANT_RENAME_ALL_DESERIALIZE,
                    )?,
                    skip_serializing: has_attr(attributes, SERDE_VARIANT_SKIP_SERIALIZING),
                    skip_deserializing: has_attr(attributes, SERDE_VARIANT_SKIP_DESERIALIZING),
                    has_serialize_with: has_attr(attributes, SERDE_VARIANT_HAS_SERIALIZE_WITH),
                    has_deserialize_with: has_attr(attributes, SERDE_VARIANT_HAS_DESERIALIZE_WITH),
                    has_with: has_attr(attributes, SERDE_VARIANT_HAS_WITH),
                    other: has_attr(attributes, SERDE_VARIANT_OTHER),
                    untagged: has_attr(attributes, SERDE_VARIANT_UNTAGGED),
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
                    rename_serialize: get_string(attributes, SERDE_FIELD_RENAME_SERIALIZE),
                    rename_deserialize: get_string(attributes, SERDE_FIELD_RENAME_DESERIALIZE),
                    aliases: get_strings(attributes, SERDE_FIELD_ALIASES),
                    default: has_attr(attributes, SERDE_FIELD_DEFAULT),
                    flatten: has_attr(attributes, SERDE_FIELD_FLATTEN),
                    skip_serializing: has_attr(attributes, SERDE_FIELD_SKIP_SERIALIZING),
                    skip_deserializing: has_attr(attributes, SERDE_FIELD_SKIP_DESERIALIZING),
                    skip_serializing_if: get_string(attributes, SERDE_FIELD_SKIP_SERIALIZING_IF),
                    has_serialize_with: has_attr(attributes, SERDE_FIELD_HAS_SERIALIZE_WITH),
                    has_deserialize_with: has_attr(attributes, SERDE_FIELD_HAS_DESERIALIZE_WITH),
                    has_with: has_attr(attributes, SERDE_FIELD_HAS_WITH),
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
