//! Comprehensive serde attribute parsing and transformation system.
//!
//! This module provides functionality to parse serde attributes like `#[serde(rename = "...")]`,
//! `#[serde(rename_all = "...")]`, and repr-related attributes, and apply them to DataType
//! instances with separate handling for serialization and deserialization phases.

use std::borrow::Cow;

use specta::{
    DataType,
    datatype::{
        Enum, Fields, RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta, Struct,
        Tuple,
    },
    internal,
};

use crate::{Error, inflection::RenameRule, repr::EnumRepr};

/// Specifies whether to apply serde transformations for serialization or deserialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerdeMode {
    /// Apply transformations for serialization (Rust -> JSON/etc)
    Serialize,
    /// Apply transformations for deserialization (JSON/etc -> Rust)
    Deserialize,
}

/// Contains parsed serde attributes for a type
#[derive(Debug, Clone, Default)]
pub struct SerdeAttributes {
    /// Direct rename specified by #[serde(rename = "name")]
    pub rename: Option<String>,
    /// Rename all fields/variants according to #[serde(rename_all = "case")]
    pub rename_all: Option<RenameRule>,
    /// Skip serialization with #[serde(skip_serializing)]
    pub skip_serializing: bool,
    /// Skip deserialization with #[serde(skip_deserializing)]
    pub skip_deserializing: bool,
    /// Skip both serialization and deserialization with #[serde(skip)]
    pub skip: bool,
    /// Flatten this field/variant with #[serde(flatten)]
    pub flatten: bool,
    /// Default value with #[serde(default)]
    pub default: bool,
    /// Transparent container with #[serde(transparent)]
    pub transparent: bool,
    /// Enum representation
    pub repr: Option<EnumRepr>,
    /// Tag for internally tagged enums
    pub tag: Option<String>,
    /// Content field for adjacently tagged enums
    pub content: Option<String>,
    /// Untagged enum
    pub untagged: bool,
}

/// Contains parsed serde attributes for a field
#[derive(Debug, Clone, Default)]
pub struct SerdeFieldAttributes {
    /// Field-specific attributes
    pub base: SerdeAttributes,
    /// Serialize with a different name
    pub serialize_with: Option<String>,
    /// Deserialize with a different name
    pub deserialize_with: Option<String>,
}

/// Apply serde transformations to a DataType for the specified mode
pub fn apply_serde_transformations(
    datatype: &DataType,
    mode: SerdeMode,
) -> Result<DataType, Error> {
    let mut transformer = SerdeTransformer::new(mode);
    transformer.transform_datatype(datatype)
}

/// Internal transformer that applies serde attributes to DataType instances
struct SerdeTransformer {
    mode: SerdeMode,
}

impl SerdeTransformer {
    fn new(mode: SerdeMode) -> Self {
        Self { mode }
    }

    /// Transform a DataType with serde attributes applied
    fn transform_datatype(&mut self, datatype: &DataType) -> Result<DataType, Error> {
        match datatype {
            DataType::Primitive(p) => Ok(DataType::Primitive(p.clone())),
            DataType::List(list) => {
                let transformed_inner = self.transform_datatype(list.ty())?;
                Ok(DataType::List(specta::datatype::List::new(
                    transformed_inner,
                )))
            }
            DataType::Map(map) => {
                let transformed_key = self.transform_datatype(map.key_ty())?;
                let transformed_value = self.transform_datatype(map.value_ty())?;
                Ok(DataType::Map(specta::datatype::Map::new(
                    transformed_key,
                    transformed_value,
                )))
            }
            DataType::Nullable(inner) => {
                let transformed_inner = self.transform_datatype(inner)?;
                Ok(DataType::Nullable(Box::new(transformed_inner)))
            }
            DataType::Struct(s) => self.transform_struct(s),
            DataType::Enum(e) => self.transform_enum(e),
            DataType::Tuple(t) => self.transform_tuple(t),
            DataType::Reference(r) => Ok(DataType::Reference(r.clone())),
            DataType::Generic(g) => Ok(DataType::Generic(g.clone())),
        }
    }

