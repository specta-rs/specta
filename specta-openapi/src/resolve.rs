//! Resolution of an operation's types to the components they are exported under.
//!
//! An operation names its bodies by type, but the component a type lands in is a property of the
//! whole collection rather than of the type: definitions are disambiguated by module path when two
//! share a name, generic instantiations fold their arguments into the name, and the result is then
//! sanitised into a legal component name.
//!
//! Rather than reproduce those rules — which would be a copy of the JSON Schema exporter's
//! internals, free to drift from them — this asks the exporter directly. A probe type holding one
//! field per referenced type is exported alongside the real ones, and the `$ref` the exporter emits
//! for each field is the answer. The probe is dropped from the components afterwards.

use std::collections::HashMap;

use openapiv3::{Components, ReferenceOr};
use specta::{
    Format, Types,
    datatype::{DataType, Field, NamedDataType, NamedReference, Reference, Struct},
};

use crate::{Error, SchemaMode, operation::Operation, transform::components};

/// Name of the probe definition. Carries a prefix no derived type will produce, so it cannot
/// collide with a real definition and change how the real ones are named.
const PROBE: &str = "__specta_openapi_probe";

/// Exports `types`, and resolves every type referenced by `operations` to its component.
///
/// Returns the components with the probe removed, so the caller exports exactly what it would have
/// without one.
pub(crate) fn resolve(
    types: &Types,
    operations: &[Operation],
    format: impl Format,
    mode: SchemaMode,
) -> Result<(Components, HashMap<NamedReference, String>), Error> {
    let mut referenced: Vec<NamedReference> = Vec::new();
    for operation in operations {
        for body in operation.bodies() {
            if !types.get(&body.reference).is_some() {
                return Err(Error::UnregisteredOperationType {
                    type_name: body.type_name.to_string(),
                });
            }
            if !referenced.contains(&body.reference) {
                referenced.push(body.reference.clone());
            }
        }
    }

    if referenced.is_empty() {
        return Ok((components(types, format, mode)?, HashMap::new()));
    }

    let mut probe_types = types.clone();
    let mut probe = Struct::named();
    for (index, reference) in referenced.iter().enumerate() {
        probe = probe.field(
            field_name(index),
            Field::new(DataType::Reference(Reference::Named(reference.clone()))),
        );
    }
    let body = probe.build();
    NamedDataType::new(PROBE, &mut probe_types, |_, ndt| {
        ndt.ty = Some(body.clone());
    });

    let mut components = components(&probe_types, format, mode)?;
    let Some(ReferenceOr::Item(schema)) = components.schemas.shift_remove(PROBE) else {
        return Err(Error::UnresolvedOperationTypes);
    };

    let probe = serde_json::to_value(&schema)?;
    let mut resolved = HashMap::new();
    for (index, reference) in referenced.into_iter().enumerate() {
        let component = probe
            .get("properties")
            .and_then(|properties| properties.get(field_name(index)))
            .and_then(|property| property.get("$ref"))
            .and_then(|reference| reference.as_str())
            .ok_or(Error::UnresolvedOperationTypes)?;
        resolved.insert(reference, component.to_string());
    }

    Ok((components, resolved))
}

fn field_name(index: usize) -> String {
    format!("op_{index}")
}
