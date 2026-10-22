//! The internal marker-key wire protocol.
//!
//! Specta carries semantic information that cannot be recovered from an exported
//! shape alone as *marker keys*: opaque string identifiers stored in the
//! [`Attributes`](crate::datatype::Attributes) map of a
//! [`DataType`](crate::datatype::DataType), [`Struct`](crate::datatype::Struct),
//! [`Enum`](crate::datatype::Enum) or [`Field`](crate::datatype::Field). For
//! example, a synthetic object wrapper that rejects unknown keys is
//! indistinguishable from a user-authored struct by shape alone, so
//! `specta-serde` records [`STRICT_OBJECT_MARKER`] on it and the exporters read
//! that back.
//!
//! # Producers and consumers
//!
//! - The [`Type`](crate::Type) derive in `specta-macros` writes every `serde:*`
//!   attribute key (`SERDE_CONTAINER_*`, `SERDE_VARIANT_*`, `SERDE_FIELD_*`)
//!   plus [`TYPE_OVERRIDE_MARKER`], [`SERDE_NEWTYPE_SKIP_IGNORED_MARKER`] and
//!   [`NULLABLE_MARKER`]. The `serde:*` keys are an inert copy of the parsed
//!   `#[serde(...)]` attributes; the derive records them but never interprets
//!   them.
//! - The hand-written `Type` implementations in `specta` itself write
//!   [`SERDE_CONTAINER_UNTAGGED`], [`SERDE_CONTAINER_TAG`] and
//!   [`SERDE_FIELD_FLATTEN`] for built-in types whose serde representation is
//!   fixed (for example `serde_json::Value`), and [`FINITE_NUMBER_MARKER`].
//! - `specta-serde` reads the `serde:*` keys, interprets them, and writes every
//!   `specta_serde:*` marker that describes the rewrite it performed. It also
//!   *writes* [`STRICT_OBJECT_MARKER`] on the wrappers it generates; it never
//!   reads that key back.
//! - The language exporters read the markers back. Each exporter reads the
//!   subset it needs; for example `specta-zod` and `specta-valibot` read
//!   [`STRICT_OBJECT_MARKER`] and [`OPTIONAL_FLATTEN_UNION_MARKER`],
//!   `specta-typescript` reads [`FINITE_NUMBER_MARKER`],
//!   [`DEFERRED_ALIAS_UNION_MARKER`] and [`ALIAS_EXCLUSION_MARKER`],
//!   `specta-jsonschema` reads [`STRICT_OBJECT_MARKER`],
//!   [`ENUM_REPR_REWRITTEN_MARKER`] and [`FLATTENED_PAYLOAD_MARKER`], and
//!   `specta-swift` reads [`SERDE_FIELD_FLATTEN`] and
//!   [`SERIALIZE_OMISSION_MARKER`]. That list is illustrative, not exhaustive.
//!
//! # Rules
//!
//! - **Every key is defined exactly here.** A producer or consumer must
//!   reference these constants instead of repeating the string literal, so a
//!   key has a single source of truth and a rename cannot silently desynchronise
//!   a producer from its consumers.
//! - **The string values are a wire protocol.** They are byte-for-byte stable
//!   and must never be renamed or re-spelled; only the Rust constant names may
//!   change. [`tests/tests/markers.rs`][tests] pins every key and its value so
//!   an accidental edit is caught.
//! - **Exporters must read markers through this module**, never by matching the
//!   literal. The literals are an implementation detail of the internal
//!   contract, not part of the public API.
//!
//! [tests]: https://github.com/specta-rs/specta/blob/main/tests/tests/markers.rs
//!
//! # Why the keys live in core
//!
//! The workspace's `AGENTS.md` asks `specta` and `specta-macros` to stay
//! format-agnostic, yet this module hosts `serde:*` and `specta_serde:*`
//! literals inside core. That deviation is deliberate and forced, not an
//! oversight: the derive has to emit a marker key through a path that resolves
//! from the *user's* crate, and the only crate guaranteed to be reachable there
//! is the one the derive already targets for `#[specta(crate = ...)]` (the
//! `specta` facade). The constants therefore have to be reachable from core no
//! matter which crate consumes them, and giving them a second home would only
//! recreate the duplication this module exists to remove. The format-agnostic
//! rule is restored properly by the typed metadata protocol that replaces this
//! channel (specta-rs/specta#598), not by moving the literals again.
//!
//! # This is not public API
//!
//! This module is `#[doc(hidden)]` and explicitly internal. The names, the
//! values, and the set of keys may change in any release. Specta's format-agnostic
//! core is deliberately not allowed to infer any of these semantics from a
//! datatype's shape: a key exists precisely because the shape is ambiguous.
//!
//! Replacing this string-keyed channel with a typed, format-neutral metadata
//! protocol is deliberate follow-up work (specta-rs/specta#598); until then the
//! values are opaque. This module is not a general-purpose attribute API: it
//! exists only to give the keys that already flow between Specta's own crates a
//! single definition.

