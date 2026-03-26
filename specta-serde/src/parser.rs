#![allow(missing_docs)]

use crate::inflection::RenameRule;
use specta::datatype::{
    Attributes, SERDE_CONTAINER_ATTRIBUTE_KEY, SERDE_FIELD_ATTRIBUTE_KEY,
    SERDE_VARIANT_ATTRIBUTE_KEY, SerdeContainerAttributeData, SerdeConversionTypeData,
    SerdeFieldAttributeData, SerdeVariantAttributeData,
};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConversionType {
    pub type_src: String,
}

#[allow(missing_docs)]
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
    pub resolved_from: Option<specta::datatype::DataType>,
    pub resolved_try_from: Option<specta::datatype::DataType>,
    pub resolved_into: Option<specta::datatype::DataType>,
    pub variant_identifier: bool,
    pub field_identifier: bool,
}

impl SerdeContainerAttrs {
    pub fn from_attributes(attributes: &Attributes) -> Option<Self> {
        attributes
            .get_named_as::<SerdeContainerAttributeData>(SERDE_CONTAINER_ATTRIBUTE_KEY)
            .map(|data| Self {
                rename_serialize: data.rename_serialize.clone(),
                rename_deserialize: data.rename_deserialize.clone(),
                rename_all_serialize: data.rename_all_serialize,
                rename_all_deserialize: data.rename_all_deserialize,
                rename_all_fields_serialize: data.rename_all_fields_serialize,
                rename_all_fields_deserialize: data.rename_all_fields_deserialize,
                deny_unknown_fields: data.deny_unknown_fields,
                tag: data.tag.clone(),
                content: data.content.clone(),
                untagged: data.untagged,
                default: data.default.clone(),
                transparent: data.transparent,
                from: data.from.as_ref().map(conversion_type),
                try_from: data.try_from.as_ref().map(conversion_type),
                into: data.into.as_ref().map(conversion_type),
                resolved_from: data.from.as_ref().map(|value| value.resolved.clone()),
                resolved_try_from: data.try_from.as_ref().map(|value| value.resolved.clone()),
                resolved_into: data.into.as_ref().map(|value| value.resolved.clone()),
                variant_identifier: data.variant_identifier,
                field_identifier: data.field_identifier,
            })
    }
}

#[allow(missing_docs)]
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
        attributes
            .get_named_as::<SerdeVariantAttributeData>(SERDE_VARIANT_ATTRIBUTE_KEY)
            .map(|data| Self {
                rename_serialize: data.rename_serialize.clone(),
                rename_deserialize: data.rename_deserialize.clone(),
                aliases: data.aliases.clone(),
                rename_all_serialize: data.rename_all_serialize,
                rename_all_deserialize: data.rename_all_deserialize,
                skip_serializing: data.skip_serializing,
                skip_deserializing: data.skip_deserializing,
                serialize_with: data.serialize_with.clone(),
                has_serialize_with: data.has_serialize_with,
                deserialize_with: data.deserialize_with.clone(),
                has_deserialize_with: data.has_deserialize_with,
                with: data.with.clone(),
                has_with: data.has_with,
                other: data.other,
                untagged: data.untagged,
            })
    }
}

#[allow(missing_docs)]
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
        attributes
            .get_named_as::<SerdeFieldAttributeData>(SERDE_FIELD_ATTRIBUTE_KEY)
            .map(|data| Self {
                rename_serialize: data.rename_serialize.clone(),
                rename_deserialize: data.rename_deserialize.clone(),
                aliases: data.aliases.clone(),
                default: data.default.clone(),
                flatten: data.flatten,
                skip_serializing: data.skip_serializing,
                skip_deserializing: data.skip_deserializing,
                skip_serializing_if: data.skip_serializing_if.clone(),
                serialize_with: data.serialize_with.clone(),
                has_serialize_with: data.has_serialize_with,
                deserialize_with: data.deserialize_with.clone(),
                has_deserialize_with: data.has_deserialize_with,
                with: data.with.clone(),
                has_with: data.has_with,
            })
    }
}

fn conversion_type(data: &SerdeConversionTypeData) -> ConversionType {
    ConversionType {
        type_src: data.type_src.clone(),
    }
}
