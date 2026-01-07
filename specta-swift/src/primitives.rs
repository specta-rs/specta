//! Primitive type conversion from Rust to Swift.

use std::borrow::Cow;

use specta::{
    TypeCollection,
    datatype::{DataType, Primitive},
};

use crate::error::{Error, Result};
use crate::swift::Swift;

/// Check if an enum is a string enum (has String repr)
fn is_string_enum(e: &specta::datatype::Enum) -> bool {
    // If all variants are unit variants, it's a string enum
    e.is_string_enum()
}

/// Helper function to get rename_all from serde attributes  
fn get_rename_all_from_attributes(
    attributes: &[specta::datatype::RuntimeAttribute],
) -> Option<String> {
    use specta::datatype::RuntimeMeta;

    for attr in attributes {
        if attr.path == "serde"
            && let RuntimeMeta::List(list) = &attr.kind
        {
            for nested in list {
                if let specta::datatype::RuntimeNestedMeta::Meta(meta) = nested
                    && let RuntimeMeta::NameValue { key, value } = meta
                    && key == "rename_all"
                    && let specta::datatype::RuntimeLiteral::Str(s) = value
                {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

/// Check if an enum is adjacently tagged
fn is_adjacently_tagged_enum(e: &specta::datatype::Enum) -> bool {
    use specta::datatype::RuntimeMeta;

    let mut has_tag = false;
    let mut has_content = false;

    for attr in e.attributes() {
        if attr.path == "serde"
            && let RuntimeMeta::List(list) = &attr.kind
        {
            for nested in list {
                if let specta::datatype::RuntimeNestedMeta::Meta(meta) = nested
                    && let RuntimeMeta::NameValue { key, .. } = meta
                {
                    if key == "tag" {
                        has_tag = true;
                    } else if key == "content" {
                        has_content = true;
                    }
                }
            }
        }
    }

    has_tag && has_content
}

/// Get the tag and content field names for an adjacently tagged enum
fn get_adjacent_tag_content(e: &specta::datatype::Enum) -> Option<(String, String)> {
    use specta::datatype::RuntimeMeta;

    let mut tag = None;
    let mut content = None;

    for attr in e.attributes() {
        if attr.path == "serde"
            && let RuntimeMeta::List(list) = &attr.kind
        {
            for nested in list {
                if let specta::datatype::RuntimeNestedMeta::Meta(meta) = nested
                    && let RuntimeMeta::NameValue { key, value } = meta
                {
                    if key == "tag" {
                        if let specta::datatype::RuntimeLiteral::Str(s) = value {
                            tag = Some(s.clone());
                        }
                    } else if key == "content"
                        && let specta::datatype::RuntimeLiteral::Str(s) = value
                    {
                        content = Some(s.clone());
                    }
                }
            }
        }
    }

    match (tag, content) {
        (Some(t), Some(c)) => Some((t, c)),
        _ => None,
    }
}

/// Export a single type to Swift.
pub fn export_type(
    swift: &Swift,
    types: &TypeCollection,
    ndt: &specta::datatype::NamedDataType,
) -> Result<String> {
    let mut result = String::new();

    // Add JSDoc-style comments if present
    if !ndt.docs().is_empty() {
        let docs = ndt.docs();
        // Handle multi-line comments properly
        for line in docs.lines() {
            result.push_str("/// ");
            // Trim leading whitespace from the line to avoid extra spaces
            result.push_str(line.trim_start());
            result.push('\n');
        }
    }

    // Add deprecated annotation if present
    if let Some(deprecated) = ndt.deprecated() {
        let message = match deprecated {
            specta::datatype::DeprecatedType::Deprecated => "This type is deprecated".to_string(),
            specta::datatype::DeprecatedType::DeprecatedWithSince { note, .. } => note.to_string(),
            _ => "This type is deprecated".to_string(),
        };
        result.push_str(&format!(
            "@available(*, deprecated, message: \"{}\")\n",
            message
        ));
    }

    // Generate the type definition
    let type_def = datatype_to_swift(swift, types, ndt.ty(), vec![], false, None)?;

    // Format based on type
    match ndt.ty() {
        DataType::Struct(_) => {
            let name = swift.naming.convert(ndt.name());
            let generics = if ndt.generics().is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    ndt.generics()
                        .iter()
                        .map(|g| g.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            result.push_str(&format!("public struct {}{}: Codable {{\n", name, generics));
            result.push_str(&type_def);
            result.push('}');
        }
        DataType::Enum(e) => {
            let name = swift.naming.convert(ndt.name());
            let generics = if ndt.generics().is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    ndt.generics()
                        .iter()
                        .map(|g| g.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            // Check if this is a string enum
            let is_string_enum_val = is_string_enum(e);

            // Check if this enum has struct-like variants (needs custom Codable)
            let has_struct_variants = e.variants().iter().any(|(_, variant)| {
                matches!(variant.fields(), specta::datatype::Fields::Named(fields) if !fields.fields().is_empty())
            });

            // Determine protocols based on whether we'll generate custom Codable
            let protocols = if is_string_enum_val {
                if has_struct_variants {
                    "String" // Custom Codable will be generated
                } else {
                    "String, Codable"
                }
            } else if has_struct_variants {
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
            let enum_body = enum_to_swift(swift, types, e, vec![], false, None, Some(&name))?;
            result.push_str(&enum_body);
            result.push('}');

            // Generate struct definitions for named field variants
            let struct_definitions =
                generate_enum_structs(swift, types, e, vec![], false, None, &name)?;
            result.push_str(&struct_definitions);

            // Generate custom Codable implementation for enums with struct variants
            if has_struct_variants {
                let codable_impl = generate_enum_codable_impl(swift, e, &name)?;
                result.push_str(&codable_impl);
            }
        }
        _ => {
            // For other types, just use the type definition
            result.push_str(&type_def);
        }
    }

    Ok(result)
}

/// Convert a DataType to Swift syntax.
pub fn datatype_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    dt: &DataType,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    reference: Option<&specta::datatype::Reference>,
) -> Result<String> {
    // Check for special standard library types first
    if let Some(special_type) = is_special_std_type(types, reference) {
        return Ok(special_type);
    }

    match dt {
        DataType::Primitive(p) => primitive_to_swift(p),
        // DataType::Literal(l) => literal_to_swift(l),
        DataType::List(l) => list_to_swift(swift, types, l),
        DataType::Map(m) => map_to_swift(swift, types, m),
        DataType::Nullable(def) => {
            let inner = datatype_to_swift(swift, types, def, location, is_export, None)?;
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
            struct_to_swift(swift, types, s, location, is_export, None)
        }
        DataType::Enum(e) => enum_to_swift(swift, types, e, location, is_export, None, None),
        DataType::Tuple(t) => tuple_to_swift(swift, types, t),
        DataType::Reference(r) => reference_to_swift(swift, types, r),
        DataType::Generic(g) => generic_to_swift(swift, g),
    }
}

/// Check if a struct is a Duration by examining its fields
pub fn is_duration_struct(s: &specta::datatype::Struct) -> bool {
    match s.fields() {
        specta::datatype::Fields::Named(fields) => {
            let field_names: Vec<String> = fields
                .fields()
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
    types: &TypeCollection,
    reference: Option<&specta::datatype::Reference>,
) -> Option<String> {
    if let Some(r) = reference
        && let Some(ndt) = r.get(types)
    {
        // Check for std::time::Duration
        if ndt.name() == "Duration" {
            return Some("RustDuration".to_string());
        }
        // Check for std::time::SystemTime
        if ndt.name() == "SystemTime" {
            return Some("Date".to_string());
        }
    }
    None
}

/// Convert primitive types to Swift.
fn primitive_to_swift(primitive: &Primitive) -> Result<String> {
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
        Primitive::String => "String".to_string(),
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
    })
}

// /// Convert literal types to Swift.
// fn literal_to_swift(literal: &specta::datatype::Literal) -> Result<String> {
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
    types: &TypeCollection,
    list: &specta::datatype::List,
) -> Result<String> {
    let element_type = datatype_to_swift(swift, types, list.ty(), vec![], false, None)?;
    Ok(format!("[{}]", element_type))
}

/// Convert map types to Swift dictionaries.
fn map_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    map: &specta::datatype::Map,
) -> Result<String> {
    let key_type = datatype_to_swift(swift, types, map.key_ty(), vec![], false, None)?;
    let value_type = datatype_to_swift(swift, types, map.value_ty(), vec![], false, None)?;
    Ok(format!("[{}: {}]", key_type, value_type))
}

/// Convert struct types to Swift.
fn struct_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    s: &specta::datatype::Struct,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    _reference: Option<&specta::datatype::Reference>,
) -> Result<String> {
    match s.fields() {
        specta::datatype::Fields::Unit => Ok("Void".to_string()),
        specta::datatype::Fields::Unnamed(fields) => {
            if fields.fields().is_empty() {
                Ok("Void".to_string())
            } else if fields.fields().len() == 1 {
                // Single field tuple struct - convert to a proper struct with a 'value' field
                let field_type = datatype_to_swift(
                    swift,
                    types,
                    fields.fields()[0]
                        .ty()
                        .expect("tuple field should have a type"),
                    location,
                    is_export,
                    None,
                )?;
                Ok(format!("    let value: {}\n", field_type))
            } else {
                // Multiple field tuple struct - convert to a proper struct with numbered fields
                let mut result = String::new();
                for (i, field) in fields.fields().iter().enumerate() {
                    let field_type = datatype_to_swift(
                        swift,
                        types,
                        field.ty().expect("tuple field should have a type"),
                        location.clone(),
                        is_export,
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

            for (original_field_name, field) in fields.fields() {
                let field_type = if let Some(ty) = field.ty() {
                    datatype_to_swift(swift, types, ty, location.clone(), is_export, None)?
                } else {
                    continue;
                };

                let optional_marker = if field.optional() { "?" } else { "" };
                let swift_field_name = swift.naming.convert_field(original_field_name);

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

/// Generate raw value for string enum variants
#[allow(clippy::unwrap_used)]
fn generate_raw_value(variant_name: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("lowercase") => variant_name.to_lowercase(),
        Some("UPPERCASE") => variant_name.to_uppercase(),
        Some("camelCase") => {
            let mut chars = variant_name.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_lowercase().chain(chars).collect(),
            }
        }
        Some("PascalCase") => {
            let mut chars = variant_name.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        }
        Some("snake_case") => variant_name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['_', c.to_lowercase().next().unwrap()]
                } else {
                    vec![c.to_lowercase().next().unwrap()]
                }
            })
            .collect(),
        Some("SCREAMING_SNAKE_CASE") => variant_name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['_', c.to_uppercase().next().unwrap()]
                } else {
                    vec![c.to_uppercase().next().unwrap()]
                }
            })
            .collect(),
        Some("kebab-case") => variant_name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['-', c.to_lowercase().next().unwrap()]
                } else {
                    vec![c.to_lowercase().next().unwrap()]
                }
            })
            .collect(),
        Some("SCREAMING-KEBAB-CASE") => variant_name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['-', c.to_uppercase().next().unwrap()]
                } else {
                    vec![c.to_uppercase().next().unwrap()]
                }
            })
            .collect(),
        _ => variant_name.to_lowercase(), // Default to lowercase
    }
}

