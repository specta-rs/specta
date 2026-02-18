use crate::{Error, JsonSchema};
use serde_json::{Map, Value, json};
use specta::{
    TypeCollection,
    datatype::{NamedDataType, skip_fields, skip_fields_named, *},
};

/// Convert a NamedDataType to a JSON Schema definition
pub fn export(
    js: &JsonSchema,
    types: &TypeCollection,
    ndt: &NamedDataType,
) -> Result<Value, Error> {
    datatype_to_schema(js, types, ndt.ty(), true)
}

/// Convert a DataType to a JSON Schema, optionally as a reference
pub fn datatype_to_schema(
    js: &JsonSchema,
    types: &TypeCollection,
    dt: &DataType,
    is_definition: bool,
) -> Result<Value, Error> {
    match dt {
        // Primitives
        DataType::Primitive(p) => Ok(primitive_to_schema(p)),

        // Nullable
        DataType::Nullable(inner) => {
            let inner_schema = datatype_to_schema(js, types, inner, false)?;
            Ok(json!({
                "anyOf": [
                    inner_schema,
                    {"type": "null"}
                ]
            }))
        }

        // List/Array
        DataType::List(list) => {
            let items = datatype_to_schema(js, types, list.ty(), false)?;

            if let Some(len) = list.length() {
                // Fixed-length array (tuple-like)
                Ok(json!({
                    "type": "array",
                    "items": items,
                    "minItems": len,
                    "maxItems": len
                }))
            } else {
                // Variable-length array
                Ok(json!({
                    "type": "array",
                    "items": items
                }))
            }
        }

        // Map
        DataType::Map(map) => {
            let value_schema = datatype_to_schema(js, types, map.value_ty(), false)?;

            // JSON Schema uses additionalProperties for maps
            Ok(json!({
                "type": "object",
                "additionalProperties": value_schema
            }))
        }

        // Struct
        DataType::Struct(s) => struct_to_schema(js, types, s, is_definition),

        // Enum
        DataType::Enum(e) => enum_to_schema(js, types, e),

        // Tuple
        DataType::Tuple(t) => tuple_to_schema(js, types, t),

        // Reference
        DataType::Reference(r) => {
            match r {
                Reference::Named(r) => {
                    if is_definition {
                        // When exporting a definition, inline it
                        if let Some(referenced_ndt) = r.get(types) {
                            datatype_to_schema(js, types, referenced_ndt.ty(), true)
                        } else {
                            Err(Error::InvalidReference(
                                "Reference not found in TypeCollection".to_string(),
                            ))
                        }
                    } else {
                        // Use $ref for references
                        let defs_key = js.schema_version.definitions_key();
                        if let Some(referenced_ndt) = r.get(types) {
                            Ok(json!({
                                "$ref": format!("#/{}/{}", defs_key, referenced_ndt.name())
                            }))
                        } else {
                            Err(Error::InvalidReference(
                                "Reference not found in TypeCollection".to_string(),
                            ))
                        }
                    }
                }
                Reference::Opaque(_) => Err(Error::UnsupportedDataType(
                    "Opaque references are not supported by JSON Schema exporter".to_string(),
                )),
            }
        }

        // Generic
        DataType::Generic(_g) => {
            // JSON Schema doesn't have generics, so we use a placeholder
            // This should typically be resolved before export
            Ok(json!({})) // Empty schema accepts anything
        }
    }
}

fn primitive_to_schema(p: &Primitive) -> Value {
    match p {
        Primitive::bool => json!({"type": "boolean"}),
        Primitive::String => json!({"type": "string"}),
        Primitive::char => json!({"type": "string", "minLength": 1, "maxLength": 1}),

        // Integers
        Primitive::i8 => json!({"type": "integer", "minimum": i8::MIN, "maximum": i8::MAX}),
        Primitive::i16 => json!({"type": "integer", "minimum": i16::MIN, "maximum": i16::MAX}),
        Primitive::i32 => json!({"type": "integer", "format": "int32"}),
        Primitive::i64 => json!({"type": "integer", "format": "int64"}),
        Primitive::i128 => json!({"type": "integer"}),
        Primitive::isize => json!({"type": "integer"}),

        Primitive::u8 => json!({"type": "integer", "minimum": 0, "maximum": u8::MAX}),
        Primitive::u16 => json!({"type": "integer", "minimum": 0, "maximum": u16::MAX}),
        Primitive::u32 => json!({"type": "integer", "minimum": 0, "format": "uint32"}),
        Primitive::u64 => json!({"type": "integer", "minimum": 0, "format": "uint64"}),
        Primitive::u128 => json!({"type": "integer", "minimum": 0}),
        Primitive::usize => json!({"type": "integer", "minimum": 0}),

        // Floats
        Primitive::f16 => json!({"type": "number", "format": "float16"}),
        Primitive::f32 => json!({"type": "number", "format": "float"}),
        Primitive::f64 => json!({"type": "number", "format": "double"}),
    }
}

