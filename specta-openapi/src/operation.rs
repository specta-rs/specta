//! Description of a single endpoint, for the `paths` object.

use std::borrow::Cow;

use specta::{Type, Types, datatype::DataType};

/// HTTP method an [`Operation`] is served on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// `GET`
    Get,
    /// `POST`
    Post,
    /// `PUT`
    Put,
    /// `PATCH`
    Patch,
    /// `DELETE`
    Delete,
    /// `HEAD`
    Head,
    /// `OPTIONS`
    Options,
    /// `TRACE`
    Trace,
}

/// Where a parameter is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParameterLocation {
    Path,
    Query,
    Header,
}

#[derive(Debug, Clone)]
pub(crate) struct Parameter {
    pub(crate) name: Cow<'static, str>,
    pub(crate) location: ParameterLocation,
    pub(crate) required: bool,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) example: Option<serde_json::Value>,
    pub(crate) ty: Body,
}

/// A parameter under construction: [`Param::path`], [`Param::query`], or
/// [`Param::header`] pick the location and type, and the builder methods add
/// what the bare [`Operation`] conveniences cannot say.
///
/// ```rust
/// # use specta_openapi::{Operation, Param};
/// # use serde_json::json;
/// let operation = Operation::get("/v1/weather/forecast")
///     .parameter(
///         Param::query::<f64>("lat")
///             .required()
///             .description("Latitude, WGS84 degrees, -90 to 90")
///             .example(json!(35.0)),
///     );
/// ```
#[derive(Debug, Clone)]
pub struct Param(pub(crate) Parameter);

impl Param {
    /// A templated path parameter of type `T`. Path parameters are always
    /// required.
    pub fn path<T: Type>(name: impl Into<Cow<'static, str>>) -> Self {
        Self(Parameter {
            name: name.into(),
            location: ParameterLocation::Path,
            required: true,
            description: None,
            example: None,
            ty: capture::<T>(),
        })
    }

    /// A query parameter of type `T`, optional unless [`required`](Self::required).
    pub fn query<T: Type>(name: impl Into<Cow<'static, str>>) -> Self {
        Self(Parameter {
            name: name.into(),
            location: ParameterLocation::Query,
            required: false,
            description: None,
            example: None,
            ty: capture::<T>(),
        })
    }

    /// A header parameter of type `T`, optional unless [`required`](Self::required).
    pub fn header<T: Type>(name: impl Into<Cow<'static, str>>) -> Self {
        Self(Parameter {
            name: name.into(),
            location: ParameterLocation::Header,
            required: false,
            description: None,
            example: None,
            ty: capture::<T>(),
        })
    }

    /// Marks the parameter required.
    pub fn required(mut self) -> Self {
        self.0.required = true;
        self
    }

    /// Sets the parameter's description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.0.description = Some(description.into());
        self
    }

    /// Sets the parameter's example value.
    pub fn example(mut self, example: serde_json::Value) -> Self {
        self.0.example = Some(example);
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Body {
    pub(crate) dt: DataType,
    /// Rust's name for the type, which stays available even when the type cannot be resolved and is
    /// the only useful thing to say in that error.
    pub(crate) type_name: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) description: Cow<'static, str>,
    pub(crate) content_type: Option<Cow<'static, str>>,
    pub(crate) body: Option<Body>,
}

