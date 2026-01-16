use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serde error: {0}")]
    Serde(#[from] specta_serde::Error),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid schema version: {0}")]
    InvalidSchemaVersion(String),

    #[error("Duplicate type name '{name}' at {location1} and {location2}")]
    DuplicateTypeName {
        name: String,
        location1: String,
        location2: String,
    },

    #[error("Invalid type name '{name}' at {path}")]
    InvalidTypeName { name: String, path: String },

    #[error("Unable to convert schema: {0}")]
    ConversionError(String),

    #[error("Unsupported DataType: {0}")]
    UnsupportedDataType(String),

    #[error("Invalid reference: {0}")]
    InvalidReference(String),
}
