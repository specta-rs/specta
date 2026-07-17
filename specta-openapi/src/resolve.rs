//! Resolution of an operation's types to the schemas they are exported as.
//!
//! An operation names its types, but how a type is exported is a property of the whole collection
//! rather than of the type: a named type becomes a component whose name is disambiguated by module
//! path when two share a name, with generic arguments folded in and then sanitised, while a
//! primitive or an `#[specta(inline)]` type has no component at all and is written out in place.
//!
//! Rather than reproduce any of that — which would be a copy of the JSON Schema exporter's
//! internals, free to drift from them — this asks the exporter directly. A probe type holding one
//! field per referenced type is exported alongside the real ones, and whatever the exporter emits
//! for each field is the answer: a `$ref` for a named type, an inline schema for anything else. The
//! probe is dropped from the components afterwards.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;
use specta::{
    Format, Types,
    datatype::{DataType, Field, NamedDataType, NamedReference, Reference, Struct},
};

use crate::{Error, SchemaMode, operation::Operation, transform::components};

/// Name of the probe definition. Carries a prefix no derived type will produce, so it cannot collide
/// with a real definition and change how the real ones are named.
const PROBE: &str = "__specta_openapi_probe";

/// What an operation's type is exported as: a `$ref` to a component, or a schema in place —
/// whichever the exporter emitted for it.
pub(crate) type Resolved = HashMap<DataType, Value>;

/// Exports `types`, and resolves every type referenced by `operations`.
///
/// Returns the components with the probe removed, so the caller exports exactly what it would have
/// without one.
pub(crate) fn resolve(
    types: &Types,
    operations: &[Operation],
    format: impl Format,
    mode: SchemaMode,
) -> Result<(BTreeMap<String, Value>, Resolved), Error> {
    let mut referenced: Vec<DataType> = Vec::new();
    for operation in operations {
        for (dt, type_name) in operation.referenced_types() {
            // A named type is exported as a component, so it has to be in the collection being
            // exported. Anything else is written in place and needs no registration.
            if let DataType::Reference(Reference::Named(reference)) = dt
                && is_component(reference)
                && types.get(reference).is_none()
            {
                return Err(Error::UnregisteredOperationType {
                    type_name: type_name.to_string(),
                });
            }
            if !referenced.contains(dt) {
                referenced.push(dt.clone());
            }
        }
    }

    if referenced.is_empty() {
        return Ok((components(types, format, mode)?, HashMap::new()));
    }

    let mut probe_types = types.clone();
    let mut probe = Struct::named();
    for (index, dt) in referenced.iter().enumerate() {
        probe = probe.field(field_name(index), Field::new(dt.clone()));
    }
    let body = probe.build();
    NamedDataType::new(PROBE, &mut probe_types, |_, ndt| {
        ndt.ty = Some(body.clone());
    });

    let mut components = components(&probe_types, format, mode)?;
    let Some(probe) = components.remove(PROBE) else {
        return Err(Error::UnresolvedOperationTypes);
    };

    let mut resolved = Resolved::new();
    for (index, dt) in referenced.into_iter().enumerate() {
        let property = probe
            .get("properties")
            .and_then(|properties| properties.get(field_name(index)))
            .ok_or(Error::UnresolvedOperationTypes)?;
        resolved.insert(dt, property.clone());
    }

    Ok((components, resolved))
}

/// Whether a named reference is exported as a component of its own.
///
/// `#[specta(inline)]` and recursive-inline references are written out at the use site instead, so
/// they never become one.
fn is_component(reference: &NamedReference) -> bool {
    matches!(
        reference.inner,
        specta::datatype::NamedReferenceType::Reference { .. }
    )
}

fn field_name(index: usize) -> String {
    format!("op_{index}")
}
