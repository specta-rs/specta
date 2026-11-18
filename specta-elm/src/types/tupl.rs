//! Rendering of tuples.

use specta::{
    Types,
    datatype::{DataType, Tuple},
};

use crate::{
    Error, Exporter,
    module::ModuleImports,
    types::{Ndt, UNIT},
};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
    tuple: &Tuple,
) -> Result<(), Error> {
    let elements = tuple.elements.iter().collect::<Vec<_>>();
    render_elements(s, imports, exporter, types, ndt, &elements)
}

/// Render a tuple-like group of types.
///
/// Elm only supports pairs and triples, so anything longer is rejected rather
/// than producing invalid Elm.
pub(crate) fn render_elements<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
    elements: &[&DataType],
) -> Result<(), Error> {
    match elements {
        [] => {
            s.push_str(UNIT);
            return Ok(());
        }
        [element] => return ndt.render(s, imports, exporter, types, element),
        _ if elements.len() > 3 => {
            return Err(ndt.unsupported(
                "Elm has no tuples with more than three elements, so this tuple cannot be exported",
            ));
        }
        _ => {}
    }

    s.push_str("( ");
    for (idx, element) in elements.iter().enumerate() {
        if idx != 0 {
            s.push_str(", ");
        }
        ndt.render(s, imports, exporter, types, element)?;
    }
    s.push_str(" )");

    Ok(())
}
