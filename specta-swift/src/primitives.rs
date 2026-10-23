//! Primitive type conversion from Rust to Swift.

use specta::{
    Format, Types,
    datatype::{
        DataType, Enum, Field, Fields, Generic, NamedFields, NamedReferenceType, Primitive,
        Reference, Struct, Variant,
    },
};

use crate::error::Error;
use crate::swift::Swift;

fn string_literal_raw_value(dt: &DataType) -> Option<&str> {
    let DataType::Enum(literal_enum) = dt else {
        return None;
    };

    let [(raw_value, literal_variant)] = literal_enum.variants.as_slice() else {
        return None;
    };

    match &literal_variant.fields {
        Fields::Unit => Some(raw_value.as_ref()),
        Fields::Unnamed(fields) => {
            let [field] = fields.fields.as_slice() else {
                return None;
            };

            string_literal_raw_value(field.ty.as_ref()?)
        }
        Fields::Named(fields) => {
            let [(_, field)] = fields.fields.as_slice() else {
                return None;
            };

            string_literal_raw_value(field.ty.as_ref()?)
        }
    }
}

fn enum_string_raw_value(variant: &Variant) -> Option<&str> {
    let payload = match &variant.fields {
        Fields::Unnamed(fields) => {
            let [field] = fields.fields.as_slice() else {
                return None;
            };

            field.ty.as_ref()?
        }
        Fields::Named(fields) => {
            let [(_, field)] = fields.fields.as_slice() else {
                return None;
            };

            field.ty.as_ref()?
        }
        Fields::Unit => return None,
    };

    string_literal_raw_value(payload)
}

fn resolved_string_enum(e: &Enum) -> Option<Vec<(&str, &str)>> {
    e.variants
        .iter()
        .map(|(variant_name, variant)| {
            enum_string_raw_value(variant).map(|raw| (variant_name.as_ref(), raw))
        })
        .collect()
}

fn serde_variant_payload<'a>(variant_name: &str, variant: &'a Variant) -> Option<&'a DataType> {
    let Fields::Named(fields) = &variant.fields else {
        return None;
    };

    let [(payload_name, payload_field)] = fields.fields.as_slice() else {
        return None;
    };

    payload_name
        .as_ref()
        .eq_ignore_ascii_case(variant_name)
        .then_some(payload_field.ty.as_ref())
        .flatten()
}

fn self_named_struct_payload<'a>(variant_name: &str, dt: &'a DataType) -> Option<&'a DataType> {
    let DataType::Struct(strct) = dt else {
        return None;
    };

    let Fields::Named(fields) = &strct.fields else {
        return None;
    };

    let [(field_name, field)] = fields.fields.as_slice() else {
        return None;
    };

    field_name
        .as_ref()
        .eq_ignore_ascii_case(variant_name)
        .then_some(field.ty.as_ref())
        .flatten()
}

fn normalized_payload<'a>(variant_name: &str, payload: &'a DataType) -> &'a DataType {
    let mut current = payload;

    while let Some(inner) = self_named_struct_payload(variant_name, current) {
        current = inner;
    }

    current
}

/// Marker `specta-serde` leaves on the single-variant unit enums it synthesises
/// for serde tags (and on the enums it has rewritten).
const ENUM_REPR_REWRITTEN: &str = "specta_serde:enum_repr_rewritten";

/// Field attribute recorded by the derive macro for `#[serde(flatten)]`.
const FLATTENED_FIELD: &str = "serde:field:flatten";

/// Field marker `specta-serde` records for `#[serde(skip_serializing_if)]`,
/// which lets serde omit an element from a tuple struct's sequence. It is kept
/// after the raw parser attribute is stripped so the omission can be told apart
/// from a `#[serde(default)]`-optional element, which serde still writes.
const SERIALIZE_OMISSION_FIELD: &str = "specta_serde:serialize_omission";

/// Field attribute recorded by the derive macro for `#[serde(default)]`.
/// `specta-serde` sets `Field::optional` for it, but so does
/// `#[serde(skip_serializing_if)]`, which serde does *not* tolerate an absent
/// key for on its own -- so the attribute itself is what distinguishes a
/// genuinely absent-tolerant field.
const DEFAULTED_FIELD: &str = "serde:field:default";

/// Container attribute for `#[serde(default)]` on a struct: serde applies the
/// container's `Default` to every absent field, so each one is absent-tolerant.
const DEFAULTED_CONTAINER: &str = "serde:container:default";

/// Field attribute the `Type` derive records when the field was *declared* as
/// `Option<T>`. It survives a `#[specta(type = ...)]` override, which changes
/// the exported datatype (and its nullability) but not serde's behaviour: serde
/// still treats the field as an `Option`, writing `null` and tolerating an
/// absent key. It is deliberately not inferred from the exported datatype,
/// which a nullable override can make `Nullable` for a non-`Option` field.
const NULLABLE_FIELD: &str = "specta:nullable";

/// A serde tag that `specta-serde` baked into a rewritten enum variant.
struct VariantTag<'a> {
    /// The wire key carrying the discriminant (serde's `tag`).
    key: &'a str,
    /// The string the discriminant must carry (the variant's serialized name).
    value: &'a str,
}

/// The fixed string a rewritten tag field carries, when the datatype is the
/// single-variant unit enum `specta-serde` synthesises for a serde tag.
///
/// The `enum_repr_rewritten` marker is required so a user-authored
/// single-variant unit enum (e.g. an inlined field type) is never mistaken for
/// a tag.
fn tag_literal_value(dt: &DataType) -> Option<&str> {
    let DataType::Enum(enm) = dt else {
        return None;
    };

    if !enm.attributes.contains_key(ENUM_REPR_REWRITTEN) {
        return None;
    }

    let [(value, variant)] = enm.variants.as_slice() else {
        return None;
    };

    matches!(variant.fields, Fields::Unit).then_some(value.as_ref())
}

/// Recover the serde tag from an internally or adjacently tagged variant.
///
/// `specta-serde` lowers both representations into a named field whose type is
/// a single-variant unit enum (a string literal). An externally tagged variant
/// keys its payload by the variant's own name instead, so a string-literal field
/// can only be a tag.
fn variant_tag<'a>(variant: &'a Variant) -> Option<VariantTag<'a>> {
    let Fields::Named(fields) = &variant.fields else {
        return None;
    };

    let mut tags = fields.fields.iter().filter_map(|(name, field)| {
        Some(VariantTag {
            key: name.as_ref(),
            value: tag_literal_value(field.ty.as_ref()?)?,
        })
    });

    let tag = tags.next()?;
    tags.next().is_none().then_some(tag)
}

/// A tagged variant's payload once its tag field is removed.
struct TaggedPayload<'a> {
    /// The payload fields, in declaration order.
    fields: Vec<(&'a str, &'a Field)>,
    /// The wire key the payload nests under when it is an adjacent variant's
    /// inline struct content.
    content_key: Option<&'a str>,
}

/// Split a tagged variant into its payload fields, dropping the tag.
fn tagged_payload<'a>(variant: &'a Variant, tag_key: &str) -> TaggedPayload<'a> {
    let Fields::Named(fields) = &variant.fields else {
        return TaggedPayload {
            fields: Vec::new(),
            content_key: None,
        };
    };

    let payload: Vec<(&str, &Field)> = fields
        .fields
        .iter()
        .filter(|(name, _)| name.as_ref() != tag_key)
        .map(|(name, field)| (name.as_ref(), field))
        .collect();

    // An adjacently tagged struct variant carries its payload as one inline
    // struct under the `content` key. The exporter has no name for that struct,
    // so hoist its fields into the wrapper and re-nest them on the wire.
    if let [(key, field)] = payload.as_slice()
        && let Some(DataType::Struct(strct)) = field.ty.as_ref()
        && let Fields::Named(named) = &strct.fields
    {
        return TaggedPayload {
            fields: named
                .fields
                .iter()
                .map(|(name, field)| (name.as_ref(), field))
                .collect(),
            content_key: Some(key),
        };
    }

    TaggedPayload {
        fields: payload,
        content_key: None,
    }
}

/// The Swift name of the struct generated to wrap a variant's named payload.
fn variant_wrapper_name(enum_name: &str, variant_name: &str) -> String {
    format!("{}{}Data", enum_name, to_pascal_case(variant_name))
}

/// The named fields a variant's generated wrapper struct holds.
///
/// Returns `None` when the variant is not represented by a wrapper struct (its
/// case carries the payload type directly, or it is a unit case).
fn wrapper_fields<'a>(
    variant_name: &str,
    variant: &'a Variant,
) -> Option<Vec<(&'a str, &'a Field)>> {
    if let Some(tag) = variant_tag(variant) {
        let payload = tagged_payload(variant, tag.key);
        return (payload.content_key.is_some() || !payload.fields.is_empty())
            .then_some(payload.fields);
    }

    let Fields::Named(fields) = &variant.fields else {
        return None;
    };

    if fields.fields.is_empty() {
        return None;
    }

    let wrapper = match serde_variant_payload(variant_name, variant) {
        Some(payload) => {
            let DataType::Struct(strct) = normalized_payload(variant_name, payload) else {
                return None;
            };
            let Fields::Named(named) = &strct.fields else {
                return None;
            };
            named
                .fields
                .iter()
                .map(|(name, field)| (name.as_ref(), field))
                .collect::<Vec<_>>()
        }
        None => fields
            .fields
            .iter()
            .map(|(name, field)| (name.as_ref(), field))
            .collect(),
    };

    (!wrapper.is_empty()).then_some(wrapper)
}

/// Whether a variant serializes as a bare string under external tagging, which
/// serde does for unit variants.
fn is_external_unit_variant(variant_name: &str, variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) => {
            fields.fields.len() == 1
                && fields.fields[0]
                    .ty
                    .as_ref()
                    .is_some_and(|ty| is_unit_payload(variant_name, ty))
        }
        Fields::Named(_) => false,
    }
}

