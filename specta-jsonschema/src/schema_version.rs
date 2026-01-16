use serde::{Deserialize, Serialize};

/// JSON Schema version specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaVersion {
    /// JSON Schema Draft 7 (2018) - Most widely supported
    Draft7,
    /// JSON Schema Draft 2019-09
    Draft2019_09,
    /// JSON Schema Draft 2020-12 (latest)
    Draft2020_12,
}

impl SchemaVersion {
    /// Returns the $schema URI for this version
    pub fn uri(&self) -> &'static str {
        match self {
            Self::Draft7 => "http://json-schema.org/draft-07/schema#",
            Self::Draft2019_09 => "https://json-schema.org/draft/2019-09/schema",
            Self::Draft2020_12 => "https://json-schema.org/draft/2020-12/schema",
        }
    }

    /// Returns the key used for definitions in this version
    /// Draft 7 uses "definitions", newer versions use "$defs"
    pub fn definitions_key(&self) -> &'static str {
        match self {
            Self::Draft7 => "definitions",
            Self::Draft2019_09 | Self::Draft2020_12 => "$defs",
        }
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::Draft7
    }
}
