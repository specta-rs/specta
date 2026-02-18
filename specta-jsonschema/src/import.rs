use crate::Error;
use schemars::Schema;
use serde_json::{Map as JsonMap, Value};
use specta::datatype::*;
use std::borrow::Cow;

/// Convert a schemars Schema to a Specta DataType
pub fn from_schema(schema: &Schema) -> Result<DataType, Error> {
    match schema.as_bool() {
        Some(true) => {
            // True schema = any
            // We use an empty struct as a placeholder for "any"
            Ok(Struct::named().build())
        }
        Some(false) => {
            // False schema = never type (nothing validates)
            Err(Error::ConversionError(
                "false schema (never type) not supported".into(),
            ))
        }
        None => schema
            .as_object()
            .ok_or_else(|| Error::ConversionError("schema must be object or bool".into()))
            .and_then(schema_object_to_datatype),
    }
}

fn value_to_datatype(value: &Value) -> Result<DataType, Error> {
    let schema: &Schema = value.try_into()?;
    from_schema(schema)
}

fn schema_object_to_datatype(obj: &JsonMap<String, Value>) -> Result<DataType, Error> {
    // Handle $ref
    if let Some(reference) = obj.get("$ref").and_then(Value::as_str) {
        // We use an opaque reference since we do not have a TypeCollection context here.
        return Ok(DataType::Reference(Reference::opaque(reference.to_owned())));
    }

    // Handle const values (literals)
    if obj.get("const").is_some() {
        // Specta does not currently expose a direct literal DataType variant.
        return Ok(DataType::Primitive(Primitive::String));
    }

    // Handle enum values (for string enums)
    if let Some(enum_values) = obj.get("enum").and_then(Value::as_array)
        && enum_values.iter().all(|v| v.is_string())
    {
        let mut e = Enum::new();
        for value in enum_values {
            if let Some(s) = value.as_str() {
                let variant = EnumVariant::unit();
                e.variants_mut().push((Cow::Owned(s.to_string()), variant));
            }
        }
        return Ok(DataType::Enum(e));
    }

    // Handle anyOf / oneOf (union types)
    if let Some(any_of) = obj.get("anyOf").and_then(Value::as_array) {
        return handle_any_of(any_of);
    }

    if let Some(one_of) = obj.get("oneOf").and_then(Value::as_array) {
        return handle_any_of(one_of);
    }

    // Handle type-based schemas
    if let Some(instance_type) = obj.get("type") {
        return instance_type_to_datatype(instance_type, obj);
    }

    // No type specified - return empty struct (acts like "any")
    Ok(Struct::named().build())
}

fn instance_type_to_datatype(
    instance_type: &Value,
    obj: &JsonMap<String, Value>,
) -> Result<DataType, Error> {
    match instance_type {
        Value::String(t) => instance_type_name_to_datatype(t, obj),
        Value::Array(types) => {
            // Multiple types - create a union (enum with unnamed variants)
            let mut e = Enum::new();

            for (i, item) in types.iter().enumerate() {
                if let Value::String(t) = item {
                    let dt = instance_type_name_to_datatype(t, obj)?;
                    let variant = EnumVariant::unnamed().field(Field::new(dt)).build();
                    e.variants_mut()
                        .push((Cow::Owned(format!("Variant{}", i)), variant));
                }
            }

            Ok(DataType::Enum(e))
        }
        _ => Err(Error::ConversionError(
            "schema `type` must be a string or array".into(),
        )),
    }
}

