use std::{borrow::Cow, collections::BTreeMap, path::Path};

use serde_json::{Map, Value, json};
use specta::{Format, Types};

use crate::{Error, operation::Operation, resolve::resolve};

/// OpenAPI Specification version of the emitted document.
///
/// OpenAPI 3.1's schema dialect is full JSON Schema, so every Specta shape is
/// expressible in it and [`SchemaMode`] has no effect. OpenAPI 3.0 uses an
/// older, restricted dialect whose unrepresentable shapes [`SchemaMode`]
/// governs. Patch digits carry no meaning of their own — tooling is told to
/// ignore them — so each variant emits the version string its consumers have
/// seen most.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OasVersion {
    /// OpenAPI 3.0, emitted as `3.0.3`, for consumers that predate 3.1.
    V3_0,
    /// OpenAPI 3.1, emitted as `3.1.0`.
    #[default]
    V3_1,
}

impl OasVersion {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            OasVersion::V3_0 => "3.0.3",
            OasVersion::V3_1 => "3.1.0",
        }
    }
}

/// How shapes unsupported by OpenAPI 3.0's schema dialect are handled.
///
/// Applies to [`OasVersion::V3_0`] alone: OpenAPI 3.1's dialect is full JSON
/// Schema, where every Specta shape is expressible, so under it both modes
/// emit the same document.
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

#[derive(Debug, Clone)]
struct Server {
    url: String,
    description: Option<String>,
}

#[derive(Debug, Clone)]
struct Tag {
    name: String,
    description: String,
}

#[derive(Debug, Clone)]
struct Contact {
    name: String,
    url: String,
}

#[derive(Debug, Clone)]
struct License {
    name: String,
    identifier: Option<String>,
}

/// OpenAPI schema exporter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OpenApi {
    title: Cow<'static, str>,
    version: Cow<'static, str>,
    description: Option<Cow<'static, str>>,
    output_format: OutputFormat,
    oas_version: OasVersion,
    schema_mode: SchemaMode,
    operations: Vec<Operation>,
    servers: Vec<Server>,
    tags: Vec<Tag>,
    contact: Option<Contact>,
    license: Option<License>,
    security_schemes: BTreeMap<String, Value>,
}

impl Default for OpenApi {
    fn default() -> Self {
        Self {
            title: Cow::Borrowed("Specta API"),
            version: Cow::Borrowed("0.0.0"),
            description: None,
            output_format: OutputFormat::Json,
            oas_version: OasVersion::default(),
            schema_mode: SchemaMode::Strict,
            operations: Vec::new(),
            servers: Vec::new(),
            tags: Vec::new(),
            contact: None,
            license: None,
            security_schemes: BTreeMap::new(),
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

    /// Add a server the API is reachable on, in the document's `servers` list.
    pub fn server(mut self, url: impl Into<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description: None,
        });
        self
    }

