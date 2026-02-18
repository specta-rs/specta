use crate::Error;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use specta::datatype::*;
use std::borrow::Cow;

/// Convert a schemars Schema to a Specta DataType
pub fn from_schema(schema: &Schema) -> Result<DataType, Error> {
    match schema {
        Schema::Bool(true) => {
            // True schema = any
            // We'll use an empty struct as a placeholder for "any"
            Ok(Struct::named().build())
        }
        Schema::Bool(false) => {
            // False schema = never type (nothing validates)
            Err(Error::ConversionError(
                "false schema (never type) not supported".into(),
            ))
        }
        Schema::Object(obj) => schema_object_to_datatype(obj),
    }
}

fn schema_object_to_datatype(obj: &SchemaObject) -> Result<DataType, Error> {
    // Handle $ref
    if let Some(reference) = &obj.reference {
        // Extract type name from $ref like "#/definitions/MyType" or "#/$defs/MyType"
        let _name = reference
            .split('/')
            .last()
            .ok_or_else(|| Error::ConversionError(format!("Invalid $ref: {}", reference)))?;

        // Create an opaque reference since we don't have the TypeCollection context
        todo!();
        // return `Ok(DataType::Reference(Reference::opaque()));
    }

    // Handle const values (literals)
    if let Some(_const_value) = &obj.const_value {
        // Since we don't have a Literal type, we'll use the type from instance_type if available
        // Otherwise, return a primitive based on the const value type
        return Ok(DataType::Primitive(Primitive::String));
    }

    // Handle enum values (for string enums)
    if let Some(enum_values) = &obj.enum_values {
        if enum_values.iter().all(|v| v.is_string()) {
            // String enum - create enum with unit variants
            let mut e = Enum::new();
            for value in enum_values {
                if let Some(s) = value.as_str() {
                    let variant = EnumVariant::unit();
                    e.variants_mut().push((Cow::Owned(s.to_string()), variant));
                }
            }
            return Ok(DataType::Enum(e));
        }
    }

    // Handle anyOf (union types)
    if let Some(subschemas) = &obj.subschemas {
        if let Some(any_of) = &subschemas.any_of {
            return handle_any_of(any_of);
        }

        if let Some(one_of) = &subschemas.one_of {
            return handle_any_of(one_of); // Treat similarly to anyOf
        }
    }

    // Handle type-based schemas
    if let Some(instance_type) = &obj.instance_type {
        return instance_type_to_datatype(instance_type, obj);
    }

    // No type specified - return empty struct (acts like "any")
    Ok(Struct::named().build())
}

fn instance_type_to_datatype(
    instance_type: &SingleOrVec<InstanceType>,
    obj: &SchemaObject,
) -> Result<DataType, Error> {
    match instance_type {
        SingleOrVec::Single(t) => match **t {
            InstanceType::Null => {
                // Null type - use empty tuple
                Ok(DataType::Tuple(Tuple::new(vec![])))
            }
            InstanceType::Boolean => Ok(DataType::Primitive(Primitive::bool)),
            InstanceType::String => Ok(DataType::Primitive(Primitive::String)),
            InstanceType::Number => {
                // Check format hint
                if let Some(format) = &obj.format {
                    match format.as_str() {
                        "float" => Ok(DataType::Primitive(Primitive::f32)),
                        "double" => Ok(DataType::Primitive(Primitive::f64)),
                        _ => Ok(DataType::Primitive(Primitive::f64)),
                    }
                } else {
                    Ok(DataType::Primitive(Primitive::f64))
                }
            }
            InstanceType::Integer => {
                // Check format hint
                if let Some(format) = &obj.format {
                    match format.as_str() {
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
            InstanceType::Array => {
                if let Some(array) = &obj.array {
                    if let Some(items) = &array.items {
                        match items {
                            SingleOrVec::Single(item_schema) => {
                                let item_dt = from_schema(item_schema)?;
                                Ok(DataType::List(List::new(item_dt)))
                            }
                            SingleOrVec::Vec(schemas) => {
                                // Tuple with specific items
                                let elements: Result<Vec<_>, _> =
                                    schemas.iter().map(from_schema).collect();
                                Ok(DataType::Tuple(Tuple::new(elements?)))
                            }
                        }
                    } else {
                        // Array without items = array of empty struct (any)
                        Ok(DataType::List(List::new(Struct::named().build())))
                    }
                } else {
                    Ok(DataType::List(List::new(Struct::named().build())))
                }
            }
            InstanceType::Object => {
                if let Some(object) = &obj.object {
                    if !object.properties.is_empty() {
                        // Build struct from properties
                        let mut builder = Struct::named();

                        for (name, schema) in &object.properties {
                            let dt = from_schema(schema)?;
                            let is_optional = !object.required.contains(name);

                            let mut field = Field::new(dt);
                            field.set_optional(is_optional);

                            builder.field_mut(Cow::Owned(name.clone()), field);
                        }

                        return Ok(builder.build());
                    } else if let Some(additional) = &object.additional_properties {
                        // Map type
                        let value_dt = from_schema(additional)?;
                        return Ok(DataType::Map(Map::new(
                            DataType::Primitive(Primitive::String),
                            value_dt,
                        )));
                    }
                }

                // Empty object
                Ok(Struct::named().build())
            }
        },
        SingleOrVec::Vec(types) => {
            // Multiple types - create a union (enum with unnamed variants)
            let mut e = Enum::new();
            for (i, instance_type) in types.iter().enumerate() {
                let mut obj = SchemaObject::default();
                obj.instance_type = Some(SingleOrVec::Single(Box::new(*instance_type)));
                let dt = schema_object_to_datatype(&obj)?;

                let variant = EnumVariant::unnamed().field(Field::new(dt)).build();
                e.variants_mut()
                    .push((Cow::Owned(format!("Variant{}", i)), variant));
            }
            Ok(DataType::Enum(e))
        }
    }
}

fn handle_any_of(schemas: &[Schema]) -> Result<DataType, Error> {
    // Check if it's a nullable pattern (type | null)
    if schemas.len() == 2 {
        let is_null = |s: &Schema| {
            matches!(s, Schema::Object(obj) if matches!(
                &obj.instance_type,
                Some(SingleOrVec::Single(t)) if **t == InstanceType::Null
            ))
        };

        if is_null(&schemas[0]) {
            return Ok(DataType::Nullable(Box::new(from_schema(&schemas[1])?)));
        } else if is_null(&schemas[1]) {
            return Ok(DataType::Nullable(Box::new(from_schema(&schemas[0])?)));
        }
    }

    // General anyOf - create enum with unnamed variants
    let mut e = Enum::new();
    for (i, schema) in schemas.iter().enumerate() {
        let dt = from_schema(schema)?;
        let variant = EnumVariant::unnamed().field(Field::new(dt)).build();
        e.variants_mut()
            .push((Cow::Owned(format!("Variant{}", i)), variant));
    }
    Ok(DataType::Enum(e))
}
