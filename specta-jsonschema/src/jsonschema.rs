use std::{borrow::Cow, path::Path};

use serde_json::{Map, Value};
use specta::{Format, Types};

use crate::{Error, SchemaVersion, render::Renderer};

/// JSON Schema exporter.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct JsonSchema {
    schema_version: SchemaVersion,
    id: Option<Cow<'static, str>>,
    title: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    comment: Option<Cow<'static, str>>,
    allow_additional_properties: bool,
}

impl JsonSchema {
    /// Construct a new exporter with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the JSON Schema draft version.
    pub fn schema_version(mut self, version: SchemaVersion) -> Self {
        self.schema_version = version;
        self
    }

    /// Configure the root schema `$id` URI.
    pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Configure the root schema title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Configure the root schema description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Configure a root `$comment` value.
    pub fn comment(mut self, comment: impl Into<Cow<'static, str>>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Allow properties not declared by named structs.
    ///
    /// By default, exported object schemas describe exact serialized shapes and
    /// set `additionalProperties` to `false`. Enable this when the schema is
    /// primarily used for deserialization compatible with Serde's default of
    /// ignoring unknown fields.
    pub fn allow_additional_properties(mut self, allow: bool) -> Self {
        self.allow_additional_properties = allow;
        self
    }

    /// Export the schema document as a pretty-printed JSON string.
    pub fn export(&self, types: &Types, format: impl Format) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(
            &self.export_value(types, format)?,
        )?)
    }

    /// Export the schema document as a [`serde_json::Value`].
    pub fn export_value(&self, types: &Types, format: impl Format) -> Result<Value, Error> {
        let roots = types
            .roots()
            .map(|root| {
                format
                    .map_type(types, root)
                    .map(Cow::into_owned)
                    .map_err(|err| Error::format("root type formatter failed", err))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let types = format
            .map_types(types)
            .map_err(|err| Error::format("type graph formatter failed", err))?;
        let types = types.as_ref();

        let renderer = Renderer::new(self.schema_version, types, self.allow_additional_properties);
        let definitions = renderer.render_definitions(&roots)?;

        let mut root = Map::new();
        root.insert(
            "$schema".to_string(),
            Value::String(self.schema_version.uri().to_string()),
        );
        if let Some(id) = &self.id {
            root.insert("$id".to_string(), Value::String(id.to_string()));
        }
        if let Some(title) = &self.title {
            root.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(description) = &self.description {
            root.insert(
                "description".to_string(),
                Value::String(description.to_string()),
            );
        }
        if let Some(comment) = &self.comment {
            root.insert("$comment".to_string(), Value::String(comment.to_string()));
        }
        root.insert(
            self.schema_version.definitions_key().to_string(),
            Value::Object(definitions),
        );

        Ok(Value::Object(root))
    }

    /// Export the schema document as a [`serde_json::Value`] with a root `$ref` into the definitions.
    pub fn export_ref_value(
        &self,
        types: &Types,
        format: impl Format,
        definition: impl AsRef<str>,
    ) -> Result<Value, Error> {
        let mut schema = self.export_value(types, format)?;
        if let Value::Object(root) = &mut schema {
            let definition = definition.as_ref();
            let definitions = root
                .get(self.schema_version.definitions_key())
                .and_then(Value::as_object);
            if !definitions.is_some_and(|definitions| definitions.contains_key(definition)) {
                return Err(Error::MissingDefinition {
                    definition: definition.to_string(),
                });
            }
            root.insert(
                "$ref".to_string(),
                Value::String(format!(
                    "#/{}/{}",
                    self.schema_version.definitions_key(),
                    crate::render::encode_ref_token(definition)
                )),
            );
        }

        Ok(schema)
    }

    /// Export the schema document as a pretty-printed JSON string with a root `$ref` into the definitions.
    pub fn export_ref(
        &self,
        types: &Types,
        format: impl Format,
        definition: impl AsRef<str>,
    ) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(
            &self.export_ref_value(types, format, definition)?,
        )?)
    }

    /// Export the schema document to a single JSON file.
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|source| Error::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        std::fs::write(path, self.export(types, format)?).map_err(|source| Error::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }
}
