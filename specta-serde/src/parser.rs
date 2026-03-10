use std::borrow::Cow;

use specta::datatype::{
    Attribute, AttributeLiteral, AttributeMeta, AttributeNestedMeta, AttributeValue, DataType,
};

use crate::{inflection::RenameRule, repr::EnumRepr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    InvalidRenameRule(String),
}

#[derive(Debug, Clone)]
pub struct ConversionType {
    pub ty: DataType,
}

#[derive(Debug, Clone, Default)]
pub struct SerdeAttributes {
    pub rename: Option<String>,
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub rename_all: Option<RenameRule>,
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    pub rename_all_fields: Option<RenameRule>,
    pub rename_all_fields_serialize: Option<RenameRule>,
    pub rename_all_fields_deserialize: Option<RenameRule>,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub skip: bool,
    pub flatten: bool,
    pub default: bool,
    pub default_with: Option<String>,
    pub transparent: bool,
    pub deny_unknown_fields: bool,
    pub repr: Option<EnumRepr>,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub remote: Option<String>,
    pub from: Option<ConversionType>,
    pub try_from: Option<ConversionType>,
    pub into: Option<ConversionType>,
    pub other: bool,
    pub alias: Vec<String>,
    pub serialize_with: Option<String>,
    pub deserialize_with: Option<String>,
    pub with: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SerdeFieldAttributes {
    pub base: SerdeAttributes,
    pub alias: Vec<String>,
    pub serialize_with: Option<String>,
    pub deserialize_with: Option<String>,
    pub with: Option<String>,
    pub skip_serializing_if: Option<String>,
}

pub fn parse_serde_attributes(attributes: &[Attribute]) -> Result<SerdeAttributes, ParserError> {
    let mut attrs = SerdeAttributes::default();

    for attr in attributes {
        if attr.path == "serde" {
            parse_serde_attribute_content(&attr.kind, &mut attrs)?;
        }
    }

    finalize_enum_repr(&mut attrs);

    Ok(attrs)
}

pub fn parse_field_serde_attributes(
    attrs: &[Attribute],
) -> Result<SerdeFieldAttributes, ParserError> {
    let mut result = SerdeFieldAttributes {
        base: parse_serde_attributes(attrs)?,
        ..Default::default()
    };

    for attr in attrs {
        if attr.path == "serde" {
            parse_serde_field_attribute_content(&attr.kind, &mut result)?;
        }
    }

    Ok(result)
}

fn parse_serde_field_attribute_content(
    meta: &AttributeMeta,
    attrs: &mut SerdeFieldAttributes,
) -> Result<(), ParserError> {
    parse_serde_attribute_content(meta, &mut attrs.base)?;

    match meta {
        AttributeMeta::NameValue { key, value } => match key.as_str() {
            "alias" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(alias_name)) = value {
                    attrs.alias.push(alias_name.clone());
                }
            }
            "serialize_with" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(func_name)) = value {
                    attrs.serialize_with = Some(func_name.clone());
                }
            }
            "deserialize_with" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(func_name)) = value {
                    attrs.deserialize_with = Some(func_name.clone());
                }
            }
            "with" => match value {
                AttributeValue::Literal(AttributeLiteral::Str(module_path))
                | AttributeValue::Expr(module_path) => {
                    attrs.with = Some(module_path.clone());
                }
                _ => {}
            },
            "skip_serializing_if" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(func_name)) = value {
                    attrs.skip_serializing_if = Some(func_name.clone());
                }
            }
            _ => {}
        },
        AttributeMeta::List(list) => {
            for nested in list {
                if let AttributeNestedMeta::Meta(nested_meta) = nested {
                    parse_serde_field_attribute_content(nested_meta, attrs)?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_serde_attribute_content(
    meta: &AttributeMeta,
    attrs: &mut SerdeAttributes,
) -> Result<(), ParserError> {
    match meta {
        AttributeMeta::Path(path) => {
            parse_serde_path_attribute(attrs, path);
        }
        AttributeMeta::NameValue { key, value } => {
            parse_serde_name_value(attrs, key, value, None)?;
        }
        AttributeMeta::List(list) => {
            let mut has_serialize_deserialize = false;
            for nested in list {
                if let AttributeNestedMeta::Meta(AttributeMeta::NameValue { key, .. }) = nested
                    && (key == "serialize" || key == "deserialize")
                {
                    has_serialize_deserialize = true;
                    break;
                }
            }

            if has_serialize_deserialize {
                for nested in list {
                    if let AttributeNestedMeta::Meta(nested_meta) = nested {
                        parse_complex_serde_attribute(nested_meta, attrs, "rename")?;
                    }
                }
            } else {
                for nested in list {
                    match nested {
                        AttributeNestedMeta::Meta(nested_meta) => {
                            parse_serde_attribute_content(nested_meta, attrs)?;
                        }
                        AttributeNestedMeta::Literal(AttributeLiteral::Str(s)) => {
                            parse_serde_path_attribute(attrs, s);
                        }
                        AttributeNestedMeta::Expr(_) | AttributeNestedMeta::Literal(_) => {}
                    }
                }
            }
        }
    }

    finalize_enum_repr(attrs);

    Ok(())
}

fn parse_serde_name_value(
    attrs: &mut SerdeAttributes,
    key: &str,
    value: &AttributeValue,
    parent_key: Option<&str>,
) -> Result<(), ParserError> {
    if let Some(parent_key) = parent_key {
        match key {
            "serialize" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(name)) = value {
                    match parent_key {
                        "rename" => attrs.rename_serialize = Some(name.clone()),
                        "rename_all" => {
                            if let Ok(rule) = RenameRule::from_str(name) {
                                attrs.rename_all_serialize = Some(rule);
                            }
                        }
                        "rename_all_fields" => {
                            if let Ok(rule) = RenameRule::from_str(name) {
                                attrs.rename_all_fields_serialize = Some(rule);
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }
            "deserialize" => {
                if let AttributeValue::Literal(AttributeLiteral::Str(name)) = value {
                    match parent_key {
                        "rename" => attrs.rename_deserialize = Some(name.clone()),
                        "rename_all" => {
                            if let Ok(rule) = RenameRule::from_str(name) {
                                attrs.rename_all_deserialize = Some(rule);
                            }
                        }
                        "rename_all_fields" => {
                            if let Ok(rule) = RenameRule::from_str(name) {
                                attrs.rename_all_fields_deserialize = Some(rule);
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(());
            }
            _ => {}
        }
    }

    match key {
        "rename" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(name)) = value {
                attrs.rename = Some(name.clone());
            }
        }
        "rename_all" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(rule_str)) = value {
                attrs.rename_all = Some(
                    RenameRule::from_str(rule_str)
                        .map_err(|_| ParserError::InvalidRenameRule(rule_str.clone()))?,
                );
            }
        }
        "rename_all_fields" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(rule_str)) = value {
                attrs.rename_all_fields = Some(
                    RenameRule::from_str(rule_str)
                        .map_err(|_| ParserError::InvalidRenameRule(rule_str.clone()))?,
                );
            }
        }
        "tag" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(tag_name)) = value {
                attrs.tag = Some(tag_name.clone());
                if attrs.repr.is_none() {
                    attrs.repr = Some(EnumRepr::Internal {
                        tag: Cow::Owned(tag_name.clone()),
                    });
                }
            }
        }
        "content" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(content_name)) = value {
                attrs.content = Some(content_name.clone());
            }
        }
        "default" => match value {
            AttributeValue::Literal(AttributeLiteral::Bool(true)) => attrs.default = true,
            AttributeValue::Literal(AttributeLiteral::Str(func_path))
            | AttributeValue::Expr(func_path) => {
                attrs.default_with = Some(func_path.clone());
            }
            _ => {}
        },
        "remote" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(remote_type)) = value {
                attrs.remote = Some(remote_type.clone());
            }
        }
        "from" => {
            if let AttributeValue::Type(ty) = value {
                attrs.from = Some(ConversionType { ty: ty.clone() });
            }
        }
        "try_from" => {
            if let AttributeValue::Type(ty) = value {
                attrs.try_from = Some(ConversionType { ty: ty.clone() });
            }
        }
        "into" => {
            if let AttributeValue::Type(ty) = value {
                attrs.into = Some(ConversionType { ty: ty.clone() });
            }
        }
        "alias" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(alias_name)) = value {
                attrs.alias.push(alias_name.clone());
            }
        }
        "serialize_with" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(serialize_fn)) = value {
                attrs.serialize_with = Some(serialize_fn.clone());
            }
        }
        "deserialize_with" => {
            if let AttributeValue::Literal(AttributeLiteral::Str(deserialize_fn)) = value {
                attrs.deserialize_with = Some(deserialize_fn.clone());
            }
        }
        "with" => match value {
            AttributeValue::Literal(AttributeLiteral::Str(with_module))
            | AttributeValue::Expr(with_module) => {
                attrs.with = Some(with_module.clone());
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}

fn parse_complex_serde_attribute(
    meta: &AttributeMeta,
    attrs: &mut SerdeAttributes,
    parent_key: &str,
) -> Result<(), ParserError> {
    match meta {
        AttributeMeta::NameValue { key, value } => {
            parse_serde_name_value(attrs, key, value, Some(parent_key))?;
        }
        AttributeMeta::List(list) => {
            for nested in list {
                if let AttributeNestedMeta::Meta(nested_meta) = nested {
                    parse_complex_serde_attribute(nested_meta, attrs, parent_key)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_serde_path_attribute(attrs: &mut SerdeAttributes, attribute_name: &str) {
    match attribute_name {
        "skip" => attrs.skip = true,
        "skip_serializing" => attrs.skip_serializing = true,
        "skip_deserializing" => attrs.skip_deserializing = true,
        "flatten" => attrs.flatten = true,
        "default" => attrs.default = true,
        "transparent" => attrs.transparent = true,
        "untagged" => attrs.untagged = true,
        "deny_unknown_fields" => attrs.deny_unknown_fields = true,
        "other" => attrs.other = true,
        _ => {}
    }
}

fn finalize_enum_repr(attrs: &mut SerdeAttributes) {
    if let (Some(tag), Some(content)) = (&attrs.tag, &attrs.content) {
        attrs.repr = Some(EnumRepr::Adjacent {
            tag: Cow::Owned(tag.clone()),
            content: Cow::Owned(content.clone()),
        });
    }

    if attrs.untagged {
        attrs.repr = Some(EnumRepr::Untagged);
    }
}