/// Whether a payload field may be absent from the wire.
///
/// `specta-serde` only sets `Field::optional` for `#[serde(default)]`; a plain
/// `Option<T>` is a `Nullable` datatype, and serde deserializes a missing key
/// for one to `None`.
fn field_is_optional(field: &Field) -> bool {
    field.optional || matches!(field.ty.as_ref(), Some(DataType::Nullable(_)))
}

/// The `?` a generated property needs when a field tolerates absence but its
/// datatype is not already nullable (a `Nullable` renders as `T?` on its own,
/// so adding the marker would produce `T??`).
fn optional_suffix(field: &Field, ty: &DataType) -> &'static str {
    if field.optional && !matches!(ty, DataType::Nullable(_)) {
        "?"
    } else {
        ""
    }
}

/// The Swift type to decode a field as, unwrapping a `Nullable` so
/// `decodeIfPresent` produces the same optional the generated property expects.
fn field_decode_type(field: &Field) -> Option<&DataType> {
    match field.ty.as_ref()? {
        DataType::Nullable(inner) => Some(inner),
        ty => Some(ty),
    }
}

/// Whether a payload renders to nothing (`Void`, or an empty string), which no
/// `Codable` can carry.
fn payload_renders_empty(ty: &DataType) -> bool {
    match ty {
        DataType::Tuple(tuple) => tuple.elements.is_empty(),
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit => true,
            Fields::Unnamed(fields) => fields.fields.iter().all(|field| field.ty.is_none()),
            Fields::Named(fields) => fields.fields.iter().all(|(_, field)| field.ty.is_none()),
        },
        _ => false,
    }
}

/// Whether a payload datatype renders as something Swift's `Codable` cannot
/// carry: a Swift tuple, an unnamed struct (a tuple payload with an optional
/// element), or an empty payload.
///
/// An inlined reference (`#[specta(inline)]`) splices its target's body into a
/// type position, so it is checked as the type it renders as.
fn unsupported_payload(ty: &DataType) -> bool {
    match ty {
        DataType::Tuple(_) => true,
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit => true,
            Fields::Unnamed(_) => true,
            Fields::Named(fields) => fields.fields.iter().all(|(_, field)| field.ty.is_none()),
        },
        // `reference_to_swift` only splices an inline reference when it is not
        // an enum and not recursive; mirror that so a recursive inline
        // reference (rendered as the named type) is not rejected.
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. }
                if !matches!(dt.as_ref(), DataType::Enum(_))
                    && !contains_recursive_reference(dt) =>
            {
                unsupported_payload(dt)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Whether a generated wrapper field's datatype is one Swift's `Codable` cannot
/// carry.
fn unsupported_payload_field(field: &Field) -> bool {
    field.ty.as_ref().is_some_and(unsupported_payload)
}

/// The element types of a payload serde writes as a JSON array (a tuple
/// variant).
///
/// `specta-serde` lowers a tuple variant to a `Tuple`, or to a preserved-arity
/// unnamed struct once an element is optional; only the former is supported.
fn tuple_payload_elements<'a>(
    variant_name: &str,
    variant: &'a Variant,
) -> Option<Vec<&'a DataType>> {
    let payload = match &variant.fields {
        Fields::Unnamed(fields) if fields.fields.len() > 1 => {
            return Some(
                fields
                    .fields
                    .iter()
                    .filter_map(|field| field.ty.as_ref())
                    .collect(),
            );
        }
        Fields::Named(_) => {
            normalized_payload(variant_name, serde_variant_payload(variant_name, variant)?)
        }
        Fields::Unit | Fields::Unnamed(_) => return None,
    };

    match payload {
        DataType::Tuple(tuple) if !tuple.elements.is_empty() => {
            Some(tuple.elements.iter().collect())
        }
        _ => None,
    }
}

/// Render a Rust string as a Swift string literal.
fn swift_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn is_unit_payload(variant_name: &str, dt: &DataType) -> bool {
    let dt = normalized_payload(variant_name, dt);

    if string_literal_raw_value(dt).is_some() {
        return true;
    }

    let DataType::Enum(enm) = dt else {
        return false;
    };

    let [(_, variant)] = enm.variants.as_slice() else {
        return false;
    };

    match &variant.fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) => fields.fields.is_empty(),
        Fields::Named(fields) => fields.fields.is_empty(),
    }
}

fn enum_payload_to_swift_type(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    variant_name: &str,
    payload: &DataType,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    let payload = normalized_payload(variant_name, payload);

    // An empty payload renders as `Void`/nothing, and a preserved-arity unnamed
    // struct renders as a struct body; neither is a Swift type a `Codable` can
    // carry.
    if payload_renders_empty(payload)
        || matches!(payload, DataType::Struct(strct) if matches!(&strct.fields, Fields::Unnamed(_)))
    {
        return Err(Error::UnsupportedType(format!(
            "variant `{variant_name}` has a payload that Swift's `Codable` cannot encode"
        )));
    }

    Ok(match payload {
        DataType::Tuple(tuple) if tuple.elements.len() > 1 => tuple
            .elements
            .iter()
            .map(|element| datatype_to_swift(swift, format, types, element, generic_scope.to_vec()))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", "),
        _ => datatype_to_swift(swift, format, types, payload, generic_scope.to_vec())?,
    })
}

fn should_emit_variant_wrapper(variant_name: &str, variant: &Variant) -> bool {
    let Fields::Named(fields) = &variant.fields else {
        return false;
    };

    if fields.fields.is_empty() {
        return false;
    }

    let Some(payload) = serde_variant_payload(variant_name, variant) else {
        return true;
    };

    let payload = normalized_payload(variant_name, payload);

    matches!(
        payload,
        DataType::Struct(strct)
            if matches!(
                &strct.fields,
                Fields::Named(named) if !named.fields.is_empty()
            )
    )
}

fn is_unit_like_variant(variant_name: &str, variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) => {
            fields.fields.is_empty()
                || (fields.fields.len() == 1
                    && fields.fields[0]
                        .ty
                        .as_ref()
                        .is_some_and(|ty| is_unit_payload(variant_name, ty)))
        }
        Fields::Named(fields) => {
            fields.fields.is_empty()
                || serde_variant_payload(variant_name, variant)
                    .is_some_and(|payload| is_unit_payload(variant_name, payload))
        }
    }
}

