//! Comprehensive serde attribute parsing and transformation system.
//!
//! This module provides functionality to parse serde attributes like `#[serde(rename = "...")]`,
//! `#[serde(rename_all = "...")]`, and repr-related attributes, and apply them to DataType
//! instances with separate handling for serialization and deserialization phases.

use std::{borrow::Cow, fmt};

use specta::{
    datatype::{
        DataType, Enum, Fields, RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta,
        Struct, Tuple,
    },
    internal,
};

use crate::{Error, inflection::RenameRule, repr::EnumRepr};

/// Specifies whether to apply serde transformations for serialization or deserialization
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerdeMode {
    /// Apply transformations for serialization (Rust -> JSON/etc)
    Serialize,
    /// Apply transformations for deserialization (JSON/etc -> Rust)
    Deserialize,
    /// Apply transformations for both serialization and deserialization
    ///
    /// This mode uses common transformations (like `rename`, `rename_all`, `skip`)
    /// but skips mode-specific attributes (like `rename_serialize`, `skip_serializing`).
    /// A field/type is only skipped if it's skipped in both modes.
    #[default]
    Both,
}

impl fmt::Display for SerdeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Contains parsed serde attributes for a type
#[derive(Debug, Clone, Default)]
pub struct SerdeAttributes {
    /// Direct rename specified by #[serde(rename = "name")]
    pub rename: Option<String>,
    /// Separate names for serialization and deserialization
    pub rename_serialize: Option<String>,
    pub rename_deserialize: Option<String>,
    /// Rename all fields/variants according to #[serde(rename_all = "case")]
    pub rename_all: Option<RenameRule>,
    /// Separate rename_all for serialization and deserialization
    pub rename_all_serialize: Option<RenameRule>,
    pub rename_all_deserialize: Option<RenameRule>,
    /// Rename all fields in enum variants according to #[serde(rename_all_fields = "case")]
    pub rename_all_fields: Option<RenameRule>,
    /// Separate rename_all_fields for serialization and deserialization
    pub rename_all_fields_serialize: Option<RenameRule>,
    pub rename_all_fields_deserialize: Option<RenameRule>,
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
    /// Default value with custom function #[serde(default = "path")]
    pub default_with: Option<String>,
    /// Transparent container with #[serde(transparent)]
    pub transparent: bool,
    /// Deny unknown fields #[serde(deny_unknown_fields)]
    pub deny_unknown_fields: bool,
    /// Enum representation
    pub repr: Option<EnumRepr>,
    /// Tag for internally tagged enums
    pub tag: Option<String>,
    /// Content field for adjacently tagged enums
    pub content: Option<String>,
    /// Untagged enum
    pub untagged: bool,
    /// Remote type definition #[serde(remote = "...")]
    pub remote: Option<String>,
    /// Convert from another type #[serde(from = "...")]
    pub from: Option<String>,
    /// Try convert from another type #[serde(try_from = "...")]
    pub try_from: Option<String>,
    /// Convert into another type #[serde(into = "...")]
    pub into: Option<String>,
    /// Deserialize unknown variants to this variant #[serde(other)]
    pub other: bool,
    /// Aliases for deserialization #[serde(alias = "name")]
    pub alias: Vec<String>,
    /// Serialize with custom function #[serde(serialize_with = "path")]
    pub serialize_with: Option<String>,
    /// Deserialize with custom function #[serde(deserialize_with = "path")]
    pub deserialize_with: Option<String>,
    /// Combined serialize/deserialize with module #[serde(with = "module")]
    pub with: Option<String>,
}

/// Contains parsed serde attributes for a field
#[derive(Debug, Clone, Default)]
pub struct SerdeFieldAttributes {
    /// Field-specific attributes
    pub base: SerdeAttributes,
    /// Field alias for deserialization #[serde(alias = "name")]
    #[allow(dead_code)]
    pub alias: Vec<String>,
    /// Serialize with custom function #[serde(serialize_with = "path")]
    pub serialize_with: Option<String>,
    /// Deserialize with custom function #[serde(deserialize_with = "path")]
    pub deserialize_with: Option<String>,
    /// Combined serialize/deserialize with module #[serde(with = "module")]
    pub with: Option<String>,
    /// Skip serializing if condition is true #[serde(skip_serializing_if = "path")]
    pub skip_serializing_if: Option<String>,
}

/// Apply serde transformations to a DataType for the specified mode
pub fn apply_serde_transformations(
    datatype: &DataType,
    mode: SerdeMode,
) -> Result<DataType, Error> {
    let mut transformer = SerdeTransformer::new(mode, None);
    transformer.transform_datatype(datatype)
}

