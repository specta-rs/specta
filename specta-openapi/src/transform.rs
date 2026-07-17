use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use specta::{Format, Types};

use crate::{Error, SchemaMode};

pub(crate) fn components(
    types: &Types,
    format: impl Format,
    mode: SchemaMode,
) -> Result<BTreeMap<String, Value>, Error> {
    let mut schema = specta_jsonschema::JsonSchema::default()
        // OpenAPI generators map numeric formats to language types.
        .number_formats(true)
        .export_value(types, format)?;
    let definitions = schema
        .as_object_mut()
        .and_then(|root| root.remove("$defs").or_else(|| root.remove("definitions")))
        .and_then(|definitions| definitions.as_object().cloned())
        .unwrap_or_default();

    let mut names = BTreeMap::new();
    let mut references = BTreeMap::new();
    for name in definitions.keys() {
        let component_name = component_name(name);
        if let Some(first) = names.insert(component_name.clone(), name.clone()) {
            return Err(Error::DefinitionNameCollision {
                name: component_name,
                first,
                second: name.clone(),
            });
        }
        references.insert(definition_ref(name), component_name);
    }

    let mut components = BTreeMap::new();
    for (name, mut schema) in definitions {
        let component_name = references
            .get(&definition_ref(&name))
            .cloned()
            .unwrap_or_else(|| component_name(&name));
        rewrite_component_refs(&mut schema, &references);
        let schema = component_schema(transform(schema, &component_name, mode)?);
        components.insert(component_name, schema);
    }
    Ok(components)
}

pub(crate) fn component_name(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            if separator && !output.is_empty() {
                output.push('_');
            }
            separator = false;
            output.push(character);
        } else {
            separator = true;
        }
    }
    if output.is_empty() {
        "Type".to_string()
    } else {
        output
    }
}

fn definition_ref(name: &str) -> String {
    let pointer = name.replace('~', "~0").replace('/', "~1");
    let mut encoded = String::with_capacity(pointer.len());
    for byte in pointer.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    format!("#/$defs/{encoded}")
}

fn rewrite_component_refs(value: &mut Value, references: &BTreeMap<String, String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref")
                && let Some(component) = references.get(reference)
            {
                *reference = format!("#/components/schemas/{component}");
            }
            object
                .values_mut()
                .for_each(|value| rewrite_component_refs(value, references));
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| rewrite_component_refs(value, references)),
        _ => {}
    }
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

    compact_string_enum(&mut schema);

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
    while collapse_nullable_any_of(&mut schema) {}

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

fn collapse_nullable_any_of(schema: &mut Map<String, Value>) -> bool {
    let Some(Value::Array(mut any_of)) = schema.remove("anyOf") else {
        return false;
    };
    let nullable_count = any_of
        .iter()
        .filter(|value| is_nullable_only(value))
        .count();
    if nullable_count != 1 {
        schema.insert("anyOf".to_string(), Value::Array(any_of));
        return false;
    }

    any_of.retain(|value| !is_nullable_only(value));
    schema.insert("nullable".to_string(), Value::Bool(true));
    schema.insert("x-specta-nullable".to_string(), Value::Bool(true));
    if any_of.len() != 1 {
        schema.insert("anyOf".to_string(), Value::Array(any_of));
        return true;
    }

    let inner = any_of.pop().unwrap_or_else(|| json!({}));
    match inner {
        Value::Object(inner) if !inner.contains_key("$ref") => schema.extend(inner),
        inner => {
            schema.insert("allOf".to_string(), Value::Array(vec![inner]));
        }
    }
    true
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

/// Collapses a `oneOf` whose members are all single-value string enums - the
/// shape a plain string enum renders as - into the compact
/// `{ "type": "string", "enum": [...] }` form generators handle best.
/// Per-variant descriptions have no home in the compact form, so when any
/// member carries one they are retained in `x-specta-enum-descriptions`,
/// keyed by value.
fn compact_string_enum(schema: &mut Map<String, Value>) {
    let Some(Value::Array(members)) = schema.get("oneOf") else {
        return;
    };
    let mut values = Vec::with_capacity(members.len());
    let mut descriptions = Map::new();
    for member in members {
        let Value::Object(member) = member else {
            return;
        };
        // Exactly a single string value - members are inspected before their
        // own transform runs, so both the JSON Schema `const` form and the
        // already-lowered single-value `enum` form appear here - with nothing
        // but an optional description beside it.
        let value = match (member.get("const"), member.get("enum")) {
            (Some(Value::String(value)), None) => value,
            (None, Some(Value::Array(constants))) => match constants.as_slice() {
                [Value::String(value)] => value,
                _ => return,
            },
            _ => return,
        };
        if member
            .keys()
            .any(|key| !matches!(key.as_str(), "const" | "enum" | "description"))
        {
            return;
        }
        if let Some(Value::String(description)) = member.get("description") {
            // Doc comments arrive with their leading space.
            descriptions.insert(value.clone(), Value::String(description.trim().to_string()));
        }
        values.push(Value::String(value.clone()));
    }
    schema.remove("oneOf");
    schema.insert("type".to_string(), Value::String("string".into()));
    schema.insert("enum".to_string(), Value::Array(values));
    if !descriptions.is_empty() {
        schema.insert(
            "x-specta-enum-descriptions".to_string(),
            Value::Object(descriptions),
        );
    }
}