/// Export a single type to Swift.
pub fn export_type(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    ndt: &specta::datatype::NamedDataType,
) -> Result<String, Error> {
    // A named tuple definition (e.g. produced by specta-serde when
    // `#[serde(skip)]` reduces a tuple struct to one live field) renders
    // exactly like a tuple struct with the same elements.
    let tuple_as_struct = match &ndt.ty {
        Some(DataType::Tuple(tuple)) => {
            let mut builder = Struct::unnamed();
            for element in &tuple.elements {
                builder = builder.field(Field::new(element.clone()));
            }
            Some(builder.build())
        }
        _ => None,
    };
    let ndt_ty = tuple_as_struct.as_ref().or(ndt.ty.as_ref());

    if !matches!(ndt_ty, Some(DataType::Struct(_) | DataType::Enum(_))) {
        return Ok(String::new());
    }
    let mut result = String::new();

    // Add JSDoc-style comments if present
    if !ndt.docs.is_empty() {
        let docs = &ndt.docs;
        // Handle multi-line comments properly
        for line in docs.lines() {
            result.push_str("/// ");
            // Trim leading whitespace from the line to avoid extra spaces
            result.push_str(line.trim_start());
            result.push('\n');
        }
    }

    // Add deprecated annotation if present
    if let Some(deprecated) = ndt.deprecated.as_ref() {
        let message = deprecated
            .note
            .as_deref()
            .filter(|note| !note.trim().is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| "This type is deprecated".to_string());
        result.push_str(&format!(
            "@available(*, deprecated, message: \"{}\")\n",
            message
        ));
    }

    let generic_scope = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();

    // Format based on type
    match ndt_ty.expect("checked above") {
        DataType::Struct(s) => {
            let name = swift.naming.convert(&ndt.name);
            let generics = if ndt.generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    ndt.generics
                        .iter()
                        .map(|g| g.name.as_ref().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            // serde only key-encodes named fields; a newtype writes its bare
            // value and a tuple struct writes an array, so those shapes need a
            // custom `Codable` rather than the synthesised keyed one.
            let shape = struct_wire_shape(s, tuple_as_struct.is_some());
            let body = match &shape {
                StructWireShape::Keyed => {
                    struct_to_swift(swift, format, types, s, generic_scope.clone())?
                }
                StructWireShape::Unit => unit_struct_body(&name),
                StructWireShape::Newtype(field) => {
                    newtype_struct_body(swift, format, types, &name, field, generic_scope.clone())?
                }
                StructWireShape::Sequence(fields) => sequence_struct_body(
                    swift,
                    format,
                    types,
                    &name,
                    fields,
                    generic_scope.clone(),
                )?,
            };

            result.push_str(&format!("public struct {}{}: Codable {{\n", name, generics));
            result.push_str(&body);
            result.push('}');
        }
        DataType::Enum(e) => {
            let formatted_enum =
                match apply_datatype_format(None, types, ndt_ty.expect("checked above"))? {
                    DataType::Enum(e) => Some(e),
                    _ => None,
                };
            let e = formatted_enum
                .as_ref()
                .filter(|e| resolved_string_enum(e).is_some())
                .unwrap_or(e);

            let name = swift.naming.convert(&ndt.name);
            let generics = if ndt.generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    ndt.generics
                        .iter()
                        .map(|g| g.name.as_ref().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            // Internal and adjacent tagging put the discriminant in a field, so
            // a synthesised `Codable` would read the wrong shape entirely.
            let has_tagged_variants = e
                .variants
                .iter()
                .any(|(_, variant)| variant_tag(variant).is_some());

            // Check if this enum has struct-like variants (needs custom Codable)
            let has_struct_variants = e
                .variants
                .iter()
                .any(|(variant_name, variant)| should_emit_variant_wrapper(variant_name, variant));

            let has_serde_payload_variants = format.is_some()
                && e.variants.iter().any(|(variant_name, variant)| {
                    serde_variant_payload(variant_name, variant)
                        .is_some_and(|_| !is_unit_like_variant(variant_name, variant))
                });

            // A tagged enum's variants are string literals only because the tag
            // is one; a `String` raw type would advertise raw values that do not
            // match the wire tag.
            let is_string_enum_val = !has_tagged_variants && resolved_string_enum(e).is_some();

            let needs_custom_codable =
                has_struct_variants || has_serde_payload_variants || has_tagged_variants;

            // Determine protocols based on whether we'll generate custom Codable
            let protocols = if is_string_enum_val {
                if needs_custom_codable {
                    "String" // Custom Codable will be generated
                } else {
                    "String, Codable"
                }
            } else if needs_custom_codable {
                "" // Custom Codable will be generated
            } else {
                "Codable"
            };

            let protocol_part = if protocols.is_empty() {
                String::new()
            } else {
                format!(": {}", protocols)
            };

            result.push_str(&format!(
                "public enum {}{}{} {{\n",
                name, generics, protocol_part
            ));
            let enum_body =
                enum_to_swift(swift, format, types, e, generic_scope.clone(), Some(&name))?;
            result.push_str(&enum_body);
            result.push('}');

            // Generate struct definitions for named field variants
            let struct_definitions =
                generate_enum_structs(swift, format, types, e, generic_scope.clone(), &name)?;
            result.push_str(&struct_definitions);

            // Generate custom Codable implementation for enums with struct variants
            if needs_custom_codable {
                let codable_impl = generate_enum_codable_impl(
                    swift,
                    format,
                    types,
                    e,
                    generic_scope.clone(),
                    &name,
                )?;
                result.push_str(&codable_impl);
            }
        }
        _ => {
            return Ok(String::new());
        }
    }

    Ok(result)
}

/// Convert a DataType to Swift syntax.
pub fn datatype_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let dt = apply_datatype_format(format, types, dt)?;

    match &dt {
        DataType::Primitive(p) => primitive_to_swift(p),
        // DataType::Literal(l) => literal_to_swift(l),
        DataType::List(l) => list_to_swift(swift, format, types, l, generic_scope.clone()),
        DataType::Map(m) => map_to_swift(swift, format, types, m, generic_scope.clone()),
        DataType::Nullable(def) => {
            let inner = datatype_to_swift(swift, format, types, def, generic_scope)?;
            Ok(match swift.optionals {
                crate::swift::OptionalStyle::QuestionMark => format!("{}?", inner),
                crate::swift::OptionalStyle::Optional => format!("Optional<{}>", inner),
            })
        }
        DataType::Struct(s) => {
            // Check if this is a Duration struct by looking at its fields
            if is_duration_struct(s) {
                return Ok("RustDuration".to_string());
            }
            struct_to_swift(swift, format, types, s, generic_scope)
        }
        DataType::Enum(e) => enum_to_swift(swift, format, types, e, generic_scope, None),
        DataType::Tuple(t) => tuple_to_swift(swift, format, types, t, generic_scope.clone()),
        DataType::Reference(r) => reference_to_swift(swift, format, types, r, &generic_scope),
        DataType::Generic(g) => generic_to_swift(g, &generic_scope),
        DataType::Intersection(_) => Err(Error::UnsupportedType(
            "Intersection types are not supported by the Swift exporter; this usually comes from \
             an internally tagged variant whose payload is merged into the tag, which Swift's \
             `Codable` cannot express"
                .to_string(),
        )),
    }
}

fn apply_datatype_format(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
) -> Result<DataType, Error> {
    if contains_generic_reference(dt) {
        let Some(format) = format else {
            return apply_datatype_format_children(None, types, dt.clone());
        };

        match format.map_type(types, dt) {
            Ok(std::borrow::Cow::Borrowed(dt)) => {
                return apply_datatype_format_children(Some(format), types, dt.clone());
            }
            Ok(std::borrow::Cow::Owned(dt)) => {
                return apply_datatype_format_children(Some(format), types, dt);
            }
            Err(err) if err.to_string().contains("Unresolved generic reference") => {
                return apply_datatype_format_children(Some(format), types, dt.clone());
            }
            Err(err) => return Err(Error::format("datatype formatter failed", err)),
        }
    }

    if contains_recursive_reference(dt) {
        return apply_datatype_format_children(format, types, dt.clone());
    }

    let Some(format) = format else {
        return apply_datatype_format_children(None, types, dt.clone());
    };

    let mapped = format
        .map_type(types, dt)
        .map_err(|err| Error::format("datatype formatter failed", err))?;

    match mapped {
        std::borrow::Cow::Borrowed(dt) => {
            apply_datatype_format_children(Some(format), types, dt.clone())
        }
        std::borrow::Cow::Owned(dt) => apply_datatype_format_children(Some(format), types, dt),
    }
}

fn apply_datatype_format_children(
    format: Option<&dyn Format>,
    types: &Types,
    mut dt: DataType,
) -> Result<DataType, Error> {
    match &mut dt {
        DataType::Primitive(_) => {}
        DataType::List(list) => {
            *list.ty = apply_datatype_format(format, types, &list.ty)?;
        }
        DataType::Map(map) => {
            let key = apply_datatype_format(format, types, map.key_ty())?;
            let value = apply_datatype_format(format, types, map.value_ty())?;
            map.set_key_ty(key);
            map.set_value_ty(value);
        }
        DataType::Nullable(inner) => {
            **inner = apply_datatype_format(format, types, inner)?;
        }
        DataType::Struct(strct) => map_fields(format, types, &mut strct.fields)?,
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                map_fields(format, types, &mut variant.fields)?;
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                *element = apply_datatype_format(format, types, element)?;
            }
        }
        DataType::Intersection(intersection) => {
            for element in intersection {
                *element = apply_datatype_format(format, types, element)?;
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            specta::datatype::NamedReferenceType::Reference { generics, .. } => {
                for (_, generic) in generics {
                    *generic = apply_datatype_format(format, types, generic)?;
                }
            }
            specta::datatype::NamedReferenceType::Inline { .. }
            | specta::datatype::NamedReferenceType::Recursive(_) => {}
        },
        DataType::Reference(Reference::Opaque(_)) | DataType::Generic(_) => {}
    }

    Ok(dt)
}

fn map_fields(
    format: Option<&dyn Format>,
    types: &Types,
    fields: &mut Fields,
) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = apply_datatype_format(format, types, ty)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &mut named.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = apply_datatype_format(format, types, ty)?;
                }
            }
        }
    }

    Ok(())
}

fn contains_generic_reference(dt: &DataType) -> bool {
    match dt {
        DataType::Primitive(_) => false,
        DataType::List(list) => contains_generic_reference(&list.ty),
        DataType::Map(map) => {
            contains_generic_reference(map.key_ty()) || contains_generic_reference(map.value_ty())
        }
        DataType::Nullable(inner) => contains_generic_reference(inner),
        DataType::Struct(strct) => fields_contain_generic_reference(&strct.fields),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| fields_contain_generic_reference(&variant.fields)),
        DataType::Tuple(tuple) => tuple.elements.iter().any(contains_generic_reference),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Reference { generics, .. } => generics
                .iter()
                .any(|(_, generic)| contains_generic_reference(generic)),
            NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => false,
        },
        DataType::Reference(Reference::Opaque(_)) => false,
        DataType::Generic(_) => true,
        DataType::Intersection(types) => types.iter().any(contains_generic_reference),
    }
}

fn fields_contain_generic_reference(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .iter()
            .any(|field| field.ty.as_ref().is_some_and(contains_generic_reference)),
        Fields::Named(named) => named
            .fields
            .iter()
            .any(|(_, field)| field.ty.as_ref().is_some_and(contains_generic_reference)),
    }
}

fn contains_recursive_reference(dt: &DataType) -> bool {
    match dt {
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => false,
        DataType::List(list) => contains_recursive_reference(&list.ty),
        DataType::Map(map) => {
            contains_recursive_reference(map.key_ty())
                || contains_recursive_reference(map.value_ty())
        }
        DataType::Nullable(inner) => contains_recursive_reference(inner),
        DataType::Struct(strct) => fields_contain_recursive_reference(&strct.fields),
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| fields_contain_recursive_reference(&variant.fields)),
        DataType::Tuple(tuple) => tuple.elements.iter().any(contains_recursive_reference),
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Recursive(_) => true,
            NamedReferenceType::Reference { generics, .. } => generics
                .iter()
                .any(|(_, generic)| contains_recursive_reference(generic)),
            NamedReferenceType::Inline { .. } => false,
        },
        DataType::Intersection(types) => types.iter().any(contains_recursive_reference),
    }
}

fn fields_contain_recursive_reference(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .iter()
            .any(|field| field.ty.as_ref().is_some_and(contains_recursive_reference)),
        Fields::Named(named) => named
            .fields
            .iter()
            .any(|(_, field)| field.ty.as_ref().is_some_and(contains_recursive_reference)),
    }
}

/// Check if a struct is a Duration by examining its fields
pub fn is_duration_struct(s: &specta::datatype::Struct) -> bool {
    match &s.fields {
        specta::datatype::Fields::Named(fields) => {
            let field_names: Vec<String> = fields
                .fields
                .iter()
                .map(|(name, _)| name.to_string())
                .collect();
            // Duration has exactly two fields: "secs" (u64) and "nanos" (u32)
            field_names.len() == 2
                && field_names.contains(&"secs".to_string())
                && field_names.contains(&"nanos".to_string())
        }
        _ => false,
    }
}

