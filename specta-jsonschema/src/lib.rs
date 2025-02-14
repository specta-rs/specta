//! [JSON Schema](https://json-schema.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

// use specta::datatype::{DataType, PrimitiveType};

use std::path::Path;

use inflector::Inflector;
use schemars::schema::{InstanceType, Schema, SingleOrVec};
use specta::{
    builder::{EnumBuilder, FieldBuilder, StructBuilder},
    datatype::{DataType, List, Literal, Primitive},
    TypeCollection,
};

#[derive(Debug, Clone)]
pub struct JsonSchema;

impl JsonSchema {
    pub fn export(&self, types: &TypeCollection) -> Result<String, ()> {
        todo!();
    }

    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), ()> {
        todo!();
    }
}

// TODO: What should we call this?
// TODO: `TypeCollection` so we can handle references?
pub fn to_ast(schema: &Schema) -> Result<DataType, ()> {
    let mut types = TypeCollection::default();

    match schema {
        Schema::Bool(b) => Ok(DataType::Literal((*b).into())),
        Schema::Object(obj) => {
            // TODO: Implement it all
            // /// Properties which annotate the [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub metadata: Option<Box<Metadata>>,
            // /// The `type` keyword.
            // ///
            // /// See [JSON Schema Validation 6.1.1. "type"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.1)
            // /// and [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
            // #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
            // pub instance_type: Option<SingleOrVec<InstanceType>>,
            // /// The `format` keyword.
            // ///
            // /// See [JSON Schema Validation 7. A Vocabulary for Semantic Content With "format"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-7).
            // #[serde(skip_serializing_if = "Option::is_none")]
            // pub format: Option<String>,
            // /// The `enum` keyword.
            // ///
            // /// See [JSON Schema Validation 6.1.2. "enum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.2)
            // #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
            // pub enum_values: Option<Vec<Value>>,
            // /// The `const` keyword.
            // ///
            // /// See [JSON Schema Validation 6.1.3. "const"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.3)
            // #[serde(
            //     rename = "const",
            //     skip_serializing_if = "Option::is_none",
            //     deserialize_with = "allow_null"
            // )]
            // pub const_value: Option<Value>,
            // /// Properties of the [`SchemaObject`] which define validation assertions in terms of other schemas.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub subschemas: Option<Box<SubschemaValidation>>,
            // /// Properties of the [`SchemaObject`] which define validation assertions for numbers.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub number: Option<Box<NumberValidation>>,
            // /// Properties of the [`SchemaObject`] which define validation assertions for strings.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub string: Option<Box<StringValidation>>,
            // /// Properties of the [`SchemaObject`] which define validation assertions for arrays.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub array: Option<Box<ArrayValidation>>,
            // /// Properties of the [`SchemaObject`] which define validation assertions for objects.
            // #[serde(flatten, deserialize_with = "skip_if_default")]
            // pub object: Option<Box<ObjectValidation>>,
            // /// The `$ref` keyword.
            // ///
            // /// See [JSON Schema 8.2.4.1. Direct References with "$ref"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.4.1).
            // #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
            // pub reference: Option<String>,
            // /// Arbitrary extra properties which are not part of the JSON Schema specification, or which `schemars` does not support.
            // #[serde(flatten)]
            // pub extensions: Map<String, Value>,

            if let Some(reference) = &obj.reference {
                todo!();
                // return Ok(DataType::Any); // TODO: Fix this
                // return Ok(DataType::Reference(DataTypeReference {

                // }));
            }

            if let Some(o) = &obj.array {
                if let Some(items) = &o.items {
                    match items {
                        SingleOrVec::Single(o) => return to_ast(&o),
                        SingleOrVec::Vec(o) => todo!(),
                    }
                }
            }

            if let Some(o) = &obj.object {
                // /// The `maxProperties` keyword.
                // ///
                // /// See [JSON Schema Validation 6.5.1. "maxProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.1).
                // #[serde(skip_serializing_if = "Option::is_none")]
                // pub max_properties: Option<u32>,
                // /// The `minProperties` keyword.
                // ///
                // /// See [JSON Schema Validation 6.5.2. "minProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.2).
                // #[serde(skip_serializing_if = "Option::is_none")]
                // pub min_properties: Option<u32>,
                // /// The `required` keyword.
                // ///
                // /// See [JSON Schema Validation 6.5.3. "required"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.3).
                // #[serde(skip_serializing_if = "Set::is_empty")]
                // pub required: Set<String>,
                // /// The `properties` keyword.
                // ///
                // /// See [JSON Schema 9.3.2.1. "properties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.1).
                // #[serde(skip_serializing_if = "Map::is_empty")]
                // pub properties: Map<String, Schema>,
                // /// The `patternProperties` keyword.
                // ///
                // /// See [JSON Schema 9.3.2.2. "patternProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.2).
                // #[serde(skip_serializing_if = "Map::is_empty")]
                // pub pattern_properties: Map<String, Schema>,
                // /// The `additionalProperties` keyword.
                // ///
                // /// See [JSON Schema 9.3.2.3. "additionalProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.3).
                // #[serde(skip_serializing_if = "Option::is_none")]
                // pub additional_properties: Option<Box<Schema>>,
                // /// The `propertyNames` keyword.
                // ///
                // /// See [JSON Schema 9.3.2.5. "propertyNames"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.5).
                // #[serde(skip_serializing_if = "Option::is_none")]
                // pub property_names: Option<Box<Schema>>,

                let mut s = StructBuilder::named(
                    obj.metadata
                        .as_ref()
                        .and_then(|v| v.title.as_ref().map(|v| v.to_class_case()))
                        .unwrap_or_else(|| "Unnamed".to_string()),
                ); // TODO: Remove fallback
                for (k, v) in o.properties.iter() {
                    s.field_mut(k.clone(), FieldBuilder::new(to_ast(v)?));
                }
                return Ok(s.build());
            }

            if let Some(o) = &obj.instance_type {
                println!("{:?}", o);

                fn from_instance_type(o: &InstanceType) -> DataType {
                    match o {
                        InstanceType::Null => DataType::Literal(Literal::None),
                        InstanceType::Boolean => DataType::Primitive(Primitive::bool),
                        InstanceType::Object => unreachable!(),
                        InstanceType::Array => unreachable!(),
                        InstanceType::String => DataType::Primitive(Primitive::String),
                        InstanceType::Number => DataType::Primitive(Primitive::f64),
                        InstanceType::Integer => DataType::Primitive(Primitive::u64),
                    }
                }

                match o {
                    SingleOrVec::Single(o) => {
                        return Ok(from_instance_type(&o));
                    }
                    SingleOrVec::Vec(o) => {
                        println!("{:?}", obj);

                        return Ok(match o.len() {
                            0 => DataType::List(List::new(from_instance_type(&o[0]), None, false)),
                            _ => {
                                let mut e = EnumBuilder::new("todo");

                                for list in o {
                                    println!("{:?}", list);
                                    // e.variant_mut(list.name.clone(), from_instance_type(&list));
                                }

                                e.build()
                            }
                        });

                        // assert!(o.len() == 1, "expected a single item in the array"); // TODO: Handle stuff

                        // match

                        // for list in o {
                        //     println!("{:?}", list);
                        // }

                        // return Ok(DataType::List(List::new(from_instance_type(&o[0]))));
                    }
                }
            }

            // match true {
            //     _ if o.o

            //     _ => {},
            // }

            todo!("{:?}", obj);
        }
    }
}

