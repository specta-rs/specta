use openapiv3::{Components, ReferenceOr, Schema};
use serde_json::{Map, Value, json};
use specta::{Format, Types};

use crate::{Error, SchemaMode};

pub(crate) fn components(
    types: &Types,
    format: impl Format,
    mode: SchemaMode,
) -> Result<Components, Error> {
    let mut schema = specta_jsonschema::JsonSchema::default().export_value(types, format)?;
    let definitions = schema
        .as_object_mut()
        .and_then(|root| root.remove("$defs").or_else(|| root.remove("definitions")))
        .and_then(|definitions| definitions.as_object().cloned())
        .unwrap_or_default();

    let mut components = Components::default();
    for (name, schema) in definitions {
        let schema = component_schema(transform(schema, &name, mode)?);
        let schema =
            serde_json::from_value::<Schema>(schema).map_err(|source| Error::InvalidSchema {
                component: name.clone(),
                source,
            })?;
        components.schemas.insert(name, ReferenceOr::Item(schema));
    }
    Ok(components)
}

// A component must be a Schema Object, while a bare `$ref` is a Reference
// Object. JSON Schema permits metadata next to `$ref`; OpenAPI 3.0 does not,
// so an `allOf` wrapper preserves both the alias and its metadata.
fn component_schema(value: Value) -> Value {
    let Value::Object(mut schema) = value else {
        return value;
    };
    let Some(reference) = schema.remove("$ref") else {
        return Value::Object(schema);
    };
    schema.insert("allOf".to_string(), json!([{ "$ref": reference }]));
    Value::Object(schema)
}

fn transform(value: Value, component: &str, mode: SchemaMode) -> Result<Value, Error> {
    Ok(match value {
        Value::Bool(true) => json!({}),
        Value::Bool(false) => json!({ "not": {} }),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| transform(value, component, mode))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(schema) => transform_object(schema, component, mode)?,
        value => value,
    })
}

fn transform_object(
    mut schema: Map<String, Value>,
    component: &str,
    mode: SchemaMode,
) -> Result<Value, Error> {
    schema.remove("$schema");
    schema.remove("$comment");
    move_unsupported_keyword(
        &mut schema,
        "propertyNames",
        "x-specta-property-names",
        "constrained map keys",
        component,
        mode,
    )?;
    move_unsupported_keyword(
        &mut schema,
        "unevaluatedProperties",
        "x-specta-unevaluated-properties",
        "closed flattened intersections",
        component,
        mode,
    )?;
    schema.remove("additionalItems");

    if let Some(Value::String(reference)) = schema.get_mut("$ref") {
        *reference = reference
            .replacen("#/$defs/", "#/components/schemas/", 1)
            .replacen("#/definitions/", "#/components/schemas/", 1);
    }

    if let Some(constant) = schema.remove("const") {
        schema.insert("enum".to_string(), Value::Array(vec![constant]));
    }

    if schema.get("type").and_then(Value::as_str) == Some("null") {
        if mode == SchemaMode::Strict {
            return Err(unsupported(component, "null-only types"));
        }
        schema.insert("type".to_string(), Value::String("object".into()));
        schema.insert("nullable".to_string(), Value::Bool(true));
        schema.insert("maxProperties".to_string(), Value::Number(0.into()));
        schema.insert("additionalProperties".to_string(), Value::Bool(false));
        schema.insert("x-specta-type".to_string(), Value::String("null".into()));
    }

    // Collapse nullable unions before recursively transforming their branches.
    // Otherwise strict mode rejects the raw `{ "type": "null" }` branch before
    // it can be represented by OpenAPI 3.0's `nullable` keyword.
    collapse_nullable_any_of(&mut schema);

    let prefix_items = schema.remove("prefixItems").or_else(|| {
        schema
            .get("items")
            .is_some_and(Value::is_array)
            .then(|| schema.remove("items"))
            .flatten()
    });
    if let Some(Value::Array(items)) = prefix_items {
        let heterogeneous = items
            .first()
            .is_some_and(|first| items.iter().skip(1).any(|item| item != first));
        if heterogeneous && mode == SchemaMode::Strict {
            return Err(unsupported(component, "heterogeneous positional tuples"));
        }
        let mut items = items
            .into_iter()
            .map(|value| transform(value, component, mode))
            .collect::<Result<Vec<_>, _>>()?;
        if heterogeneous {
            schema.insert(
                "x-specta-prefix-items".to_string(),
                Value::Array(items.clone()),
            );
        }
        let item = match items.len() {
            0 => json!({}),
            1 => items.pop().unwrap_or_else(|| json!({})),
            _ if heterogeneous => json!({ "oneOf": items }),
            _ => items.pop().unwrap_or_else(|| json!({})),
        };
        schema.insert("items".to_string(), item);
    }

    for key in ["items", "not"] {
        if let Some(value) = schema.get_mut(key) {
            *value = transform(value.take(), component, mode)?;
        }
    }
    if let Some(value @ Value::Object(_)) = schema.get_mut("additionalProperties") {
        *value = transform(value.take(), component, mode)?;
    }
    for key in ["properties"] {
        if let Some(Value::Object(values)) = schema.get_mut(key) {
            for value in values.values_mut() {
                *value = transform(value.take(), component, mode)?;
            }
        }
    }
    for key in ["oneOf", "allOf", "anyOf"] {
        if let Some(Value::Array(values)) = schema.get_mut(key) {
            for value in values {
                *value = transform(value.take(), component, mode)?;
            }
        }
    }

    collapse_nullable_any_of(&mut schema);
    wrap_reference_with_siblings(&mut schema);
    if mode == SchemaMode::Strict
        && schema.get("nullable") == Some(&Value::Bool(true))
        && !schema.contains_key("type")
    {
        return Err(unsupported(
            component,
            "nullable references or composed schemas",
        ));
    }
    Ok(Value::Object(schema))
}

