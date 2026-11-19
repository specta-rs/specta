//! Rendering of [`DataType`]s into Elm type expressions.

use std::borrow::Cow;

use specta::{
    Types,
    datatype::{DataType, NamedDataType},
};

use crate::{Error, Exporter, elm::RESERVED_TYPE_NAMES, module::ModuleImports};

pub(crate) mod elm;
pub(crate) mod enu;
pub(crate) mod fields;
pub(crate) mod list;
pub(crate) mod map;
pub(crate) mod primitive;
pub(crate) mod reference;
pub(crate) mod stru;
pub(crate) mod tupl;

/// Indentation of one Elm block.
pub(crate) const INDENT: &str = "    ";

/// Elm's unit type, used for Rust types which serialise to nothing.
pub(crate) const UNIT: &str = "()";

/// Wrapper around a [`NamedDataType`] adding the traversal context Elm rendering needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ndt<'a> {
    ndt: &'a NamedDataType,
}

impl<'a> From<&'a NamedDataType> for Ndt<'a> {
    fn from(ndt: &'a NamedDataType) -> Self {
        Self { ndt }
    }
}

impl<'a> Ndt<'a> {
    /// Render `dt` as an Elm type expression.
    pub(crate) fn render<E: Exporter>(
        &self,
        s: &mut String,
        imports: &mut ModuleImports,
        exporter: &E,
        types: &Types,
        dt: &DataType,
    ) -> Result<(), Error> {
        match dt {
            DataType::Primitive(p) => s.push_str(primitive::render(p, self.rust_type_path())?),
            DataType::Generic(_) => {
                return Err(self.unsupported("generic types are not supported by Elm"));
            }
            DataType::List(list) => list::render(s, imports, exporter, types, list, self)?,
            DataType::Map(map) => map::render(s, imports, types, map, self, exporter)?,
            DataType::Nullable(inner) => {
                let mut rendered = String::new();
                self.render(&mut rendered, imports, exporter, types, inner)?;

                s.push_str("Maybe ");
                s.push_str(&as_type_argument(&rendered));
            }
            DataType::Struct(st) => stru::render(s, imports, exporter, types, self, st)?,
            DataType::Enum(enm) => enu::render(s, imports, exporter, types, enm, self)?,
            DataType::Tuple(tuple) => tupl::render(s, imports, exporter, types, self, tuple)?,
            DataType::Intersection(_) => {
                return Err(self.unsupported(
                    "intersection types (such as serde's `flatten`) are not supported by Elm",
                ));
            }
            DataType::Reference(reference) => {
                reference::render(s, imports, exporter, types, reference, self)?
            }
        }

        Ok(())
    }

    /// The named datatype being rendered.
    pub(crate) fn inner(&self) -> &'a NamedDataType {
        self.ndt
    }

    /// The Rust path of the type, used in error messages.
    pub(crate) fn rust_type_path(&self) -> Cow<'static, str> {
        let ndt = self.ndt;
        if ndt.module_path.is_empty() {
            ndt.name.clone()
        } else {
            Cow::Owned(format!("{}::{}", ndt.module_path, ndt.name))
        }
    }

    /// The name of the Elm type this datatype is exported as.
    pub(crate) fn sanitise_type_name(&self) -> Result<String, Error> {
        let ident = &self.ndt.name;

        if ident.is_empty() {
            return Err(Error::empty_name(self.rust_type_path()));
        }

        if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
            return Err(Error::forbidden_name(self.rust_type_path(), name));
        }

        let mut chars = ident.chars();
        if let Some(first_char) = chars.next()
            && !first_char.is_uppercase()
        {
            return Err(Error::invalid_name(
                self.rust_type_path(),
                ident.to_string(),
            ));
        }

        if chars.any(|c| !c.is_alphanumeric() && c != '_') {
            return Err(Error::invalid_name(
                self.rust_type_path(),
                ident.to_string(),
            ));
        }

        Ok(ident.to_string())
    }

    pub(crate) fn unsupported(&self, reason: impl Into<Cow<'static, str>>) -> Error {
        Error::unsupported(self.rust_type_path(), reason)
    }
}

/// Wrap a rendered type in parentheses when it is used as a type argument.
///
/// Elm type application is left associative, so `List Maybe String` parses as
/// `((List Maybe) String)`. Any argument which is itself an application (or a
/// variable) has to be parenthesised: `List (Maybe String)`. Tuples already
/// carry their own parentheses.
pub(crate) fn as_type_argument(rendered: &str) -> Cow<'_, str> {
    if !rendered.contains(' ') || rendered.starts_with('(') {
        Cow::Borrowed(rendered)
    } else {
        Cow::Owned(format!("({rendered})"))
    }
}
