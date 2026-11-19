//! Rendering of enums as Elm custom types.

use specta::{
    Types,
    datatype::{DataType, Enum, Fields},
};

use crate::{
    Error, Exporter,
    module::ModuleImports,
    types::{INDENT, Ndt, as_type_argument, fields, tupl},
};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    e: &Enum,
    ndt: &Ndt,
) -> Result<(), Error> {
    let variants = e
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();

    if variants.is_empty() {
        return Err(ndt.unsupported(
            "Elm custom types need at least one variant, and every variant of this enum is skipped",
        ));
    }

    for (idx, (name, variant)) in variants.into_iter().enumerate() {
        if !name.starts_with(char::is_uppercase) {
            return Err(ndt.unsupported(format!(
                "enum variant {name:?} must start with an uppercase letter to be a valid Elm constructor"
            )));
        }

        if idx != 0 {
            s.push('\n');
            s.push_str(INDENT);
            s.push_str("| ");
        }

        s.push_str(name);

        match &variant.fields {
            Fields::Unit => {}
            Fields::Unnamed(unnamed) => {
                let field_tys = unnamed
                    .fields
                    .iter()
                    .filter_map(|field: &specta::datatype::Field| field.ty.as_ref())
                    .collect::<Vec<&DataType>>();

                s.push(' ');
                match field_tys.as_slice() {
                    [] => s.push_str("()"),
                    [ty] => {
                        let mut rendered = String::new();
                        ndt.render(&mut rendered, imports, exporter, types, ty)?;
                        s.push_str(&as_type_argument(&rendered));
                    }
                    elements => {
                        tupl::render_elements(s, imports, exporter, types, ndt, elements)?;
                    }
                }
            }
            Fields::Named(named) => {
                let named = named
                    .fields
                    .iter()
                    .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
                    .collect::<Vec<_>>();

                if named.is_empty() {
                    s.push_str(" {}");
                    continue;
                }

                s.push_str(" { ");
                for (idx, (key, field, ty)) in named.into_iter().enumerate() {
                    if idx != 0 {
                        s.push_str(", ");
                    }
                    fields::render(s, imports, exporter, types, ndt, key.clone(), field, ty)?;
                }
                s.push_str(" }");
            }
        }
    }

    Ok(())
}