/// Apply serde transformations to a DataType with a type name (for struct tagging)
pub fn apply_serde_transformations_with_name(
    datatype: &DataType,
    type_name: &Cow<'static, str>,
    mode: SerdeMode,
) -> Result<DataType, Error> {
    let mut transformer = SerdeTransformer::new(mode, Some(type_name.to_string()));
    transformer.transform_datatype(datatype)
}

/// Internal transformer that applies serde attributes to DataType instances
struct SerdeTransformer {
    mode: SerdeMode,
    type_name: Option<String>,
}

impl SerdeTransformer {
    fn new(mode: SerdeMode, type_name: Option<String>) -> Self {
        Self { mode, type_name }
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

        let mut transformed_fields = self.transform_fields(struct_type.fields(), &attrs)?;

        // If the struct has a tag attribute (for struct tagging), add the tag field
        if let Some(tag_name) = &attrs.tag {
            transformed_fields =
                self.add_struct_tag_field(tag_name, struct_type, transformed_fields)?;
        }

        let mut new_struct = Struct::unit();
        new_struct.set_fields(transformed_fields);
        new_struct.set_attributes(struct_type.attributes().clone());

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
        let repr = attrs.repr.clone().unwrap_or(EnumRepr::External);

        // Handle string enums specially
        if enum_type.is_string_enum() && attrs.rename_all.is_some() {
            return self.transform_string_enum(enum_type, &attrs);
        }

        let mut transformed_variants = Vec::new();

        for (variant_name, variant) in enum_type.variants() {
            let variant_attrs = parse_serde_attributes(variant.attributes())?;

            if self.should_skip_type(&variant_attrs) {
                continue;
            }

            let transformed_name = self.apply_rename_rule(
                variant_name,
                attrs.rename_all,
                &variant_attrs.rename,
                &variant_attrs,
                true, // is_variant
            )?;

            // For enum variant fields, use rename_all_fields instead of rename_all
            let mut variant_field_attrs = attrs.clone();
            // Apply rename_all_fields to variant fields based on mode
            match self.mode {
                SerdeMode::Serialize => {
                    if let Some(rule) = attrs
                        .rename_all_fields_serialize
                        .or(attrs.rename_all_fields)
                    {
                        variant_field_attrs.rename_all = Some(rule);
                        variant_field_attrs.rename_all_serialize = Some(rule);
                    }
                }
                SerdeMode::Deserialize => {
                    if let Some(rule) = attrs
                        .rename_all_fields_deserialize
                        .or(attrs.rename_all_fields)
                    {
                        variant_field_attrs.rename_all = Some(rule);
                        variant_field_attrs.rename_all_deserialize = Some(rule);
                    }
                }
                SerdeMode::Both => {
                    // For Both mode, use rename_all_fields if available
                    if let Some(rule) = attrs.rename_all_fields {
                        variant_field_attrs.rename_all = Some(rule);
                    }
                }
            }

            let transformed_fields =
                self.transform_fields(variant.fields(), &variant_field_attrs)?;

            // Apply enum tagging transformation based on representation
            let final_fields =
                self.apply_enum_tagging(&repr, &transformed_name, transformed_fields)?;

            let mut new_variant = variant.clone();
            new_variant.set_fields(final_fields);

            transformed_variants.push((transformed_name, new_variant));
        }

        let mut new_enum = Enum::new();
        *new_enum.variants_mut() = transformed_variants;
        *new_enum.attributes_mut() = enum_type.attributes().clone();

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

            let variant_attrs = parse_serde_attributes(variant.attributes())?;
            if self.should_skip_type(&variant_attrs) {
                continue;
            }

            let transformed_name = self.apply_rename_rule(
                variant_name,
                attrs.rename_all,
                &variant_attrs.rename,
                &variant_attrs,
                true,
            )?;

            transformed_variants.push((transformed_name, variant.clone()));
        }

