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
    pub(crate) ty: Body,
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
    pub(crate) body: Option<Body>,
}

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
    pub(crate) responses: Vec<Response>,
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
            responses: Vec::new(),
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
    pub fn path_param<T: Type>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameters.push(Parameter {
            name: name.into(),
            location: ParameterLocation::Path,
            required: true,
            description: None,
            ty: capture::<T>(),
        });
        self
    }

    /// Declares an optional query parameter of type `T`.
    pub fn query_param<T: Type>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameters.push(Parameter {
            name: name.into(),
            location: ParameterLocation::Query,
            required: false,
            description: None,
            ty: capture::<T>(),
        });
        self
    }

    /// Declares an optional header parameter of type `T`.
    pub fn header_param<T: Type>(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.parameters.push(Parameter {
            name: name.into(),
            location: ParameterLocation::Header,
            required: false,
            description: None,
            ty: capture::<T>(),
        });
        self
    }

    /// Declares the JSON request body as `T`.
    pub fn request_body<T: Type>(mut self) -> Self {
        self.request_body = Some(capture::<T>());
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
            body: Some(capture::<T>()),
        });
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