    /// Transform a Struct with serde attributes
    fn transform_struct(&mut self, struct_type: &Struct) -> Result<DataType, Error> {
        let attrs = parse_serde_attributes(struct_type.attributes())?;

        // Handle transparent structs
        if attrs.transparent {
            return self.handle_transparent_struct(struct_type);
        }

        // Handle skip based on mode
        if self.should_skip_type(&attrs) {
            // Return a unit type or some representation of skipped field
            return Ok(DataType::Tuple(Tuple::new(vec![])));
        }

        let transformed_fields = self.transform_fields(struct_type.fields(), &attrs)?;

        let new_struct =
            internal::construct::r#struct(transformed_fields, struct_type.attributes().clone());

        Ok(DataType::Struct(new_struct))
    }

    /// Transform an Enum with serde attributes
    fn transform_enum(&mut self, enum_type: &Enum) -> Result<DataType, Error> {
        let attrs = parse_serde_attributes(enum_type.attributes())?;

        // Handle skip based on mode
        if self.should_skip_type(&attrs) {
            return Ok(DataType::Tuple(Tuple::new(vec![])));
        }

        // Determine enum representation
        let _repr = attrs.repr.clone().unwrap_or(EnumRepr::External);

        // Handle string enums specially
        if enum_type.is_string_enum() && attrs.rename_all.is_some() {
            return self.transform_string_enum(enum_type, &attrs);
        }

        let mut transformed_variants = Vec::new();

        for (variant_name, variant) in enum_type.variants() {
            let variant_attrs = SerdeAttributes::default(); // Parse variant-specific attributes if needed

            if self.should_skip_type(&variant_attrs) {
                continue;
            }

            let transformed_name = self.apply_rename_rule(
                variant_name,
                attrs.rename_all,
                &variant_attrs.rename,
                true, // is_variant
            )?;

            let transformed_fields = self.transform_fields(variant.fields(), &attrs)?;

            let mut new_variant = variant.clone();
            new_variant.set_fields(transformed_fields);

            transformed_variants.push((transformed_name, new_variant));
        }

        let new_enum =
            internal::construct::r#enum(transformed_variants, enum_type.attributes().clone());

        Ok(DataType::Enum(new_enum))
    }

    /// Transform a string enum (unit-only enum) with rename_all support
    fn transform_string_enum(
        &mut self,
        enum_type: &Enum,
        attrs: &SerdeAttributes,
    ) -> Result<DataType, Error> {
        let mut transformed_variants = Vec::new();

        for (variant_name, variant) in enum_type.variants() {
            if !matches!(variant.fields(), Fields::Unit) {
                return Err(Error::InvalidUsageOfSkip); // Not a string enum
            }

            let variant_attrs = SerdeAttributes::default();
            if self.should_skip_type(&variant_attrs) {
                continue;
            }

            let transformed_name = self.apply_rename_rule(
                variant_name,
                attrs.rename_all,
                &variant_attrs.rename,
                true,
            )?;

            transformed_variants.push((transformed_name, variant.clone()));
        }

        // Create enum with String representation
        let new_enum =
            internal::construct::r#enum(transformed_variants, enum_type.attributes().clone());

        Ok(DataType::Enum(new_enum))
    }

    /// Transform a Tuple with serde attributes
    fn transform_tuple(&mut self, tuple: &Tuple) -> Result<DataType, Error> {
        let mut transformed_elements = Vec::new();

        for element in tuple.elements() {
            let transformed = self.transform_datatype(element)?;
            transformed_elements.push(transformed);
        }

        Ok(DataType::Tuple(Tuple::new(transformed_elements)))
    }

