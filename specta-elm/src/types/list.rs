//! Rendering of lists and sets.

use specta::{Types, datatype::List};

use crate::{
    Error, Exporter,
    elm::ElmCoreLibImport,
    module::ModuleImports,
    types::{Ndt, as_type_argument},
};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    list: &List,
    ndt: &Ndt,
) -> Result<(), Error> {
    let mut element = String::new();
    ndt.render(&mut element, imports, exporter, types, &list.ty)?;

    if list.unique {
        s.push_str("Set ");
        imports.require_core(ElmCoreLibImport::Set);
    } else {
        s.push_str("List ");
    }
    s.push_str(&as_type_argument(&element));

    Ok(())
}
