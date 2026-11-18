//! Rendering of struct fields, both in type aliases and enum variants.

use std::borrow::Cow;

use specta::{
    Types,
    datatype::{DataType, Field},
};

use crate::{
    Error, Exporter,
    module::ModuleImports,
    types::{Ndt, as_type_argument, elm::sanitise_key},
};

/// Render `key: value` for a single field.
pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
    key: Cow<'static, str>,
    field: &Field,
    ty: &DataType,
) -> Result<(), Error> {
    let key = sanitise_key(key, &ndt.rust_type_path())?;

    let mut value = String::new();
    ndt.render(&mut value, imports, exporter, types, ty)?;

    s.push_str(&key);
    s.push_str(": ");
    if field.optional {
        // `#[specta(optional)]` fields may be missing from the payload, which Elm
        // models with `Maybe`.
        s.push_str("Maybe ");
        s.push_str(&as_type_argument(&value));
    } else {
        s.push_str(&value);
    }

    Ok(())
}
