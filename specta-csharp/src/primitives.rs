//! Low-level C# rendering helpers.
//!
//! Most applications should use [`crate::CSharp::export`] so the selected [`specta::Format`]
//! can transform the complete type graph. These helpers are useful to framework authors that
//! already manage collection and formatting.

use specta::{
    Types,
    datatype::{DataType, NamedDataType},
};

use crate::{CSharp, Error, render};

/// Render a datatype at a C# use site.
pub fn datatype(exporter: &CSharp, types: &Types, datatype: &DataType) -> Result<String, Error> {
    render::render_datatype(exporter, types, datatype)
}

/// Render selected named datatypes without applying a [`specta::Format`].
pub fn export<'a>(
    exporter: &CSharp,
    types: &Types,
    datatypes: impl Iterator<Item = &'a NamedDataType>,
) -> Result<String, Error> {
    render::render_named_types(exporter, types, datatypes.collect())
}