/// Convert primitive types to Swift.
fn primitive_to_swift(primitive: &Primitive) -> Result<String, Error> {
    Ok(match primitive {
        Primitive::i8 => "Int8".to_string(),
        Primitive::i16 => "Int16".to_string(),
        Primitive::i32 => "Int32".to_string(),
        Primitive::i64 => "Int64".to_string(),
        Primitive::isize => "Int".to_string(),
        Primitive::u8 => "UInt8".to_string(),
        Primitive::u16 => "UInt16".to_string(),
        Primitive::u32 => "UInt32".to_string(),
        Primitive::u64 => "UInt64".to_string(),
        Primitive::usize => "UInt".to_string(),
        Primitive::f32 => "Float".to_string(),
        Primitive::f64 => "Double".to_string(),
        Primitive::bool => "Bool".to_string(),
        Primitive::char => "Character".to_string(),
        Primitive::str => "String".to_string(),
        Primitive::i128 | Primitive::u128 => {
            return Err(Error::UnsupportedType(
                "Swift does not support 128-bit integers".to_string(),
            ));
        }
        Primitive::f16 => {
            return Err(Error::UnsupportedType(
                "Swift does not support f16".to_string(),
            ));
        }
        Primitive::f128 => {
            return Err(Error::UnsupportedType(
                "Swift does not support f128".to_string(),
            ));
        }
    })
}

// /// Convert literal types to Swift.
// fn literal_to_swift(literal: &specta::datatype::Literal) -> Result<String, Error> {
//     Ok(match literal {
//         specta::datatype::Literal::i8(v) => v.to_string(),
//         specta::datatype::Literal::i16(v) => v.to_string(),
//         specta::datatype::Literal::i32(v) => v.to_string(),
//         specta::datatype::Literal::u8(v) => v.to_string(),
//         specta::datatype::Literal::u16(v) => v.to_string(),
//         specta::datatype::Literal::u32(v) => v.to_string(),
//         specta::datatype::Literal::f32(v) => v.to_string(),
//         specta::datatype::Literal::f64(v) => v.to_string(),
//         specta::datatype::Literal::bool(v) => v.to_string(),
//         specta::datatype::Literal::String(s) => format!("\"{}\"", s),
//         specta::datatype::Literal::char(c) => format!("\"{}\"", c),
//         specta::datatype::Literal::None => "nil".to_string(),
//         _ => {
//             return Err(Error::UnsupportedType(
//                 "Unsupported literal type".to_string(),
//             ));
//         }
//     })
// }

/// Convert list types to Swift arrays.
fn list_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    list: &specta::datatype::List,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let element_type = datatype_to_swift(swift, format, types, &list.ty, generic_scope)?;
    Ok(format!("[{}]", element_type))
}

/// Convert map types to Swift dictionaries.
fn map_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    map: &specta::datatype::Map,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let key_type = datatype_to_swift(swift, format, types, map.key_ty(), generic_scope.clone())?;
    let value_type = datatype_to_swift(swift, format, types, map.value_ty(), generic_scope)?;
    Ok(format!("[{}: {}]", key_type, value_type))
}

/// Convert struct types to Swift.
fn struct_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    s: &specta::datatype::Struct,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    match &s.fields {
        specta::datatype::Fields::Unit => Ok("Void".to_string()),
        specta::datatype::Fields::Unnamed(fields) => {
            if fields.fields.is_empty() {
                Ok("Void".to_string())
            } else if fields.fields.len() == 1 {
                // Single field tuple struct - convert to a proper struct with a 'value' field
                let Some(ty) = fields.fields[0].ty.as_ref() else {
                    // The sole field is skipped (`ty: None`), so nothing
                    // reaches the wire.
                    return Ok("Void".to_string());
                };
                let field_type = datatype_to_swift(swift, format, types, ty, generic_scope)?;
                Ok(format!("    let value: {}\n", field_type))
            } else {
                // Multiple field tuple struct - convert to a proper struct with numbered fields
                let mut result = String::new();
                for (i, field) in fields.fields.iter().enumerate() {
                    // Skipped slots (`ty: None` markers preserving the
                    // declared arity) never reach the wire.
                    let Some(ty) = field.ty.as_ref() else {
                        continue;
                    };
                    let field_type =
                        datatype_to_swift(swift, format, types, ty, generic_scope.clone())?;
                    // A `#[serde(default)]` trailing tuple element may be
                    // absent on deserialize; Swift's closest representation
                    // is an Optional property.
                    let optional_marker = optional_suffix(field, ty);
                    result.push_str(&format!(
                        "    public let field{}: {}{}\n",
                        i, field_type, optional_marker
                    ));
                }
                Ok(result)
            }
        }
        specta::datatype::Fields::Named(fields) => {
            let container_default = s.attributes.contains_key(DEFAULTED_CONTAINER);
            let live = keyed_fields(
                swift,
                format,
                types,
                fields,
                container_default,
                &generic_scope,
            )?;

            // An empty named struct writes `{}` and reads `{}`, which the
            // synthesised keyed `Codable` already produces exactly. Only the
            // public initializer is added so it stays constructible from
            // another module, like every other named struct.
            if live.is_empty() {
                return Ok("    public init() {}\n".to_string());
            }

            let mut result = String::new();
            for field in &live {
                result.push_str(&format!(
                    "    public let {}: {}\n",
                    field.property, field.property_type
                ));
            }

            // A hand-written codec needs its `CodingKeys` spelled out: the
            // compiler only synthesises them alongside a synthesised codec.
            result.push_str("\n    private enum CodingKeys: String, CodingKey {\n");
            for field in &live {
                result.push_str(&format!(
                    "        case {} = {}\n",
                    field.property,
                    swift_string_literal(field.wire_key)
                ));
            }
            result.push_str("    }\n");

            // The synthesised memberwise initializer is internal, so a public
            // struct needs an explicit `public init`.
            let parameters = live
                .iter()
                .map(|field| format!("{}: {}", field.property, field.property_type))
                .collect::<Vec<_>>()
                .join(", ");
            let assignments = live
                .iter()
                .map(|field| format!("        self.{} = {}", field.property, field.property))
                .collect::<Vec<_>>()
                .join("\n");
            result.push_str(&format!(
                "\n    public init({parameters}) {{\n{assignments}\n    }}\n"
            ));

            // serde accepts an absent key only for an `Option<T>` or a
            // `#[serde(default)]` field (see `may_be_absent_on_decode`); every
            // other field, including one with a bare `skip_serializing_if`, is
            // required.
            result.push_str("\n    public init(from decoder: Decoder) throws {\n");
            result.push_str(
                "        let container = try decoder.container(keyedBy: CodingKeys.self)\n",
            );
            for field in &live {
                let call = if field.may_be_absent_on_decode() {
                    "decodeIfPresent"
                } else {
                    "decode"
                };
                result.push_str(&format!(
                    "        self.{} = try container.{call}({}.self, forKey: .{})\n",
                    field.property, field.decode_type, field.property
                ));
            }
            result.push_str("    }\n");

            // serde writes every field -- a plain `Option<T>` and a
            // `#[serde(default)] Option<T>` are written as `null` -- and omits
            // only a `skip_serializing_if` field. A defaulted non-`Option`
            // field is also omitted when nil, because Swift cannot know the
            // Rust default that serde would otherwise write.
            result.push_str("\n    public func encode(to encoder: Encoder) throws {\n");
            result
                .push_str("        var container = encoder.container(keyedBy: CodingKeys.self)\n");
            for field in &live {
                if field.may_be_absent() {
                    result.push_str(&format!(
                        "        if let {} = {} {{\n            try container.encode({}, forKey: .{})\n        }}\n",
                        field.property, field.property, field.property, field.property
                    ));
                } else {
                    result.push_str(&format!(
                        "        try container.encode({}, forKey: .{})\n",
                        field.property, field.property
                    ));
                }
            }
            result.push_str("    }\n");

            Ok(result)
        }
    }
}

/// A live named field of a keyed struct, with the names and types its generated
/// `Codable` needs.
struct KeyedField<'a> {
    /// The generated Swift property name (`convert_field`, keyword-escaped).
    property: String,
    /// The wire key serde uses, which the `CodingKeys` case maps to.
    wire_key: &'a str,
    /// The declared field, which carries the optionality flags.
    field: &'a Field,
    /// The rendered Swift property type, including any `?`.
    property_type: String,
    /// The rendered type handed to `decode`/`decodeIfPresent`: the field's
    /// `Nullable` is unwrapped so a single `?` from `decodeIfPresent` yields
    /// the property's optionality.
    decode_type: String,
    /// Whether serde tolerates an absent key for this field on its own: it
    /// carries `#[serde(default)]`, or its struct does.
    defaulted: bool,
    /// Whether the field was *declared* as `Option<T>` (the `specta:nullable`
    /// marker), which survives a `#[specta(type = ...)]` override that hides
    /// the optionality from the exported datatype.
    rust_nullable: bool,
}