// ---------------------------------------------------------------------------
// `serde:*` container attributes
//
// All three `serde:*` sections below are written by the derive -- or, for the
// built-ins whose serde representation is fixed, by a hand-written `Type` impl
// in `specta` (`SERDE_CONTAINER_TAG`, `SERDE_CONTAINER_UNTAGGED` and
// `SERDE_FIELD_FLATTEN`). `specta-serde` interprets them, and also writes a few
// itself while lowering the representation: it rewrites the container renames
// in place, and stamps `SERDE_CONTAINER_DENY_UNKNOWN_FIELDS` onto the synthetic
// payload structs and rewritten enums it generates. Ten of the keys are read
// directly by an exporter as well, and each of those says so on its own item.
// ---------------------------------------------------------------------------

/// `#[serde(rename = "...")]` on a container, serialize direction.
///
/// Read by `specta-rust` (which reproduces the rename on the generated type) as
/// well as by `specta-serde`.
pub const SERDE_CONTAINER_RENAME_SERIALIZE: &str = "serde:container:rename_serialize";
/// `#[serde(rename(deserialize = "..."))]` on a container.
///
/// Read by `specta-rust` as well as by `specta-serde`.
pub const SERDE_CONTAINER_RENAME_DESERIALIZE: &str = "serde:container:rename_deserialize";
/// `#[serde(rename_all = "...")]` on a container, serialize direction.
///
/// Read by `specta-rust` (which reproduces the rule on the generated type) as
/// well as by `specta-serde`.
pub const SERDE_CONTAINER_RENAME_ALL_SERIALIZE: &str = "serde:container:rename_all_serialize";
/// `#[serde(rename_all(deserialize = "..."))]` on a container.
///
/// Read by `specta-rust` as well as by `specta-serde`.
pub const SERDE_CONTAINER_RENAME_ALL_DESERIALIZE: &str = "serde:container:rename_all_deserialize";
/// `#[serde(rename_all_fields = "...")]` on an enum, serialize direction.
pub const SERDE_CONTAINER_RENAME_ALL_FIELDS_SERIALIZE: &str =
    "serde:container:rename_all_fields_serialize";
/// `#[serde(rename_all_fields(deserialize = "..."))]` on an enum.
pub const SERDE_CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE: &str =
    "serde:container:rename_all_fields_deserialize";
/// `#[serde(tag = "...")]` on a container.
///
/// Read by `specta-rust` and `specta-rescript` (to refuse a tagged enum) as
/// well as by `specta-serde`.
pub const SERDE_CONTAINER_TAG: &str = "serde:container:tag";
/// `#[serde(content = "...")]` on an adjacently tagged container.
///
/// Read by `specta-rust` as well as by `specta-serde`.
pub const SERDE_CONTAINER_CONTENT: &str = "serde:container:content";
/// `#[serde(untagged)]` on a container.
///
/// Read by `specta-jsonschema`, `specta-kotlin`, `specta-rescript`,
/// `specta-rust`, `specta-valibot` and `specta-zod` as well as by
/// `specta-serde`.
pub const SERDE_CONTAINER_UNTAGGED: &str = "serde:container:untagged";
/// `#[serde(default)]` on a container.
pub const SERDE_CONTAINER_DEFAULT: &str = "serde:container:default";
/// `#[serde(transparent)]` on a container.
pub const SERDE_CONTAINER_TRANSPARENT: &str = "serde:container:transparent";
/// `#[serde(deny_unknown_fields)]` on a container.
///
/// Read by `specta-jsonschema` (to close the generated object) as well as by
/// `specta-serde`. Distinct from [`STRICT_OBJECT_MARKER`].
pub const SERDE_CONTAINER_DENY_UNKNOWN_FIELDS: &str = "serde:container:deny_unknown_fields";
/// The `type_src` half of `#[serde(from = "...")]` on a container.
pub const SERDE_CONTAINER_FROM_TYPE_SRC: &str = "serde:container:from_type_src";
/// The resolved [`DataType`](crate::datatype::DataType) half of `#[serde(from = "...")]`.
pub const SERDE_CONTAINER_FROM_RESOLVED: &str = "serde:container:from_resolved";
/// The `type_src` half of `#[serde(try_from = "...")]` on a container.
pub const SERDE_CONTAINER_TRY_FROM_TYPE_SRC: &str = "serde:container:try_from_type_src";
/// The resolved [`DataType`](crate::datatype::DataType) half of `#[serde(try_from = "...")]`.
pub const SERDE_CONTAINER_TRY_FROM_RESOLVED: &str = "serde:container:try_from_resolved";
/// The `type_src` half of `#[serde(into = "...")]` on a container.
pub const SERDE_CONTAINER_INTO_TYPE_SRC: &str = "serde:container:into_type_src";
/// The resolved [`DataType`](crate::datatype::DataType) half of `#[serde(into = "...")]`.
pub const SERDE_CONTAINER_INTO_RESOLVED: &str = "serde:container:into_resolved";
/// `#[serde(variant_identifier)]` on a container.
pub const SERDE_CONTAINER_VARIANT_IDENTIFIER: &str = "serde:container:variant_identifier";
/// `#[serde(field_identifier)]` on a container.
pub const SERDE_CONTAINER_FIELD_IDENTIFIER: &str = "serde:container:field_identifier";

