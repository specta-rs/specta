use std::{borrow::Cow, path::Path};

use serde_json::{Map, Value};
use specta::{Format, Types};

use crate::{Error, SchemaVersion, render::Renderer};

/// JSON Schema exporter.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct JsonSchema {
    schema_version: SchemaVersion,
    title: Option<Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    comment: Option<Cow<'static, str>>,
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

        let renderer = Renderer::new(self.schema_version, types);
        let definitions = renderer.render_definitions(&roots)?;

        let mut root = Map::new();
        root.insert(
            "$schema".to_string(),
            Value::String(self.schema_version.uri().to_string()),
        );
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
            root.insert(
                "$ref".to_string(),
                Value::String(format!(
                    "#/{}/{}",
                    self.schema_version.definitions_key(),
                    definition.as_ref().replace('~', "~0").replace('/', "~1")
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, self.export(types, format)?)?;
        Ok(())
    }
}
