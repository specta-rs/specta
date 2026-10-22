//! Internal wire-format marker keys shared by `specta-serde` and the language
//! exporters.
//!
//! These keys are inserted into a datatype's attribute map to carry a semantic
//! that cannot be recovered from the exported shape alone (for example, whether a
//! generated object wrapper rejects unknown keys), and are read back by the
//! exporters that render the affected types. The string values form an internal
//! wire protocol between the producing and consuming crates, so they must stay
//! byte-for-byte stable and must not be renamed.
//!
//! This module exists so that each key has a single definition instead of being
//! re-declared as a literal in every crate that produces or consumes it
//! (see specta-rs/specta#598). Replacing these ad-hoc string keys with a typed
//! metadata protocol is deliberate follow-up work; until then treat the values as
//! opaque.
//!
//! DO NOT USE THESE OUTSIDE OF SPECTA ITSELF!

/// Marks alias unions that an exporter may preserve symbolically instead of
/// distributing their enclosing intersection into a Cartesian union.
pub const DEFERRED_ALIAS_UNION_MARKER: &str = "specta_serde:deferred_alias_union";

/// Marks optional `never` fields synthesized solely to make alias spellings
/// mutually exclusive. Genuine user-authored uninhabited fields must survive
/// contextual flatten relaxation.
pub const ALIAS_EXCLUSION_MARKER: &str = "specta_serde:alias_exclusion";

/// Marks generated object wrappers whose wire format rejects unknown keys.
/// Exporters can consume this semantic attribute without inferring tagging from
/// a shape that user-authored untagged variants can also have.
pub const STRICT_OBJECT_MARKER: &str = "specta:strict_object";