impl KeyedField<'_> {
    /// Whether the exported datatype is already nullable (`Option<T>`).
    fn is_nullable(&self) -> bool {
        matches!(self.field.ty.as_ref(), Some(DataType::Nullable(_)))
    }

    /// Whether the field is an `Option` to serde, from either the exported
    /// datatype or the declared Rust type behind a `#[specta(type = ...)]`
    /// override.
    fn nullable(&self) -> bool {
        self.is_nullable() || self.rust_nullable
    }

    /// Whether the generated property is optional, which every codec branch
    /// that reads a value into it depends on.
    fn property_optional(&self) -> bool {
        self.is_nullable() || self.field.optional || self.rust_nullable
    }

    /// Whether serde may omit the key when encoding.
    ///
    /// `#[serde(skip_serializing_if = ...)]` omits the field when it is nil. A
    /// defaulted non-`Option` field is also omittable: Swift cannot know the
    /// Rust default, so a nil value is only representable by dropping the key.
    /// An `Option` (including one hidden behind a `#[specta(type = ...)]`
    /// override) is *not* omittable -- serde writes it as `null`.
    fn may_be_absent(&self) -> bool {
        self.field.attributes.contains_key(SERIALIZE_OMISSION_FIELD)
            || (self.field.optional && !self.nullable())
    }

    /// Whether serde accepts an absent key when decoding, as opposed to
    /// requiring it.
    ///
    /// serde tolerates absence only for a field whose *declared Rust type* is
    /// `Option<T>` (a missing key yields `None`) or a `#[serde(default)]`
    /// field/struct. A bare `#[serde(skip_serializing_if = ...)]` does *not*:
    /// it omits the key when serializing but still errors on the absent key
    /// when deserializing. Nor does a `#[specta(type = Option<...>)]` override
    /// on a non-`Option` field: it only changes the exported datatype, not what
    /// serde accepts. `specta-serde` marks a `skip_serializing_if` field
    /// `optional`, so this cannot key off `Field::optional` alone. A field
    /// whose property is not optional must still `decode` (a `decodeIfPresent`
    /// would not fit the property type).
    fn may_be_absent_on_decode(&self) -> bool {
        self.property_optional() && (self.rust_nullable || self.defaulted)
    }
}

/// The `?` a keyed field's generated property needs.
///
/// A `Nullable` datatype renders as `T?` on its own. A `#[specta(type = ...)]`
/// override can hide a declared `Option<T>` behind a non-nullable datatype, but
/// serde still treats the field as an `Option` (writing `null` and tolerating
/// an absent key), so the property must be optional to represent that.
fn keyed_optional_suffix(field: &Field, ty: &DataType, rust_nullable: bool) -> &'static str {
    if (field.optional || rust_nullable) && !matches!(ty, DataType::Nullable(_)) {
        "?"
    } else {
        ""
    }
}

/// The live named fields of a keyed struct, in declaration order.
fn keyed_fields<'a>(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    fields: &'a NamedFields,
    container_default: bool,
    generic_scope: &[Generic],
) -> Result<Vec<KeyedField<'a>>, Error> {
    let mut out = Vec::new();
    for (name, field) in &fields.fields {
        // A skipped field (`ty: None`) never reaches the wire.
        let Some(original) = field.ty.as_ref() else {
            continue;
        };
        let rust_nullable = field.attributes.contains_key(NULLABLE_FIELD);
        let property_type = format!(
            "{}{}",
            datatype_to_swift(swift, format, types, original, generic_scope.to_vec())?,
            keyed_optional_suffix(field, original, rust_nullable)
        );
        let decode_type = datatype_to_swift(
            swift,
            format,
            types,
            field_decode_type(field).expect("a live field has a decode type"),
            generic_scope.to_vec(),
        )?;
        out.push(KeyedField {
            property: swift.naming.convert_field(name.as_ref()),
            wire_key: name.as_ref(),
            field,
            property_type,
            decode_type,
            defaulted: container_default || field.attributes.contains_key(DEFAULTED_FIELD),
            rust_nullable,
        });
    }
    Ok(out)
}

/// The serde wire shape of a named struct, which decides which `Codable`
/// container the generated Swift has to use.
///
/// serde only key-encodes structs with named fields. A newtype struct writes its
/// bare inner value and a tuple struct writes a JSON array, so relying on the
/// synthesised keyed `Codable` cannot round-trip either shape
/// (<https://github.com/specta-rs/specta/issues/596>).
enum StructWireShape<'a> {
    /// A named-field struct: serde key-encodes every field (a `nil` `Option<T>`
    /// is written as `null`, not omitted), so the codec is written by hand
    /// rather than left to the synthesised one, which uses `encodeIfPresent`.
    Keyed,
    /// A unit struct: serde writes `null`.
    Unit,
    /// A genuine newtype struct (declared arity 1): serde writes the bare inner
    /// value, so the codec needs a single-value container.
    Newtype(&'a Field),
    /// A tuple struct, or a named `DataType::Tuple` definition: serde writes a
    /// JSON array, so the codec needs an unkeyed container. Each declared field
    /// keeps its position so a skipped slot does not renumber the live elements.
    Sequence(Vec<(usize, &'a Field)>),
}

fn struct_wire_shape<'a>(strct: &'a Struct, from_tuple_definition: bool) -> StructWireShape<'a> {
    if from_tuple_definition {
        // `export_type` synthesises this from a `DataType::Tuple`, which serde
        // always writes as an array -- even a one-element tuple.
        let Fields::Unnamed(fields) = &strct.fields else {
            unreachable!("a tuple definition is built with unnamed fields")
        };
        return StructWireShape::Sequence(fields.fields.iter().enumerate().collect());
    }

    match &strct.fields {
        Fields::Unit => StructWireShape::Unit,
        Fields::Named(_) => StructWireShape::Keyed,
        Fields::Unnamed(fields) => match fields.fields.as_slice() {
            [] => StructWireShape::Sequence(Vec::new()),
            [field] => StructWireShape::Newtype(field),
            fields => StructWireShape::Sequence(fields.iter().enumerate().collect()),
        },
    }
}

/// A live element of a newtype or tuple struct's wire sequence.
struct WireElement<'a> {
    /// The generated Swift property name: `value` for a newtype, `field{i}`
    /// (using the declared index) for a tuple.
    property: String,
    /// The declared field, which carries the element's optionality.
    field: &'a Field,
    /// The element's datatype with any `Nullable` unwrapped, so the property
    /// type and the container `decode` call agree on optionality.
    ty: &'a DataType,
    /// Whether the wire value may be `null` (`Option<T>`).
    nullable: bool,
    /// Whether serde may omit the element from the serialized sequence
    /// (`#[serde(skip_serializing_if = ...)]`).
    omittable: bool,
}

impl WireElement<'_> {
    /// Whether the element may be absent from the serialized sequence.
    ///
    /// A `skip_serializing_if` element is omitted whenever it is nil. A
    /// `#[serde(default)]` element with a non-`Option` datatype is also
    /// omittable on encode: Swift cannot know the Rust default, so a nil value
    /// is only representable by dropping the element. A defaulted `Option<T>`
    /// is *not* omittable — serde writes it as `null`.
    fn may_be_absent(&self) -> bool {
        self.omittable || (self.field.optional && !self.nullable)
    }
}

fn wire_elements<'a>(
    struct_name: &str,
    fields: &[(usize, &'a Field)],
    newtype: bool,
) -> Result<Vec<WireElement<'a>>, Error> {
    let mut elements = Vec::new();
    for (index, field) in fields {
        // A skipped slot (`ty: None`) preserves the declared arity but never
        // reaches the wire.
        let Some(ty) = field_decode_type(field) else {
            continue;
        };
        if unsupported_payload(ty) {
            return Err(Error::UnsupportedType(format!(
                "struct `{struct_name}` has an element that Swift's `Codable` cannot encode \
                 (a tuple or empty payload)"
            )));
        }
        elements.push(WireElement {
            property: if newtype {
                "value".to_string()
            } else {
                format!("field{index}")
            },
            field,
            ty,
            nullable: matches!(field.ty.as_ref(), Some(DataType::Nullable(_))),
            omittable: field.attributes.contains_key(SERIALIZE_OMISSION_FIELD),
        });
    }
    Ok(elements)
}

/// The Swift type of a wire element's generated property, including the `?` a
/// trailing omittable element needs.
fn element_property_type(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    element: &WireElement<'_>,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let original = element
        .field
        .ty
        .as_ref()
        .expect("a live element has a type");
    let base = datatype_to_swift(swift, format, types, original, generic_scope)?;
    Ok(format!(
        "{base}{}",
        optional_suffix(element.field, original)
    ))
}

/// The Swift type to pass to `decode`: the element's `Nullable` is unwrapped and
/// re-applied as `?` so a `null` decodes to `nil` without a second `?`.
fn element_decode_type(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    element: &WireElement<'_>,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let base = datatype_to_swift(swift, format, types, element.ty, generic_scope)?;
    Ok(if element.nullable {
        format!("{base}?")
    } else {
        base
    })
}

/// Render a unit struct's body. serde writes `null` for a unit struct, whereas
/// the synthesised keyed `Codable` would write `{}`.
fn unit_struct_body(struct_name: &str) -> String {
    let mut body = String::new();
    body.push_str("    public init() {}\n\n");
    body.push_str("    public init(from decoder: Decoder) throws {\n");
    body.push_str("        let container = try decoder.singleValueContainer()\n");
    body.push_str("        if !container.decodeNil() {\n");
    body.push_str(&format!(
        "            throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"Expected null for unit struct {struct_name}\"))\n"
    ));
    body.push_str("        }\n");
    body.push_str("    }\n\n");
    body.push_str("    public func encode(to encoder: Encoder) throws {\n");
    body.push_str("        var container = encoder.singleValueContainer()\n");
    body.push_str("        try container.encodeNil()\n");
    body.push_str("    }\n");
    body
}

/// Render a genuine newtype struct's body: serde delegates straight to the inner
/// value, so the codec uses a single-value container.
fn newtype_struct_body(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    struct_name: &str,
    field: &Field,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let Some(element) = wire_elements(struct_name, &[(0, field)], true)?
        .into_iter()
        .next()
    else {
        // The sole declared field is skipped. serde ignores `skip` on a
        // newtype's only field, so this cannot be a serde-shaped datatype; the
        // keyed `Codable` that would otherwise be emitted silently disagrees
        // with the wire.
        return Err(Error::UnsupportedType(format!(
            "newtype struct `{struct_name}` has no live field, which Swift's `Codable` cannot \
             express"
        )));
    };

    // serde ignores `#[serde(default)]` on a newtype's only field, so it must
    // not make the property optional; a `Nullable` already renders as `T?`.
    let original = field.ty.as_ref().expect("a live element has a type");
    let property_type = datatype_to_swift(swift, format, types, original, generic_scope.clone())?;
    let decode_type = element_decode_type(swift, format, types, &element, generic_scope)?;

    let mut body = String::new();
    body.push_str(&format!("    public let value: {property_type}\n\n"));
    body.push_str(&format!(
        "    public init(value: {property_type}) {{\n        self.value = value\n    }}\n\n"
    ));
    body.push_str("    public init(from decoder: Decoder) throws {\n");
    body.push_str("        let container = try decoder.singleValueContainer()\n");
    body.push_str(&format!(
        "        self.value = try container.decode({decode_type}.self)\n"
    ));
    body.push_str("    }\n\n");
    body.push_str("    public func encode(to encoder: Encoder) throws {\n");
    body.push_str("        var container = encoder.singleValueContainer()\n");
    body.push_str("        try container.encode(value)\n");
    body.push_str("    }\n");
    Ok(body)
}