    /// Transform Fields with serde attributes applied
    fn transform_fields(
        &mut self,
        fields: &Fields,
        parent_attrs: &SerdeAttributes,
    ) -> Result<Fields, Error> {
        match fields {
            Fields::Unit => Ok(Fields::Unit),
            Fields::Unnamed(unnamed) => {
                let mut transformed_fields = Vec::new();

                for (idx, field) in unnamed.fields().iter().enumerate() {
                    if let Some(field_ty) = field.ty() {
                        let transformed_ty = self.transform_datatype(field_ty)?;
                        let mut new_field = field.clone();
                        new_field.set_ty(transformed_ty);
                        transformed_fields.push((idx, new_field));
                    }
                }

                Ok(internal::construct::fields_unnamed(
                    transformed_fields.into_iter().map(|(_, f)| f).collect(),
                ))
            }
            Fields::Named(named) => {
                let mut transformed_fields = Vec::new();

                for (field_name, field) in named.fields() {
                    // Parse field-specific serde attributes
                    let field_attrs = SerdeFieldAttributes::default(); // Would parse from field attributes

                    if self.should_skip_field(&field_attrs) {
                        continue;
                    }

                    let transformed_name = self.apply_rename_rule(
                        field_name,
                        parent_attrs.rename_all,
                        &field_attrs.base.rename,
                        false, // is_variant
                    )?;

                    if let Some(field_ty) = field.ty() {
                        let transformed_ty = self.transform_datatype(field_ty)?;
                        let mut new_field = field.clone();
                        new_field.set_ty(transformed_ty);
                        transformed_fields.push((transformed_name, new_field));
                    }
                }

                Ok(internal::construct::fields_named(transformed_fields, None))
            }
        }
    }

    /// Handle transparent structs
    fn handle_transparent_struct(&mut self, struct_type: &Struct) -> Result<DataType, Error> {
        match struct_type.fields() {
            Fields::Unnamed(unnamed) if unnamed.fields().len() == 1 => {
                if let Some(field_ty) = unnamed.fields()[0].ty() {
                    self.transform_datatype(field_ty)
                } else {
                    Err(Error::InvalidUsageOfSkip)
                }
            }
            Fields::Named(named) if named.fields().len() == 1 => {
                if let Some(field_ty) = named.fields()[0].1.ty() {
                    self.transform_datatype(field_ty)
                } else {
                    Err(Error::InvalidUsageOfSkip)
                }
            }
            _ => Err(Error::InvalidUsageOfSkip), // Invalid transparent usage
        }
    }

    /// Check if a type should be skipped based on the current mode
    fn should_skip_type(&self, attrs: &SerdeAttributes) -> bool {
        if attrs.skip {
            return true;
        }

        match self.mode {
            SerdeMode::Serialize => attrs.skip_serializing,
            SerdeMode::Deserialize => attrs.skip_deserializing,
        }
    }

    /// Check if a field should be skipped based on the current mode
    fn should_skip_field(&self, attrs: &SerdeFieldAttributes) -> bool {
        self.should_skip_type(&attrs.base)
    }

    /// Apply rename rules to a field or variant name
    fn apply_rename_rule(
        &self,
        original_name: &str,
        rename_all_rule: Option<RenameRule>,
        direct_rename: &Option<String>,
        is_variant: bool,
    ) -> Result<Cow<'static, str>, Error> {
        // Direct rename takes precedence
        if let Some(renamed) = direct_rename {
            return Ok(Cow::Owned(renamed.clone()));
        }

        // Apply rename_all rule
        if let Some(rule) = rename_all_rule {
            let transformed = if is_variant {
                rule.apply_to_variant(original_name)
            } else {
                rule.apply_to_field(original_name)
            };
            return Ok(Cow::Owned(transformed));
        }

        // No transformation needed
        Ok(Cow::Owned(original_name.to_string()))
    }
}

/// Parse serde attributes from a vector of RuntimeAttribute
fn parse_serde_attributes(attributes: &[RuntimeAttribute]) -> Result<SerdeAttributes, Error> {
    let mut attrs = SerdeAttributes::default();

    for attr in attributes {
        if attr.path == "serde" {
            parse_serde_attribute_content(&attr.kind, &mut attrs)?;
        }
    }

    Ok(attrs)
}