fn struct_to_schema(
    js: &JsonSchema,
    types: &TypeCollection,
    s: &Struct,
    _is_definition: bool,
) -> Result<Value, Error> {
    match s.fields() {
        Fields::Unit => {
            // Unit struct = null
            Ok(json!({"type": "null"}))
        }
        Fields::Unnamed(fields) => {
            // Tuple struct - represent as array
            let items: Result<Vec<_>, _> = skip_fields(fields.fields())
                .map(|(_, ty)| datatype_to_schema(js, types, ty, false))
                .collect();

            let items = items?;
            Ok(json!({
                "type": "array",
                "prefixItems": items,
                "items": false,
                "minItems": items.len(),
                "maxItems": items.len()
            }))
        }
        Fields::Named(fields) => {
            // Named fields = object
            let mut properties = Map::new();
            let mut required = Vec::new();

            for (name, (field, ty)) in skip_fields_named(fields.fields()) {
                let schema = datatype_to_schema(js, types, ty, false)?;
                properties.insert(name.clone().into_owned(), schema);

                if !field.optional() {
                    required.push(Value::String(name.clone().into_owned()));
                }
            }

            let mut obj = json!({
                "type": "object",
                "properties": properties
            });

            if !required.is_empty() {
                obj.as_object_mut()
                    .unwrap()
                    .insert("required".to_string(), Value::Array(required));
            }

            Ok(obj)
        }
    }
}

fn enum_to_schema(js: &JsonSchema, types: &TypeCollection, e: &Enum) -> Result<Value, Error> {
    let variants: Result<Vec<_>, _> = e
        .variants()
        .iter()
        .filter(|(_, variant)| !variant.skip())
        .map(|(name, variant)| variant_to_schema(js, types, name, variant))
        .collect();

    let variants = variants?;

    if variants.is_empty() {
        return Err(Error::ConversionError(
            "Enum has no non-skipped variants".to_string(),
        ));
    }

    if variants.len() == 1 {
        Ok(variants.into_iter().next().unwrap())
    } else {
        Ok(json!({
            "anyOf": variants
        }))
    }
}

fn variant_to_schema(
    js: &JsonSchema,
    types: &TypeCollection,
    name: &str,
    variant: &EnumVariant,
) -> Result<Value, Error> {
    // Get enum representation from attributes
    // For now, default to external tagging

    match variant.fields() {
        Fields::Unit => {
            // Unit variant = string literal
            Ok(json!({"const": name}))
        }
        Fields::Unnamed(fields) => {
            // Tuple variant with external tagging: { "VariantName": [...] }
            let items: Result<Vec<_>, _> = skip_fields(fields.fields())
                .map(|(_, ty)| datatype_to_schema(js, types, ty, false))
                .collect();

            let items = items?;

            if items.len() == 1 {
                // Single item - unwrap the array
                Ok(json!({
                    "type": "object",
                    "required": [name],
                    "properties": {
                        name: items[0].clone()
                    },
                    "additionalProperties": false
                }))
            } else {
                Ok(json!({
                    "type": "object",
                    "required": [name],
                    "properties": {
                        name: {
                            "type": "array",
                            "prefixItems": items.clone(),
                            "items": false,
                            "minItems": items.len(),
                            "maxItems": items.len()
                        }
                    },
                    "additionalProperties": false
                }))
            }
        }
        Fields::Named(fields) => {
            // Named variant with external tagging: { "VariantName": {...} }
            let mut properties = Map::new();
            let mut required = Vec::new();

            for (field_name, (field, ty)) in skip_fields_named(fields.fields()) {
                let schema = datatype_to_schema(js, types, ty, false)?;
                properties.insert(field_name.clone().into_owned(), schema);

                if !field.optional() {
                    required.push(Value::String(field_name.clone().into_owned()));
                }
            }

            let mut inner_obj = json!({
                "type": "object",
                "properties": properties
            });

            if !required.is_empty() {
                inner_obj
                    .as_object_mut()
                    .unwrap()
                    .insert("required".to_string(), Value::Array(required));
            }

            Ok(json!({
                "type": "object",
                "required": [name],
                "properties": {
                    name: inner_obj
                },
                "additionalProperties": false
            }))
        }
    }
}

fn tuple_to_schema(js: &JsonSchema, types: &TypeCollection, t: &Tuple) -> Result<Value, Error> {
    if t.elements().is_empty() {
        // Empty tuple = null
        return Ok(json!({"type": "null"}));
    }

    let items: Result<Vec<_>, _> = t
        .elements()
        .iter()
        .map(|ty| datatype_to_schema(js, types, ty, false))
        .collect();

    let items = items?;
    Ok(json!({
        "type": "array",
        "prefixItems": items.clone(),
        "items": false,
        "minItems": items.len(),
        "maxItems": items.len()
    }))
}