    /// Add a server with a human-readable description.
    pub fn server_described(
        mut self,
        url: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description: Some(description.into()),
        });
        self
    }

    /// Configure the API contact in the generated document's `info` object.
    pub fn contact(mut self, name: impl Into<String>, url: impl Into<String>) -> Self {
        self.contact = Some(Contact {
            name: name.into(),
            url: url.into(),
        });
        self
    }

    /// Configure the API license in the generated document's `info` object.
    pub fn license(mut self, name: impl Into<String>) -> Self {
        self.license = Some(License {
            name: name.into(),
            identifier: None,
        });
        self
    }

    /// Configure the API license with its SPDX identifier. The `identifier`
    /// field is OpenAPI 3.1's; under [`OasVersion::V3_0`] the name alone is
    /// emitted.
    pub fn license_spdx(mut self, name: impl Into<String>, identifier: impl Into<String>) -> Self {
        self.license = Some(License {
            name: name.into(),
            identifier: Some(identifier.into()),
        });
        self
    }

    /// Add a tag with a description, which generators and documentation use
    /// to group operations declared with [`Operation::tag`].
    pub fn tag(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.tags.push(Tag {
            name: name.into(),
            description: description.into(),
        });
        self
    }

    /// Register a security scheme under `name`, which operations reference
    /// with [`Operation::security`]. The scheme is given as a
    /// [Security Scheme Object](https://spec.openapis.org/oas/v3.0.3#security-scheme-object)
    /// in JSON; [`bearer_security_scheme`](Self::bearer_security_scheme)
    /// covers the common token case.
    pub fn security_scheme(mut self, name: impl Into<String>, scheme: Value) -> Self {
        self.security_schemes.insert(name.into(), scheme);
        self
    }

    /// Register an HTTP bearer-token security scheme under `name`.
    pub fn bearer_security_scheme(
        self,
        name: impl Into<String>,
        bearer_format: impl Into<String>,
    ) -> Self {
        self.security_scheme(
            name,
            json!({
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": bearer_format.into(),
            }),
        )
    }

    /// Configure JSON or YAML serialization.
    pub fn output_format(mut self, output_format: OutputFormat) -> Self {
        self.output_format = output_format;
        self
    }

    /// Configure the OpenAPI Specification version of the emitted document.
    /// Documents target OpenAPI 3.1 unless told otherwise; see [`OasVersion`].
    pub fn oas_version(mut self, oas_version: OasVersion) -> Self {
        self.oas_version = oas_version;
        self
    }

    /// Configure whether OpenAPI 3.0-incompatible shapes are approximated or
    /// rejected. See [`SchemaMode`].
    pub fn schema_mode(mut self, schema_mode: SchemaMode) -> Self {
        self.schema_mode = schema_mode;
        self
    }

    /// Describe an endpoint, which is exported into the document's `paths` object.
    ///
    /// Bodies are declared as types and resolved to their exported components, so the document and
    /// the schemas cannot disagree. A handler that returns more than one status has no single Rust
    /// return type to infer from, so each response is stated:
    ///
    /// ```rust
    /// # use specta::{Type, Types};
    /// # use specta_openapi::{OpenApi, Operation};
    /// # #[derive(Type)]
    /// # struct Recipe { name: String }
    /// # #[derive(Type)]
    /// # struct ApiError { message: String }
    /// let types = Types::default().register::<Recipe>().register::<ApiError>();
    /// let document = OpenApi::default()
    ///     .operation(
    ///         Operation::get("/recipes/{slug}")
    ///             .path_param::<String>("slug")
    ///             .response::<Recipe>(200, "The recipe")
    ///             .response::<ApiError>(404, "No such recipe"),
    ///     )
    ///     .export(&types, specta_serde::Format)
    ///     .unwrap();
    /// assert!(document.contains("/recipes/{slug}"));
    /// ```
    pub fn operation(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Describe several endpoints at once.
    pub fn operations(mut self, operations: impl IntoIterator<Item = Operation>) -> Self {
        self.operations.extend(operations);
        self
    }

    /// Export the supplied types as a complete OpenAPI document, as JSON.
    pub fn export_document(&self, types: &Types, format: impl Format) -> Result<Value, Error> {
        let (schemas, resolved) = resolve(
            types,
            &self.operations,
            format,
            self.schema_mode,
            self.oas_version,
        )?;
        let paths = if self.operations.is_empty() {
            Value::Object(Map::new())
        } else {
            crate::paths::paths(&self.operations, &resolved)?
        };

        let mut components = Map::new();
        if !schemas.is_empty() {
            components.insert(
                "schemas".to_string(),
                Value::Object(schemas.into_iter().collect()),
            );
        }
        if !self.security_schemes.is_empty() {
            components.insert(
                "securitySchemes".to_string(),
                Value::Object(self.security_schemes.clone().into_iter().collect()),
            );
        }

        let mut info = Map::new();
        info.insert("title".to_string(), json!(self.title));
        info.insert("version".to_string(), json!(self.version));
        if let Some(description) = &self.description {
            info.insert("description".to_string(), json!(description));
        }
        if let Some(contact) = &self.contact {
            info.insert(
                "contact".to_string(),
                json!({ "name": contact.name, "url": contact.url }),
            );
        }
        if let Some(license) = &self.license {
            info.insert(
                "license".to_string(),
                match (&license.identifier, self.oas_version) {
                    (Some(identifier), OasVersion::V3_1) => {
                        json!({ "name": license.name, "identifier": identifier })
                    }
                    // `identifier` is an OpenAPI 3.1 field; 3.0's license
                    // object carries the name alone.
                    _ => json!({ "name": license.name }),
                },
            );
        }

        let mut document = Map::new();
        document.insert("openapi".to_string(), json!(self.oas_version.as_str()));
        document.insert("info".to_string(), Value::Object(info));
        if !self.servers.is_empty() {
            document.insert(
                "servers".to_string(),
                Value::Array(
                    self.servers
                        .iter()
                        .map(|server| match &server.description {
                            Some(description) => {
                                json!({ "url": server.url, "description": description })
                            }
                            None => json!({ "url": server.url }),
                        })
                        .collect(),
                ),
            );
        }
        if !self.tags.is_empty() {
            document.insert(
                "tags".to_string(),
                Value::Array(
                    self.tags
                        .iter()
                        .map(|tag| json!({ "name": tag.name, "description": tag.description }))
                        .collect(),
                ),
            );
        }
        document.insert("paths".to_string(), paths);
        document.insert("components".to_string(), Value::Object(components));
        Ok(Value::Object(document))
    }

    /// Export only the reusable schema components for merging into an
    /// application-owned OpenAPI document.
    pub fn export_components(
        &self,
        types: &Types,
        format: impl Format,
    ) -> Result<BTreeMap<String, Value>, Error> {
        crate::transform::components(types, format, self.schema_mode, self.oas_version)
    }

    /// Add exported schemas to an existing document without replacing any
    /// existing schema component.
    ///
    /// The target is any OpenAPI document as JSON, whichever tool produced it.
    pub fn add_to_document(
        &self,
        document: &mut Value,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        let exported = self.export_components(types, format)?;

        let root = document
            .as_object_mut()
            .ok_or(Error::InvalidTargetDocument)?;
        let components = root
            .entry("components")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or(Error::InvalidTargetDocument)?;
        let target = components
            .entry("schemas")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or(Error::InvalidTargetDocument)?;

        if let Some(name) = exported.keys().find(|name| target.contains_key(*name)) {
            return Err(Error::DuplicateComponent(name.clone()));
        }
        target.extend(exported);
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