        // Create enum with String representation
        let mut new_enum = Enum::new();
        *new_enum.variants_mut() = transformed_variants;
        *new_enum.attributes_mut() = enum_type.attributes().clone();

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
                    vec![],
                ))
            }
            Fields::Named(named) => {
                let mut transformed_fields = Vec::new();

                for (field_name, field) in named.fields() {
                    // Parse field-specific serde attributes from stored runtime attributes
                    let field_attrs = parse_field_serde_attributes(field.attributes());

                    if self.should_skip_field(&field_attrs) {
                        continue;
                    }

                    let transformed_name = self.apply_rename_rule(
                        field_name,
                        parent_attrs.rename_all,
                        &field_attrs.base.rename,
                        &field_attrs.base,
                        false, // is_variant
                    )?;

                    if let Some(field_ty) = field.ty() {
                        let transformed_ty = self.transform_datatype(field_ty)?;
                        let mut new_field = field.clone();
                        new_field.set_ty(transformed_ty);

                        // Set optional flag based on serde attributes
                        // This matches the behavior that was previously in the macro
                        if field_attrs.skip_serializing_if.is_some() || field_attrs.base.default {
                            new_field.set_optional(true);
                        }

                        transformed_fields.push((transformed_name, new_field));
                    }
                }

                Ok(internal::construct::fields_named(
                    transformed_fields,
                    vec![],
                ))
            }
        }
    }

    /// Handle transparent structs
    fn handle_transparent_struct(&mut self, struct_type: &Struct) -> Result<DataType, Error> {
        use specta::datatype::{skip_fields, skip_fields_named};

        match struct_type.fields() {
            Fields::Unnamed(unnamed) => {
                // Collect non-skipped fields
                let non_skipped: Vec<_> = skip_fields(unnamed.fields()).collect();
                
                if non_skipped.len() == 1 {
                    self.transform_datatype(non_skipped[0].1)
                } else {
                    Err(Error::InvalidUsageOfSkip)
                }
            }
            Fields::Named(named) => {
                // Collect non-skipped fields
                let non_skipped: Vec<_> = skip_fields_named(named.fields()).collect();
                
                if non_skipped.len() == 1 {
                    self.transform_datatype(non_skipped[0].1.1)
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
            // For Both mode, only skip if skipped in both directions
            SerdeMode::Both => attrs.skip_serializing && attrs.skip_deserializing,
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
        attrs: &SerdeAttributes,
        is_variant: bool,
    ) -> Result<Cow<'static, str>, Error> {
        // Direct rename takes precedence
        if let Some(renamed) = direct_rename {
            return Ok(Cow::Owned(renamed.clone()));
        }

        // Check for mode-specific renames
        match self.mode {
            SerdeMode::Serialize => {
                if let Some(renamed) = &attrs.rename_serialize {
                    return Ok(Cow::Owned(renamed.clone()));
                }
            }
            SerdeMode::Deserialize => {
                if let Some(renamed) = &attrs.rename_deserialize {
                    return Ok(Cow::Owned(renamed.clone()));
                }
            }
            SerdeMode::Both => {
                // For Both mode, don't use mode-specific renames
                // Only use if serialize and deserialize renames match
                if let (Some(ser), Some(de)) = (&attrs.rename_serialize, &attrs.rename_deserialize)
                    && ser == de
                {
                    return Ok(Cow::Owned(ser.clone()));
                }
            }
        }

        // Apply mode-specific rename_all rule
        let rule = match self.mode {
            SerdeMode::Serialize => attrs
                .rename_all_serialize
                .or(attrs.rename_all_fields_serialize)
                .or(rename_all_rule),
            SerdeMode::Deserialize => attrs
                .rename_all_deserialize
                .or(attrs.rename_all_fields_deserialize)
                .or(rename_all_rule),
            SerdeMode::Both => {
                // For Both mode, use common rename_all or only if both modes match
                if let (Some(ser), Some(de)) =
                    (attrs.rename_all_serialize, attrs.rename_all_deserialize)
                {
                    if ser == de {
                        Some(ser)
                    } else {
                        rename_all_rule // Fall back to common rule
                    }
                } else {
                    rename_all_rule
                }
            }
        };

        // Apply rename_all rule
        if let Some(rule) = rule {
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

    /// Apply enum tagging transformation to variant fields based on representation
    /// This transforms the DataType to include tag/content fields for Internal/Adjacent representations
    fn apply_enum_tagging(
        &self,
        repr: &EnumRepr,
        variant_name: &Cow<'static, str>,
        fields: Fields,
    ) -> Result<Fields, Error> {
        match repr {
            EnumRepr::External | EnumRepr::Untagged | EnumRepr::String { .. } => {
                // No transformation needed for external/untagged enums
                Ok(fields)
            }
            EnumRepr::Internal { tag } => {
                // Add tag field to the variant
                self.add_tag_field(tag, variant_name, fields)
            }
            EnumRepr::Adjacent { tag, content } => {
                // Wrap fields in content and add tag field
                self.add_adjacent_tag_fields(tag, content, variant_name, fields)
            }
        }
    }

    /// Create a literal string type using an enum with a single unit variant
    fn create_literal_string_type(value: &str) -> DataType {
        use specta::datatype::EnumVariant;

        let mut literal_enum = Enum::new();
        let unit_variant = EnumVariant::unit();
        *literal_enum.variants_mut() = vec![(Cow::Owned(value.to_string()), unit_variant)];
        DataType::Enum(literal_enum)
    }

    /// Add a tag field to a struct (for `#[serde(tag = "...")]` on structs)
    fn add_struct_tag_field(
        &self,
        tag_name: &str,
        _struct_type: &Struct,
        fields: Fields,
    ) -> Result<Fields, Error> {
        use specta::datatype::Field;

        // Get the struct name from the transformer
        let struct_name = self
            .type_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Unknown");
        let tag_type = Self::create_literal_string_type(struct_name);

        match fields {
            Fields::Named(named) => {
                let mut new_fields = vec![];

                // Add tag field first
                let tag_field = Field::new(tag_type);
                new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

                // Add existing fields
                for (field_name, field) in named.fields() {
                    new_fields.push((field_name.clone(), field.clone()));
                }

                Ok(internal::construct::fields_named(new_fields, vec![]))
            }
            _ => {
                // Struct tagging only works with named fields
                Ok(fields)
            }
        }
    }

    /// Add a tag field to variant fields for internally tagged enums
    fn add_tag_field(
        &self,
        tag_name: &str,
        variant_name: &Cow<'static, str>,
        fields: Fields,
    ) -> Result<Fields, Error> {
        use specta::datatype::Field;

        let tag_type = Self::create_literal_string_type(variant_name);

        match fields {
            Fields::Unit => {
                // Unit variant becomes { tag: "VariantName" }
                let tag_field = Field::new(tag_type);
                Ok(internal::construct::fields_named(
                    vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                    vec![],
                ))
            }
            Fields::Named(named) => {
                // Named variant: add tag field to existing fields
                let mut new_fields = vec![];

                // Add tag field first
                let tag_field = Field::new(tag_type);
                new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

                // Add existing fields
                for (field_name, field) in named.fields() {
                    new_fields.push((field_name.clone(), field.clone()));
                }

                Ok(internal::construct::fields_named(new_fields, vec![]))
            }
            Fields::Unnamed(_) => {
                // Tuple variants in internally tagged enums are not well-supported by serde
                // For now, just add the tag as a named field
                let tag_field = Field::new(tag_type);
                Ok(internal::construct::fields_named(
                    vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                    vec![],
                ))
            }
        }
    }

    /// Add tag and content fields for adjacently tagged enums
    fn add_adjacent_tag_fields(
        &self,
        tag_name: &str,
        content_name: &str,
        variant_name: &Cow<'static, str>,
        fields: Fields,
    ) -> Result<Fields, Error> {
        use specta::datatype::Field;

        let tag_type = Self::create_literal_string_type(variant_name);

        match fields {
            Fields::Unit => {
                // Unit variant becomes { tag: "VariantName" } (no content field)
                let tag_field = Field::new(tag_type);
                Ok(internal::construct::fields_named(
                    vec![(Cow::Owned(tag_name.to_string()), tag_field)],
                    vec![],
                ))
            }
            Fields::Named(named) => {
                // Named variant: { tag: "VariantName", content: { ...fields } }
                let mut new_fields = vec![];

                // Add tag field
                let tag_field = Field::new(tag_type);
                new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

                // Wrap existing fields in a struct for content
                let mut content_struct = Struct::unit();
                content_struct.set_fields(Fields::Named(named.clone()));

                let content_field = Field::new(DataType::Struct(content_struct));
                new_fields.push((Cow::Owned(content_name.to_string()), content_field));

                Ok(internal::construct::fields_named(new_fields, vec![]))
            }
            Fields::Unnamed(unnamed) => {
                // Tuple variant: { tag: "VariantName", content: tuple }
                let mut new_fields = vec![];

                // Add tag field
                let tag_field = Field::new(tag_type);
                new_fields.push((Cow::Owned(tag_name.to_string()), tag_field));

                // Wrap tuple fields as content
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

                let content_field = Field::new(content_type);
                new_fields.push((Cow::Owned(content_name.to_string()), content_field));

                Ok(internal::construct::fields_named(new_fields, vec![]))
            }
        }
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
        RuntimeMeta::Path(path) => {
            // Handle path-only attributes (e.g., #[serde(untagged)], #[serde(skip)])
            parse_serde_path_attribute(attrs, path);
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
                "rename_all_fields" => {
                    if let RuntimeLiteral::Str(rule_str) = value {
                        attrs.rename_all_fields = Some(
                            RenameRule::from_str(rule_str)
                                .map_err(|_| Error::InvalidUsageOfSkip)?,
                        );
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
                "default" => match value {
                    RuntimeLiteral::Bool(true) => attrs.default = true,
                    RuntimeLiteral::Str(func_path) => {
                        attrs.default_with = Some(func_path.clone());
                    }
                    _ => {}
                },
                "remote" => {
                    if let RuntimeLiteral::Str(remote_type) = value {
                        attrs.remote = Some(remote_type.clone());
                    }
                }
                "from" => {
                    if let RuntimeLiteral::Str(from_type) = value {
                        attrs.from = Some(from_type.clone());
                    }
                }
                "try_from" => {
                    if let RuntimeLiteral::Str(try_from_type) = value {
                        attrs.try_from = Some(try_from_type.clone());
                    }
                }
                "into" => {
                    if let RuntimeLiteral::Str(into_type) = value {
                        attrs.into = Some(into_type.clone());
                    }
                }
                "alias" => {
                    if let RuntimeLiteral::Str(alias_name) = value {
                        attrs.alias.push(alias_name.clone());
                    }
                }
                "serialize_with" => {
                    if let RuntimeLiteral::Str(serialize_fn) = value {
                        attrs.serialize_with = Some(serialize_fn.clone());
                    }
                }
                "deserialize_with" => {
                    if let RuntimeLiteral::Str(deserialize_fn) = value {
                        attrs.deserialize_with = Some(deserialize_fn.clone());
                    }
                }
                "with" => {
                    if let RuntimeLiteral::Str(with_module) = value {
                        attrs.with = Some(with_module.clone());
                    }
                }
                _ => {}
            }
        }
        RuntimeMeta::List(list) => {
            // Check if this is a complex attribute with serialize/deserialize modifiers
            let mut has_serialize_deserialize = false;
            for nested in list {
                if let RuntimeNestedMeta::Meta(RuntimeMeta::NameValue { key, .. }) = nested
                    && (key == "serialize" || key == "deserialize")
                {
                    has_serialize_deserialize = true;
                    break;
                }
            }

            if has_serialize_deserialize {
                // This is a complex attribute like rename(serialize="...", deserialize="...")
                for nested in list {
                    if let RuntimeNestedMeta::Meta(nested_meta) = nested {
                        parse_complex_serde_attribute(nested_meta, attrs, "rename")?;
                    }
                }
            } else {
                // Regular list processing
                for nested in list {
                    match nested {
                        RuntimeNestedMeta::Meta(nested_meta) => {
                            parse_serde_attribute_content(nested_meta, attrs)?;
                        }
                        RuntimeNestedMeta::Literal(RuntimeLiteral::Str(s)) => {
                            // Handle string literals that might be path attributes
                            parse_serde_path_attribute(attrs, s);
                        }
                        RuntimeNestedMeta::Literal(_) => {
                            // Handle other literal values in lists if needed
                        }
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

fn parse_complex_serde_attribute(
    meta: &RuntimeMeta,
    attrs: &mut SerdeAttributes,
    parent_key: &str,
) -> Result<(), Error> {
    match meta {
        RuntimeMeta::NameValue { key, value } => match key.as_str() {
            "serialize" => {
                if let RuntimeLiteral::Str(name) = value {
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
            }
            "deserialize" => {
                if let RuntimeLiteral::Str(name) = value {
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
            }
            _ => {}
        },
        RuntimeMeta::List(list) => {
            // Handle nested complex attributes
            for nested in list {
                if let RuntimeNestedMeta::Meta(nested_meta) = nested {
                    parse_complex_serde_attribute(nested_meta, attrs, parent_key)?;
                }
            }
        }
        _ => {}
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
        "deny_unknown_fields" => attrs.deny_unknown_fields = true,
        "other" => attrs.other = true,
        _ => {}
    }
}

/// Parse serde field attributes from a vector of RuntimeAttribute
#[allow(dead_code)]
fn parse_serde_field_attributes(attrs: &[RuntimeAttribute]) -> Result<SerdeFieldAttributes, Error> {
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

/// Parse the content of a serde field attribute
#[allow(dead_code)]
fn parse_serde_field_attribute_content(
    meta: &RuntimeMeta,
    attrs: &mut SerdeFieldAttributes,
) -> Result<(), Error> {
    // First parse as base attributes
    parse_serde_attribute_content(meta, &mut attrs.base)?;

    // Then parse field-specific attributes
    match meta {
        RuntimeMeta::Path(_path) => {
            // Path-only attributes are already handled by parse_serde_attribute_content above
        }
        RuntimeMeta::NameValue { key, value } => match key.as_str() {
            "alias" => {
                if let RuntimeLiteral::Str(alias_name) = value {
                    attrs.alias.push(alias_name.clone());
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
            "with" => {
                if let RuntimeLiteral::Str(module_path) = value {
                    attrs.with = Some(module_path.clone());
                }
            }
            "skip_serializing_if" => {
                if let RuntimeLiteral::Str(func_name) = value {
                    attrs.skip_serializing_if = Some(func_name.clone());
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

fn parse_field_serde_attributes(
    attributes: &[specta::datatype::RuntimeAttribute],
) -> SerdeFieldAttributes {
    let mut field_attrs = SerdeFieldAttributes::default();

    for attr in attributes {
        if attr.path == "serde" {
            match &attr.kind {
                specta::datatype::RuntimeMeta::Path(path) => {
                    // Handle simple #[serde(path)] attributes (e.g., #[serde(skip)])
                    parse_serde_path_attribute(&mut field_attrs.base, path);
                }
                specta::datatype::RuntimeMeta::List(nested) => {
                    // Parse nested serde attributes
                    for nested_meta in nested {
                        if let specta::datatype::RuntimeNestedMeta::Literal(literal) = nested_meta
                            && let specta::datatype::RuntimeLiteral::Str(content) = literal
                        {
                            // Parse the serialized attribute content
                            parse_serde_attribute_string(content, &mut field_attrs);
                        }
                    }
                }
                specta::datatype::RuntimeMeta::NameValue { key, value } => {
                    // Handle key-value serde attributes
                    apply_serde_field_attribute(key, value, &mut field_attrs);
                }
            }
        }
    }

    field_attrs
}

fn parse_serde_attribute_string(content: &str, field_attrs: &mut SerdeFieldAttributes) {
    // Simple parsing for common serde attributes
    // This is a basic implementation that can be expanded
    if content.contains("skip") {
        field_attrs.base.skip = true;
    }
    if content.contains("skip_serializing") {
        field_attrs.base.skip_serializing = true;
    }
    if content.contains("skip_deserializing") {
        field_attrs.base.skip_deserializing = true;
    }
    if content.contains("flatten") {
        field_attrs.base.flatten = true;
    }
    if content.contains("default") && !content.contains("default =") {
        field_attrs.base.default = true;
    }

    // Parse rename attribute
    if let Some(start) = content.find("rename = \"")
        && let Some(end) = content[start + 10..].find("\"")
    {
        let rename_value = &content[start + 10..start + 10 + end];
        field_attrs.base.rename = Some(rename_value.to_string());
    }
}

fn apply_serde_field_attribute(
    key: &str,
    value: &specta::datatype::RuntimeLiteral,
    field_attrs: &mut SerdeFieldAttributes,
) {
    match key {
        "rename" => {
            if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.base.rename = Some(s.clone());
            }
        }
        "skip" => {
            if let specta::datatype::RuntimeLiteral::Bool(true) = value {
                field_attrs.base.skip = true;
            }
        }
        "skip_serializing" => {
            if let specta::datatype::RuntimeLiteral::Bool(true) = value {
                field_attrs.base.skip_serializing = true;
            }
        }
        "skip_deserializing" => {
            if let specta::datatype::RuntimeLiteral::Bool(true) = value {
                field_attrs.base.skip_deserializing = true;
            }
        }
        "flatten" => {
            if let specta::datatype::RuntimeLiteral::Bool(true) = value {
                field_attrs.base.flatten = true;
            }
        }
        "default" => {
            if let specta::datatype::RuntimeLiteral::Bool(true) = value {
                field_attrs.base.default = true;
            } else if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.base.default_with = Some(s.clone());
            }
        }
        "serialize_with" => {
            if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.serialize_with = Some(s.clone());
            }
        }
        "deserialize_with" => {
            if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.deserialize_with = Some(s.clone());
            }
        }
        "with" => {
            if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.with = Some(s.clone());
            }
        }
        "skip_serializing_if" => {
            if let specta::datatype::RuntimeLiteral::Str(s) = value {
                field_attrs.skip_serializing_if = Some(s.clone());
            }
        }
        _ => {
            // Ignore unknown attributes
        }
    }
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
        let transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
        attrs.skip = true;
        assert!(transformer.should_skip_type(&attrs));

        attrs.skip = false;
        attrs.skip_serializing = true;
        assert!(transformer.should_skip_type(&attrs));

        let deserialize_transformer = SerdeTransformer::new(SerdeMode::Deserialize, None);
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
        let transformer = SerdeTransformer::new(SerdeMode::Serialize, None);

        // Test field renaming
        let result = transformer
            .apply_rename_rule(
                "test_field",
                Some(RenameRule::CamelCase),
                &None,
                &SerdeAttributes::default(),
                false,
            )
            .unwrap();
        assert_eq!(result, "testField");

        // Test variant renaming
        let result = transformer
            .apply_rename_rule(
                "TestVariant",
                Some(RenameRule::SnakeCase),
                &None,
                &SerdeAttributes::default(),
                true,
            )
            .unwrap();
        assert_eq!(result, "test_variant");

        // Test direct rename takes precedence
        let result = transformer
            .apply_rename_rule(
                "test_field",
                Some(RenameRule::CamelCase),
                &Some("customName".to_string()),
                &SerdeAttributes::default(),
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
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
        let primitive = DataType::Primitive(Primitive::String);

        let result = transformer.transform_datatype(&primitive).unwrap();
        assert_eq!(result, primitive);
    }

    #[test]
    fn test_nullable_type_transformation() {
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
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
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
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
        let ser_transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
        let de_transformer = SerdeTransformer::new(SerdeMode::Deserialize, None);

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
        let mut transformer = SerdeTransformer::new(SerdeMode::Serialize, None);

        // Create a transparent struct with single field using List format
        let transparent_attr = RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::List(vec![RuntimeNestedMeta::Literal(RuntimeLiteral::Str(
                "transparent".to_string(),
            ))]),
        };

        let field = specta::datatype::Field::new(DataType::Primitive(Primitive::String));
        let mut struct_dt = match Struct::unnamed().field(field).build() {
            DataType::Struct(s) => s,
            _ => unreachable!(),
        };
        struct_dt.set_attributes(vec![transparent_attr]);

        let datatype = DataType::Struct(struct_dt);
        let result = transformer.transform_datatype(&datatype).unwrap();

        // Should resolve to the inner type
        assert_eq!(result, DataType::Primitive(Primitive::String));
    }

    #[test]
    fn test_field_attributes_parsing() {
        let serialize_with_attr = RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::NameValue {
                key: "serialize_with".to_string(),
                value: RuntimeLiteral::Str("custom_serialize".to_string()),
            },
        };

        let attrs = parse_serde_field_attributes(&[serialize_with_attr]).unwrap();
        assert_eq!(attrs.serialize_with, Some("custom_serialize".to_string()));
    }

    #[test]
    fn test_other_attribute() {
        let mut attrs = SerdeAttributes::default();
        parse_serde_path_attribute(&mut attrs, "other");
        assert!(attrs.other);
    }

    #[test]
    fn test_alias_attribute() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "alias".to_string(),
            value: RuntimeLiteral::Str("alternative_name".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.alias, vec!["alternative_name".to_string()]);
    }

    #[test]
    fn test_serialize_with_attribute() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "serialize_with".to_string(),
            value: RuntimeLiteral::Str("custom_serialize".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.serialize_with, Some("custom_serialize".to_string()));
    }

    #[test]
    fn test_with_attribute() {
        let mut attrs = SerdeAttributes::default();
        let meta = RuntimeMeta::NameValue {
            key: "with".to_string(),
            value: RuntimeLiteral::Str("custom_module".to_string()),
        };

        parse_serde_attribute_content(&meta, &mut attrs).expect("Failed to parse serde attribute");
        assert_eq!(attrs.with, Some("custom_module".to_string()));
    }

    #[test]
    fn test_complex_rename_attribute() {
        let mut attrs = SerdeAttributes::default();

        // Simulate parsing rename(serialize = "ser_name", deserialize = "de_name")
        let serialize_meta = RuntimeMeta::NameValue {
            key: "serialize".to_string(),
            value: RuntimeLiteral::Str("ser_name".to_string()),
        };
        let deserialize_meta = RuntimeMeta::NameValue {
            key: "deserialize".to_string(),
            value: RuntimeLiteral::Str("de_name".to_string()),
        };

        parse_complex_serde_attribute(&serialize_meta, &mut attrs, "rename")
            .expect("Failed to parse serialize");
        parse_complex_serde_attribute(&deserialize_meta, &mut attrs, "rename")
            .expect("Failed to parse deserialize");

        assert_eq!(attrs.rename_serialize, Some("ser_name".to_string()));
        assert_eq!(attrs.rename_deserialize, Some("de_name".to_string()));
    }

    #[test]
    fn test_complex_rename_all_attribute() {
        let mut attrs = SerdeAttributes::default();

        // Simulate parsing rename_all(serialize = "camelCase", deserialize = "snake_case")
        let serialize_meta = RuntimeMeta::NameValue {
            key: "serialize".to_string(),
            value: RuntimeLiteral::Str("camelCase".to_string()),
        };
        let deserialize_meta = RuntimeMeta::NameValue {
            key: "deserialize".to_string(),
            value: RuntimeLiteral::Str("snake_case".to_string()),
        };

        parse_complex_serde_attribute(&serialize_meta, &mut attrs, "rename_all")
            .expect("Failed to parse serialize");
        parse_complex_serde_attribute(&deserialize_meta, &mut attrs, "rename_all")
            .expect("Failed to parse deserialize");

        assert_eq!(attrs.rename_all_serialize, Some(RenameRule::CamelCase));
        assert_eq!(attrs.rename_all_deserialize, Some(RenameRule::SnakeCase));
    }

    #[test]
    fn test_mode_specific_rename_behavior() {
        let mut attrs = SerdeAttributes::default();
        attrs.rename_serialize = Some("ser_name".to_string());
        attrs.rename_deserialize = Some("de_name".to_string());

        let ser_transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
        let de_transformer = SerdeTransformer::new(SerdeMode::Deserialize, None);

        let ser_result = ser_transformer
            .apply_rename_rule("original", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(ser_result, "ser_name");

        let de_result = de_transformer
            .apply_rename_rule("original", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(de_result, "de_name");
    }

    #[test]
    fn test_mode_specific_rename_all_behavior() {
        let mut attrs = SerdeAttributes::default();
        attrs.rename_all_serialize = Some(RenameRule::CamelCase);
        attrs.rename_all_deserialize = Some(RenameRule::SnakeCase);

        let ser_transformer = SerdeTransformer::new(SerdeMode::Serialize, None);
        let de_transformer = SerdeTransformer::new(SerdeMode::Deserialize, None);

        // Test field renaming (fields start as snake_case in Rust)
        let ser_result = ser_transformer
            .apply_rename_rule("test_field", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(ser_result, "testField");

        let de_result = de_transformer
            .apply_rename_rule("test_field", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(de_result, "test_field"); // snake_case rule returns input unchanged for fields

        // Test variant renaming (variants start as PascalCase in Rust)
        let ser_result = ser_transformer
            .apply_rename_rule("TestVariant", None, &None, &attrs, true)
            .unwrap();
        assert_eq!(ser_result, "testVariant"); // camelCase

        let de_result = de_transformer
            .apply_rename_rule("TestVariant", None, &None, &attrs, true)
            .unwrap();
        assert_eq!(de_result, "test_variant"); // snake_case
    }

    #[test]
    fn test_both_mode_skip_behavior() {
        let mut attrs = SerdeAttributes::default();
        let both_transformer = SerdeTransformer::new(SerdeMode::Both, None);

        // Test that Both mode only skips if both modes skip
        attrs.skip_serializing = true;
        attrs.skip_deserializing = false;
        assert!(!both_transformer.should_skip_type(&attrs));

        attrs.skip_serializing = false;
        attrs.skip_deserializing = true;
        assert!(!both_transformer.should_skip_type(&attrs));

        // Should skip only when both are true
        attrs.skip_serializing = true;
        attrs.skip_deserializing = true;
        assert!(both_transformer.should_skip_type(&attrs));

        // Universal skip should still work
        attrs.skip_serializing = false;
        attrs.skip_deserializing = false;
        attrs.skip = true;
        assert!(both_transformer.should_skip_type(&attrs));
    }

    #[test]
    fn test_both_mode_rename_behavior() {
        let both_transformer = SerdeTransformer::new(SerdeMode::Both, None);
        let mut attrs = SerdeAttributes::default();

        // Test that Both mode uses common rename
        attrs.rename = Some("commonName".to_string());
        let result = both_transformer
            .apply_rename_rule("original", None, &attrs.rename, &attrs, false)
            .unwrap();
        assert_eq!(result, "commonName");

        // Test that Both mode uses matching mode-specific renames
        attrs.rename = None;
        attrs.rename_serialize = Some("sameName".to_string());
        attrs.rename_deserialize = Some("sameName".to_string());
        let result = both_transformer
            .apply_rename_rule("original", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(result, "sameName");

        // Test that Both mode ignores non-matching mode-specific renames
        attrs.rename_serialize = Some("serName".to_string());
        attrs.rename_deserialize = Some("deName".to_string());
        let result = both_transformer
            .apply_rename_rule("original", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(result, "original"); // Falls back to original name
    }

    #[test]
    fn test_both_mode_rename_all_behavior() {
        let both_transformer = SerdeTransformer::new(SerdeMode::Both, None);
        let mut attrs = SerdeAttributes::default();

        // Test that Both mode uses common rename_all
        let result = both_transformer
            .apply_rename_rule(
                "test_field",
                Some(RenameRule::CamelCase),
                &None,
                &attrs,
                false,
            )
            .unwrap();
        assert_eq!(result, "testField");

        // Test that Both mode uses matching mode-specific rename_all
        attrs.rename_all_serialize = Some(RenameRule::PascalCase);
        attrs.rename_all_deserialize = Some(RenameRule::PascalCase);
        let result = both_transformer
            .apply_rename_rule("test_field", None, &None, &attrs, false)
            .unwrap();
        assert_eq!(result, "TestField");

        // Test that Both mode falls back to common rule when modes differ
        attrs.rename_all_serialize = Some(RenameRule::CamelCase);
        attrs.rename_all_deserialize = Some(RenameRule::SnakeCase);
        let result = both_transformer
            .apply_rename_rule(
                "test_field",
                Some(RenameRule::PascalCase),
                &None,
                &attrs,
                false,
            )
            .unwrap();
        assert_eq!(result, "TestField"); // Uses common rule
    }

    #[test]
    fn test_variant_attribute_parsing() {
        // Test that variant attributes are parsed when transforming enums
        let variant_attr = RuntimeAttribute {
            path: "serde".to_string(),
            kind: RuntimeMeta::NameValue {
                key: "rename".to_string(),
                value: RuntimeLiteral::Str("custom_variant".to_string()),
            },
        };

        let attrs = parse_serde_attributes(&[variant_attr]).unwrap();
        assert_eq!(attrs.rename, Some("custom_variant".to_string()));
    }
}
