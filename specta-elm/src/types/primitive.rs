//! Rendering of Rust primitives and of a named type's declaration header.

use specta::{
    Types,
    datatype::{DataType, Primitive},
};

use crate::{
    Error, Exporter,
    elm::{ELM_ENUM_SYMBOL, ELM_STRUCT_SYMBOL},
    module::ModuleImports,
    types::{INDENT, Ndt},
};

/// Elm's integer type.
pub(crate) const INTEGER: &str = "Int";
/// Elm's floating point type.
pub(crate) const FLOAT: &str = "Float";
/// Elm's boolean type.
pub(crate) const BOOL: &str = "Bool";
/// Elm's string type.
pub(crate) const STRING: &str = "String";

/// Map a Rust primitive onto its Elm equivalent.
pub(crate) fn render(p: &Primitive, location: impl Into<String>) -> Result<&'static str, Error> {
    use specta::datatype::Primitive::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 => INTEGER,
        f16 | f32 | f64 => FLOAT,
        usize | isize | i64 | u64 | i128 | u128 | f128 => {
            return Err(Error::bigint_forbidden(location));
        }
        Primitive::bool => BOOL,
        str | char => STRING,
    })
}

/// Render a named type's declaration, writing the result into `s`.
pub(crate) fn render_declaration<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
) -> Result<(), Error> {
    let name = ndt
        .sanitise_type_name()
        .map_err(|err| err.with_named_datatype(ndt))?;

    let Some(body) = ndt.inner().ty.as_ref() else {
        return Ok(());
    };

    match body {
        DataType::Enum(_) => {
            s.push_str(ELM_ENUM_SYMBOL);
            s.push_str(&name);
            s.push('\n');
            s.push_str(INDENT);
            s.push_str("= ");
        }
        _ => {
            s.push_str(ELM_STRUCT_SYMBOL);
            s.push_str(&name);
            s.push_str(" = ");
        }
    }

    ndt.render(s, imports, exporter, types, body)
        .map_err(|err| err.with_named_datatype(ndt))?;
    s.push_str("\n\n");

    Ok(())
}