/// Convert enum types to Swift.
fn enum_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    e: &specta::datatype::Enum,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    _reference: Option<&specta::datatype::Reference>,
    enum_name: Option<&str>,
) -> Result<String> {
    let mut result = String::new();

    // Check if this is a string enum
    let is_string_enum = is_string_enum(e);

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let variant_name = swift.naming.convert_enum_case(original_variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                if is_string_enum {
                    // For string enums, generate raw value assignments
                    let raw_value = generate_raw_value(
                        original_variant_name,
                        get_rename_all_from_attributes(e.attributes()).as_deref(),
                    );
                    result.push_str(&format!("    case {} = \"{}\"\n", variant_name, raw_value));
                } else {
                    result.push_str(&format!("    case {}\n", variant_name));
                }
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if fields.fields().is_empty() {
                    result.push_str(&format!("    case {}\n", variant_name));
                } else {
                    let types_str = fields
                        .fields()
                        .iter()
                        .map(|f| {
                            datatype_to_swift(
                                swift,
                                types,
                                f.ty().expect("enum variant field should have a type"),
                                location.clone(),
                                is_export,
                                None,
                            )
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?
                        .join(", ");
                    result.push_str(&format!("    case {}({})\n", variant_name, types_str));
                }
            }
            specta::datatype::Fields::Named(fields) => {
                if fields.fields().is_empty() {
                    result.push_str(&format!("    case {}\n", variant_name));
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
    types: &TypeCollection,
    e: &specta::datatype::Enum,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    _reference: Option<&specta::datatype::Reference>,
    enum_name: &str,
) -> Result<String> {
    let mut result = String::new();

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        if let specta::datatype::Fields::Named(fields) = variant.fields()
            && !fields.fields().is_empty()
        {
            let pascal_variant_name = to_pascal_case(original_variant_name);
            let struct_name = format!("{}{}Data", enum_name, pascal_variant_name);

            // Generate struct definition with custom CodingKeys for field name mapping
            result.push_str(&format!("\npublic struct {}: Codable {{\n", struct_name));

            // Generate struct fields
            let mut field_mappings = Vec::new();
            for (original_field_name, field) in fields.fields() {
                if let Some(ty) = field.ty() {
                    let field_type =
                        datatype_to_swift(swift, types, ty, location.clone(), is_export, None)?;
                    let optional_marker = if field.optional() { "?" } else { "" };
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
    types: &TypeCollection,
    t: &specta::datatype::Tuple,
) -> Result<String> {
    if t.elements().is_empty() {
        Ok("Void".to_string())
    } else if t.elements().len() == 1 {
        datatype_to_swift(swift, types, &t.elements()[0], vec![], false, None)
    } else {
        let types_str = t
            .elements()
            .iter()
            .map(|e| datatype_to_swift(swift, types, e, vec![], false, None))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", ");
        Ok(format!("({})", types_str))
    }
}

/// Convert reference types to Swift.
fn reference_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    r: &specta::datatype::Reference,
) -> Result<String> {
    // Get the name from the TypeCollection using the SID
    let name = if let Some(ndt) = r.get(types) {
        swift.naming.convert(ndt.name())
    } else {
        return Err(Error::InvalidIdentifier(
            "Reference to unknown type".to_string(),
        ));
    };

    if r.generics().is_empty() {
        Ok(name)
    } else {
        let generics = r
            .generics()
            .iter()
            .map(|(_, t)| datatype_to_swift(swift, types, t, vec![], false, None))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join(", ");
        Ok(format!("{}<{}>", name, generics))
    }
}

/// Convert generic types to Swift.
fn generic_to_swift(_swift: &Swift, g: &specta::datatype::Generic) -> Result<String> {
    Ok(g.to_string())
}

/// Generate custom Codable implementation for enums with struct-like variants
fn generate_enum_codable_impl(
    swift: &Swift,
    e: &specta::datatype::Enum,
    enum_name: &str,
) -> Result<String> {
    let mut result = String::new();

    result.push_str(&format!(
        "\n// MARK: - {} Codable Implementation\n",
        enum_name
    ));
    result.push_str(&format!("extension {}: Codable {{\n", enum_name));

    // Check if this is an adjacently tagged enum
    let is_adjacently_tagged = is_adjacently_tagged_enum(e);

    if is_adjacently_tagged {
        return generate_adjacently_tagged_codable(swift, e, enum_name);
    }

    // Generate CodingKeys enum
    result.push_str("    private enum CodingKeys: String, CodingKey {\n");
    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }
        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);
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

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!("            self = .{}\n", swift_case_name));
            }
            specta::datatype::Fields::Unnamed(fields) => {
                if fields.fields().is_empty() {
                    result.push_str(&format!("        case .{}:\n", swift_case_name));
                    result.push_str(&format!("            self = .{}\n", swift_case_name));
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
                let pascal_variant_name = to_pascal_case(original_variant_name);
                let struct_name = format!("{}{}Data", enum_name, pascal_variant_name);

                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            let data = try container.decode({}.self, forKey: .{})\n",
                    struct_name, swift_case_name
                ));
                result.push_str(&format!("            self = .{}(data)\n", swift_case_name));
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

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            try container.encodeNil(forKey: .{})\n",
                    swift_case_name
                ));
            }
            specta::datatype::Fields::Unnamed(_) => {
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
                result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
                result.push_str(&format!(
                    "            try container.encode(data, forKey: .{})\n",
                    swift_case_name
                ));
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n");
    result.push_str("}\n");

    Ok(result)
}

