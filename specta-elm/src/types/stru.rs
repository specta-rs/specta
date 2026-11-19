//! Rendering of structs as Elm type aliases.

use specta::{
    Types,
    datatype::{Fields, Struct, UnnamedFields},
};

use crate::{
    Error, Exporter,
    module::ModuleImports,
    types::{INDENT, Ndt, UNIT, fields, tupl},
};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
    st: &Struct,
) -> Result<(), Error> {
    match &st.fields {
        Fields::Unit => s.push_str(UNIT),
        Fields::Unnamed(unnamed) => unnamed_fields(s, imports, exporter, types, ndt, unnamed)?,
        Fields::Named(named) => {
            let fields = named
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
                .collect::<Vec<_>>();

            if fields.is_empty() {
                s.push_str(UNIT);
                return Ok(());
            }

            for (idx, (key, field, ty)) in fields.into_iter().enumerate() {
                s.push_str(&if idx == 0 {
                    format!("\n{INDENT}{{ ")
                } else {
                    format!("\n{INDENT}, ")
                });

                fields::render(s, imports, exporter, types, ndt, key.clone(), field, ty)?;
            }

            s.push_str(" }");
        }
    }

    Ok(())
}

fn unnamed_fields<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    ndt: &Ndt,
    fields: &UnnamedFields,
) -> Result<(), Error> {
    let field_tys = fields
        .fields
        .iter()
        .filter_map(|field| field.ty.as_ref())
        .collect::<Vec<_>>();

    match field_tys.as_slice() {
        // A newtype struct serialises as its only field, so it is just an alias.
        [ty] => ndt.render(s, imports, exporter, types, ty),
        _ => tupl::render_elements(s, imports, exporter, types, ndt, &field_tys),
    }
}
