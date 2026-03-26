#![allow(missing_docs)]

use super::DataType;
use std::fmt::{self, Debug, Display};

pub const SERDE_CONTAINER_ATTRIBUTE_KEY: &str = "serde:container";
pub const SERDE_FIELD_ATTRIBUTE_KEY: &str = "serde:field";
pub const SERDE_VARIANT_ATTRIBUTE_KEY: &str = "serde:variant";

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SerdeRenameRule {
    None,
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

static RENAME_RULES: &[(&str, SerdeRenameRule)] = &[
    ("lowercase", SerdeRenameRule::LowerCase),
    ("UPPERCASE", SerdeRenameRule::UpperCase),
    ("PascalCase", SerdeRenameRule::PascalCase),
    ("camelCase", SerdeRenameRule::CamelCase),
    ("snake_case", SerdeRenameRule::SnakeCase),
    ("SCREAMING_SNAKE_CASE", SerdeRenameRule::ScreamingSnakeCase),
    ("kebab-case", SerdeRenameRule::KebabCase),
    ("SCREAMING-KEBAB-CASE", SerdeRenameRule::ScreamingKebabCase),
];

impl SerdeRenameRule {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(rename_all_str: &str) -> Result<Self, ParseError<'_>> {
        for (name, rule) in RENAME_RULES {
            if rename_all_str == *name {
                return Ok(*rule);
            }
        }

        Err(ParseError {
            unknown: rename_all_str,
        })
    }

    pub fn apply_to_variant(self, variant: &str) -> String {
        match self {
            Self::None | Self::PascalCase => variant.to_owned(),
            Self::LowerCase => variant.to_ascii_lowercase(),
            Self::UpperCase => variant.to_ascii_uppercase(),
            Self::CamelCase => variant[..1].to_ascii_lowercase() + &variant[1..],
            Self::SnakeCase => {
                let mut snake = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            }
            Self::ScreamingSnakeCase => Self::SnakeCase
                .apply_to_variant(variant)
                .to_ascii_uppercase(),
            Self::KebabCase => Self::SnakeCase.apply_to_variant(variant).replace('_', "-"),
            Self::ScreamingKebabCase => Self::ScreamingSnakeCase
                .apply_to_variant(variant)
                .replace('_', "-"),
        }
    }

    pub fn apply_to_field(self, field: &str) -> String {
        match self {
            Self::None | Self::LowerCase | Self::SnakeCase => field.to_owned(),
            Self::UpperCase => field.to_ascii_uppercase(),
            Self::PascalCase => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            Self::CamelCase => {
                let pascal = Self::PascalCase.apply_to_field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            }
            Self::ScreamingSnakeCase => field.to_ascii_uppercase(),
            Self::KebabCase => field.replace('_', "-"),
            Self::ScreamingKebabCase => Self::ScreamingSnakeCase
                .apply_to_field(field)
                .replace('_', "-"),
        }
    }
}

pub struct ParseError<'a> {
    unknown: &'a str,
}

impl Display for ParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown rename rule `rename_all = ")?;
        Debug::fmt(self.unknown, f)?;
        f.write_str("`, expected one of ")?;
        for (i, (name, _)) in RENAME_RULES.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            Debug::fmt(name, f)?;
        }
        Ok(())
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SerdeConversionTypeData {
    pub type_src: String,
    pub resolved: DataType,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeContainerAttributeData {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub rename_all_serialize: Option<SerdeRenameRule>,
    pub rename_all_deserialize: Option<SerdeRenameRule>,
    pub rename_all_fields_serialize: Option<SerdeRenameRule>,
    pub rename_all_fields_deserialize: Option<SerdeRenameRule>,
    pub deny_unknown_fields: bool,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub default: Option<String>,
    pub transparent: bool,
    pub from: Option<SerdeConversionTypeData>,
    pub try_from: Option<SerdeConversionTypeData>,
    pub into: Option<SerdeConversionTypeData>,
    pub variant_identifier: bool,
    pub field_identifier: bool,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeVariantAttributeData {
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    pub aliases: Vec<String>,
    pub rename_all_serialize: Option<SerdeRenameRule>,
    pub rename_all_deserialize: Option<SerdeRenameRule>,
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

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SerdeFieldAttributeData {
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