fn move_unsupported_keyword(
    schema: &mut Map<String, Value>,
    keyword: &str,
    extension: &str,
    feature: &'static str,
    component: &str,
    mode: SchemaMode,
) -> Result<(), Error> {
    let Some(mut value) = schema.remove(keyword) else {
        return Ok(());
    };
    if mode == SchemaMode::Strict {
        return Err(unsupported(component, feature));
    }
    rewrite_definition_refs(&mut value);
    schema.insert(extension.to_string(), value);
    Ok(())
}

fn rewrite_definition_refs(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                *reference = reference
                    .replacen("#/$defs/", "#/components/schemas/", 1)
                    .replacen("#/definitions/", "#/components/schemas/", 1);
            }
            object.values_mut().for_each(rewrite_definition_refs);
        }
        Value::Array(values) => values.iter_mut().for_each(rewrite_definition_refs),
        _ => {}
    }
}

fn wrap_reference_with_siblings(schema: &mut Map<String, Value>) {
    if schema.len() <= 1 {
        return;
    }
    let Some(reference) = schema.remove("$ref") else {
        return;
    };
    schema.insert("allOf".to_string(), json!([{ "$ref": reference }]));
}

fn unsupported(component: &str, feature: &'static str) -> Error {
    Error::UnsupportedSchemaFeature {
        component: component.to_string(),
        feature,
    }
}

fn collapse_nullable_any_of(schema: &mut Map<String, Value>) {
    let Some(Value::Array(mut any_of)) = schema.remove("anyOf") else {
        return;
    };
    let nullable_count = any_of
        .iter()
        .filter(|value| is_nullable_only(value))
        .count();
    if nullable_count != 1 {
        schema.insert("anyOf".to_string(), Value::Array(any_of));
        return;
    }

    any_of.retain(|value| !is_nullable_only(value));
    schema.insert("nullable".to_string(), Value::Bool(true));
    schema.insert("x-specta-nullable".to_string(), Value::Bool(true));
    if any_of.len() != 1 {
        schema.insert("anyOf".to_string(), Value::Array(any_of));
        return;
    }

    let inner = any_of.pop().unwrap_or_else(|| json!({}));
    match inner {
        Value::Object(inner) if !inner.contains_key("$ref") => schema.extend(inner),
        inner => {
            schema.insert("allOf".to_string(), Value::Array(vec![inner]));
        }
    }
}

fn is_nullable_only(value: &Value) -> bool {
    value.as_object().is_some_and(|schema| {
        (schema.get("type").and_then(Value::as_str) == Some("null")
            && schema
                .keys()
                .all(|key| matches!(key.as_str(), "type" | "title" | "description")))
            || (schema.get("nullable") == Some(&Value::Bool(true))
                && schema.keys().all(|key| {
                    matches!(
                        key.as_str(),
                        "nullable" | "title" | "description" | "x-specta-type"
                    )
                }))
    })
}
