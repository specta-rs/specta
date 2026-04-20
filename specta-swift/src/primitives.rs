//! Primitive type conversion from Rust to Swift.

use specta::{
    Types,
    datatype::{DataType, Enum, Fields, Generic, Primitive, Reference, Variant},
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
    types: &Types,
    variant_name: &str,
    payload: &DataType,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    let payload = normalized_payload(variant_name, payload);

    Ok(match payload {
        DataType::Tuple(tuple) if tuple.elements.len() > 1 => tuple
            .elements
            .iter()
            .map(|element| datatype_to_swift(swift, types, element, generic_scope.to_vec(), None))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", "),
        _ => datatype_to_swift(swift, types, payload, generic_scope.to_vec(), None)?,
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

fn wrapper_variant_fields<'a>(variant_name: &str, variant: &'a Variant) -> Option<&'a Fields> {
    if let Some(payload) = serde_variant_payload(variant_name, variant) {
        let DataType::Struct(strct) = normalized_payload(variant_name, payload) else {
            return None;
        };

        return Some(&strct.fields);
    }

    Some(&variant.fields)
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
    types: &Types,
    ndt: &specta::datatype::NamedDataType,
) -> Result<String, Error> {
    if !matches!(&ndt.ty, DataType::Struct(_) | DataType::Enum(_)) {
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

    let generic_scope = ndt.generics.to_vec();

    // Format based on type
    match &ndt.ty {
        DataType::Struct(s) => {
            let type_def = struct_to_swift(swift, types, s, generic_scope.clone(), None)?;
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

            result.push_str(&format!("public struct {}{}: Codable {{\n", name, generics));
            result.push_str(&type_def);
            result.push('}');
        }
        DataType::Enum(e) => {
            let formatted_enum = match apply_datatype_format(swift, types, &ndt.ty)? {
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

            // Check if this is a string enum
            let is_string_enum_val = resolved_string_enum(e).is_some();

            // Check if this enum has struct-like variants (needs custom Codable)
            let has_struct_variants = e
                .variants
                .iter()
                .any(|(variant_name, variant)| should_emit_variant_wrapper(variant_name, variant));

            let has_serde_payload_variants = swift.format.is_some()
                && e.variants
                    .iter()
                    .any(|(variant_name, variant)| !is_unit_like_variant(variant_name, variant));

            let needs_custom_codable = has_struct_variants || has_serde_payload_variants;

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
                enum_to_swift(swift, types, e, generic_scope.clone(), None, Some(&name))?;
            result.push_str(&enum_body);
            result.push('}');

            // Generate struct definitions for named field variants
            let struct_definitions =
                generate_enum_structs(swift, types, e, generic_scope.clone(), None, &name)?;
            result.push_str(&struct_definitions);

            // Generate custom Codable implementation for enums with struct variants
            if needs_custom_codable {
                let codable_impl =
                    generate_enum_codable_impl(swift, types, e, generic_scope.clone(), &name)?;
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
    types: &Types,
    dt: &DataType,
    generic_scope: Vec<Generic>,
    reference: Option<&specta::datatype::Reference>,
) -> Result<String, Error> {
    let dt = apply_datatype_format(swift, types, dt)?;

    // Check for special standard library types first
    if let Some(special_type) = is_special_std_type(types, reference) {
        return Ok(special_type);
    }

    match &dt {
        DataType::Primitive(p) => primitive_to_swift(p),
        // DataType::Literal(l) => literal_to_swift(l),
        DataType::List(l) => list_to_swift(swift, types, l, generic_scope.clone()),
        DataType::Map(m) => map_to_swift(swift, types, m, generic_scope.clone()),
        DataType::Nullable(def) => {
            let inner = datatype_to_swift(swift, types, def, generic_scope, None)?;
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
            struct_to_swift(swift, types, s, generic_scope, None)
        }
        DataType::Enum(e) => enum_to_swift(swift, types, e, generic_scope, None, None),
        DataType::Tuple(t) => tuple_to_swift(swift, types, t, generic_scope.clone()),
        DataType::Reference(r) => reference_to_swift(swift, types, r, &generic_scope),
    }
}

fn apply_datatype_format(swift: &Swift, types: &Types, dt: &DataType) -> Result<DataType, Error> {
    if contains_generic_reference(dt) {
        return apply_datatype_format_children(swift, types, dt.clone());
    }

    let Some(format) = swift.format.as_ref() else {
        return Ok(dt.clone());
    };

    let mapped = (format.datatype)(types, dt)
        .map_err(|err| Error::format("datatype formatter failed", err))?;

    match mapped {
        std::borrow::Cow::Borrowed(dt) => apply_datatype_format_children(swift, types, dt.clone()),
        std::borrow::Cow::Owned(dt) => apply_datatype_format_children(swift, types, dt),
    }
}

fn apply_datatype_format_children(
    swift: &Swift,
    types: &Types,
    mut dt: DataType,
) -> Result<DataType, Error> {
    match &mut dt {
        DataType::Primitive(_) => {}
        DataType::List(list) => {
            list.ty = Box::new(apply_datatype_format(swift, types, &list.ty)?);
        }
        DataType::Map(map) => {
            let key = apply_datatype_format(swift, types, map.key_ty())?;
            let value = apply_datatype_format(swift, types, map.value_ty())?;
            map.set_key_ty(key);
            map.set_value_ty(value);
        }
        DataType::Nullable(inner) => {
            **inner = apply_datatype_format(swift, types, inner)?;
        }
        DataType::Struct(strct) => map_fields(swift, types, &mut strct.fields)?,
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                map_fields(swift, types, &mut variant.fields)?;
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                *element = apply_datatype_format(swift, types, element)?;
            }
        }
        DataType::Reference(Reference::Named(reference)) => {
            for (_, generic) in &mut reference.generics {
                *generic = apply_datatype_format(swift, types, generic)?;
            }
        }
        DataType::Reference(Reference::Generic(_) | Reference::Opaque(_)) => {}
    }

    Ok(dt)
}

fn map_fields(swift: &Swift, types: &Types, fields: &mut Fields) -> Result<(), Error> {
    match fields {
        Fields::Unit => {}
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = apply_datatype_format(swift, types, ty)?;
                }
            }
        }
        Fields::Named(named) => {
            for (_, field) in &mut named.fields {
                if let Some(ty) = field.ty.as_mut() {
                    *ty = apply_datatype_format(swift, types, ty)?;
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
        DataType::Reference(Reference::Named(reference)) => reference
            .generics
            .iter()
            .any(|(_, generic)| contains_generic_reference(generic)),
        DataType::Reference(Reference::Generic(_)) => true,
        DataType::Reference(Reference::Opaque(_)) => false,
    }
}

fn fields_contain_generic_reference(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => false,
        Fields::Unnamed(unnamed) => unnamed
            .fields
            .iter()
            .any(|field| field.ty.as_ref().is_some_and(|ty| contains_generic_reference(ty))),
        Fields::Named(named) => named
            .fields
            .iter()
            .any(|(_, field)| field.ty.as_ref().is_some_and(|ty| contains_generic_reference(ty))),
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

/// Check if a type is a special standard library type that needs special handling
fn is_special_std_type(
    types: &Types,
    reference: Option<&specta::datatype::Reference>,
) -> Option<String> {
    if let Some(Reference::Named(r)) = reference
        && let Some(ndt) = r.get(types)
    {
        // Check for std::time::Duration
        if &ndt.name == "Duration" {
            return Some("RustDuration".to_string());
        }
        // Check for std::time::SystemTime
        if &ndt.name == "SystemTime" {
            return Some("Date".to_string());
        }
    }
    None
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
    types: &Types,
    list: &specta::datatype::List,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let element_type = datatype_to_swift(swift, types, &list.ty, generic_scope, None)?;
    Ok(format!("[{}]", element_type))
}

/// Convert map types to Swift dictionaries.
fn map_to_swift(
    swift: &Swift,
    types: &Types,
    map: &specta::datatype::Map,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    let key_type = datatype_to_swift(swift, types, map.key_ty(), generic_scope.clone(), None)?;
    let value_type = datatype_to_swift(swift, types, map.value_ty(), generic_scope, None)?;
    Ok(format!("[{}: {}]", key_type, value_type))
}

/// Convert struct types to Swift.
fn struct_to_swift(
    swift: &Swift,
    types: &Types,
    s: &specta::datatype::Struct,
    generic_scope: Vec<Generic>,
    _reference: Option<&specta::datatype::Reference>,
) -> Result<String, Error> {
    match &s.fields {
        specta::datatype::Fields::Unit => Ok("Void".to_string()),
        specta::datatype::Fields::Unnamed(fields) => {
            if fields.fields.is_empty() {
                Ok("Void".to_string())
            } else if fields.fields.len() == 1 {
                // Single field tuple struct - convert to a proper struct with a 'value' field
                let field_type = datatype_to_swift(
                    swift,
                    types,
                    fields.fields[0]
                        .ty
                        .as_ref()
                        .expect("tuple field should have a type"),
                    generic_scope,
                    None,
                )?;
                Ok(format!("    let value: {}\n", field_type))
            } else {
                // Multiple field tuple struct - convert to a proper struct with numbered fields
                let mut result = String::new();
                for (i, field) in fields.fields.iter().enumerate() {
                    let field_type = datatype_to_swift(
                        swift,
                        types,
                        field.ty.as_ref().expect("tuple field should have a type"),
                        generic_scope.clone(),
                        None,
                    )?;
                    result.push_str(&format!("    public let field{}: {}\n", i, field_type));
                }
                Ok(result)
            }
        }
        specta::datatype::Fields::Named(fields) => {
            let mut result = String::new();
            let mut field_mappings = Vec::new();

            for (original_field_name, field) in &fields.fields {
                let field_type = if let Some(ty) = field.ty.as_ref() {
                    datatype_to_swift(swift, types, ty, generic_scope.clone(), None)?
                } else {
                    continue;
                };

                let optional_marker = if field.optional { "?" } else { "" };
                let swift_field_name = swift.naming.convert_field(original_field_name.as_ref());

                result.push_str(&format!(
                    "    public let {}: {}{}\n",
                    swift_field_name, field_type, optional_marker
                ));

                field_mappings.push((swift_field_name, original_field_name.to_string()));
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

            Ok(result)
        }
    }
}

/// Convert enum types to Swift.
fn enum_to_swift(
    swift: &Swift,
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    _reference: Option<&specta::datatype::Reference>,
    enum_name: Option<&str>,
) -> Result<String, Error> {
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
                    let raw_value =
                        enum_string_raw_value(variant).unwrap_or_else(|| original_variant_name.as_ref());
                    result.push_str(&format!("    case {} = \"{}\"\n", variant_name, raw_value));
                } else if fields.fields.len() == 1
                    && fields.fields[0]
                        .ty
                        .as_ref()
                        .is_some_and(|ty| is_unit_payload(original_variant_name, ty))
                {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else if fields.fields.is_empty() {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else {
                    let types_str = fields
                        .fields
                        .iter()
                        .map(|f| {
                            datatype_to_swift(
                                swift,
                                types,
                                f.ty.as_ref()
                                    .expect("enum variant field should have a type"),
                                generic_scope.clone(),
                                None,
                            )
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?
                        .join(", ");
                    result.push_str(&format!("    case {}({})\n", variant_name, types_str));
                }
            }
            specta::datatype::Fields::Named(fields) => {
                if fields.fields.is_empty() {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else if !should_emit_variant_wrapper(original_variant_name, variant) {
                    let payload = serde_variant_payload(original_variant_name, variant)
                        .expect("serde payload variants should contain a payload");

                    if is_unit_payload(original_variant_name, payload) {
                        result.push_str(&format!("    case {}\n", variant_name));
                    } else {
                        let payload_ty = enum_payload_to_swift_type(
                            swift,
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
                    let pascal_variant_name = to_pascal_case(original_variant_name);
                    let struct_name = if let Some(enum_name) = enum_name {
                        format!("{}{}Data", enum_name, pascal_variant_name)
                    } else {
                        format!("{}Data", pascal_variant_name)
                    };

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
    types: &Types,
    e: &specta::datatype::Enum,
    generic_scope: Vec<Generic>,
    _reference: Option<&specta::datatype::Reference>,
    enum_name: &str,
) -> Result<String, Error> {
    let mut result = String::new();

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        if let Some(Fields::Named(fields)) = wrapper_variant_fields(original_variant_name, variant)
            && !fields.fields.is_empty()
            && should_emit_variant_wrapper(original_variant_name, variant)
        {
            let pascal_variant_name = to_pascal_case(original_variant_name.as_ref());
            let struct_name = format!("{}{}Data", enum_name, pascal_variant_name);

            // Generate struct definition with custom CodingKeys for field name mapping
            result.push_str(&format!("\npublic struct {}: Codable {{\n", struct_name));

            // Generate struct fields
            let mut field_mappings = Vec::new();
            for (original_field_name, field) in &fields.fields {
                if let Some(ty) = field.ty.as_ref() {
                    let field_type =
                        datatype_to_swift(swift, types, ty, generic_scope.clone(), None)?;
                    let optional_marker = if field.optional { "?" } else { "" };
                    let swift_field_name = swift.naming.convert_field(original_field_name.as_ref());
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
    types: &Types,
    t: &specta::datatype::Tuple,
    generic_scope: Vec<Generic>,
) -> Result<String, Error> {
    if t.elements.is_empty() {
        Ok("Void".to_string())
    } else if t.elements.len() == 1 {
        datatype_to_swift(swift, types, &t.elements[0], generic_scope, None)
    } else {
        let types_str = t
            .elements
            .iter()
            .map(|e| datatype_to_swift(swift, types, e, generic_scope.clone(), None))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", ");
        Ok(format!("({})", types_str))
    }
}

/// Convert reference types to Swift.
fn reference_to_swift(
    swift: &Swift,
    types: &Types,
    r: &specta::datatype::Reference,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    match r {
        Reference::Named(r) => {
            let Some(ndt) = r.get(types) else {
                return Err(Error::InvalidIdentifier(
                    "Reference to unknown type".to_string(),
                ));
            };

            if ndt.name == "String" {
                return Ok("String".to_string());
            }

            if matches!(ndt.name.as_ref(), "Uuid" | "DateTime" | "NaiveDateTime") {
                return Ok("String".to_string());
            }

            if ndt.name == "Vec"
                && let Some((_, inner_ty)) = r.generics.first()
            {
                let inner =
                    datatype_to_swift(swift, types, inner_ty, generic_scope.to_vec(), None)?;
                return Ok(format!("[{inner}]"));
            }

            let name = swift.naming.convert(&ndt.name);

            if r.generics.is_empty() {
                Ok(name)
            } else {
                let generics = r
                    .generics
                    .iter()
                    .map(|(_, t)| datatype_to_swift(swift, types, t, generic_scope.to_vec(), None))
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}<{}>", name, generics))
            }
        }
        Reference::Opaque(_) => Err(Error::UnsupportedType(
            "Opaque references are not supported by Swift exporter".to_string(),
        )),
        Reference::Generic(g) => generic_to_swift(swift, g, generic_scope),
    }
}

/// Convert generic types to Swift.
fn generic_to_swift(
    _swift: &Swift,
    g: &specta::datatype::GenericReference,
    generic_scope: &[Generic],
) -> Result<String, Error> {
    generic_scope
        .iter()
        .find_map(|generic| (generic.reference() == *g).then(|| generic.name.to_string()))
        .ok_or_else(|| Error::GenericConstraint(format!("Unresolved generic reference: {g:?}")))
}

/// Generate custom Codable implementation for enums with struct-like variants
fn generate_enum_codable_impl(
    swift: &Swift,
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

    // Generate init(from decoder:)
    result.push_str("    public init(from decoder: Decoder) throws {\n");
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

        match &variant.fields {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!("            self = .{}\n", swift_case_name));
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if fields.fields.is_empty() {
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!("            self = .{}\n", swift_case_name));
                } else if fields.fields.len() == 1
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
                        types,
                        fields.fields[0]
                            .ty
                            .as_ref()
                            .expect("enum variant field should have a type"),
                        generic_scope.clone(),
                        None,
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
                            types,
                            original_variant_name,
                            payload,
                            &generic_scope,
                        )?;
                        result.push_str(&format!(
                            "            let data = try container.decode({}.self, forKey: .{})\n",
                            payload_ty, swift_case_name
                        ));
                        result.push_str(&format!("            self = .{}(data)\n", swift_case_name));
                    }
                }
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n\n");

    // Generate encode(to encoder:)
    result.push_str("    public func encode(to encoder: Encoder) throws {\n");
    result.push_str("        var container = encoder.container(keyedBy: CodingKeys.self)\n");
    result.push_str("        \n");
    result.push_str("        switch self {\n");

    for (original_variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }

        let swift_case_name = swift
            .naming
            .convert_enum_case(original_variant_name.as_ref());

        match &variant.fields {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            try container.encodeNil(forKey: .{})\n",
                    swift_case_name
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
                    result.push_str(&format!(
                        "            try container.encodeNil(forKey: .{})\n",
                        swift_case_name
                    ));
                    continue;
                } else if fields.fields.len() == 1 {
                    result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
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
                    result.push_str(&format!(
                        "            try container.encode(data, forKey: .{})\n",
                        swift_case_name
                    ));
                } else {
                    let payload = serde_variant_payload(original_variant_name, variant)
                        .expect("serde payload variants should contain a payload");

                    if is_unit_payload(original_variant_name, payload) {
                        result.push_str(&format!("        case .{}:\n", swift_case_name));
                        result.push_str(&format!(
                            "            try container.encodeNil(forKey: .{})\n",
                            swift_case_name
                        ));
                    } else {
                        result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
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
