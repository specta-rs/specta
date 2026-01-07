use crate::{Error, Layout, SchemaVersion, primitives};
use serde_json::Value;
use specta::{TypeCollection, datatype::NamedDataType};
use specta_serde::SerdeMode;
use std::collections::BTreeMap;
use std::path::Path;

/// JSON Schema exporter configuration
#[derive(Debug, Clone)]
pub struct JsonSchema {
    /// JSON Schema version to use
    pub schema_version: SchemaVersion,
    /// Optional serde mode for transformations
    pub serde: Option<SerdeMode>,
    /// Layout for output organization
    pub layout: Layout,
    /// Optional title for the root schema
    pub title: Option<String>,
    /// Optional description for the root schema
    pub description: Option<String>,
}

impl Default for JsonSchema {
    fn default() -> Self {
        Self {
            schema_version: SchemaVersion::default(),
            serde: None,
            layout: Layout::default(),
            title: None,
            description: None,
        }
    }
}

impl JsonSchema {
    /// Create a new JsonSchema exporter with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set JSON Schema version
    pub fn schema_version(mut self, version: SchemaVersion) -> Self {
        self.schema_version = version;
        self
    }

    /// Enable serde transformations with specific mode
    pub fn with_serde(mut self, mode: SerdeMode) -> Self {
        self.serde = Some(mode);
        self
    }

    /// Enable serde transformations for serialization
    pub fn with_serde_serialize(self) -> Self {
        self.with_serde(SerdeMode::Serialize)
    }

    /// Enable serde transformations for deserialization
    pub fn with_serde_deserialize(self) -> Self {
        self.with_serde(SerdeMode::Deserialize)
    }

    /// Set output layout
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Set root schema title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set root schema description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Export types to JSON Schema as a JSON string
    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        let value = self.export_as_value(types)?;
        Ok(serde_json::to_string_pretty(&value)?)
    }

    /// Export types to JSON Schema as serde_json::Value
    pub fn export_as_value(&self, types: &TypeCollection) -> Result<Value, Error> {
        // Apply serde transformations if configured
        let processed_types = if let Some(mode) = self.serde {
            let mut cloned = types.clone();
            specta_serde::apply(&mut cloned, mode)?;
            cloned
        } else {
            types.clone()
        };

        match self.layout {
            Layout::SingleFile => self.export_single_file(&processed_types),
            Layout::Files => Err(Error::ConversionError(
                "Use export_to() for Files layout".to_string(),
            )),
        }
    }

    /// Export to file or directory
    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        let path = path.as_ref();

        // Apply serde transformations if configured
        let processed_types = if let Some(mode) = self.serde {
            let mut cloned = types.clone();
            specta_serde::apply(&mut cloned, mode)?;
            cloned
        } else {
            types.clone()
        };

        match self.layout {
            Layout::SingleFile => {
                let json = self.export_single_file(&processed_types)?;
                std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
                Ok(())
            }
            Layout::Files => self.export_files(path, &processed_types),
        }
    }

    fn export_single_file(&self, types: &TypeCollection) -> Result<Value, Error> {
        let mut definitions = BTreeMap::new();

        // Convert each type to a schema
        for ndt in types.into_sorted_iter() {
            let schema = primitives::export(self, types, &ndt)?;
            let name = ndt.name().to_string();
            definitions.insert(name, schema);
        }

        // Build root schema
        let defs_key = self.schema_version.definitions_key();
        let mut root = serde_json::json!({
            "$schema": self.schema_version.uri(),
            defs_key: definitions,
        });

        if let Some(title) = &self.title {
            root.as_object_mut()
                .unwrap()
                .insert("title".to_string(), Value::String(title.clone()));
        }

        if let Some(description) = &self.description {
            root.as_object_mut().unwrap().insert(
                "description".to_string(),
                Value::String(description.clone()),
            );
        }

        Ok(root)
    }

    fn export_files(&self, base_path: &Path, types: &TypeCollection) -> Result<(), Error> {
        // Create base directory
        std::fs::create_dir_all(base_path)?;

        // Group types by module path
        let mut by_module: BTreeMap<String, Vec<NamedDataType>> = BTreeMap::new();

        for ndt in types.into_sorted_iter() {
            // module_path returns &Cow<'static, str> which is like &String
            // We need to convert path segments to a string
            let module = ndt.module_path().to_string().replace("::", "/");
            by_module.entry(module).or_default().push(ndt.clone());
        }

        // Write each type to its own file
        for (module, ndts) in by_module {
            let module_dir = if module.is_empty() {
                base_path.to_path_buf()
            } else {
                base_path.join(&module)
            };

            std::fs::create_dir_all(&module_dir)?;

            for ndt in &ndts {
                let schema = primitives::export(self, types, ndt)?;
                let filename = format!("{}.schema.json", ndt.name());
                let file_path = module_dir.join(filename);

                // Create a root schema for this type
                let mut root = serde_json::json!({
                    "$schema": self.schema_version.uri(),
                });

                // Merge in the type's schema properties
                if let Some(obj) = schema.as_object() {
                    for (k, v) in obj {
                        root.as_object_mut().unwrap().insert(k.clone(), v.clone());
                    }
                }

                std::fs::write(file_path, serde_json::to_string_pretty(&root)?)?;
            }
        }

        Ok(())
    }
}