fn instance_type_name_to_datatype(
    instance_type: &str,
    obj: &JsonMap<String, Value>,
) -> Result<DataType, Error> {
    match instance_type {
        "null" => {
            // Null type - use empty tuple
            Ok(DataType::Tuple(Tuple::new(vec![])))
        }
        "boolean" => Ok(DataType::Primitive(Primitive::bool)),
        "string" => Ok(DataType::Primitive(Primitive::String)),
        "number" => {
            if let Some(format) = obj.get("format").and_then(Value::as_str) {
                match format {
                    "float" => Ok(DataType::Primitive(Primitive::f32)),
                    "double" => Ok(DataType::Primitive(Primitive::f64)),
                    _ => Ok(DataType::Primitive(Primitive::f64)),
                }
            } else {
                Ok(DataType::Primitive(Primitive::f64))
            }
        }
        "integer" => {
            if let Some(format) = obj.get("format").and_then(Value::as_str) {
                match format {
                    "int32" => Ok(DataType::Primitive(Primitive::i32)),
                    "int64" => Ok(DataType::Primitive(Primitive::i64)),
                    "uint32" => Ok(DataType::Primitive(Primitive::u32)),
                    "uint64" => Ok(DataType::Primitive(Primitive::u64)),
                    _ => Ok(DataType::Primitive(Primitive::i32)),
                }
            } else {
                Ok(DataType::Primitive(Primitive::i32))
            }
        }
        "array" => {
            if let Some(items) = obj.get("items") {
                match items {
                    Value::Object(_) | Value::Bool(_) => {
                        let item_dt = value_to_datatype(items)?;
                        Ok(DataType::List(List::new(item_dt)))
                    }
                    Value::Array(schemas) => {
                        // Tuple with specific items
                        let elements: Result<Vec<_>, _> =
                            schemas.iter().map(value_to_datatype).collect();
                        Ok(DataType::Tuple(Tuple::new(elements?)))
                    }
                    _ => Err(Error::ConversionError(
                        "array `items` must be a schema or list of schemas".into(),
                    )),
                }
            } else {
                // Array without items = array of empty struct (any)
                Ok(DataType::List(List::new(Struct::named().build())))
            }
        }
        "object" => {
            if let Some(properties) = obj.get("properties").and_then(Value::as_object)
                && !properties.is_empty()
            {
                // Build struct from properties
                let mut builder = Struct::named();

                let required: Vec<&str> = obj
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();

                for (name, schema) in properties {
                    let dt = value_to_datatype(schema)?;
                    let is_optional = !required.contains(&name.as_str());

                    let mut field = Field::new(dt);
                    field.set_optional(is_optional);

                    builder.field_mut(Cow::Owned(name.clone()), field);
                }

                return Ok(builder.build());
            }

            if let Some(additional) = obj.get("additionalProperties") {
                match additional {
                    Value::Object(_) => {
                        let value_dt = value_to_datatype(additional)?;
                        return Ok(DataType::Map(Map::new(
                            DataType::Primitive(Primitive::String),
                            value_dt,
                        )));
                    }
                    Value::Bool(true) => {
                        return Ok(DataType::Map(Map::new(
                            DataType::Primitive(Primitive::String),
                            Struct::named().build(),
                        )));
                    }
                    Value::Bool(false) => {}
                    _ => {
                        return Err(Error::ConversionError(
                            "`additionalProperties` must be a boolean or schema".into(),
                        ));
                    }
                }
            }

            Ok(Struct::named().build())
        }
        _ => Ok(Struct::named().build()),
    }
}

fn handle_any_of(schemas: &[Value]) -> Result<DataType, Error> {
    // Check if it's a nullable pattern (type | null)
    if schemas.len() == 2 {
        let is_null = |s: &Value| {
            s.as_object()
                .and_then(|obj| obj.get("type"))
                .is_some_and(|ty| matches!(ty, Value::String(t) if t == "null"))
        };

        if is_null(&schemas[0]) {
            return Ok(DataType::Nullable(Box::new(value_to_datatype(&schemas[1])?)));
        }

        if is_null(&schemas[1]) {
            return Ok(DataType::Nullable(Box::new(value_to_datatype(&schemas[0])?)));
        }
    }

    // General anyOf - create enum with unnamed variants
    let mut e = Enum::new();
    for (i, schema) in schemas.iter().enumerate() {
        let dt = value_to_datatype(schema)?;
        let variant = EnumVariant::unnamed().field(Field::new(dt)).build();
        e.variants_mut()
            .push((Cow::Owned(format!("Variant{}", i)), variant));
    }

    Ok(DataType::Enum(e))
}