// // pub fn to_openapi_export(def: &DataType) -> Result<openapiv3::Schema, String> {
// //     Ok(match &def {
// //         // Named struct
// //         // DataType::Struct(StructType {
// //         //     name,
// //         //     generics,
// //         //     fields,
// //         //     ..
// //         // }) => match fields.len() {
// //         //     0 => format!("export type {name} = {inline_ts}"),
// //         //     _ => {
// //         //         let generics = match generics.len() {
// //         //             0 => "".into(),
// //         //             _ => format!("<{}>", generics.to_vec().join(", ")),
// //         //         };

// //         //         format!("export interface {name}{generics} {inline_ts}")
// //         //     }
// //         // },
// //         // // Enum
// //         // DataType::Enum(EnumType { name, generics, .. }) => {
// //         //     let generics = match generics.len() {
// //         //         0 => "".into(),
// //         //         _ => format!("<{}>", generics.to_vec().join(", ")),
// //         //     };

// //         //     format!("export type {name}{generics} = {inline_ts}")
// //         // }
// //         // // Unnamed struct
// //         // DataType::Tuple(TupleType { name, .. }) => {
// //         //     format!("export type {name} = {inline_ts}")
// //         // }
// //         _ => todo!(), // return Err(format!("Type cannot be exported: {:?}", def)),
// //     })
// // }