/// Render a tuple struct's body: serde writes a JSON array, so the codec uses an
/// unkeyed container. Skipped slots are omitted (serde compacts the sequence)
/// and a trailing run of omittable elements is decoded by container exhaustion
/// rather than an unconditional `Optional`.
fn sequence_struct_body(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    struct_name: &str,
    fields: &[(usize, &Field)],
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let elements = wire_elements(struct_name, fields, false)?;

    let mut body = String::new();
    let mut parameters = Vec::new();
    let mut assignments = Vec::new();
    for element in &elements {
        let property_type =
            element_property_type(swift, format, types, element, generic_scope.clone())?;
        body.push_str(&format!(
            "    public let {}: {property_type}\n",
            element.property
        ));
        parameters.push(format!("{}: {property_type}", element.property));
        assignments.push(format!(
            "        self.{} = {}",
            element.property, element.property
        ));
    }

    if elements.is_empty() {
        body.push_str("    public init() {}\n\n");
    } else {
        body.push('\n');
        body.push_str(&format!(
            "    public init({}) {{\n{}\n    }}\n\n",
            parameters.join(", "),
            assignments.join("\n")
        ));
    }
    body.push_str("    public init(from decoder: Decoder) throws {\n");
    if elements.is_empty() {
        body.push_str("        let container = try decoder.unkeyedContainer()\n");
        body.push_str("        if !container.isAtEnd {\n");
        body.push_str("            throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"Expected an empty array\"))\n");
        body.push_str("        }\n");
    } else {
        body.push_str("        var container = try decoder.unkeyedContainer()\n");
        for element in &elements {
            let decode_type =
                element_decode_type(swift, format, types, element, generic_scope.clone())?;
            if element.field.optional {
                let property_type =
                    element_property_type(swift, format, types, element, generic_scope.clone())?;
                body.push_str(&format!(
                    "        let {}: {property_type}\n",
                    element.property
                ));
                body.push_str("        if container.isAtEnd {\n");
                body.push_str(&format!("            {} = nil\n", element.property));
                body.push_str("        } else {\n");
                body.push_str(&format!(
                    "            {} = try container.decode({decode_type}.self)\n",
                    element.property
                ));
                body.push_str("        }\n");
            } else {
                body.push_str(&format!(
                    "        let {} = try container.decode({decode_type}.self)\n",
                    element.property
                ));
            }
        }
        let arguments = elements
            .iter()
            .map(|element| format!("{}: {}", element.property, element.property))
            .collect::<Vec<_>>()
            .join(", ");
        body.push_str(&format!("        self.init({arguments})\n"));
    }
    body.push_str("    }\n\n");

    body.push_str("    public func encode(to encoder: Encoder) throws {\n");
    if elements.is_empty() {
        body.push_str("        _ = encoder.unkeyedContainer()\n");
    } else {
        body.push_str("        var container = encoder.unkeyedContainer()\n");
        for (index, element) in elements.iter().enumerate() {
            if !element.may_be_absent() {
                body.push_str(&format!(
                    "        try container.encode({})\n",
                    element.property
                ));
                continue;
            }

            body.push_str(&format!(
                "        if let {} = {} {{\n",
                element.property, element.property
            ));
            body.push_str(&format!(
                "            try container.encode({})\n",
                element.property
            ));

            // Only a nil *suffix* may be omitted: serde's sequence format has
            // no way to express a gap, so a nil element followed by a present
            // one would silently shift every following value.
            let later = &elements[index + 1..];
            let throw = format!(
                "            throw EncodingError.invalidValue(self, EncodingError.Context(codingPath: encoder.codingPath, debugDescription: \"`{}` is nil but a later element is present, which serde's sequence format cannot represent\"))\n",
                element.property
            );
            if later.is_empty() {
                body.push_str("        }\n");
            } else if later.iter().any(|later| !later.may_be_absent()) {
                // A later element is always present.
                body.push_str("        } else {\n");
                body.push_str(&throw);
                body.push_str("        }\n");
            } else {
                let condition = later
                    .iter()
                    .map(|later| format!("{} != nil", later.property))
                    .collect::<Vec<_>>()
                    .join(" || ");
                body.push_str(&format!("        }} else if {condition} {{\n"));
                body.push_str(&throw);
                body.push_str("        }\n");
            }
        }
    }
    body.push_str("    }\n");
    Ok(body)
}