// ---------------------------------------------------------------------------
// `serde:*` variant attributes
// ---------------------------------------------------------------------------

/// `#[serde(rename = "...")]` on a variant, serialize direction.
pub const SERDE_VARIANT_RENAME_SERIALIZE: &str = "serde:variant:rename_serialize";
/// `#[serde(rename(deserialize = "..."))]` on a variant.
pub const SERDE_VARIANT_RENAME_DESERIALIZE: &str = "serde:variant:rename_deserialize";
/// `#[serde(alias = "...")]` on a variant, as a list of strings.
pub const SERDE_VARIANT_ALIASES: &str = "serde:variant:aliases";
/// `#[serde(rename_all = "...")]` on a variant, serialize direction.
pub const SERDE_VARIANT_RENAME_ALL_SERIALIZE: &str = "serde:variant:rename_all_serialize";
/// `#[serde(rename_all(deserialize = "..."))]` on a variant.
pub const SERDE_VARIANT_RENAME_ALL_DESERIALIZE: &str = "serde:variant:rename_all_deserialize";
/// `#[serde(skip_serializing)]` on a variant.
pub const SERDE_VARIANT_SKIP_SERIALIZING: &str = "serde:variant:skip_serializing";
/// `#[serde(skip_deserializing)]` on a variant.
pub const SERDE_VARIANT_SKIP_DESERIALIZING: &str = "serde:variant:skip_deserializing";
/// `#[serde(serialize_with = "...")]` on a variant.
pub const SERDE_VARIANT_HAS_SERIALIZE_WITH: &str = "serde:variant:has_serialize_with";
/// `#[serde(deserialize_with = "...")]` on a variant.
pub const SERDE_VARIANT_HAS_DESERIALIZE_WITH: &str = "serde:variant:has_deserialize_with";
/// `#[serde(with = "...")]` on a variant.
pub const SERDE_VARIANT_HAS_WITH: &str = "serde:variant:has_with";
/// `#[serde(other)]` on a variant.
pub const SERDE_VARIANT_OTHER: &str = "serde:variant:other";
/// `#[serde(untagged)]` on a variant.
///
/// Read by `specta-jsonschema`, `specta-valibot` and `specta-zod` as well as by
/// `specta-serde`.
pub const SERDE_VARIANT_UNTAGGED: &str = "serde:variant:untagged";

// ---------------------------------------------------------------------------
// `serde:*` field attributes
// ---------------------------------------------------------------------------

/// `#[serde(rename = "...")]` on a field, serialize direction.
pub const SERDE_FIELD_RENAME_SERIALIZE: &str = "serde:field:rename_serialize";
/// `#[serde(rename(deserialize = "..."))]` on a field.
pub const SERDE_FIELD_RENAME_DESERIALIZE: &str = "serde:field:rename_deserialize";
/// `#[serde(alias = "...")]` on a field, as a list of strings.
pub const SERDE_FIELD_ALIASES: &str = "serde:field:aliases";
/// `#[serde(default)]` on a field.
pub const SERDE_FIELD_DEFAULT: &str = "serde:field:default";
/// `#[serde(flatten)]` on a field.
///
/// Read by `specta-swift` (which refuses a flattened variant field) as well as
/// by `specta-serde`.
pub const SERDE_FIELD_FLATTEN: &str = "serde:field:flatten";
/// `#[serde(skip_serializing)]` on a field.
pub const SERDE_FIELD_SKIP_SERIALIZING: &str = "serde:field:skip_serializing";
/// `#[serde(skip_deserializing)]` on a field.
pub const SERDE_FIELD_SKIP_DESERIALIZING: &str = "serde:field:skip_deserializing";
/// `#[serde(skip_serializing_if = "...")]` on a field.
pub const SERDE_FIELD_SKIP_SERIALIZING_IF: &str = "serde:field:skip_serializing_if";
/// `#[serde(serialize_with = "...")]` on a field.
pub const SERDE_FIELD_HAS_SERIALIZE_WITH: &str = "serde:field:has_serialize_with";
/// `#[serde(deserialize_with = "...")]` on a field.
pub const SERDE_FIELD_HAS_DESERIALIZE_WITH: &str = "serde:field:has_deserialize_with";
/// `#[serde(with = "...")]` on a field.
pub const SERDE_FIELD_HAS_WITH: &str = "serde:field:has_with";