// macro_rules! primitive_def {
//     ($($t:ident)+) => {
//         $(DataType::Primitive(PrimitiveType::$t))|+
//     }
// }

// pub fn to_openapi(typ: &DataType) -> ReferenceOr<Schema> {
//     let mut schema_data = SchemaData {
//         nullable: false,
//         deprecated: false, // TODO
//         example: None,     // TODO
//         title: None,       // TODO
//         description: None, // TODO
//         default: None,     // TODO
//         ..Default::default()
//     };

//     match &typ {
//         DataType::Any => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::Object(openapiv3::ObjectType::default())), // TODO: Use official "Any Type"
//         }),

//         primitive_def!(i8 i16 i32 isize u8 u16 u32 usize f32 f64) => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::Number(NumberType::default())), // TODO: Configure number type. Ts: `number`
//         }),
//         primitive_def!(i64 u64 i128 u128) => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::Number(NumberType::default())), // TODO: Configure number type. Ts: `bigint`
//         }),
//         primitive_def!(String char) => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::String(StringType::default())), // TODO: Configure string type. Ts: `string`
//         }),
//         primitive_def!(bool) => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::Boolean(BooleanType::default())),
//         }),
//         // primitive_def!(Never) => "never".into(),
//         DataType::Nullable(def) => {
//             let schema = to_openapi(def);
//             // schema.schema_data.nullable = true; // TODO
//             schema
//         }
//         // DataType::Map(def) => {
//         //     format!("Record<{}, {}>", to_openapi(&def.0), to_openapi(&def.1))
//         // }
//         DataType::List(def) => ReferenceOr::Item(Schema {
//             schema_data,
//             schema_kind: SchemaKind::Type(Type::Array(ArrayType {
//                 items: Some(match to_openapi(def.ty()) {
//                     ReferenceOr::Item(schema) => ReferenceOr::Item(Box::new(schema)),
//                     ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference },
//                 }),
//                 // TODO: This type is missing `Default`
//                 min_items: None,
//                 max_items: None,
//                 unique_items: false,
//             })),
//         }),
//         DataType::Tuple(tuple) => match &tuple.elements()[..] {
//             [] => {
//                 schema_data.nullable = true;
//                 ReferenceOr::Item(Schema {
//                     schema_data,
//                     schema_kind: SchemaKind::Type(Type::Object(openapiv3::ObjectType::default())), // TODO: This should be `null` type
//                 })
//             }
//             [ty] => to_openapi(ty),
//             tys => todo!(),
//         },
//         DataType::Struct(s) => {
//             let fields = s.fields();
//             let name = s.name();

//             // match &fields[..] {
//             //     [] => todo!(), // "null".to_string(),
//             //     fields => {
//             //         // let mut out = match tag {
//             //         //     Some(tag) => vec![format!("{tag}: \"{name}\"")],
//             //         //     None => vec![],
//             //         // };

//             //         // let field_defs = object_fields(fields);

//             //         // out.extend(field_defs);

//             //         // format!("{{ {} }}", out.join(", "))

//             //         ReferenceOr::Item(Schema {
//             //             schema_data,
//             //             schema_kind: SchemaKind::Type(Type::Object(openapiv3::ObjectType {
//             //                 properties: fields
//             //                     .iter()
//             //                     .map(
//             //                         |ObjectField {
//             //                              name, ty, optional, ..
//             //                          }| {
//             //                             (
//             //                                 name.clone(),
//             //                                 match to_openapi(ty) {
//             //                                     ReferenceOr::Item(v) => {
//             //                                         ReferenceOr::Item(Box::new(v))
//             //                                     }
//             //                                     ReferenceOr::Reference { reference } => {
//             //                                         ReferenceOr::Reference { reference }
//             //                                     }
//             //                                 },
//             //                             )
//             //                         },
//             //                     )
//             //                     .collect(),
//             //                 ..Default::default()
//             //             })),
//             //         })
//             //     }
//             // }
//             todo!();
//         }
//         DataType::Enum(e) => {
//             // let variants = e.variants();