/// Convert enum types to Swift.
fn enum_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    enum_name: Option<&str>,
) -> Result<String, Error> {
    // An inline enum with no variants is uninhabited, which Swift names `Never`. A named
    // one is declared as an empty enum by the caller instead, but this path only builds
    // the type to splice in, so it would otherwise splice in nothing at all.
    if enum_name.is_none() && e.variants.iter().all(|(_, variant)| variant.skip) {
        return Ok("Never".to_owned());
    }

    let mut result = String::new();

    // Check if this is a string enum
    let is_string_enum = resolved_string_enum(e).is_some();

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let variant_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());

        match &variant.fields {
            specta::datatype::Fields::Unit => {
                if is_string_enum {
                    let raw_value = enum_string_raw_value(variant)
                        .unwrap_or_else(|| original_variant_name.as_ref());
                    result.push_str(&format!("    case {} = \"{}\"\n", variant_name, raw_value));
                } else {
                    result.push_str(&format!("    case {}\n", variant_name));
                }
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if is_string_enum {
                    let raw_value = enum_string_raw_value(variant)
                        .unwrap_or_else(|| original_variant_name.as_ref());
                    result.push_str(&format!("    case {} = \"{}\"\n", variant_name, raw_value));
                } else if fields.fields.is_empty()
                    || fields.fields.len() == 1
                        && fields.fields[0]
                            .ty
                            .as_ref()
                            .is_some_and(|ty| is_unit_payload(original_variant_name, ty))
                {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else {
                    let types_str = fields
                        .fields
                        .iter()
                        .map(|f| {
                            datatype_to_swift(
                                swift,
                                format,
                                types,
                                f.ty.as_ref()
                                    .expect("enum variant field should have a type"),
                                generic_scope.clone(),
                            )
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?
                        .join(", ");
                    result.push_str(&format!("    case {}({})\n", variant_name, types_str));
                }
            }
            specta::datatype::Fields::Named(fields) => {
                if let Some(tag) = variant_tag(variant) {
                    // Internally and adjacently tagged variants carry their
                    // discriminant in a field; a payload-less variant is a plain
                    // case and the rest wrap their payload in a struct.
                    let payload = tagged_payload(variant, tag.key);
                    if payload.fields.is_empty() && payload.content_key.is_none() {
                        result.push_str(&format!("    case {}\n", variant_name));
                    } else {
                        let struct_name =
                            variant_wrapper_name(enum_name.unwrap_or(""), original_variant_name);
                        result.push_str(&format!("    case {}({})\n", variant_name, struct_name));
                    }
                } else if fields.fields.is_empty() {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else if !should_emit_variant_wrapper(original_variant_name, variant) {
                    let payload = serde_variant_payload(original_variant_name, variant)
                        .expect("serde payload variants should contain a payload");

                    if is_unit_payload(original_variant_name, payload) {
                        result.push_str(&format!("    case {}\n", variant_name));
                    } else {
                        let payload_ty = enum_payload_to_swift_type(
                            swift,
                            format,
                            types,
                            original_variant_name,
                            payload,
                            &generic_scope,
                        )?;
                        result.push_str(&format!("    case {}({})\n", variant_name, payload_ty));
                    }
                } else {
                    // Generate struct for named fields
                    // Use the original variant name for PascalCase struct name
                    let struct_name =
                        variant_wrapper_name(enum_name.unwrap_or(""), original_variant_name);

                    // Generate enum case that references the struct
                    result.push_str(&format!("    case {}({})\n", variant_name, struct_name));
                }
            }
        }
    }

    Ok(result)
}

/// Generate struct definitions for enum variants with named fields
fn generate_enum_structs(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    enum_name: &str,
) -> Result<String, Error> {
    let mut result = String::new();

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let Some(wrapper) = wrapper_fields(original_variant_name, variant) else {
            continue;
        };

        let struct_name = variant_wrapper_name(enum_name, original_variant_name);

        // Generate struct definition with custom CodingKeys for field name mapping
        result.push_str(&format!("\npublic struct {}: Codable {{\n", struct_name));

        // Generate struct fields
        let mut field_mappings = Vec::new();
        for (original_field_name, field) in &wrapper {
            if let Some(ty) = field.ty.as_ref() {
                if field.attributes.contains_key(FLATTENED_FIELD) {
                    return Err(Error::UnsupportedType(format!(
                        "variant `{original_variant_name}` flattens field `{original_field_name}` \
                         into the variant, which the Swift exporter cannot express"
                    )));
                }

                if unsupported_payload_field(field) {
                    return Err(Error::UnsupportedType(format!(
                        "variant `{original_variant_name}` has a field `{original_field_name}` that \
                         Swift's `Codable` cannot encode (a tuple or empty payload)"
                    )));
                }

                let field_type =
                    datatype_to_swift(swift, format, types, ty, generic_scope.clone())?;
                let optional_marker = optional_suffix(field, ty);
                let swift_field_name = swift.naming.convert_field(original_field_name);
                result.push_str(&format!(
                    "    public let {}: {}{}\n",
                    swift_field_name, field_type, optional_marker
                ));
                field_mappings.push((swift_field_name, original_field_name.to_string()));
            }
        }

        // Generate custom CodingKeys if field names were converted
        let needs_custom_coding_keys = field_mappings
            .iter()
            .any(|(swift_name, rust_name)| swift_name != rust_name);
        if needs_custom_coding_keys {
            result.push_str("\n    private enum CodingKeys: String, CodingKey {\n");
            for (swift_name, rust_name) in &field_mappings {
                result.push_str(&format!(
                    "        case {} = \"{}\"\n",
                    swift_name, rust_name
                ));
            }
            result.push_str("    }\n");
        }

        result.push_str("}\n");
    }

    Ok(result)
}

/// Convert a string to PascalCase
fn to_pascal_case(s: &str) -> String {
    // If it's already PascalCase (starts with uppercase), return as-is
    if s.chars().next().is_some_and(|c| c.is_uppercase()) {
        return s.to_string();
    }

    // Otherwise, convert snake_case to PascalCase
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else {
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
    }

    result
}

/// Convert tuple types to Swift.
fn tuple_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    t: &specta::datatype::Tuple,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    if t.elements.is_empty() {
        Ok("Void".to_string())
    } else if t.elements.len() == 1 {
        datatype_to_swift(swift, format, types, &t.elements[0], generic_scope)
    } else {
        let types_str = t
            .elements
            .iter()
            .map(|e| datatype_to_swift(swift, format, types, e, generic_scope.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", ");
        Ok(format!("({})", types_str))
    }
}

/// Convert reference types to Swift.
fn reference_to_swift(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    r: &specta::datatype::Reference,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    match r {
        Reference::Named(r) => {
            let Some(ndt) = types.get(r) else {
                return Err(Error::InvalidIdentifier(
                    "Reference to unknown type".to_string(),
                ));
            };

            if let NamedReferenceType::Inline { dt, .. } = &r.inner
                && !matches!(dt.as_ref(), DataType::Enum(_))
                && !contains_recursive_reference(dt)
            {
                return datatype_to_swift(swift, format, types, dt, generic_scope.to_vec());
            }

            let generics = match &r.inner {
                NamedReferenceType::Reference { generics, .. } => generics.as_slice(),
                NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive(_) => &[],
            };

            let name = swift.naming.convert(&ndt.name);

            if ndt.name.as_ref() == "String" {
                return Ok("String".to_string());
            }

            if ndt.name.as_ref() == "Vec"
                && let [(_, element)] = generics
            {
                let element =
                    datatype_to_swift(swift, format, types, element, generic_scope.to_vec())?;
                return Ok(format!("[{element}]"));
            }

            if generics.is_empty() {
                Ok(name)
            } else {
                let generics = generics
                    .iter()
                    .map(|(_, t)| {
                        datatype_to_swift(swift, format, types, t, generic_scope.to_vec())
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}<{}>", name, generics))
            }
        }
        Reference::Opaque(_) => Err(Error::UnsupportedType(
            "Opaque references are not supported by Swift exporter".to_string(),
        )),
    }
}

/// Convert generic types to Swift.
fn generic_to_swift(
    g: &specta::datatype::GenericReference,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    generic_scope
        .iter()
        .find_map(|generic| (generic.reference() == *g).then(|| generic.name().to_string()))
        .ok_or_else(|| Error::GenericConstraint(format!("Unresolved generic reference: {g:?}")))
}

/// Generate a custom `Codable` implementation that matches the serde wire
/// format `specta-serde` baked into the enum's shape.
fn generate_enum_codable_impl(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    enum_name: &str,
) -> Result<String, Error> {
    // Internal and adjacent tagging name the variant in a tag field; external
    // tagging uses the variant name as the sole object key instead.
    let tags = e
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(_, variant)| variant_tag(variant).map(|tag| tag.key))
        .collect::<Vec<_>>();
    let first = tags.first().copied().flatten();

    let tag_key = match first {
        Some(key) if tags.iter().all(|tag| *tag == Some(key)) => key,
        // Some variants are tagged and others are not (variant-level
        // `#[serde(untagged)]`, or a `#[serde(other)]` tag widened to a plain
        // string). Neither shape maps onto a single Swift `Codable`.
        Some(_) => {
            return Err(Error::UnsupportedType(format!(
                "enum `{enum_name}` mixes tagged and untagged serde variants, which the Swift \
                 exporter cannot express as one `Codable`"
            )));
        }
        None if tags.iter().any(Option::is_some) => {
            return Err(Error::UnsupportedType(format!(
                "enum `{enum_name}` has a serde tag that the Swift exporter cannot recover"
            )));
        }
        None => {
            return generate_external_enum_codable(
                swift,
                format,
                types,
                e,
                generic_scope,
                enum_name,
            );
        }
    };

    generate_tagged_enum_codable(swift, format, types, e, generic_scope, enum_name, tag_key)
}

/// Generate the `Codable` conformance for an internally or adjacently tagged
/// enum, whose variants all share a discriminant field.
fn generate_tagged_enum_codable(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    enum_name: &str,
    tag_key: &str,
) -> Result<String, Error> {
    let mut result = String::new();

    result.push_str(&format!(
        "\n// MARK: - {} Codable Implementation\n",
        enum_name
    ));
    result.push_str(&format!("extension {}: Codable {{\n", enum_name));

    // Wire keys are looked up by their serde name, so a dynamic key type avoids
    // synthesising a `CodingKeys` case per field (and the collisions between
    // variants that would come with it).
    result.push_str("    private struct CodingKeys: CodingKey {\n");
    result.push_str("        let stringValue: String\n");
    result.push_str("        var intValue: Int? { nil }\n");
    result.push_str("        init(_ stringValue: String) { self.stringValue = stringValue }\n");
    result.push_str("        init?(stringValue: String) { self.stringValue = stringValue }\n");
    result.push_str("        init?(intValue: Int) { nil }\n");
    result.push_str("    }\n\n");

    // Generate init(from decoder:)
    result.push_str("    public init(from decoder: Decoder) throws {\n");
    result.push_str("        let container = try decoder.container(keyedBy: CodingKeys.self)\n");
    result.push_str(&format!(
        "        let tag = try container.decode(String.self, forKey: CodingKeys({}))\n",
        swift_string_literal(tag_key)
    ));
    result.push_str("        switch tag {\n");

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let Some(tag) = variant_tag(variant) else {
            continue;
        };
        let case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());
        let payload = tagged_payload(variant, tag.key);

        result.push_str(&format!(
            "        case {}:\n",
            swift_string_literal(tag.value)
        ));

        if let Some(content_key) = payload.content_key {
            let struct_name = variant_wrapper_name(enum_name, original_variant_name);
            result.push_str(&format!(
                "            let data = try container.decode({}.self, forKey: CodingKeys({}))\n",
                struct_name,
                swift_string_literal(content_key)
            ));
            result.push_str(&format!("            self = .{}(data)\n", case_name));
            continue;
        }

        if payload.fields.is_empty() {
            result.push_str(&format!("            self = .{}\n", case_name));
            continue;
        }

        let struct_name = variant_wrapper_name(enum_name, original_variant_name);
        let mut arguments = Vec::new();
        for (field_name, field) in &payload.fields {
            let Some(ty) = field_decode_type(field) else {
                continue;
            };
            if unsupported_payload_field(field) {
                return Err(Error::UnsupportedType(format!(
                    "variant `{original_variant_name}` has a field `{field_name}` that Swift's \
                     `Codable` cannot encode (a tuple or empty payload)"
                )));
            }

            let field_type = datatype_to_swift(swift, format, types, ty, generic_scope.clone())?;
            let swift_field_name = swift.naming.convert_field(field_name);
            // Appending to a backticked identifier (`` `default`Value ``) does
            // not parse, so build the binding from the raw name.
            let binding = format!("{}Value", swift_field_name.trim_matches('`'));
            let decode = if field_is_optional(field) {
                format!(
                    "try container.decodeIfPresent({}.self, forKey: CodingKeys({}))",
                    field_type,
                    swift_string_literal(field_name)
                )
            } else {
                format!(
                    "try container.decode({}.self, forKey: CodingKeys({}))",
                    field_type,
                    swift_string_literal(field_name)
                )
            };
            result.push_str(&format!("            let {} = {}\n", binding, decode));
            // Argument labels are not identifier positions, so an escaped
            // keyword there is unnecessary (and warns).
            arguments.push(format!(
                "{}: {}",
                swift_field_name.trim_matches('`'),
                binding
            ));
        }
        result.push_str(&format!(
            "            self = .{}({}({}))\n",
            case_name,
            struct_name,
            arguments.join(", ")
        ));
    }

    result.push_str("        default:\n");
    result.push_str("            throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"Unknown variant \\(tag)\"))\n");
    result.push_str("        }\n");
    result.push_str("    }\n\n");

    // Generate encode(to encoder:)
    result.push_str("    public func encode(to encoder: Encoder) throws {\n");
    result.push_str("        var container = encoder.container(keyedBy: CodingKeys.self)\n");
    result.push_str("        switch self {\n");

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let Some(tag) = variant_tag(variant) else {
            continue;
        };
        let case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());
        let payload = tagged_payload(variant, tag.key);
        let has_payload = payload.content_key.is_some() || !payload.fields.is_empty();
        // A payload of only skipped fields still has an associated value, but
        // nothing to encode from it.
        let uses_data = payload.content_key.is_some()
            || payload.fields.iter().any(|(_, field)| field.ty.is_some());
        let binding = match (has_payload, uses_data) {
            (false, _) => "",
            (true, true) => "(let data)",
            (true, false) => "(_)",
        };

        result.push_str(&format!("        case .{}{}:\n", case_name, binding));
        result.push_str(&format!(
            "            try container.encode({}, forKey: CodingKeys({}))\n",
            swift_string_literal(tag.value),
            swift_string_literal(tag_key)
        ));

        if let Some(content_key) = payload.content_key {
            result.push_str(&format!(
                "            try container.encode(data, forKey: CodingKeys({}))\n",
                swift_string_literal(content_key)
            ));
            continue;
        }

        for (field_name, field) in &payload.fields {
            if field.ty.is_none() {
                continue;
            }
            let swift_field_name = swift.naming.convert_field(field_name);
            // serde omits an absent optional/defaulted field rather than
            // writing `null` (which its non-`Option` types reject), so encode
            // only what is present.
            let encode = if field_is_optional(field) {
                "encodeIfPresent"
            } else {
                "encode"
            };
            result.push_str(&format!(
                "            try container.{}(data.{}, forKey: CodingKeys({}))\n",
                encode,
                swift_field_name,
                swift_string_literal(field_name)
            ));
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n");
    result.push_str("}\n");

    Ok(result)
}