// ---------------------------------------------------------------------------
// `specta:*` markers
//
// Unlike the `serde:*` keys these are not a copy of a serde attribute: they
// record facts about the declared Rust source that the exported shape cannot
// express. They are written by the `Type` derive, by `specta`'s built-in `Type`
// impls, or by `specta-serde`, and read by whichever crate needs the fact.
// ---------------------------------------------------------------------------

/// Marks a generated object wrapper whose wire format rejects unknown keys.
/// Exporters can consume this semantic attribute without inferring tagging from
/// a shape that user-authored untagged variants can also have.
///
/// `specta-serde` sets it on the wrapper it generates for an externally tagged
/// variant. This is not authored metadata -- it describes the tagging
/// representation itself -- so such a wrapper is closed regardless of the
/// exporter's schema policy. Adjacently tagged variants deliberately do not
/// carry it, because serde does accept extra keys alongside `tag`/`content`.
///
/// Written by `specta-serde`; read by `specta-jsonschema`, `specta-zod` and
/// `specta-valibot`, never by `specta-serde` itself.
pub const STRICT_OBJECT_MARKER: &str = "specta:strict_object";

/// Marks a field or variant whose exported shape came from an explicit
/// `#[specta(type = ...)]` override rather than the declared Rust type. It lets
/// `specta-serde` accept a custom `serialize_with`/`deserialize_with`/`with`
/// codec (whose wire shape specta cannot know) without the exporter guessing.
///
/// Written by the derive; read by `specta-serde`.
pub const TYPE_OVERRIDE_MARKER: &str = "specta:type_override";

/// Marks the sole field of a newtype struct whose `#[serde(skip)]` serde
/// ignores: it serializes the bare inner value anyway. The derive keeps the
/// field live and records this marker instead, so the declared newtype arity
/// survives while the skipped-field provenance stays visible to `specta-serde`.
///
/// Written by the derive; read by `specta-serde`.
pub const SERDE_NEWTYPE_SKIP_IGNORED_MARKER: &str = "specta:serde_newtype_skip_ignored";

/// Marks a field whose *declared Rust type* was `Option<T>`.
///
/// Serde's `missing_field` helper special-cases `Option` on deserialize (a
/// missing key yields `None`), which changes what an adjacently tagged newtype
/// variant requires for its `content` key -- both when the sole field is skipped
/// (collapse handling) and when it is live. Never inferred from the exported
/// datatype: a `#[specta(type = Option<...>)]` override produces a
/// [`Nullable`](crate::datatype::DataType::Nullable) that serde knows nothing
/// about.
///
/// Written exclusively by the derive from the real field syntax; read by
/// `specta-serde`.
pub const NULLABLE_MARKER: &str = "specta:nullable";

/// Marks a built-in number type that is finite on the wire, so an exporter can
/// collapse a `f64`/`i64` union to a plain number without inheriting the
/// nullability of an arbitrary `f64`.
///
/// Written by `specta`'s built-in `Type` impl for `serde_json::Number`; read by
/// `specta-typescript`, never by `specta-serde`.
pub const FINITE_NUMBER_MARKER: &str = "specta:finite_number";

// ---------------------------------------------------------------------------
// `specta_serde:*` markers
//
// Written by `specta-serde` after it rewrites a type. Most are read back by one
// or more language exporters; a few only round-trip inside `specta-serde`
// itself, and those say so below.
// ---------------------------------------------------------------------------

/// Marks alias unions that an exporter may preserve symbolically instead of
/// distributing their enclosing intersection into a Cartesian union.
///
/// Written and removed by `specta-serde`; read by `specta-typescript`.
pub const DEFERRED_ALIAS_UNION_MARKER: &str = "specta_serde:deferred_alias_union";