/// One way a caller may satisfy an operation's security: scheme names (as
/// registered on the document) to their required scopes. An empty requirement
/// means the operation also works anonymously.
pub(crate) type SecurityRequirement = Vec<(Cow<'static, str>, Vec<String>)>;

/// A single endpoint: one method on one path.
///
/// Bodies are given as types rather than names, and are resolved to their exported component when
/// the document is built. The type must be registered in the [`Types`] passed to the exporter.
///
/// ```rust
/// use specta::Type;
/// use specta_openapi::Operation;
///
/// #[derive(Type)]
/// struct Recipe { name: String }
///
/// let operation = Operation::get("/recipes/{slug}")
///     .summary("Fetch one recipe")
///     .path_param::<String>("slug")
///     .response::<Recipe>(200, "The recipe");
/// ```
#[derive(Debug, Clone)]
pub struct Operation {
    pub(crate) method: Method,
    pub(crate) path: Cow<'static, str>,
    pub(crate) summary: Option<Cow<'static, str>>,
    pub(crate) description: Option<Cow<'static, str>>,
    pub(crate) operation_id: Option<Cow<'static, str>>,
    pub(crate) tags: Vec<Cow<'static, str>>,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) request_body: Option<Body>,
    /// Serialized eagerly; a failure is carried as the error message and
    /// raised loudly at export rather than dropped.
    pub(crate) request_body_example: Option<Result<serde_json::Value, String>>,
    pub(crate) responses: Vec<Response>,
    pub(crate) security: Vec<SecurityRequirement>,
}

impl Operation {
    /// Describes an operation on `method` and `path`.
    ///
    /// Path templating follows OpenAPI: `/recipes/{slug}` expects a matching
    /// [`path_param`](Self::path_param).
    pub fn new(method: Method, path: impl Into<Cow<'static, str>>) -> Self {
        Self {
            method,
            path: path.into(),
            summary: None,
            description: None,
            operation_id: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            request_body: None,
            request_body_example: None,
            responses: Vec::new(),
            security: Vec::new(),
        }
    }

    /// Describes a `GET` operation.
    pub fn get(path: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Method::Get, path)
    }

    /// Describes a `POST` operation.
    pub fn post(path: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Method::Post, path)
    }

    /// Describes a `PUT` operation.
    pub fn put(path: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Method::Put, path)
    }

    /// Describes a `PATCH` operation.
    pub fn patch(path: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Method::Patch, path)
    }

    /// Describes a `DELETE` operation.
    pub fn delete(path: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Method::Delete, path)
    }

    /// Sets the operation's short summary.
    pub fn summary(mut self, summary: impl Into<Cow<'static, str>>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Sets the operation's long description.
    pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the operation's `operationId`, which generators use to name the produced client method.
    pub fn operation_id(mut self, operation_id: impl Into<Cow<'static, str>>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    /// Adds a tag, which generators use to group operations.
    pub fn tag(mut self, tag: impl Into<Cow<'static, str>>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Declares a templated path parameter of type `T`. Path parameters are always required.
    ///
    /// `T` is what the extractor parses the segment into, so `/users/{id}` served by a
    /// `Path<u32>` is `path_param::<u32>("id")` and exports as an integer.
    pub fn path_param<T: Type>(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameter(Param::path::<T>(name))
    }

    /// Declares an optional query parameter of type `T`.
    pub fn query_param<T: Type>(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameter(Param::query::<T>(name))
    }

    /// Declares an optional header parameter of type `T`.
    pub fn header_param<T: Type>(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameter(Param::header::<T>(name))
    }

    /// Declares a parameter built with [`Param`], which can say what the bare
    /// conveniences cannot: required query parameters, descriptions, and
    /// example values.
    pub fn parameter(mut self, param: Param) -> Self {
        self.parameters.push(param.0);
        self
    }

    /// Declares the JSON request body as `T`.
    pub fn request_body<T: Type>(mut self) -> Self {
        self.request_body = Some(capture::<T>());
        self
    }

    /// Declares the JSON request body as `T` with an example: a real value of
    /// the body type, serialized through the same serde path as production
    /// traffic and carried in the media type's `example`. The compiler keeps
    /// the example true to the schema — it cannot drift the way an untyped
    /// annotation can.
    pub fn request_body_with_example<T: Type + serde::Serialize>(mut self, example: T) -> Self {
        self.request_body = Some(capture::<T>());
        self.request_body_example =
            Some(serde_json::to_value(&example).map_err(|error| error.to_string()));
        self
    }

    /// Declares a JSON response body of `T` for `status`.
    ///
    /// Call once per status an endpoint can return; the multi-status case has no single Rust return
    /// type to infer from, so it is stated rather than derived.
    pub fn response<T: Type>(
        mut self,
        status: u16,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.responses.push(Response {
            status,
            description: description.into(),
            content_type: None,
            body: Some(capture::<T>()),
        });
        self
    }

    /// Declares a response body of `T` for `status` served with a content
    /// type other than `application/json`, such as RFC 9457's
    /// `application/problem+json`.
    pub fn response_as<T: Type>(
        mut self,
        status: u16,
        description: impl Into<Cow<'static, str>>,
        content_type: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.responses.push(Response {
            status,
            description: description.into(),
            content_type: Some(content_type.into()),
            body: Some(capture::<T>()),
        });
        self
    }

    /// Adds one way a caller may satisfy this operation's security: scheme
    /// names (as registered on the document) to their required scopes. Each
    /// call adds an alternative; a caller needs to satisfy only one.
    pub fn security<N: Into<Cow<'static, str>>>(
        mut self,
        requirement: impl IntoIterator<Item = (N, Vec<String>)>,
    ) -> Self {
        self.security.push(
            requirement
                .into_iter()
                .map(|(name, scopes)| (name.into(), scopes))
                .collect(),
        );
        self
    }

    /// Also allows the operation to be called with no credentials at all: the
    /// empty security requirement, which makes any [`security`](Self::security)
    /// alternatives optional.
    pub fn security_optional(mut self) -> Self {
        self.security.push(Vec::new());
        self
    }

    /// Every type this operation references, with Rust's name for it.
    pub(crate) fn referenced_types(&self) -> impl Iterator<Item = (&DataType, &'static str)> {
        self.request_body
            .iter()
            .chain(
                self.responses
                    .iter()
                    .filter_map(|response| response.body.as_ref()),
            )
            .chain(self.parameters.iter().map(|parameter| &parameter.ty))
            .map(|body| (&body.dt, body.type_name))
    }

    /// Declares a response with no body, such as a `204`.
    pub fn empty_response(
        mut self,
        status: u16,
        description: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.responses.push(Response {
            status,
            description: description.into(),
            content_type: None,
            body: None,
        });
        self
    }
}

/// Captures `T`'s datatype so it can be resolved against the exporter's collection later.
///
/// Taken from a scratch collection: a derived type's identity is its module path and name, so the
/// result stays resolvable against whichever collection is exported.
fn capture<T: Type>() -> Body {
    let mut scratch = Types::default();
    Body {
        dt: T::definition(&mut scratch),
        type_name: std::any::type_name::<T>(),
    }
}
