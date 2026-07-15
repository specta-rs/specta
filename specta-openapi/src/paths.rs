//! Lowering of [`Operation`]s into the document's `paths` object.

use crate::{
    Error,
    operation::{Body, Method, Operation, ParameterLocation},
    resolve::Resolved,
};
use indexmap::IndexMap;
use openapiv3::{
    MediaType, Operation as OpenApiOperation, Parameter as OpenApiParameter, ParameterData,
    ParameterSchemaOrContent, PathItem, Paths, ReferenceOr, RequestBody, Response, Responses,
    StatusCode,
};

const JSON: &str = "application/json";

pub(crate) fn paths(operations: &[Operation], resolved: &Resolved) -> Result<Paths, Error> {
    let mut paths: IndexMap<String, ReferenceOr<PathItem>> = IndexMap::new();

    for operation in operations {
        let lowered = lower(operation, resolved)?;
        let entry = paths
            .entry(operation.path.to_string())
            .or_insert_with(|| ReferenceOr::Item(PathItem::default()));
        let ReferenceOr::Item(item) = entry else {
            continue;
        };

        let slot = match operation.method {
            Method::Get => &mut item.get,
            Method::Post => &mut item.post,
            Method::Put => &mut item.put,
            Method::Patch => &mut item.patch,
            Method::Delete => &mut item.delete,
            Method::Head => &mut item.head,
            Method::Options => &mut item.options,
            Method::Trace => &mut item.trace,
        };
        if slot.is_some() {
            return Err(Error::DuplicateOperation {
                method: format!("{:?}", operation.method).to_uppercase(),
                path: operation.path.to_string(),
            });
        }
        *slot = Some(lowered);
    }

    Ok(Paths {
        paths,
        ..Default::default()
    })
}

fn lower(operation: &Operation, resolved: &Resolved) -> Result<OpenApiOperation, Error> {
    if operation.responses.is_empty() {
        return Err(Error::OperationWithoutResponses {
            path: operation.path.to_string(),
        });
    }

    let mut responses = Responses::default();
    for response in &operation.responses {
        responses.responses.insert(
            StatusCode::Code(response.status),
            ReferenceOr::Item(Response {
                description: response.description.to_string(),
                content: content_of(response.body.as_ref(), resolved)?,
                ..Default::default()
            }),
        );
    }

    let request_body = match &operation.request_body {
        Some(body) => Some(ReferenceOr::Item(RequestBody {
            content: content_of(Some(body), resolved)?,
            required: true,
            ..Default::default()
        })),
        None => None,
    };

    Ok(OpenApiOperation {
        tags: operation.tags.iter().map(ToString::to_string).collect(),
        summary: operation.summary.as_ref().map(ToString::to_string),
        description: operation.description.as_ref().map(ToString::to_string),
        operation_id: operation.operation_id.as_ref().map(ToString::to_string),
        parameters: operation
            .parameters
            .iter()
            .map(|p| parameter(p, resolved))
            .collect::<Result<Vec<_>, _>>()?,
        request_body,
        responses,
        ..Default::default()
    })
}

fn content_of(
    body: Option<&Body>,
    resolved: &Resolved,
) -> Result<IndexMap<String, MediaType>, Error> {
    let Some(body) = body else {
        return Ok(IndexMap::new());
    };
    let schema = resolved
        .get(&body.dt)
        .ok_or(Error::UnresolvedOperationTypes)?;
    Ok(IndexMap::from_iter([(
        JSON.to_string(),
        MediaType {
            schema: Some(schema.clone()),
            ..Default::default()
        },
    )]))
}

/// Lowers a parameter, carrying the schema of whatever the extractor parses it into.
fn parameter(
    parameter: &crate::operation::Parameter,
    resolved: &Resolved,
) -> Result<ReferenceOr<OpenApiParameter>, Error> {
    let schema = resolved
        .get(&parameter.ty.dt)
        .ok_or(Error::UnresolvedOperationTypes)?;
    let data = ParameterData {
        name: parameter.name.to_string(),
        description: parameter.description.as_ref().map(ToString::to_string),
        required: parameter.required,
        deprecated: None,
        format: ParameterSchemaOrContent::Schema(schema.clone()),
        example: None,
        examples: IndexMap::new(),
        explode: None,
        extensions: IndexMap::new(),
    };

    Ok(ReferenceOr::Item(match parameter.location {
        ParameterLocation::Path => OpenApiParameter::Path {
            parameter_data: data,
            style: Default::default(),
        },
        ParameterLocation::Query => OpenApiParameter::Query {
            parameter_data: data,
            allow_reserved: false,
            style: Default::default(),
            allow_empty_value: None,
        },
        ParameterLocation::Header => OpenApiParameter::Header {
            parameter_data: data,
            style: Default::default(),
        },
    }))
}
