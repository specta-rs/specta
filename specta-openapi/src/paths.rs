//! Lowering of [`Operation`]s into the document's `paths` object.

use serde_json::{Map, Value, json};

use crate::{
    Error,
    operation::{Body, Method, Operation, ParameterLocation},
    resolve::Resolved,
};

const JSON: &str = "application/json";

pub(crate) fn paths(operations: &[Operation], resolved: &Resolved) -> Result<Value, Error> {
    let mut paths = Map::new();

    for operation in operations {
        let lowered = lower(operation, resolved)?;
        let item = paths
            .entry(operation.path.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let Some(item) = item.as_object_mut() else {
            continue;
        };

        let method = method_key(operation.method);
        if item.contains_key(method) {
            return Err(Error::DuplicateOperation {
                method: method.to_uppercase(),
                path: operation.path.to_string(),
            });
        }
        item.insert(method.to_string(), lowered);
    }

    Ok(Value::Object(paths))
}

fn method_key(method: Method) -> &'static str {
    match method {
        Method::Get => "get",
        Method::Post => "post",
        Method::Put => "put",
        Method::Patch => "patch",
        Method::Delete => "delete",
        Method::Head => "head",
        Method::Options => "options",
        Method::Trace => "trace",
    }
}

fn lower(operation: &Operation, resolved: &Resolved) -> Result<Value, Error> {
    if operation.responses.is_empty() {
        return Err(Error::OperationWithoutResponses {
            path: operation.path.to_string(),
        });
    }

    let mut responses = Map::new();
    for response in &operation.responses {
        let mut object = Map::new();
        object.insert("description".to_string(), json!(response.description));
        if let Some(body) = &response.body {
            let content_type = response.content_type.as_deref().unwrap_or(JSON);
            object.insert(
                "content".to_string(),
                json!({ content_type: { "schema": response_schema_of(body, resolved)? } }),
            );
        }
        responses.insert(response.status.to_string(), Value::Object(object));
    }

    let mut object = Map::new();
    if !operation.tags.is_empty() {
        object.insert("tags".to_string(), json!(operation.tags));
    }
    if let Some(summary) = &operation.summary {
        object.insert("summary".to_string(), json!(summary));
    }
    if let Some(description) = &operation.description {
        object.insert("description".to_string(), json!(description));
    }
    if let Some(operation_id) = &operation.operation_id {
        object.insert("operationId".to_string(), json!(operation_id));
    }
    if !operation.parameters.is_empty() {
        object.insert(
            "parameters".to_string(),
            Value::Array(
                operation
                    .parameters
                    .iter()
                    .map(|parameter| self::parameter(parameter, resolved))
                    .collect::<Result<_, _>>()?,
            ),
        );
    }
    if let Some(body) = &operation.request_body {
        let mut media = Map::new();
        media.insert("schema".to_string(), request_schema_of(body, resolved)?);
        if let Some(example) = &operation.request_body_example {
            let example = example
                .clone()
                .map_err(|message| Error::ExampleSerialization {
                    path: operation.path.to_string(),
                    message,
                })?;
            media.insert("example".to_string(), example);
        }
        object.insert(
            "requestBody".to_string(),
            json!({
                "content": { JSON: media },
                "required": true,
            }),
        );
    }
    object.insert("responses".to_string(), Value::Object(responses));
    if !operation.security.is_empty() {
        object.insert(
            "security".to_string(),
            Value::Array(
                operation
                    .security
                    .iter()
                    .map(|requirement| {
                        Value::Object(
                            requirement
                                .iter()
                                .map(|(name, scopes)| (name.to_string(), json!(scopes)))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        );
    }

    Ok(Value::Object(object))
}

/// Lowers a parameter, carrying the schema of whatever the extractor parses it into.
fn parameter(parameter: &crate::operation::Parameter, resolved: &Resolved) -> Result<Value, Error> {
    let mut object = Map::new();
    object.insert("name".to_string(), json!(parameter.name));
    object.insert(
        "in".to_string(),
        json!(match parameter.location {
            ParameterLocation::Path => "path",
            ParameterLocation::Query => "query",
            ParameterLocation::Header => "header",
        }),
    );
    if let Some(description) = &parameter.description {
        object.insert("description".to_string(), json!(description));
    }
    if parameter.required {
        object.insert("required".to_string(), json!(true));
    }
    if let Some(example) = &parameter.example {
        object.insert("example".to_string(), example.clone());
    }
    object.insert(
        "schema".to_string(),
        request_schema_of(&parameter.ty, resolved)?,
    );
    Ok(Value::Object(object))
}

/// What the exporter emitted for a request-side type - a body or parameter,
/// resolved through the deserialize phase: a `$ref` when it has a component,
/// its schema in place when it does not.
fn request_schema_of(body: &Body, resolved: &Resolved) -> Result<Value, Error> {
    resolved
        .request(&body.dt)
        .cloned()
        .ok_or(Error::UnresolvedOperationTypes)
}

/// What the exporter emitted for a response-side type, resolved through the
/// serialize phase.
fn response_schema_of(body: &Body, resolved: &Resolved) -> Result<Value, Error> {
    resolved
        .response(&body.dt)
        .cloned()
        .ok_or(Error::UnresolvedOperationTypes)
}