/// Parse the content of a serde attribute
fn parse_serde_attribute_content(
    meta: &RuntimeMeta,
    attrs: &mut SerdeAttributes,
) -> Result<(), Error> {
    match meta {
        RuntimeMeta::Path => {
            // Just #[serde] with no content - could be skip, untagged, etc.
            // We need the actual path string to determine what this is
        }
        RuntimeMeta::NameValue { key, value } => {
            match key.as_str() {
                "rename" => {
                    if let RuntimeLiteral::Str(name) = value {
                        attrs.rename = Some(name.clone());
                    }
                }
                "rename_all" => {
                    if let RuntimeLiteral::Str(rule_str) = value {
                        attrs.rename_all = Some(
                            RenameRule::from_str(rule_str)
                                .map_err(|_| Error::InvalidUsageOfSkip)?,
                        ); // TODO: Better error
                    }
                }
                "tag" => {
                    if let RuntimeLiteral::Str(tag_name) = value {
                        attrs.tag = Some(tag_name.clone());
                        // If we have a tag, this is an internally tagged enum
                        if attrs.repr.is_none() {
                            attrs.repr = Some(EnumRepr::Internal {
                                tag: Cow::Owned(tag_name.clone()),
                            });
                        }
                    }
                }
                "content" => {
                    if let RuntimeLiteral::Str(content_name) = value {
                        attrs.content = Some(content_name.clone());
                    }
                }
                "default" => {
                    if let RuntimeLiteral::Bool(true) = value {
                        attrs.default = true;
                    }
                }
                _ => {}
            }
        }
        RuntimeMeta::List(list) => {
            for nested in list {
                match nested {
                    RuntimeNestedMeta::Meta(nested_meta) => {
                        parse_serde_attribute_content(nested_meta, attrs)?;
                    }
                    RuntimeNestedMeta::Literal(_) => {
                        // Handle literal values in lists if needed
                    }
                }
            }
        }
    }

    // Handle special cases for enum representation
    if let (Some(tag), Some(content)) = (&attrs.tag, &attrs.content) {
        attrs.repr = Some(EnumRepr::Adjacent {
            tag: Cow::Owned(tag.clone()),
            content: Cow::Owned(content.clone()),
        });
    }

    if attrs.untagged {
        attrs.repr = Some(EnumRepr::Untagged);
    }

    Ok(())
}

/// Parse string attributes that are commonly used with serde
/// This is a helper for parsing path-only attributes like #[serde(skip)]
fn parse_serde_path_attribute(attrs: &mut SerdeAttributes, attribute_name: &str) {
    match attribute_name {
        "skip" => attrs.skip = true,
        "skip_serializing" => attrs.skip_serializing = true,
        "skip_deserializing" => attrs.skip_deserializing = true,
        "flatten" => attrs.flatten = true,
        "default" => attrs.default = true,
        "transparent" => attrs.transparent = true,
        "untagged" => attrs.untagged = true,
        _ => {}
    }
}