/// Generate custom Codable implementation for externally tagged enums.
fn generate_external_enum_codable(
    swift: &Swift,
    format: Option<&dyn Format>,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    enum_name: &str,
) -> Result<String, Error> {
    let mut result = String::new();

    result.push_str(&format!(
        "\n// MARK: - {} Codable Implementation\n",
        enum_name
    ));
    result.push_str(&format!("extension {}: Codable {{\n", enum_name));

    // Generate CodingKeys enum
    result.push_str("    private enum CodingKeys: String, CodingKey {\n");
    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }
        let swift_case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());
        result.push_str(&format!(
            "        case {} = \"{}\"\n",
            swift_case_name, original_variant_name
        ));
    }
    result.push_str("    }\n\n");

    // Externally tagged unit variants serialize as a bare string, while every
    // other variant is a single-key object, so try the string form first.
    let unit_variants: Vec<&str> = e
        .variants
        .iter()
        .filter(|(name, variant)| !variant.skip && is_external_unit_variant(name, variant))
        .map(|(name, _)| name.as_ref())
        .collect();

    // Generate init(from decoder:)
    result.push_str("    public init(from decoder: Decoder) throws {\n");
    if !unit_variants.is_empty() {
        result.push_str(
            "        if let value = try? decoder.singleValueContainer().decode(String.self) {\n",
        );
        result.push_str("            switch value {\n");
        for name in &unit_variants {
            let swift_case_name = swift.naming.convert_enum_case(name);
            result.push_str(&format!(
                "            case {}:\n",
                swift_string_literal(name)
            ));
            result.push_str(&format!("                self = .{}\n", swift_case_name));
            result.push_str("                return\n");
        }
        result.push_str("            default:\n");
        result.push_str("                break\n");
        result.push_str("            }\n");
        result.push_str("        }\n\n");
    }
    result.push_str("        let container = try decoder.container(keyedBy: CodingKeys.self)\n");
    result.push_str("        \n");
    result.push_str("        if container.allKeys.count != 1 {\n");
    result.push_str("            throw DecodingError.dataCorrupted(\n");
    result.push_str("                DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"Invalid number of keys found, expected one.\")\n");
    result.push_str("            )\n");
    result.push_str("        }\n\n");
    result.push_str("        let key = container.allKeys.first!\n");
    result.push_str("        switch key {\n");

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let swift_case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());

        // serde writes an externally tagged tuple variant as a JSON array under
        // the variant key, so decode it from an unkeyed container.
        if let Some(elements) = tuple_payload_elements(original_variant_name, variant) {
            result.push_str(&format!("        case .{}:\n", swift_case_name));
            result.push_str(&format!(
                "            var nested = try container.nestedUnkeyedContainer(forKey: .{})\n",
                swift_case_name
            ));
            let mut bindings = Vec::new();
            for (index, ty) in elements.iter().enumerate() {
                let element_ty =
                    datatype_to_swift(swift, format, types, ty, generic_scope.clone())?;
                let binding = format!("value{index}");
                result.push_str(&format!(
                    "            let {binding} = try nested.decode({element_ty}.self)\n"
                ));
                bindings.push(binding);
            }
            result.push_str(&format!(
                "            self = .{}({})\n",
                swift_case_name,
                bindings.join(", ")
            ));
            continue;
        }

        match &variant.fields {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!("            self = .{}\n", swift_case_name));
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if fields.fields.is_empty()
                    || fields.fields.len() == 1
                        && fields.fields[0]
                            .ty
                            .as_ref()
                            .is_some_and(|ty| is_unit_payload(original_variant_name, ty))
                {
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!("            self = .{}\n", swift_case_name));
                } else if fields.fields.len() == 1 {
                    let payload_ty = datatype_to_swift(
                        swift,
                        format,
                        types,
                        fields.fields[0]
                            .ty
                            .as_ref()
                            .expect("enum variant field should have a type"),
                        generic_scope.clone(),
                    )?;
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!(
                        "            let data = try container.decode({}.self, forKey: .{})\n",
                        payload_ty, swift_case_name
                    ));
                    result.push_str(&format!("            self = .{}(data)\n", swift_case_name));
                } else {
                    // For tuple variants, decode as array
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!(
                        "            // TODO: Implement tuple variant decoding for {}\n",
                        swift_case_name
                    ));
                    result.push_str(
                        "            fatalError(\"Tuple variant decoding not implemented\")\n",
                    );
                }
            }
            specta::datatype::Fields::Named(_) => {
                if should_emit_variant_wrapper(original_variant_name, variant) {
                    let pascal_variant_name = to_pascal_case(original_variant_name.as_ref());
                    let struct_name = format!("{}{}Data", enum_name, pascal_variant_name);

                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!(
                        "            let data = try container.decode({}.self, forKey: .{})\n",
                        struct_name, swift_case_name
                    ));
                    result.push_str(&format!("            self = .{}(data)\n", swift_case_name));
                } else {
                    let payload = serde_variant_payload(original_variant_name, variant)
                        .expect("serde payload variants should contain a payload");

                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    if is_unit_payload(original_variant_name, payload) {
                        result.push_str(&format!("            self = .{}\n", swift_case_name));
                    } else {
                        let payload_ty = enum_payload_to_swift_type(
                            swift,
                            format,
                            types,
                            original_variant_name,
                            payload,
                            &generic_scope,
                        )?;
                        result.push_str(&format!(
                            "            let data = try container.decode({}.self, forKey: .{})\n",
                            payload_ty, swift_case_name
                        ));
                        result
                            .push_str(&format!("            self = .{}(data)\n", swift_case_name));
                    }
                }
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n\n");

    // Generate encode(to encoder:)
    result.push_str("    public func encode(to encoder: Encoder) throws {\n");
    result.push_str("        switch self {\n");

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let swift_case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());

        // A tuple variant is written as a JSON array under the variant key.
        if let Some(elements) = tuple_payload_elements(original_variant_name, variant) {
            let bindings = (0..elements.len())
                .map(|index| format!("value{index}"))
                .collect::<Vec<_>>();
            result.push_str(&format!(
                "        case .{}({}):\n",
                swift_case_name,
                bindings
                    .iter()
                    .map(|binding| format!("let {binding}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            result.push_str(
                "            var container = encoder.container(keyedBy: CodingKeys.self)\n",
            );
            result.push_str(&format!(
                "            var nested = container.nestedUnkeyedContainer(forKey: .{})\n",
                swift_case_name
            ));
            for binding in &bindings {
                result.push_str(&format!("            try nested.encode({binding})\n"));
            }
            continue;
        }

        match &variant.fields {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str("            var container = encoder.singleValueContainer()\n");
                result.push_str(&format!(
                    "            try container.encode({})\n",
                    swift_string_literal(original_variant_name)
                ));
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if fields.fields.len() == 1
                    && fields.fields[0]
                        .ty
                        .as_ref()
                        .is_some_and(|ty| is_unit_payload(original_variant_name, ty))
                {
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str("            var container = encoder.singleValueContainer()\n");
                    result.push_str(&format!(
                        "            try container.encode({})\n",
                        swift_string_literal(original_variant_name)
                    ));
                    continue;
                } else if fields.fields.len() == 1 {
                    result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
                    result.push_str(
                        "            var container = encoder.container(keyedBy: CodingKeys.self)\n",
                    );
                    result.push_str(&format!(
                        "            try container.encode(data, forKey: .{})\n",
                        swift_case_name
                    ));
                    continue;
                }

                // TODO: Handle tuple variants
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            // TODO: Implement tuple variant encoding for {}\n",
                    swift_case_name
                ));
                result.push_str(
                    "            fatalError(\"Tuple variant encoding not implemented\")\n",
                );
            }
            specta::datatype::Fields::Named(_) => {
                if should_emit_variant_wrapper(original_variant_name, variant) {
                    result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
                    result.push_str(
                        "            var container = encoder.container(keyedBy: CodingKeys.self)\n",
                    );
                    result.push_str(&format!(
                        "            try container.encode(data, forKey: .{})\n",
                        swift_case_name
                    ));
                } else {
                    let payload = serde_variant_payload(original_variant_name, variant)
                        .expect("serde payload variants should contain a payload");

                    if is_unit_payload(original_variant_name, payload) {
                        result.push_str(&format!("        case .{}:\n", swift_case_name));
                        result.push_str(
                            "            var container = encoder.singleValueContainer()\n",
                        );
                        result.push_str(&format!(
                            "            try container.encode({})\n",
                            swift_string_literal(original_variant_name)
                        ));
                    } else {
                        result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
                        result.push_str(
                            "            var container = encoder.container(keyedBy: CodingKeys.self)\n",
                        );
                        result.push_str(&format!(
                            "            try container.encode(data, forKey: .{})\n",
                            swift_case_name
                        ));
                    }
                }
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n");
    result.push_str("}\n");

    Ok(result)
}