/// Marks optional `never` fields synthesized solely to make alias spellings
/// mutually exclusive. Genuine user-authored uninhabited fields must survive
/// contextual flatten relaxation.
///
/// Written by `specta-serde`; read by `specta-serde` and `specta-typescript`.
pub const ALIAS_EXCLUSION_MARKER: &str = "specta_serde:alias_exclusion";

/// Marks an [`Enum`](crate::datatype::Enum) once it is in its final serde
/// representation, so a later rewrite pass (for example `specta-serde`'s
/// `PhasesFormat` second pass over split generated types) does not transform it
/// again. Set by every synthetic-enum builder whose output is already final.
///
/// This replaces a former shape-sniffing heuristic that tried to detect
/// "already rewritten" enums by inspecting variant shapes. That heuristic was
/// unsound: every shape the transform produces is also a valid shape a user can
/// author directly.
///
/// Written by `specta-serde`; read by `specta-serde` itself and, among the
/// exporters, only by `specta-rust`, `specta-swift`, `specta-jsonschema`,
/// `specta-zod`, `specta-valibot`, `specta-kotlin`, `specta-java` and
/// `specta-csharp`. Exporters that do not need to know the representation was
/// rewritten (for example `specta-rescript`, `specta-go` and `specta-python`)
/// do not read it.
pub const ENUM_REPR_REWRITTEN_MARKER: &str = "specta_serde:enum_repr_rewritten";

/// Marks an external variant whose serde payload was hidden from Specta before
/// the enum was rewritten. The generated string-literal shape is otherwise
/// indistinguishable from a genuine unit variant.
///
/// Only read back by `specta-serde` itself; no exporter consults it.
pub const HIDDEN_EXTERNAL_PAYLOAD_MARKER: &str = "specta_serde:hidden_external_payload";

/// Marks a rewritten deserialize-only `#[serde(other)]` branch. It lets an
/// exporter distinguish the widened catch-all from authored variant-level
/// untagged branches with an equally broad string field.
///
/// Written by `specta-serde`; read by `specta-jsonschema`.
pub const ENUM_OTHER_VARIANT_MARKER: &str = "specta_serde:variant_other";

/// Marks the synthetic present/absent union produced for optional flattened
/// payloads, so exporters can reject malformed values that use payload keys.
///
/// Written by `specta-serde`; read by `specta-zod` and `specta-valibot`.
pub const OPTIONAL_FLATTEN_UNION_MARKER: &str = "specta_serde:optional_flatten_union";

/// Marks the payload structs generated for a variant when skipped `ty: None`
/// slots are kept to preserve the variant's declared tuple arity (so a
/// skip-reduced single live element still renders as a sequence). Later rewrite
/// passes honor it by keeping those slots instead of applying the default
/// live-fields-only retain for unnamed structs.
///
/// Only read back by `specta-serde` itself; no exporter consults it.
pub const PRESERVED_ARITY_PAYLOAD_MARKER: &str = "specta_serde:preserved_arity_payload";

/// Marks flattened fields whose payload may be absent from serialization.
/// [`Field::optional`](crate::datatype::Field) cannot carry this distinction
/// because serde defaults can also set it, while defaults do not make flattened
/// payloads optional.
///
/// Only read back by `specta-serde` itself; no exporter consults it.
pub const CONDITIONAL_OMISSION_MARKER: &str = "specta_serde:conditional_omission";

/// Marks a field that `#[serde(skip_serializing_if = ...)]` may omit from a
/// serialized sequence. The raw parser attribute is stripped during the phase
/// rewrite (a phase-split field already carries the decision in
/// [`Field::optional`](crate::datatype::Field)), so an exporter reads this
/// semantic marker to tell the omission apart from a `#[serde(default)]`-optional
/// field, which serde still writes.
///
/// Written by `specta-serde`; read by `specta-swift`.
pub const SERIALIZE_OMISSION_MARKER: &str = "specta_serde:serialize_omission";

/// Marks a union synthesized for a set of `#[serde(alias = "...")]` spellings.
///
/// Only read back by `specta-serde` itself; no exporter consults it.
pub const ALIAS_UNION_MARKER: &str = "specta_serde:alias_union";

/// Marks a struct that `specta-serde` inlined as a flattened payload. Serde
/// ignores an inner struct's `deny_unknown_fields` when it is flattened, so a
/// marked part must not close the merged object.
///
/// Written by `specta-serde`; read by `specta-jsonschema`.
pub const FLATTENED_PAYLOAD_MARKER: &str = "specta_serde:flattened_payload";