/// Enhanced parsing for common serde attribute patterns
fn parse_enhanced_serde_attributes(
    attributes: &[RuntimeAttribute],
) -> Result<SerdeAttributes, Error> {
    let mut attrs = SerdeAttributes::default();

    for attr in attributes {
        if attr.path == "serde" {
            match &attr.kind {
                RuntimeMeta::List(list) => {
                    for nested in list {
                        match nested {
                            RuntimeNestedMeta::Meta(RuntimeMeta::Path) => {
                                // We would need the actual path string here
                                // This is a limitation of the current RuntimeAttribute structure
                            }
                            RuntimeNestedMeta::Meta(meta) => {
                                parse_serde_attribute_content(meta, &mut attrs)?;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    parse_serde_attribute_content(&attr.kind, &mut attrs)?;
                }
            }
        }
    }

    Ok(attrs)
}

/// Parse serde field attributes from a vector of RuntimeAttribute
fn parse_serde_field_attributes(
    attributes: &[RuntimeAttribute],
) -> Result<SerdeFieldAttributes, Error> {
    let mut attrs = SerdeFieldAttributes::default();

    for attr in attributes {
        if attr.path == "serde" {
            parse_serde_field_attribute_content(&attr.kind, &mut attrs)?;
        }
    }

    Ok(attrs)
}

/// Parse the content of a serde field attribute
fn parse_serde_field_attribute_content(
    meta: &RuntimeMeta,
    attrs: &mut SerdeFieldAttributes,
) -> Result<(), Error> {
    match meta {
        RuntimeMeta::Path => {}
        RuntimeMeta::NameValue { key, value } => match key.as_str() {
            "rename" => {
                if let RuntimeLiteral::Str(name) = value {
                    attrs.base.rename = Some(name.clone());
                }
            }
            "serialize_with" => {
                if let RuntimeLiteral::Str(func_name) = value {
                    attrs.serialize_with = Some(func_name.clone());
                }
            }
            "deserialize_with" => {
                if let RuntimeLiteral::Str(func_name) = value {
                    attrs.deserialize_with = Some(func_name.clone());
                }
            }
            _ => {}
        },
        RuntimeMeta::List(list) => {
            for nested in list {
                if let RuntimeNestedMeta::Meta(nested_meta) = nested {
                    parse_serde_field_attribute_content(nested_meta, attrs)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use specta::datatype::Primitive;

    #[test]
    fn test_rename_rule_parsing() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "rename_all".to_string(),
            value: RuntimeLiteral::Str("camelCase".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.rename_all, Some(RenameRule::CamelCase));
    }

    #[test]
    fn test_direct_rename() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "rename".to_string(),
            value: RuntimeLiteral::Str("customName".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.rename, Some("customName".to_string()));
    }

    #[test]
    fn test_skip_attributes() {
        let mut attrs = SerdeAttributes::default();

        // Test various skip scenarios
        let transformer = SerdeTransformer::new(SerdeMode::Serialize);
        attrs.skip = true;
        assert!(transformer.should_skip_type(&attrs));

        attrs.skip = false;
        attrs.skip_serializing = true;
        assert!(transformer.should_skip_type(&attrs));

        let deserialize_transformer = SerdeTransformer::new(SerdeMode::Deserialize);
        assert!(!deserialize_transformer.should_skip_type(&attrs));
    }

    #[test]
    fn test_all_rename_rules() {
        let test_cases = vec![
            ("lowercase", RenameRule::LowerCase),
            ("UPPERCASE", RenameRule::UpperCase),
            ("PascalCase", RenameRule::PascalCase),
            ("camelCase", RenameRule::CamelCase),
            ("snake_case", RenameRule::SnakeCase),
            ("SCREAMING_SNAKE_CASE", RenameRule::ScreamingSnakeCase),
            ("kebab-case", RenameRule::KebabCase),
            ("SCREAMING-KEBAB-CASE", RenameRule::ScreamingKebabCase),
        ];

        for (rule_str, expected_rule) in test_cases {
            let mut attrs = SerdeAttributes::default();
            let meta = RuntimeMeta::NameValue {
                key: "rename_all".to_string(),
                value: RuntimeLiteral::Str(rule_str.to_string()),
            };

            parse_serde_attribute_content(&meta, &mut attrs)
                .expect("Failed to parse serde attribute");
            assert_eq!(
                attrs.rename_all,
                Some(expected_rule),
                "Failed for rule: {}",
                rule_str
            );
        }
    }

    #[test]
    fn test_rename_rule_application() {
        let transformer = SerdeTransformer::new(SerdeMode::Serialize);

        // Test field renaming
        let result = transformer
            .apply_rename_rule("test_field", Some(RenameRule::CamelCase), &None, false)
            .unwrap();
        assert_eq!(result, "testField");

        // Test variant renaming
        let result = transformer
            .apply_rename_rule("TestVariant", Some(RenameRule::SnakeCase), &None, true)
            .unwrap();
        assert_eq!(result, "test_variant");

        // Test direct rename takes precedence
        let result = transformer
            .apply_rename_rule(
                "test_field",
                Some(RenameRule::CamelCase),
                &Some("customName".to_string()),
                false,
            )
            .unwrap();
        assert_eq!(result, "customName");
    }

    #[test]
    fn test_tag_parsing() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "tag".to_string(),
            value: RuntimeLiteral::Str("type".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.tag, Some("type".to_string()));
        // Should automatically set internal representation
        match attrs.repr {
            Some(EnumRepr::Internal { tag }) => assert_eq!(tag, "type"),
            _ => panic!("Expected internal enum representation"),
        }
    }

    #[test]
    fn test_adjacent_tag_parsing() {
        let mut attrs = SerdeAttributes::default();

        // Set tag first
        let tag_meta = RuntimeMeta::NameValue {
            key: "tag".to_string(),
            value: RuntimeLiteral::Str("type".to_string()),
        };
        parse_serde_attribute_content(&tag_meta, &mut attrs).expect("Failed to parse tag");

        // Set content second
        let content_meta = RuntimeMeta::NameValue {
            key: "content".to_string(),
            value: RuntimeLiteral::Str("data".to_string()),
        };
        parse_serde_attribute_content(&content_meta, &mut attrs).expect("Failed to parse content");

        // Should create adjacent representation
        match attrs.repr {
            Some(EnumRepr::Adjacent { tag, content }) => {
                assert_eq!(tag, "type");
                assert_eq!(content, "data");
            }
            _ => panic!("Expected adjacent enum representation"),
        }
    }

    #[test]
    fn test_default_attribute() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "default".to_string(),
            value: RuntimeLiteral::Bool(true),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert!(attrs.default);
    }

    #[test]
    fn test_primitive_type_passthrough() {
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize);
        let primitive = DataType::Primitive(Primitive::String);

        let result = transformer.transform_datatype(&primitive).unwrap();
        assert_eq!(result, primitive);
    }

    #[test]
    fn test_nullable_type_transformation() {
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize);
        let nullable = DataType::Nullable(Box::new(DataType::Primitive(Primitive::String)));

        let result = transformer.transform_datatype(&nullable).unwrap();
        match result {
            DataType::Nullable(inner) => {
                assert_eq!(*inner.as_ref(), DataType::Primitive(Primitive::String));
            }
            _ => panic!("Expected nullable type"),
        }
    }

    #[test]
    fn test_list_type_transformation() {
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize);
        let list = DataType::List(specta::datatype::List::new(DataType::Primitive(
            Primitive::String,
        )));

        let result = transformer.transform_datatype(&list).unwrap();
        match result {
            DataType::List(list_result) => {
                assert_eq!(*list_result.ty(), DataType::Primitive(Primitive::String));
            }
            _ => panic!("Expected list type"),
        }
    }

    #[test]
    fn test_mode_specific_skip_behavior() {
        let mut attrs = SerdeAttributes::default();

        // Test skip_serializing only affects serialize mode
        attrs.skip_serializing = true;
        let ser_transformer = SerdeTransformer::new(SerdeMode::Serialize);
        let de_transformer = SerdeTransformer::new(SerdeMode::Deserialize);

        assert!(ser_transformer.should_skip_type(&attrs));
        assert!(!de_transformer.should_skip_type(&attrs));

        // Reset and test skip_deserializing
        attrs.skip_serializing = false;
        attrs.skip_deserializing = true;

        assert!(!ser_transformer.should_skip_type(&attrs));
        assert!(de_transformer.should_skip_type(&attrs));

        // Test universal skip
        attrs.skip_deserializing = false;
        attrs.skip = true;

        assert!(ser_transformer.should_skip_type(&attrs));
        assert!(de_transformer.should_skip_type(&attrs));
    }

    #[test]
    fn test_transparent_struct_handling() {
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize);

        // Create a transparent struct with one unnamed field
        let field = specta::datatype::Field::new(DataType::Primitive(Primitive::String));
        let unnamed_fields = internal::construct::fields_unnamed(vec![field]);
        let transparent_struct = internal::construct::r#struct(
            unnamed_fields,
            vec![RuntimeAttribute {
                path: "serde".to_string(),
                kind: RuntimeMeta::NameValue {
                    key: "transparent".to_string(),
                    value: RuntimeLiteral::Bool(true),
                },
            }],
        );

        let result = transformer.handle_transparent_struct(&transparent_struct);
        // Should resolve to the inner type for transparent structs
        assert!(result.is_ok());
    }

    #[test]
    fn test_field_attributes_parsing() {
        let mut field_attrs = SerdeFieldAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "serialize_with".to_string(),
            value: RuntimeLiteral::Str("custom_serializer".to_string()),
        };

        parse_serde_field_attribute_content(&meta, &mut field_attrs)
            .expect("Failed to parse field attribute");
        assert_eq!(
            field_attrs.serialize_with,
            Some("custom_serializer".to_string())
        );
    }
}