//             // match &variants[..] {
//             //     [] => todo!(), // "never".to_string(),
//             //     variants => {
//             //         // variants
//             //         // .iter()
//             //         // .map(|variant| {
//             //         //     let sanitised_name = sanitise_name(variant.name());

//             //         //     match (repr, variant) {
//             //         //         (EnumRepr::Internal { tag }, EnumVariant::Unit(_)) => {
//             //         //             format!("{{ {tag}: \"{sanitised_name}\" }}")
//             //         //         }
//             //         //         (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
//             //         //             let typ = to_openapi(&DataType::Tuple(tuple.clone()));

//             //         //             format!("{{ {tag}: \"{sanitised_name}\" }} & {typ}")
//             //         //         }
//             //         //         (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
//             //         //             let mut fields = vec![format!("{tag}: \"{sanitised_name}\"")];

//             //         //             fields.extend(object_fields(&obj.fields));

//             //         //             format!("{{ {} }}", fields.join(", "))
//             //         //         }
//             //         //         (EnumRepr::External, EnumVariant::Unit(_)) => {
//             //         //             format!("\"{sanitised_name}\"")
//             //         //         }
//             //         //         (EnumRepr::External, v) => {
//             //         //             let ts_values = to_openapi(&v.data_type());

//             //         //             format!("{{ {sanitised_name}: {ts_values} }}")
//             //         //         }
//             //         //         (EnumRepr::Untagged, EnumVariant::Unit(_)) => "null".to_string(),
//             //         //         (EnumRepr::Untagged, v) => to_openapi(&v.data_type()),
//             //         //         (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit(_)) => {
//             //         //             format!("{{ {tag}: \"{sanitised_name}\" }}")
//             //         //         }
//             //         //         (EnumRepr::Adjacent { tag, content }, v) => {
//             //         //             let ts_values = to_openapi(&v.data_type());

//             //         //             format!("{{ {tag}: \"{sanitised_name}\", {content}: {ts_values} }}")
//             //         //         }
//             //         //     }
//             //         // })
//             //         // .collect::<Vec<_>>()
//             //         // .join(" | ");

//             //         ReferenceOr::Item(Schema {
//             //             schema_data,
//             //             schema_kind: SchemaKind::AnyOf {
//             //                 any_of: variants
//             //                     .iter()
//             //                     .map(|variant| match variant {
//             //                         EnumVariants::Unit(_) => ReferenceOr::Item(Schema {
//             //                             schema_data: Default::default(),
//             //                             schema_kind: SchemaKind::Type(Type::Object(
//             //                                 openapiv3::ObjectType::default(), // TODO: Is this correct?
//             //                             )),
//             //                         }),
//             //                         EnumVariants::Unnamed(tuple) => {
//             //                             to_openapi(&DataType::Tuple(tuple.clone()))
//             //                         }
//             //                         EnumVariant::Named(obj) => {
//             //                             to_openapi(&DataType::Struct(obj.clone()))
//             //                         }
//             //                     })
//             //                     .collect(),
//             //             },
//             //         })
//             //     }
//             // }

//             todo!();
//         }
//         DataType::Reference(reference) => match &reference.generics()[..] {
//             [] => ReferenceOr::Item(Schema {
//                 schema_data,
//                 schema_kind: SchemaKind::OneOf {
//                     one_of: vec![ReferenceOr::Reference {
//                         reference: format!("#/components/schemas/{}", reference.name()),
//                     }],
//                 },
//             }),
//             generics => {
//                 // let generics = generics
//                 //     .iter()
//                 //     .map(to_openapi)
//                 //     .collect::<Vec<_>>()
//                 //     .join(", ");

//                 // format!("{name}<{generics}>")
//                 todo!();
//             }
//         },
//         // DataType::Generic(ident) => ident.to_string(),
//         x => {
//             println!("{:?} {:?}", x, typ);
//             todo!();
//         }
//     }
// }