/// Generate custom Codable implementation for adjacently tagged enums
fn generate_adjacently_tagged_codable(
    swift: &Swift,
    e: &specta::datatype::Enum,
    enum_name: &str,
) -> Result<String> {
    let mut result = String::new();

    // Get tag and content field names
    let (tag_field, content_field) = get_adjacent_tag_content(e)
        .ok_or_else(|| Error::UnsupportedType("Expected adjacently tagged enum".to_string()))?;

    result.push_str(&format!(
        "\n// MARK: - {} Adjacently Tagged Codable Implementation\n",
        enum_name
    ));
    result.push_str(&format!("extension {}: Codable {{\n", enum_name));

    // Generate TypeKeys enum for the tag and content fields
    result.push_str("    private enum TypeKeys: String, CodingKey {\n");
    result.push_str(&format!("        case tag = \"{}\"\n", tag_field));
    result.push_str(&format!("        case content = \"{}\"\n", content_field));
    result.push_str("    }\n\n");

    // Generate VariantType enum for variant names
    result.push_str("    private enum VariantType: String, Codable {\n");
    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }
        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);
        result.push_str(&format!(
            "        case {} = \"{}\"\n",
            swift_case_name, original_variant_name
        ));
    }
    result.push_str("    }\n\n");

    // Generate init(from decoder:)
    result.push_str("    public init(from decoder: Decoder) throws {\n");
    result.push_str("        let container = try decoder.container(keyedBy: TypeKeys.self)\n");
    result.push_str(
        "        let variantType = try container.decode(VariantType.self, forKey: .tag)\n",
    );
    result.push_str("        \n");
    result.push_str("        switch variantType {\n");

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!("            self = .{}\n", swift_case_name));
            }
            specta::datatype::Fields::Unnamed(_) => {
                // TODO: Handle tuple variants for adjacently tagged
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str("            fatalError(\"Adjacently tagged tuple variants not implemented\")\n");
            }
            specta::datatype::Fields::Named(_) => {
                let pascal_variant_name = to_pascal_case(original_variant_name);
                let struct_name = format!("{}{}Data", enum_name, pascal_variant_name);

                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            let data = try container.decode({}.self, forKey: .content)\n",
                    struct_name
                ));
                result.push_str(&format!("            self = .{}(data)\n", swift_case_name));
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n\n");

    // Generate encode(to encoder:)
    result.push_str("    public func encode(to encoder: Encoder) throws {\n");
    result.push_str("        var container = encoder.container(keyedBy: TypeKeys.self)\n");
    result.push_str("        \n");
    result.push_str("        switch self {\n");

    for (original_variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let swift_case_name = swift.naming.convert_enum_case(original_variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str(&format!(
                    "            try container.encode(VariantType.{}, forKey: .tag)\n",
                    swift_case_name
                ));
            }
            specta::datatype::Fields::Unnamed(_) => {
                // TODO: Handle tuple variants
                result.push_str(&format!("        case .{}:\n", swift_case_name));
                result.push_str("            fatalError(\"Adjacently tagged tuple variants not implemented\")\n");
            }
            specta::datatype::Fields::Named(_) => {
                result.push_str(&format!("        case .{}(let data):\n", swift_case_name));
                result.push_str(&format!(
                    "            try container.encode(VariantType.{}, forKey: .tag)\n",
                    swift_case_name
                ));
                result.push_str("            try container.encode(data, forKey: .content)\n");
            }
        }
    }

    result.push_str("        }\n");
    result.push_str("    }\n");
    result.push_str("}\n");

    Ok(result)
}
