#![allow(missing_docs)]

use crate::inflection::RenameRule;
use specta::datatype::{Attributes, DataType};

const CONTAINER_RENAME_SERIALIZE: &str = "serde:container:rename_serialize";
const CONTAINER_RENAME_DESERIALIZE: &str = "serde:container:rename_deserialize";
const CONTAINER_RENAME_ALL_SERIALIZE: &str = "serde:container:rename_all_serialize";
const CONTAINER_RENAME_ALL_DESERIALIZE: &str = "serde:container:rename_all_deserialize";
const CONTAINER_RENAME_ALL_FIELDS_SERIALIZE: &str = "serde:container:rename_all_fields_serialize";
const CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE: &str =
    "serde:container:rename_all_fields_deserialize";
const CONTAINER_DENY_UNKNOWN_FIELDS: &str = "serde:container:deny_unknown_fields";
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
const VARIANT_SERIALIZE_WITH: &str = "serde:variant:serialize_with";
const VARIANT_HAS_SERIALIZE_WITH: &str = "serde:variant:has_serialize_with";
const VARIANT_DESERIALIZE_WITH: &str = "serde:variant:deserialize_with";
const VARIANT_HAS_DESERIALIZE_WITH: &str = "serde:variant:has_deserialize_with";
const VARIANT_WITH: &str = "serde:variant:with";
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
const FIELD_SERIALIZE_WITH: &str = "serde:field:serialize_with";
const FIELD_HAS_SERIALIZE_WITH: &str = "serde:field:has_serialize_with";
const FIELD_DESERIALIZE_WITH: &str = "serde:field:deserialize_with";
const FIELD_HAS_DESERIALIZE_WITH: &str = "serde:field:has_deserialize_with";
const FIELD_WITH: &str = "serde:field:with";
const FIELD_HAS_WITH: &str = "serde:field:has_with";

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
    pub deny_unknown_fields: bool,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub default: Option<String>,
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
    pub fn from_attributes(attributes: &Attributes) -> Option<Self> {
        let attrs = Self {
            rename_serialize: get_string(attributes, CONTAINER_RENAME_SERIALIZE),
            rename_deserialize: get_string(attributes, CONTAINER_RENAME_DESERIALIZE),
            rename_all_serialize: get_rename_rule(attributes, CONTAINER_RENAME_ALL_SERIALIZE),
            rename_all_deserialize: get_rename_rule(attributes, CONTAINER_RENAME_ALL_DESERIALIZE),
            rename_all_fields_serialize: get_rename_rule(
                attributes,
                CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
            ),
            rename_all_fields_deserialize: get_rename_rule(
                attributes,
                CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
            ),
            deny_unknown_fields: get_bool(attributes, CONTAINER_DENY_UNKNOWN_FIELDS),
            tag: get_string(attributes, CONTAINER_TAG),
            content: get_string(attributes, CONTAINER_CONTENT),
            untagged: get_bool(attributes, CONTAINER_UNTAGGED),
            default: get_string(attributes, CONTAINER_DEFAULT),
            transparent: get_bool(attributes, CONTAINER_TRANSPARENT),
            from: get_string(attributes, CONTAINER_FROM_TYPE_SRC)
                .map(|type_src| ConversionType { type_src }),
            try_from: get_string(attributes, CONTAINER_TRY_FROM_TYPE_SRC)
                .map(|type_src| ConversionType { type_src }),
            into: get_string(attributes, CONTAINER_INTO_TYPE_SRC)
                .map(|type_src| ConversionType { type_src }),
            resolved_from: get_datatype(attributes, CONTAINER_FROM_RESOLVED),
            resolved_try_from: get_datatype(attributes, CONTAINER_TRY_FROM_RESOLVED),
            resolved_into: get_datatype(attributes, CONTAINER_INTO_RESOLVED),
            variant_identifier: get_bool(attributes, CONTAINER_VARIANT_IDENTIFIER),
            field_identifier: get_bool(attributes, CONTAINER_FIELD_IDENTIFIER),
        };

        has_any_container_attr(attributes).then_some(attrs)
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
    pub serialize_with: Option<String>,
    pub has_serialize_with: bool,
    pub deserialize_with: Option<String>,
    pub has_deserialize_with: bool,
    pub with: Option<String>,
    pub has_with: bool,
    pub other: bool,
    pub untagged: bool,
}

impl SerdeVariantAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Option<Self> {
        let attrs = Self {
            rename_serialize: get_string(attributes, VARIANT_RENAME_SERIALIZE),
            rename_deserialize: get_string(attributes, VARIANT_RENAME_DESERIALIZE),
            aliases: get_strings(attributes, VARIANT_ALIASES),
            rename_all_serialize: get_rename_rule(attributes, VARIANT_RENAME_ALL_SERIALIZE),
            rename_all_deserialize: get_rename_rule(attributes, VARIANT_RENAME_ALL_DESERIALIZE),
            skip_serializing: get_bool(attributes, VARIANT_SKIP_SERIALIZING),
            skip_deserializing: get_bool(attributes, VARIANT_SKIP_DESERIALIZING),
            serialize_with: get_string(attributes, VARIANT_SERIALIZE_WITH),
            has_serialize_with: get_bool(attributes, VARIANT_HAS_SERIALIZE_WITH),
            deserialize_with: get_string(attributes, VARIANT_DESERIALIZE_WITH),
            has_deserialize_with: get_bool(attributes, VARIANT_HAS_DESERIALIZE_WITH),
            with: get_string(attributes, VARIANT_WITH),
            has_with: get_bool(attributes, VARIANT_HAS_WITH),
            other: get_bool(attributes, VARIANT_OTHER),
            untagged: get_bool(attributes, VARIANT_UNTAGGED),
        };

