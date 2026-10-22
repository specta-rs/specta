//! Pins the string values of the internal marker keys.
//!
//! The keys in [`specta::markers`] are a hidden wire protocol between the
//! `Type` derive, `specta-serde` and the language exporters. Renaming a Rust
//! constant is harmless; changing its *string value* silently desynchronises a
//! producer from its consumers (a datatype would carry a marker nobody reads,
//! or an exporter would look for a marker nobody writes).
//!
//! This is the deliberate exception to the rule that no marker literal appears
//! outside `specta::markers`: it asserts the exact bytes so an accidental edit
//! is caught by the test suite rather than by a downstream exporter at runtime.

use specta::markers::*;

#[test]
fn marker_key_values_are_stable() {
    let table: &[(&str, &str)] = &[
        // `serde:*` container attributes.
        (
            SERDE_CONTAINER_RENAME_SERIALIZE,
            "serde:container:rename_serialize",
        ),
        (
            SERDE_CONTAINER_RENAME_DESERIALIZE,
            "serde:container:rename_deserialize",
        ),
        (
            SERDE_CONTAINER_RENAME_ALL_SERIALIZE,
            "serde:container:rename_all_serialize",
        ),
        (
            SERDE_CONTAINER_RENAME_ALL_DESERIALIZE,
            "serde:container:rename_all_deserialize",
        ),
        (
            SERDE_CONTAINER_RENAME_ALL_FIELDS_SERIALIZE,
            "serde:container:rename_all_fields_serialize",
        ),
        (
            SERDE_CONTAINER_RENAME_ALL_FIELDS_DESERIALIZE,
            "serde:container:rename_all_fields_deserialize",
        ),
        (SERDE_CONTAINER_TAG, "serde:container:tag"),
        (SERDE_CONTAINER_CONTENT, "serde:container:content"),
        (SERDE_CONTAINER_UNTAGGED, "serde:container:untagged"),
        (SERDE_CONTAINER_DEFAULT, "serde:container:default"),
        (SERDE_CONTAINER_TRANSPARENT, "serde:container:transparent"),
        (
            SERDE_CONTAINER_DENY_UNKNOWN_FIELDS,
            "serde:container:deny_unknown_fields",
        ),
        (
            SERDE_CONTAINER_FROM_TYPE_SRC,
            "serde:container:from_type_src",
        ),
        (
            SERDE_CONTAINER_FROM_RESOLVED,
            "serde:container:from_resolved",
        ),
        (
            SERDE_CONTAINER_TRY_FROM_TYPE_SRC,
            "serde:container:try_from_type_src",
        ),
        (
            SERDE_CONTAINER_TRY_FROM_RESOLVED,
            "serde:container:try_from_resolved",
        ),
        (
            SERDE_CONTAINER_INTO_TYPE_SRC,
            "serde:container:into_type_src",
        ),
        (
            SERDE_CONTAINER_INTO_RESOLVED,
            "serde:container:into_resolved",
        ),
        (
            SERDE_CONTAINER_VARIANT_IDENTIFIER,
            "serde:container:variant_identifier",
        ),
        (
            SERDE_CONTAINER_FIELD_IDENTIFIER,
            "serde:container:field_identifier",
        ),
        // `serde:*` variant attributes.
        (
            SERDE_VARIANT_RENAME_SERIALIZE,
            "serde:variant:rename_serialize",
        ),
        (
            SERDE_VARIANT_RENAME_DESERIALIZE,
            "serde:variant:rename_deserialize",
        ),
        (SERDE_VARIANT_ALIASES, "serde:variant:aliases"),
        (
            SERDE_VARIANT_RENAME_ALL_SERIALIZE,
            "serde:variant:rename_all_serialize",
        ),
        (
            SERDE_VARIANT_RENAME_ALL_DESERIALIZE,
            "serde:variant:rename_all_deserialize",
        ),
        (
            SERDE_VARIANT_SKIP_SERIALIZING,
            "serde:variant:skip_serializing",
        ),
        (
            SERDE_VARIANT_SKIP_DESERIALIZING,
            "serde:variant:skip_deserializing",
        ),
        (
            SERDE_VARIANT_HAS_SERIALIZE_WITH,
            "serde:variant:has_serialize_with",
        ),
        (
            SERDE_VARIANT_HAS_DESERIALIZE_WITH,
            "serde:variant:has_deserialize_with",
        ),
        (SERDE_VARIANT_HAS_WITH, "serde:variant:has_with"),
        (SERDE_VARIANT_OTHER, "serde:variant:other"),
        (SERDE_VARIANT_UNTAGGED, "serde:variant:untagged"),
        // `serde:*` field attributes.
        (SERDE_FIELD_RENAME_SERIALIZE, "serde:field:rename_serialize"),
        (
            SERDE_FIELD_RENAME_DESERIALIZE,
            "serde:field:rename_deserialize",
        ),
        (SERDE_FIELD_ALIASES, "serde:field:aliases"),
        (SERDE_FIELD_DEFAULT, "serde:field:default"),
        (SERDE_FIELD_FLATTEN, "serde:field:flatten"),
        (SERDE_FIELD_SKIP_SERIALIZING, "serde:field:skip_serializing"),
        (
            SERDE_FIELD_SKIP_DESERIALIZING,
            "serde:field:skip_deserializing",
        ),
        (
            SERDE_FIELD_SKIP_SERIALIZING_IF,
            "serde:field:skip_serializing_if",
        ),
        (
            SERDE_FIELD_HAS_SERIALIZE_WITH,
            "serde:field:has_serialize_with",
        ),
        (
            SERDE_FIELD_HAS_DESERIALIZE_WITH,
            "serde:field:has_deserialize_with",
        ),
        (SERDE_FIELD_HAS_WITH, "serde:field:has_with"),
        // `specta:*` markers.
        (STRICT_OBJECT_MARKER, "specta:strict_object"),
        (TYPE_OVERRIDE_MARKER, "specta:type_override"),
        (
            SERDE_NEWTYPE_SKIP_IGNORED_MARKER,
            "specta:serde_newtype_skip_ignored",
        ),
        (NULLABLE_MARKER, "specta:nullable"),
        (FINITE_NUMBER_MARKER, "specta:finite_number"),
        // `specta_serde:*` markers.
        (
            DEFERRED_ALIAS_UNION_MARKER,
            "specta_serde:deferred_alias_union",
        ),
        (ALIAS_EXCLUSION_MARKER, "specta_serde:alias_exclusion"),
        (
            ENUM_REPR_REWRITTEN_MARKER,
            "specta_serde:enum_repr_rewritten",
        ),
        (
            HIDDEN_EXTERNAL_PAYLOAD_MARKER,
            "specta_serde:hidden_external_payload",
        ),
        (ENUM_OTHER_VARIANT_MARKER, "specta_serde:variant_other"),
        (
            OPTIONAL_FLATTEN_UNION_MARKER,
            "specta_serde:optional_flatten_union",
        ),
        (
            PRESERVED_ARITY_PAYLOAD_MARKER,
            "specta_serde:preserved_arity_payload",
        ),
        (
            CONDITIONAL_OMISSION_MARKER,
            "specta_serde:conditional_omission",
        ),
        (SERIALIZE_OMISSION_MARKER, "specta_serde:serialize_omission"),
        (ALIAS_UNION_MARKER, "specta_serde:alias_union"),
        (FLATTENED_PAYLOAD_MARKER, "specta_serde:flattened_payload"),
    ];

    for (actual, expected) in table {
        assert_eq!(
            actual, expected,
            "marker key `{expected}` changed value; this is an internal wire protocol"
        );
    }
}
