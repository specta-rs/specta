//! Primitive type conversion from Rust to Swift.

use std::borrow::Cow;

use specta::{
    datatype::{DataType, Primitive},
    SpectaID, TypeCollection,
};

use crate::error::{Error, Result};
use crate::swift::Swift;

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
    let type_def = datatype_to_swift(swift, types, ndt.ty(), vec![], false, Some(ndt.sid()))?;

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

            result.push_str(&format!("struct {}{}: Codable {{\n", name, generics));
            result.push_str(&type_def);
            result.push_str("}");
        }
        DataType::Enum(_) => {
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

            result.push_str(&format!("enum {}{}: Codable {{\n", name, generics));
            result.push_str(&type_def);
            result.push_str("}");
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
    sid: Option<SpectaID>,
) -> Result<String> {
    match dt {
        DataType::Primitive(p) => primitive_to_swift(p),
        DataType::Literal(l) => literal_to_swift(l),
        DataType::List(l) => list_to_swift(swift, types, l),
        DataType::Map(m) => map_to_swift(swift, types, m),
        DataType::Nullable(def) => {
            let inner = datatype_to_swift(swift, types, def, location, is_export, sid)?;
            Ok(match swift.optionals {
                crate::swift::OptionalStyle::QuestionMark => format!("{}?", inner),
                crate::swift::OptionalStyle::Optional => format!("Optional<{}>", inner),
            })
        }
        DataType::Struct(s) => struct_to_swift(swift, types, s, location, is_export, sid),
        DataType::Enum(e) => enum_to_swift(swift, types, e, location, is_export, sid),
        DataType::Tuple(t) => tuple_to_swift(swift, types, t),
        DataType::Reference(r) => reference_to_swift(swift, types, r),
        DataType::Generic(g) => generic_to_swift(swift, g),
    }
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

/// Convert literal types to Swift.
fn literal_to_swift(literal: &specta::datatype::Literal) -> Result<String> {
    Ok(match literal {
        specta::datatype::Literal::i8(v) => v.to_string(),
        specta::datatype::Literal::i16(v) => v.to_string(),
        specta::datatype::Literal::i32(v) => v.to_string(),
        specta::datatype::Literal::u8(v) => v.to_string(),
        specta::datatype::Literal::u16(v) => v.to_string(),
        specta::datatype::Literal::u32(v) => v.to_string(),
        specta::datatype::Literal::f32(v) => v.to_string(),
        specta::datatype::Literal::f64(v) => v.to_string(),
        specta::datatype::Literal::bool(v) => v.to_string(),
        specta::datatype::Literal::String(s) => format!("\"{}\"", s),
        specta::datatype::Literal::char(c) => format!("\"{}\"", c),
        specta::datatype::Literal::None => "nil".to_string(),
        _ => {
            return Err(Error::UnsupportedType(
                "Unsupported literal type".to_string(),
            ))
        }
    })
}

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
    sid: Option<SpectaID>,
) -> Result<String> {
    match s.fields() {
        specta::datatype::Fields::Unit => Ok("Void".to_string()),
        specta::datatype::Fields::Unnamed(fields) => {
            if fields.fields().is_empty() {
                Ok("Void".to_string())
            } else if fields.fields().len() == 1 {
                datatype_to_swift(
                    swift,
                    types,
                    &fields.fields()[0].ty().unwrap(),
                    location,
                    is_export,
                    sid,
                )
            } else {
                let types_str = fields
                    .fields()
                    .iter()
                    .map(|f| {
                        datatype_to_swift(
                            swift,
                            types,
                            f.ty().unwrap(),
                            location.clone(),
                            is_export,
                            sid,
                        )
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("({})", types_str))
            }
        }
        specta::datatype::Fields::Named(fields) => {
            let mut result = String::new();

            for (name, field) in fields.fields() {
                let field_type = if let Some(ty) = field.ty() {
                    datatype_to_swift(swift, types, ty, location.clone(), is_export, sid)?
                } else {
                    continue;
                };

                let optional_marker = if field.optional() { "?" } else { "" };
                let field_name = swift.naming.convert_field(name);

                result.push_str(&format!(
                    "    let {}: {}{}\n",
                    field_name, field_type, optional_marker
                ));
            }

            Ok(result)
        }
    }
}

/// Convert enum types to Swift.
fn enum_to_swift(
    swift: &Swift,
    types: &TypeCollection,
    e: &specta::datatype::Enum,
    location: Vec<Cow<'static, str>>,
    is_export: bool,
    sid: Option<SpectaID>,
) -> Result<String> {
    let mut result = String::new();

    for (variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        let variant_name = swift.naming.convert_enum_case(variant_name);

        match variant.fields() {
            specta::datatype::Fields::Unit => {
                result.push_str(&format!("    case {}\n", variant_name));
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
                                f.ty().unwrap(),
                                location.clone(),
                                is_export,
                                sid,
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
                    let mut field_strs = Vec::new();
                    for (field_name, field) in fields.fields() {
                        let field_type = if let Some(ty) = field.ty() {
                            datatype_to_swift(swift, types, ty, location.clone(), is_export, sid)?
                        } else {
                            continue;
                        };
                        let optional_marker = if field.optional() { "?" } else { "" };
                        let field_name = swift.naming.convert_field(field_name);
                        field_strs
                            .push(format!("{}: {}{}", field_name, field_type, optional_marker));
                    }
                    result.push_str(&format!(
                        "    case {}({})\n",
                        variant_name,
                        field_strs.join(", ")
                    ));
                }
            }
        }
    }

    Ok(result)
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
    let name = if let Some(ndt) = types.get(r.sid()) {
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
