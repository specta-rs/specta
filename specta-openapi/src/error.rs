use std::{io, path::PathBuf};

/// Error returned by the OpenAPI exporter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The intermediate JSON Schema exporter rejected the type graph.
    #[error(transparent)]
    JsonSchema(#[from] specta_jsonschema::Error),

    /// An exported schema could not be represented by the OpenAPI 3.0 model.
    #[error("invalid OpenAPI schema component {component:?}: {source}")]
    InvalidSchema {
        /// Component being converted.
        component: String,
        /// Deserialization error from the strongly typed OpenAPI model.
        source: serde_json::Error,
    },

    /// A Specta shape cannot be represented exactly by OpenAPI 3.0.
    #[error("OpenAPI 3.0 cannot represent {feature} exactly in component {component:?}")]
    UnsupportedSchemaFeature {
        /// Component containing the unsupported shape.
        component: String,
        /// Unsupported JSON Schema feature or shape.
        feature: &'static str,
    },

    /// JSON output serialization failed.
    #[error("failed to serialize OpenAPI JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML output serialization failed.
    #[error("failed to serialize OpenAPI YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// A component with the same name already exists in a target document.
    #[error("OpenAPI document already contains schema component {0:?}")]
    DuplicateComponent(String),

    /// A directory could not be created.
    #[error("failed to create output directory {path:?}: {source}")]
    CreateDir {
        /// Directory path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },

    /// The exported document could not be written.
    #[error("failed to write OpenAPI document {path:?}: {source}")]
    WriteFile {
        /// Output path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },
}
