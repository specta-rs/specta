use crate::{Error, Layout, SchemaVersion, primitives};
use serde_json::Value;
use specta::{
    Format, Types,
    datatype::{DataType, Fields, NamedDataType, Reference},
};
use std::{borrow::Cow, collections::BTreeMap, path::Path};

/// JSON Schema exporter configuration
#[derive(Debug, Clone)]
pub struct JsonSchema {
    /// JSON Schema version to use
    pub schema_version: SchemaVersion,
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
    pub fn export(&self, types: &Types, format: impl Format) -> Result<String, Error> {
        let value = self.export_as_value(types, format)?;
        Ok(serde_json::to_string_pretty(&value)?)
    }

    /// Export types to JSON Schema as serde_json::Value
    pub fn export_as_value(&self, types: &Types, format: impl Format) -> Result<Value, Error> {
        let exporter = self.clone();
        let formatted_types = format_types(&exporter, types, &format)?;
        let types = formatted_types.as_ref();

        match exporter.layout {
            Layout::SingleFile => exporter.export_single_file(types),
            Layout::Files => Err(Error::ConversionError(
                "Use export_to() for Files layout".to_string(),
            )),
        }
    }

    /// Export to file or directory
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        types: &Types,
        format: impl Format,
    ) -> Result<(), Error> {
        let exporter = self.clone();
        let formatted_types = format_types(&exporter, types, &format)?;
        let types = formatted_types.as_ref();
        let path = path.as_ref();

        match exporter.layout {
            Layout::SingleFile => {
                let json = exporter.export_single_file(types)?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
                Ok(())
            }
            Layout::Files => exporter.export_files(path, types),
        }
    }

    fn export_single_file(&self, types: &Types) -> Result<Value, Error> {
        let mut definitions = BTreeMap::new();

        // Convert each type to a schema
        for ndt in types.into_sorted_iter() {
            let schema = primitives::export(self, types, &ndt)?;
            let name = ndt.name.to_string();
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

    fn export_files(&self, base_path: &Path, types: &Types) -> Result<(), Error> {
        // Create base directory
        std::fs::create_dir_all(base_path)?;

        // Group types by module path
        let mut by_module: BTreeMap<String, Vec<NamedDataType>> = BTreeMap::new();

        for ndt in types.into_sorted_iter() {
            // module_path returns &Cow<'static, str> which is like &String
            // We need to convert path segments to a string
            let module = ndt.module_path.to_string().replace("::", "/");
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
                let filename = format!("{}.schema.json", ndt.name);
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

fn format_types<'a>(
    exporter: &JsonSchema,
    types: &'a Types,
    format: &dyn Format,
) -> Result<Cow<'a, Types>, Error> {
    let mapped_types = format
        .map_types(types)
        .map_err(|err| Error::format("type graph formatter failed", err))?;
    Ok(Cow::Owned(
        map_types_for_datatype_format(exporter, mapped_types.as_ref(), Some(format))?.into_owned(),
    ))
}

fn map_datatype_format(
    exporter: &JsonSchema,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
) -> Result<DataType, Error> {
    let Some(format) = format else {
        return Ok(dt.clone());
    };

    let mapped = format
        .map_type(types, dt)
        .map_err(|err| Error::format("datatype formatter failed", err))?;

    match mapped {
        Cow::Borrowed(dt) => {
            map_datatype_format_children(exporter, Some(format), types, dt.clone())
        }
        Cow::Owned(dt) => map_datatype_format_children(exporter, Some(format), types, dt),
    }
}

fn map_datatype_format_children(
    exporter: &JsonSchema,
    format: Option<&dyn Format>,
    types: &Types,
    mut dt: DataType,
) -> Result<DataType, Error> {
    match &mut dt {
        DataType::Primitive(_) => {}
        DataType::List(list) => {
            list.ty = Box::new(map_datatype_format(exporter, format, types, &list.ty)?);
        }
        DataType::Map(map) => {
            let key = map_datatype_format(exporter, format, types, map.key_ty())?;
            let value = map_datatype_format(exporter, format, types, map.value_ty())?;
            map.set_key_ty(key);
            map.set_value_ty(value);
        }
        DataType::Nullable(inner) => {
            **inner = map_datatype_format(exporter, format, types, inner)?;
        }
        DataType::Struct(strct) => map_datatype_fields(exporter, format, types, &mut strct.fields)?,
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                map_datatype_fields(exporter, format, types, &mut variant.fields)?;
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                *element = map_datatype_format(exporter, format, types, element)?;
            }
        }
        DataType::Intersection(intersection) => {
            for element in intersection {
                *element = map_datatype_format(exporter, format, types, element)?;
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            if let specta::datatype::NamedReferenceType::Reference { generics, .. } = &mut reference.inner {
                for (_, generic) in generics {
                    *generic = map_datatype_format(exporter, format, types, generic)?;
                }
            }
        }
        DataType::Reference(Reference::Opaque(_)) | DataType::Generic(_) => {}
    }

    Ok(dt)
}

fn map_datatype_fields(
    exporter: &JsonSchema,
    format: Option<&dyn Format>,
    types: &Types,
    fields: &mut Fields,
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = map_datatype_format(exporter, format, types, ty)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &mut named.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = map_datatype_format(exporter, format, types, ty)?;
                }
            }
        }
    }

    Ok(())
}

fn map_types_for_datatype_format<'a>(
    exporter: &JsonSchema,
    types: &'a Types,
    format: Option<&dyn Format>,
) -> Result<Cow<'a, Types>, Error> {
    if format.is_none() {
        return Ok(Cow::Borrowed(types));
    }

    let mut mapped_types = types.clone();
    let mut map_err = None;
    mapped_types.iter_mut(|ndt| {
        if map_err.is_some() {
            return;
        }

        let Some(ty) = &ndt.ty else {
            return;
        };

        match map_datatype_format(exporter, format, types, ty) {
            Ok(mapped) => ndt.ty = Some(mapped),
            Err(err) => map_err = Some(err),
        }
    });

    if let Some(err) = map_err {
        return Err(err);
    }

    Ok(Cow::Owned(mapped_types))
}
