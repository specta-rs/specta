use std::borrow::Cow;

use serde_json::{Map, Value};
use specta::{Format, Types};

use crate::{Error, SchemaVersion, render::Renderer};

#[derive(Debug, Default, Clone)]
pub(crate) struct Exporter {
    pub(crate) schema_version: SchemaVersion,
    pub(crate) title: Option<Cow<'static, str>>,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) comment: Option<Cow<'static, str>>,
}

impl Exporter {
    pub(crate) fn export_value(&self, types: &Types, format: &dyn Format) -> Result<Value, Error> {
        let types = format
            .map_types(types)
            .map_err(|err| Error::format("type graph formatter failed", err))?;
        let types = types.as_ref();

        let renderer = Renderer::new(self.schema_version, types);
        let definitions = renderer.render_definitions()?;

        let mut root = Map::new();
        root.insert(
            "$schema".to_string(),
            Value::String(self.schema_version.uri().to_string()),
        );
        if let Some(title) = &self.title {
            root.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(description) = &self.description {
            root.insert(
                "description".to_string(),
                Value::String(description.to_string()),
            );
        }
        if let Some(comment) = &self.comment {
            root.insert("$comment".to_string(), Value::String(comment.to_string()));
        }
        root.insert(
            self.schema_version.definitions_key().to_string(),
            Value::Object(definitions),
        );

        Ok(Value::Object(root))
    }
}
