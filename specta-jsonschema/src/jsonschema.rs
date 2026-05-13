use std::{borrow::Cow, path::Path};

use serde_json::Value;
use specta::{Format, Types};

use crate::{Error, SchemaVersion, exporter::Exporter};

/// JSON Schema exporter.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct JsonSchema(Exporter);

impl JsonSchema {
    /// Construct a new exporter with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the JSON Schema draft version.
    pub fn schema_version(mut self, version: SchemaVersion) -> Self {
        self.0.schema_version = version;
        self
    }

    /// Configure the root schema title.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.0.title = Some(title.into());
        self
    }

    /// Configure the root schema description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    /// Configure a root `$comment` value.
    pub fn comment(mut self, comment: impl Into<Cow<'static, str>>) -> Self {
        self.0.comment = Some(comment.into());
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
        self.0.export_value(types, &format)
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
