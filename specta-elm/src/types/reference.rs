//! Rendering of references to other named types.

use specta::{
    Types,
    datatype::{NamedReferenceType, Reference},
};

use crate::{Error, Exporter, module::ModuleImports, types::Ndt};

pub(crate) fn render<E: Exporter>(
    s: &mut String,
    imports: &mut ModuleImports,
    exporter: &E,
    types: &Types,
    reference: &Reference,
    ndt: &Ndt,
) -> Result<(), Error> {
    match reference {
        Reference::Named(reference) => match &reference.inner {
            NamedReferenceType::Reference { generics, .. } => {
                if !generics.is_empty() {
                    return Err(ndt.unsupported("generic types are not supported by Elm"));
                }

                s.push_str(&exporter.format_reference(reference, types)?);
            }
            NamedReferenceType::Inline { dt, .. } => {
                ndt.render(s, imports, exporter, types, dt)?;
            }
            NamedReferenceType::Recursive(cycle) => {
                return Err(Error::infinite_recursive_inline_type(
                    ndt.rust_type_path(),
                    format!("{reference:?}"),
                    cycle.clone(),
                ));
            }
        },
        Reference::Opaque(reference) => {
            return Err(Error::unsupported(
                ndt.rust_type_path(),
                format!(
                    "opaque reference '{}' has no Elm representation",
                    reference.type_name()
                ),
            ));
        }
    };

    Ok(())
}
