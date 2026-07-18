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
//!
//! Resolution is phase-aware: request-side types (bodies and parameters, the shapes the server
//! deserializes) resolve through the deserialize phase, response-side types through the serialize
//! phase. Under [`specta_serde::Format`] the phases are unified and both resolve identically;
//! under [`specta_serde::PhasesFormat`] a type whose phases diverge resolves to its
//! `_Serialize`/`_Deserialize` projection per use — the format is the switch, not a knob here.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use serde_json::Value;
use specta::{
    Format, Types,
    datatype::{DataType, Field, NamedDataType, NamedReference, Reference, Struct},
};
use specta_serde::{Phase, select_phase_datatype};

use crate::{Error, OasVersion, SchemaMode, operation::Operation, transform::components};

/// Name of the probe definition. Carries a prefix no derived type will produce, so it cannot collide
/// with a real definition and change how the real ones are named.
const PROBE: &str = "__specta_openapi_probe";

/// What an operation's types are exported as, per side: a `$ref` to a component, or a schema in
/// place — whichever the exporter emitted. Keyed by the type as the operation declared it; the
/// phase projection is internal.
pub(crate) struct Resolved {
    request: HashMap<DataType, Value>,
    response: HashMap<DataType, Value>,
}

impl Resolved {
    pub(crate) fn request(&self, dt: &DataType) -> Option<&Value> {
        self.request.get(dt)
    }

    pub(crate) fn response(&self, dt: &DataType) -> Option<&Value> {
        self.response.get(dt)
    }
}

/// A format whose collection mapping has already been applied. Mapping runs exactly once however
/// many exports resolution performs; per-datatype mapping still delegates to the real format.
struct Premapped<F>(F);

impl<F: Format> Format for Premapped<F> {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        // The returned borrow is tied to `&self`, not `types`, so identity
        // still clones - the same cost class as the probe collection clone.
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        types: &Types,
        dt: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        self.0.map_type(types, dt)
    }
}

/// Exports `types`, and resolves every type referenced by `operations` per side.
///
/// Returns the components with the probe removed, so the caller exports exactly what it would have
/// without one.
pub(crate) fn resolve(
    types: &Types,
    operations: &[Operation],
    format: impl Format,
    mode: SchemaMode,
    version: OasVersion,
) -> Result<(BTreeMap<String, Value>, Resolved), Error> {
    let mapped = format.map_types(types)?.into_owned();

    // (original declaration, phase projection, Rust name) per side, deduplicated by projection.
    let mut request: Vec<(DataType, DataType)> = Vec::new();
    let mut response: Vec<(DataType, DataType)> = Vec::new();
    let mut probe_fields: Vec<DataType> = Vec::new();
    for operation in operations {
        for (phase, roles, side) in [
            (
                Phase::Deserialize,
                operation.request_types().collect::<Vec<_>>(),
                &mut request,
            ),
            (
                Phase::Serialize,
                operation.response_types().collect::<Vec<_>>(),
                &mut response,
            ),
        ] {
            for (dt, type_name) in roles {
                let selected = select_phase_datatype(dt, &mapped, phase);
                // A named type is exported as a component, so it has to be in the collection being
                // exported. Anything else is written in place and needs no registration.
                if let DataType::Reference(Reference::Named(reference)) = &selected
                    && is_component(reference)
                    && mapped.get(reference).is_none()
                {
                    return Err(Error::UnregisteredOperationType {
                        type_name: type_name.to_string(),
                    });
                }
                if !probe_fields.contains(&selected) {
                    probe_fields.push(selected.clone());
                }
                if !side.iter().any(|(original, _)| original == dt) {
                    side.push((dt.clone(), selected));
                }
            }
        }
    }

    if probe_fields.is_empty() {
        return Ok((
            components(&mapped, Premapped(format), mode, version)?,
            Resolved {
                request: HashMap::new(),
                response: HashMap::new(),
            },
        ));
    }

    let mut probe_types = mapped.clone();
    let mut probe = Struct::named();
    for (index, dt) in probe_fields.iter().enumerate() {
        probe = probe.field(field_name(index), Field::new(dt.clone()));
    }
    let body = probe.build();
    NamedDataType::new(PROBE, &mut probe_types, |_, ndt| {
        ndt.ty = Some(body.clone());
    });

    let mut components = components(&probe_types, Premapped(format), mode, version)?;
    let Some(probe) = components.remove(PROBE) else {
        return Err(Error::UnresolvedOperationTypes);
    };

    let schema_for = |selected: &DataType| -> Result<Value, Error> {
        let index = probe_fields
            .iter()
            .position(|candidate| candidate == selected)
            .ok_or(Error::UnresolvedOperationTypes)?;
        probe
            .get("properties")
            .and_then(|properties| properties.get(field_name(index)))
            .cloned()
            .ok_or(Error::UnresolvedOperationTypes)
    };

    let mut resolved = Resolved {
        request: HashMap::new(),
        response: HashMap::new(),
    };
    for (original, selected) in request {
        let schema = schema_for(&selected)?;
        resolved.request.insert(original, schema);
    }
    for (original, selected) in response {
        let schema = schema_for(&selected)?;
        resolved.response.insert(original, schema);
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
