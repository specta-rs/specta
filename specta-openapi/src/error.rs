use std::{io, path::PathBuf};

/// Error returned by the OpenAPI exporter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The intermediate JSON Schema exporter rejected the type graph.
    #[error(transparent)]
    JsonSchema(#[from] specta_jsonschema::Error),

    /// A Specta shape cannot be represented exactly by OpenAPI 3.0.
    #[error(
        "OpenAPI 3.0 cannot represent {feature} exactly in component {component:?}. \
         Export with SchemaMode::Compatible to emit the closest schema and keep the exact \
         constraints in x-specta-* extensions, or target OasVersion::V3_1, whose schema \
         dialect expresses this natively."
    )]
    UnsupportedSchemaFeature {
        /// Component containing the unsupported shape.
        component: String,
        /// Unsupported JSON Schema feature or shape.
        feature: &'static str,
    },

    /// Two JSON Schema definition names map to the same OpenAPI component name.
    #[error("OpenAPI definition name collision for {name:?}: {first:?} and {second:?}")]
    DefinitionNameCollision {
        /// Colliding OpenAPI component name.
        name: String,
        /// First JSON Schema definition name.
        first: String,
        /// Second JSON Schema definition name.
        second: String,
    },

    /// An operation references a type that is not in the exported collection.
    #[error(
        "operation references type {type_name:?}, which is not registered in the exported collection"
    )]
    UnregisteredOperationType {
        /// Name of the unregistered type.
        type_name: String,
    },

    /// Two operations describe the same method and path.
    #[error("duplicate operation {method} {path:?}")]
    DuplicateOperation {
        /// HTTP method.
        method: String,
        /// Templated path.
        path: String,
    },

    /// An operation declares no responses.
    #[error("operation {path:?} declares no responses")]
    OperationWithoutResponses {
        /// Templated path.
        path: String,
    },

    /// An operation's types could not be resolved to components.
    ///
    /// Component names are obtained by asking the exporter what it names each referenced type. This
    /// means it did not answer, and is raised rather than emitting a `$ref` that resolves to
    /// nothing.
    #[error("could not resolve operation types to exported components")]
    UnresolvedOperationTypes,

    /// JSON output serialization failed.
    #[error("failed to serialize OpenAPI JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML output serialization failed.
    #[error("failed to serialize OpenAPI YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// A component with the same name already exists in a target document.
    #[error("OpenAPI document already contains schema component {0:?}")]
    DuplicateComponent(String),

    /// A target document is not a JSON object where an object is required.
    #[error("cannot add components to a non-object OpenAPI document")]
    InvalidTargetDocument,

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
