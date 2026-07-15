use serde::{Deserialize, Serialize};

/// JSON Schema version specification.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaVersion {
    /// JSON Schema Draft 7.
    Draft7,
    /// JSON Schema Draft 2019-09.
    Draft201909,
    /// JSON Schema Draft 2020-12.
    #[default]
    Draft202012,
}

impl SchemaVersion {
    /// Returns the $schema URI for this version
    pub fn uri(&self) -> &'static str {
        match self {
            Self::Draft7 => "http://json-schema.org/draft-07/schema#",
            Self::Draft201909 => "https://json-schema.org/draft/2019-09/schema",
            Self::Draft202012 => "https://json-schema.org/draft/2020-12/schema",
        }
    }

    /// Returns the key used for definitions in this version
    /// Draft 7 uses "definitions", newer versions use "$defs"
    pub fn definitions_key(&self) -> &'static str {
        match self {
            Self::Draft7 => "definitions",
            Self::Draft201909 | Self::Draft202012 => "$defs",
        }
    }

    pub(crate) fn uses_prefix_items(self) -> bool {
        matches!(self, Self::Draft202012)
    }

    pub(crate) fn supports_unevaluated_properties(self) -> bool {
        !matches!(self, Self::Draft7)
    }
}