        has_any_variant_attr(attributes).then_some(attrs)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeFieldAttrs {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub default: Option<String>,
    pub flatten: bool,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub skip_serializing_if: Option<String>,
    pub serialize_with: Option<String>,
    pub has_serialize_with: bool,
    pub deserialize_with: Option<String>,
    pub has_deserialize_with: bool,
    pub with: Option<String>,
    pub has_with: bool,
}

impl SerdeFieldAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Option<Self> {
        let attrs = Self {
            rename_serialize: get_string(attributes, FIELD_RENAME_SERIALIZE),
            rename_deserialize: get_string(attributes, FIELD_RENAME_DESERIALIZE),
            aliases: get_strings(attributes, FIELD_ALIASES),
            default: get_string(attributes, FIELD_DEFAULT),
            flatten: get_bool(attributes, FIELD_FLATTEN),
            skip_serializing: get_bool(attributes, FIELD_SKIP_SERIALIZING),
            skip_deserializing: get_bool(attributes, FIELD_SKIP_DESERIALIZING),
            skip_serializing_if: get_string(attributes, FIELD_SKIP_SERIALIZING_IF),
            serialize_with: get_string(attributes, FIELD_SERIALIZE_WITH),
            has_serialize_with: get_bool(attributes, FIELD_HAS_SERIALIZE_WITH),
            deserialize_with: get_string(attributes, FIELD_DESERIALIZE_WITH),
            has_deserialize_with: get_bool(attributes, FIELD_HAS_DESERIALIZE_WITH),
            with: get_string(attributes, FIELD_WITH),
            has_with: get_bool(attributes, FIELD_HAS_WITH),
        };

        has_any_field_attr(attributes).then_some(attrs)
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

fn get_bool(attributes: &Attributes, key: &str) -> bool {
    attributes
        .get_named_as::<bool>(key)
        .copied()
        .unwrap_or(false)
}

fn get_datatype(attributes: &Attributes, key: &str) -> Option<DataType> {
    attributes.get_named_as::<DataType>(key).cloned()
}

fn get_rename_rule(attributes: &Attributes, key: &str) -> Option<RenameRule> {
    get_string(attributes, key).map(|value| {
        RenameRule::from_str(&value)
            .unwrap_or_else(|_| panic!("invalid serde rename rule: {value}"))
    })
}

fn has_any_container_attr(attributes: &Attributes) -> bool {
    [
        CONTAINER_RENAME_SERIALIZE,
        CONTAINER_RENAME_DESERIALIZE,
        CONTAINER_RENAME_ALL_SERIALIZE,
        CONTAINER_RENAME_ALL_DESERIALIZE,
        CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
        CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
        CONTAINER_DENY_UNKNOWN_FIELDS,
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
    ]
    .into_iter()
    .any(|key| attributes.contains_key(key))
}

fn has_any_variant_attr(attributes: &Attributes) -> bool {
    [
        VARIANT_RENAME_SERIALIZE,
        VARIANT_RENAME_DESERIALIZE,
        VARIANT_ALIASES,
        VARIANT_RENAME_ALL_SERIALIZE,
        VARIANT_RENAME_ALL_DESERIALIZE,
        VARIANT_SKIP_SERIALIZING,
        VARIANT_SKIP_DESERIALIZING,
        VARIANT_SERIALIZE_WITH,
        VARIANT_HAS_SERIALIZE_WITH,
        VARIANT_DESERIALIZE_WITH,
        VARIANT_HAS_DESERIALIZE_WITH,
        VARIANT_WITH,
        VARIANT_HAS_WITH,
        VARIANT_OTHER,
        VARIANT_UNTAGGED,
    ]
    .into_iter()
    .any(|key| attributes.contains_key(key))
}

fn has_any_field_attr(attributes: &Attributes) -> bool {
    [
        FIELD_RENAME_SERIALIZE,
        FIELD_RENAME_DESERIALIZE,
        FIELD_ALIASES,
        FIELD_DEFAULT,
        FIELD_FLATTEN,
        FIELD_SKIP_SERIALIZING,
        FIELD_SKIP_DESERIALIZING,
        FIELD_SKIP_SERIALIZING_IF,
        FIELD_SERIALIZE_WITH,
        FIELD_HAS_SERIALIZE_WITH,
        FIELD_DESERIALIZE_WITH,
        FIELD_HAS_DESERIALIZE_WITH,
        FIELD_WITH,
        FIELD_HAS_WITH,
    ]
    .into_iter()
    .any(|key| attributes.contains_key(key))
}
