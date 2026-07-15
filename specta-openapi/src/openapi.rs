use std::{borrow::Cow, path::Path};

use openapiv3::{Components, Info, OpenAPI, Paths};
use specta::{Format, Types};

use crate::{Error, transform::components};

/// How shapes unsupported by OpenAPI 3.0's schema dialect are handled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SchemaMode {
    /// Return an error for unsupported structural OpenAPI 3.0 schema features.
    #[default]
    Strict,
    /// Emit the closest OpenAPI 3.0 schema and retain exact constraints in
    /// `x-specta-*` specification extensions.
    Compatible,
}

/// Serialization format used by [`OpenApi::export`] and [`OpenApi::export_to`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Pretty-printed JSON.
    #[default]
    Json,
    /// YAML.
    Yaml,
}

/// OpenAPI 3.0 schema exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OpenApi {
    title: Cow<'static, str>,
    version: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    output_format: OutputFormat,
    schema_mode: SchemaMode,
}

impl Default for OpenApi {
    fn default() -> Self {
        Self {
            title: Cow::Borrowed("Specta API"),
            version: Cow::Borrowed("0.0.0"),
            description: None,
            output_format: OutputFormat::Json,
            schema_mode: SchemaMode::Strict,
        }
    }
}

impl OpenApi {
    /// Construct an exporter with default document metadata and JSON output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the API title in the generated document's `info` object.
    pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    /// Configure the API version in the generated document's `info` object.
    pub fn version(mut self, version: impl Into<Cow<'static, str>>) -> Self {
        self.version = version.into();
        self
    }

    /// Configure the API description in the generated document's `info` object.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Configure JSON or YAML serialization.
    pub fn output_format(mut self, output_format: OutputFormat) -> Self {
        self.output_format = output_format;
        self
    }

    /// Configure whether OpenAPI 3.0-incompatible shapes are approximated or
    /// rejected. See [`SchemaMode`].
    pub fn schema_mode(mut self, schema_mode: SchemaMode) -> Self {
        self.schema_mode = schema_mode;
        self
    }

    /// Export the supplied types as a complete OpenAPI 3.0 document.
    pub fn export_document(&self, types: &Types, format: impl Format) -> Result<OpenAPI, Error> {
        Ok(OpenAPI {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: self.title.to_string(),
                description: self.description.as_ref().map(ToString::to_string),
                version: self.version.to_string(),
                ..Default::default()
            },
            paths: Paths::default(),
            components: Some(self.export_components(types, format)?),
            ..Default::default()
        })
    }

    /// Export only the reusable `components` object for merging into an
    /// application-owned OpenAPI document.
    pub fn export_components(
        &self,
        types: &Types,
        format: impl Format,
    ) -> Result<Components, Error> {
        components(types, format, self.schema_mode)
    }

    /// Add exported schemas to an existing document without replacing any
    /// existing schema component.
    pub fn add_to_document(
        &self,
        document: &mut OpenAPI,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        let exported = self.export_components(types, format)?;
        let target = &mut document
            .components
            .get_or_insert_with(Components::default)
            .schemas;

        if let Some(name) = exported
            .schemas
            .keys()
            .find(|name| target.contains_key(*name))
        {
            return Err(Error::DuplicateComponent(name.clone()));
        }
        target.extend(exported.schemas);
        Ok(())
    }

    /// Export the supplied types to a JSON or YAML string.
    pub fn export(&self, types: &Types, format: impl Format) -> Result<String, Error> {
        let document = self.export_document(types, format)?;
        match self.output_format {
            OutputFormat::Json => Ok(serde_json::to_string_pretty(&document)?),
            OutputFormat::Yaml => Ok(serde_yaml::to_string(&document)?),
        }
    }

    /// Export the supplied types to a JSON or YAML file.
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let output = self.export(types, format)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(path, output).map_err(|source| Error::WriteFile {
            path: path.to_path_buf(),
            source,
        })
    }
}
