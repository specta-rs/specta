use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
    path::Path,
};

use openapiv3::{
    AdditionalProperties, AnySchema, ArrayType, BooleanType, Components, Info, IntegerFormat,
    IntegerType, NumberFormat, NumberType, ObjectType, OpenAPI as OpenApiDocument, ReferenceOr,
    Schema, SchemaData, SchemaKind, Server, StringType, Type,
};
use serde_json::{json, Value};
use specta::{
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, GenericReference, NamedDataType, NamedReference,
        Primitive, Reference, Struct, Tuple, Variant,
    },
    ResolvedTypes, Types,
};

use crate::Error;

const SERDE_CONTAINER_TAG: &str = "serde:container:tag";
const SERDE_CONTAINER_CONTENT: &str = "serde:container:content";
const SERDE_CONTAINER_UNTAGGED: &str = "serde:container:untagged";
const SERDE_VARIANT_RENAME_SERIALIZE: &str = "serde:variant:rename_serialize";
const SERDE_FIELD_RENAME_SERIALIZE: &str = "serde:field:rename_serialize";
const SERDE_FIELD_FLATTEN: &str = "serde:field:flatten";

/// Controls how generic references are represented in the generated OpenAPI document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GenericHandling {
    /// Generate concrete synthetic components for each generic instantiation.
    #[default]
    MonomorphizedComponents,
    /// Use `$dynamicRef`/`$dynamicAnchor` style schemas for generic references.
    DynamicRef,
}

/// OpenAPI specification version string to emit in the root document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpenApiVersion {
    /// OpenAPI 3.0.3.
    #[default]
    V3_0_3,
    /// OpenAPI 3.1.0.
    V3_1_0,
}

impl OpenApiVersion {
    fn as_str(self) -> &'static str {
        match self {
            Self::V3_0_3 => "3.0.3",
            Self::V3_1_0 => "3.1.0",
        }
    }

    fn supports_dynamic_ref(self) -> bool {
        matches!(self, Self::V3_1_0)
    }
}

/// OpenAPI exporter builder.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OpenAPI {
    /// Root document metadata.
    pub info: Info,
    /// OpenAPI spec version.
    pub openapi_version: OpenApiVersion,
    /// Generic export strategy.
    pub generic_handling: GenericHandling,
    /// Root document servers.
    pub servers: Vec<Server>,
}

impl Default for OpenAPI {
    fn default() -> Self {
        Self {
            info: Info {
                title: "Specta OpenAPI".into(),
                description: None,
                terms_of_service: None,
                contact: None,
                license: None,
                version: "0.0.0".into(),
                extensions: Default::default(),
            },
            openapi_version: OpenApiVersion::default(),
            generic_handling: GenericHandling::default(),
            servers: Vec::new(),
        }
    }
}

impl OpenAPI {
    /// Construct a new OpenAPI exporter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the OpenAPI document title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.info.title = title.into();
        self
    }

    /// Set the OpenAPI document description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    /// Set the OpenAPI document version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.info.version = version.into();
        self
    }

    /// Set the OpenAPI specification version.
    pub fn openapi_version(mut self, version: OpenApiVersion) -> Self {
        self.openapi_version = version;
        self
    }

    /// Set the generic handling strategy.
    pub fn generic_handling(mut self, generic_handling: GenericHandling) -> Self {
        self.generic_handling = generic_handling;
        self
    }

    /// Append a server to the document.
    pub fn server(mut self, server: Server) -> Self {
        self.servers.push(server);
        self
    }

    /// Export the collected Specta types as an OpenAPI document.
    pub fn export(&self, resolved_types: &ResolvedTypes) -> Result<OpenApiDocument, Error> {
        if matches!(self.generic_handling, GenericHandling::DynamicRef)
            && !self.openapi_version.supports_dynamic_ref()
        {
            return Err(Error::unsupported_openapi_version_for_dynamic_ref(
                self.openapi_version.as_str().to_string(),
            ));
        }

        let mut exporter = Exporter::new(self, resolved_types.as_types());
        exporter.export_document()
    }

    /// Export the collected Specta types as pretty JSON.
    pub fn export_json(&self, resolved_types: &ResolvedTypes) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(&self.export(resolved_types)?)?)
    }

    /// Export the document to a file.
    pub fn export_to(
        &self,
        path: impl AsRef<Path>,
        resolved_types: &ResolvedTypes,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let json = self.export_json(resolved_types)?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|source| Error::unable_to_export_path(path.to_path_buf(), source))?;
        }

        std::fs::write(path, json)
            .map_err(|source| Error::unable_to_export_path(path.to_path_buf(), source))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TypeIdentity {
    module_path: String,
    name: String,
    file: String,
    line: u32,
    column: u32,
}

impl TypeIdentity {
    fn from_named(ndt: &NamedDataType) -> Self {
        Self {
            module_path: ndt.module_path().to_string(),
            name: ndt.name().to_string(),
            file: ndt.location().file().to_string(),
            line: ndt.location().line(),
            column: ndt.location().column(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ComponentKey {
    identity: TypeIdentity,
    generic_handling: GenericHandling,
    generics: Vec<String>,
}

struct Exporter<'a> {
    config: &'a OpenAPI,
    types: &'a Types,
    duplicate_name_counts: HashMap<String, usize>,
    components: BTreeMap<String, ReferenceOr<Schema>>,
    monomorphized_names: HashMap<ComponentKey, String>,
    generated_components: HashSet<ComponentKey>,
    exporting_components: HashSet<ComponentKey>,
    exported_dynamic_templates: HashSet<TypeIdentity>,
}

impl<'a> Exporter<'a> {
    fn new(config: &'a OpenAPI, types: &'a Types) -> Self {
        let duplicate_name_counts =
            types
                .into_unsorted_iter()
                .fold(HashMap::new(), |mut acc, ndt| {
                    *acc.entry(ndt.name().to_string()).or_default() += 1;
                    acc
                });

        Self {
            config,
            types,
            duplicate_name_counts,
            components: BTreeMap::new(),
            monomorphized_names: HashMap::new(),
            generated_components: HashSet::new(),
            exporting_components: HashSet::new(),
            exported_dynamic_templates: HashSet::new(),
        }
    }

    fn export_document(&mut self) -> Result<OpenApiDocument, Error> {
        for ndt in self.types.into_sorted_iter() {
            if ndt.generics().is_empty() {
                let component_name = self.base_component_name(ndt);
                let schema = self.export_named_type(ndt, &component_name, &[])?;
                self.components
                    .insert(component_name, ReferenceOr::Item(schema));
            } else if matches!(self.config.generic_handling, GenericHandling::DynamicRef) {
                let component_name = self.base_component_name(ndt);
                let schema = self.export_dynamic_template(ndt, &component_name)?;
                self.components
                    .insert(component_name, ReferenceOr::Item(schema));
            }
        }

        Ok(OpenApiDocument {
            openapi: self.config.openapi_version.as_str().to_string(),
            info: self.config.info.clone(),
            servers: self.config.servers.clone(),
            paths: Default::default(),
            components: Some(Components {
                schemas: self.components.clone().into_iter().collect(),
                ..Default::default()
            }),
            security: None,
            tags: Vec::new(),
            external_docs: None,
            extensions: Default::default(),
        })
    }

    fn export_named_type(
        &mut self,
        ndt: &NamedDataType,
        component_name: &str,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let mut schema = self.export_data_type(ndt.ty(), generics)?;
        schema.schema_data.title = Some(component_name.to_string());
        if !ndt.docs().is_empty() {
            schema.schema_data.description = Some(ndt.docs().to_string());
        }
        schema.schema_data.deprecated = ndt.deprecated().is_some();
        Ok(schema)
    }

    fn export_dynamic_template(
        &mut self,
        ndt: &NamedDataType,
        component_name: &str,
    ) -> Result<Schema, Error> {
        let identity = TypeIdentity::from_named(ndt);
        if !self.exported_dynamic_templates.insert(identity) {
            return Ok(self
                .components
                .get(component_name)
                .and_then(ReferenceOr::as_item)
                .cloned()
                .unwrap_or_else(|| self.any_schema()));
        }

        let mut schema = self.export_named_type(ndt, component_name, &[])?;
        schema.schema_data.extensions.insert(
            "$dynamicAnchor".into(),
            Value::String(component_name.to_string()),
        );
        Ok(schema)
    }

    fn export_data_type(
        &mut self,
        dt: &DataType,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let schema = match dt {
            DataType::Primitive(p) => self.primitive_schema(p),
            DataType::Nullable(inner) => {
                let mut schema = self.export_data_type(inner, generics)?;
                schema.schema_data.nullable = true;
                schema
            }
            DataType::List(list) => Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                    items: Some(self.export_reference_or_boxed_schema(list.ty(), generics)?),
                    min_items: list.length(),
                    max_items: list.length(),
                    unique_items: false,
                })),
            },
            DataType::Map(map) => {
                if !self.is_valid_map_key(map.key_ty(), generics) {
                    return Err(Error::invalid_map_key(format!(
                        "OpenAPI object keys must serialize as strings, but key type was {:?}",
                        map.key_ty()
                    )));
                }

                Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                        properties: Default::default(),
                        required: Vec::new(),
                        additional_properties: Some(AdditionalProperties::Schema(Box::new(
                            self.export_data_type(map.value_ty(), generics)
                                .map(ReferenceOr::Item)?,
                        ))),
                        min_properties: None,
                        max_properties: None,
                    })),
                }
            }
            DataType::Struct(strct) => self.export_struct(strct, generics)?,
            DataType::Enum(enm) => self.export_enum(enm, generics)?,
            DataType::Tuple(tuple) => self.export_tuple(tuple, generics)?,
            DataType::Reference(reference) => self.export_reference_schema(reference, generics)?,
        };

        Ok(schema)
    }

    fn export_reference_or_boxed_schema(
        &mut self,
        dt: &DataType,
        generics: &[(GenericReference, DataType)],
    ) -> Result<ReferenceOr<Box<Schema>>, Error> {
        Ok(ReferenceOr::boxed_item(
            self.export_data_type(dt, generics)?,
        ))
    }

    fn export_tuple(
        &mut self,
        tuple: &Tuple,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        match tuple.elements() {
            [] => Ok(Schema {
                schema_data: SchemaData {
                    nullable: true,
                    ..Default::default()
                },
                schema_kind: SchemaKind::Any(AnySchema {
                    enumeration: vec![Value::Null],
                    ..Default::default()
                }),
            }),
            [single] => self.export_data_type(single, generics),
            elements => {
                let any_of = elements
                    .iter()
                    .map(|element| {
                        self.export_data_type(element, generics)
                            .map(ReferenceOr::Item)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Any(AnySchema {
                        typ: Some("array".into()),
                        min_items: Some(elements.len()),
                        max_items: Some(elements.len()),
                        items: Some(ReferenceOr::boxed_item(Schema {
                            schema_data: Default::default(),
                            schema_kind: SchemaKind::AnyOf { any_of },
                        })),
                        ..Default::default()
                    }),
                })
            }
        }
    }

    fn export_struct(
        &mut self,
        strct: &Struct,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        match strct.fields() {
            Fields::Unit => Ok(Schema {
                schema_data: SchemaData {
                    nullable: true,
                    ..Default::default()
                },
                schema_kind: SchemaKind::Any(AnySchema {
                    enumeration: vec![Value::Null],
                    ..Default::default()
                }),
            }),
            Fields::Unnamed(fields) => {
                let items = fields
                    .fields()
                    .iter()
                    .filter_map(Field::ty)
                    .map(|ty| self.export_data_type(ty, generics).map(ReferenceOr::Item))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Any(AnySchema {
                        typ: Some("array".into()),
                        min_items: Some(items.len()),
                        max_items: Some(items.len()),
                        items: Some(ReferenceOr::boxed_item(Schema {
                            schema_data: Default::default(),
                            schema_kind: SchemaKind::AnyOf { any_of: items },
                        })),
                        ..Default::default()
                    }),
                })
            }
            Fields::Named(fields) => {
                let mut properties = BTreeMap::new();
                let mut required = Vec::new();
                let mut all_of = Vec::new();

                for (name, field) in fields.fields() {
                    let Some(ty) = field.ty() else {
                        continue;
                    };

                    if self.field_is_flattened(field) {
                        all_of.push(ReferenceOr::Item(self.export_data_type(ty, generics)?));
                        continue;
                    }

                    let field_name = self.field_name(name, field);
                    properties.insert(
                        field_name.clone(),
                        self.export_reference_or_boxed_schema(ty, generics)?,
                    );

                    if !field.optional() {
                        required.push(field_name);
                    }
                }

                let mut object = AnySchema {
                    typ: Some("object".into()),
                    properties: properties.into_iter().collect(),
                    required,
                    ..Default::default()
                };

                if !all_of.is_empty() {
                    all_of.push(ReferenceOr::Item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Any(object),
                    }));

                    Ok(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::AllOf { all_of },
                    })
                } else {
                    object.additional_properties = Some(AdditionalProperties::Any(false));
                    Ok(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Any(object),
                    })
                }
            }
        }
    }

    fn export_enum(
        &mut self,
        enm: &Enum,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let container = ContainerAttrs::from_enum(enm);

        if container.untagged {
            return self.export_untagged_enum(enm, generics);
        }

        if let Some(tag) = container.tag.as_deref() {
            if let Some(content) = container.content.as_deref() {
                return self.export_adjacent_enum(enm, tag, content, generics);
            }

            return self.export_internal_enum(enm, tag, generics);
        }

        if enm.is_string_enum() {
            let enumeration = enm
                .variants()
                .iter()
                .filter(|(_, variant)| !variant.skip())
                .map(|(name, variant)| Some(self.variant_name(name, variant)))
                .collect::<Vec<_>>();

            return Ok(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    enumeration,
                    ..Default::default()
                })),
            });
        }

        self.export_external_enum(enm, generics)
    }

    fn export_external_enum(
        &mut self,
        enm: &Enum,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let one_of = enm
            .variants()
            .iter()
            .filter(|(_, variant)| !variant.skip())
            .map(|(name, variant)| {
                self.export_external_variant(name, variant, generics)
                    .map(ReferenceOr::Item)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::OneOf { one_of },
        })
    }

    fn export_external_variant(
        &mut self,
        name: &Cow<'static, str>,
        variant: &Variant,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let variant_name = self.variant_name(name, variant);
        match variant.fields() {
            Fields::Unit => Ok(Schema {
                schema_data: self.schema_data_for_docs(variant.docs(), variant.deprecated()),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    enumeration: vec![Some(variant_name)],
                    ..Default::default()
                })),
            }),
            _ => {
                let value = self.export_variant_payload(variant, generics)?;
                let object = AnySchema {
                    typ: Some("object".into()),
                    properties: [(variant_name.clone(), ReferenceOr::boxed_item(value))]
                        .into_iter()
                        .collect(),
                    required: vec![variant_name],
                    additional_properties: Some(AdditionalProperties::Any(false)),
                    ..Default::default()
                };
                Ok(Schema {
                    schema_data: self.schema_data_for_docs(variant.docs(), variant.deprecated()),
                    schema_kind: SchemaKind::Any(object),
                })
            }
        }
    }

    fn export_internal_enum(
        &mut self,
        enm: &Enum,
        tag: &str,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let one_of = enm
            .variants()
            .iter()
            .filter(|(_, variant)| !variant.skip())
            .map(|(name, variant)| {
                let variant_name = self.variant_name(name, variant);
                let mut base = AnySchema {
                    typ: Some("object".into()),
                    properties: [(
                        tag.to_string(),
                        ReferenceOr::boxed_item(Schema {
                            schema_data: Default::default(),
                            schema_kind: SchemaKind::Type(Type::String(StringType {
                                enumeration: vec![Some(variant_name)],
                                ..Default::default()
                            })),
                        }),
                    )]
                    .into_iter()
                    .collect(),
                    required: vec![tag.to_string()],
                    additional_properties: Some(AdditionalProperties::Any(false)),
                    ..Default::default()
                };

                match variant.fields() {
                    Fields::Unit => Ok::<ReferenceOr<Schema>, Error>(ReferenceOr::Item(Schema {
                        schema_data: self
                            .schema_data_for_docs(variant.docs(), variant.deprecated()),
                        schema_kind: SchemaKind::Any(base),
                    })),
                    Fields::Named(named) => {
                        for (field_name, field) in named.fields() {
                            let Some(ty) = field.ty() else {
                                continue;
                            };
                            if self.field_is_flattened(field) {
                                return Ok::<ReferenceOr<Schema>, Error>(ReferenceOr::Item(
                                    Schema {
                                        schema_data: self.schema_data_for_docs(
                                            variant.docs(),
                                            variant.deprecated(),
                                        ),
                                        schema_kind: SchemaKind::AllOf {
                                            all_of: vec![
                                                ReferenceOr::Item(Schema {
                                                    schema_data: Default::default(),
                                                    schema_kind: SchemaKind::Any(base),
                                                }),
                                                ReferenceOr::Item(
                                                    self.export_data_type(ty, generics)?,
                                                ),
                                            ],
                                        },
                                    },
                                ));
                            }

                            let field_name = self.field_name(field_name, field);
                            base.properties.insert(
                                field_name.clone(),
                                self.export_reference_or_boxed_schema(ty, generics)?,
                            );
                            if !field.optional() {
                                base.required.push(field_name);
                            }
                        }

                        Ok::<ReferenceOr<Schema>, Error>(ReferenceOr::Item(Schema {
                            schema_data: self
                                .schema_data_for_docs(variant.docs(), variant.deprecated()),
                            schema_kind: SchemaKind::Any(base),
                        }))
                    }
                    Fields::Unnamed(_) => {
                        Ok::<ReferenceOr<Schema>, Error>(ReferenceOr::Item(Schema {
                            schema_data: self
                                .schema_data_for_docs(variant.docs(), variant.deprecated()),
                            schema_kind: SchemaKind::AllOf {
                                all_of: vec![
                                    ReferenceOr::Item(Schema {
                                        schema_data: Default::default(),
                                        schema_kind: SchemaKind::Any(base),
                                    }),
                                    ReferenceOr::Item(
                                        self.export_variant_payload(variant, generics)?,
                                    ),
                                ],
                            },
                        }))
                    }
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::OneOf { one_of },
        })
    }

    fn export_adjacent_enum(
        &mut self,
        enm: &Enum,
        tag: &str,
        content: &str,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let one_of = enm
            .variants()
            .iter()
            .filter(|(_, variant)| !variant.skip())
            .map(|(name, variant)| {
                let variant_name = self.variant_name(name, variant);
                let mut properties = BTreeMap::from([(
                    tag.to_string(),
                    ReferenceOr::boxed_item(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType {
                            enumeration: vec![Some(variant_name)],
                            ..Default::default()
                        })),
                    }),
                )]);
                let mut required = vec![tag.to_string()];

                if !matches!(variant.fields(), Fields::Unit) {
                    properties.insert(
                        content.to_string(),
                        ReferenceOr::boxed_item(self.export_variant_payload(variant, generics)?),
                    );
                    required.push(content.to_string());
                }

                Ok::<ReferenceOr<Schema>, Error>(ReferenceOr::Item(Schema {
                    schema_data: self.schema_data_for_docs(variant.docs(), variant.deprecated()),
                    schema_kind: SchemaKind::Any(AnySchema {
                        typ: Some("object".into()),
                        properties: properties.into_iter().collect(),
                        required,
                        additional_properties: Some(AdditionalProperties::Any(false)),
                        ..Default::default()
                    }),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::OneOf { one_of },
        })
    }

    fn export_untagged_enum(
        &mut self,
        enm: &Enum,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let any_of = enm
            .variants()
            .iter()
            .filter(|(_, variant)| !variant.skip())
            .map(|(_, variant)| {
                self.export_variant_payload(variant, generics)
                    .map(ReferenceOr::Item)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::AnyOf { any_of },
        })
    }

    fn export_variant_payload(
        &mut self,
        variant: &Variant,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        match variant.fields() {
            Fields::Unit => Ok(Schema {
                schema_data: SchemaData {
                    nullable: true,
                    ..Default::default()
                },
                schema_kind: SchemaKind::Any(AnySchema {
                    enumeration: vec![Value::Null],
                    ..Default::default()
                }),
            }),
            Fields::Unnamed(fields) => {
                let values = fields
                    .fields()
                    .iter()
                    .filter_map(Field::ty)
                    .map(|ty| self.export_data_type(ty, generics))
                    .collect::<Result<Vec<_>, _>>()?;

                match values.as_slice() {
                    [single] => Ok(single.clone()),
                    _ => Ok(Schema {
                        schema_data: Default::default(),
                        schema_kind: SchemaKind::Any(AnySchema {
                            typ: Some("array".into()),
                            min_items: Some(values.len()),
                            max_items: Some(values.len()),
                            items: Some(ReferenceOr::boxed_item(Schema {
                                schema_data: Default::default(),
                                schema_kind: SchemaKind::AnyOf {
                                    any_of: values.into_iter().map(ReferenceOr::Item).collect(),
                                },
                            })),
                            ..Default::default()
                        }),
                    }),
                }
            }
            Fields::Named(_) => {
                let mut strct = Struct::unit();
                strct.set_fields(variant.fields().clone());
                self.export_struct(&strct, generics)
            }
        }
    }

    fn export_reference_schema(
        &mut self,
        reference: &Reference,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        match reference {
            Reference::Named(r) => self.export_named_reference_schema(r, generics),
            Reference::Generic(g) => {
                if let Some((_, dt)) = generics.iter().find(|(candidate, _)| candidate == g) {
                    if matches!(dt, DataType::Reference(Reference::Generic(inner)) if inner == g) {
                        return Err(Error::unresolved_generic_reference(format!("{g:?}")));
                    }
                    return self.export_data_type(dt, generics);
                }

                match self.config.generic_handling {
                    GenericHandling::MonomorphizedComponents => {
                        Err(Error::unresolved_generic_reference(format!("{g:?}")))
                    }
                    GenericHandling::DynamicRef => Ok(self.dynamic_ref_schema(&format!(
                        "generic-{}",
                        self.hash_generic_reference(g)
                    ))),
                }
            }
            Reference::Opaque(r) => Err(Error::unsupported_opaque_reference(
                r.type_name().to_string(),
            )),
        }
    }

    fn export_named_reference_schema(
        &mut self,
        reference: &NamedReference,
        generics: &[(GenericReference, DataType)],
    ) -> Result<Schema, Error> {
        let ndt = reference
            .get(self.types)
            .ok_or_else(|| Error::dangling_named_reference(format!("{reference:?}")))?;
        let combined_generics = merge_generics(generics, reference.generics());

        if reference.inline() {
            return self.export_data_type(ndt.ty(), &combined_generics);
        }

        match self.config.generic_handling {
            GenericHandling::MonomorphizedComponents => {
                let component_name = if reference.generics().is_empty() && ndt.generics().is_empty()
                {
                    self.base_component_name(ndt)
                } else {
                    self.ensure_monomorphized_component(ndt, &combined_generics)?
                };

                Ok(self.ref_schema(format!("#/components/schemas/{component_name}")))
            }
            GenericHandling::DynamicRef => {
                let component_name = self.base_component_name(ndt);
                if ndt.generics().is_empty() {
                    Ok(self.ref_schema(format!("#/components/schemas/{component_name}")))
                } else {
                    let template = self.export_dynamic_template(ndt, &component_name)?;
                    self.components
                        .entry(component_name.clone())
                        .or_insert(ReferenceOr::Item(template));
                    Ok(self
                        .dynamic_ref_schema(&component_name)
                        .with_generic_arguments(
                            reference
                                .generics()
                                .iter()
                                .map(|(_, dt)| self.export_data_type(dt, generics))
                                .collect::<Result<Vec<_>, _>>()?,
                        ))
                }
            }
        }
    }

    fn ensure_monomorphized_component(
        &mut self,
        ndt: &NamedDataType,
        generics: &[(GenericReference, DataType)],
    ) -> Result<String, Error> {
        let key = ComponentKey {
            identity: TypeIdentity::from_named(ndt),
            generic_handling: self.config.generic_handling,
            generics: generics.iter().map(|(_, dt)| format!("{dt:?}")).collect(),
        };

        if let Some(name) = self.monomorphized_names.get(&key) {
            return Ok(name.clone());
        }

        let base_name = self.base_component_name(ndt);
        let component_name = if generics.is_empty() {
            base_name
        } else {
            let hash = stable_hash(&key.generics.join("|"));
            format!("{base_name}__{hash}")
        };

        self.monomorphized_names
            .insert(key.clone(), component_name.clone());

        if self.generated_components.contains(&key) || self.exporting_components.contains(&key) {
            return Ok(component_name);
        }

        self.exporting_components.insert(key.clone());
        let schema = self.export_named_type(ndt, &component_name, generics)?;
        self.components
            .insert(component_name.clone(), ReferenceOr::Item(schema));
        self.exporting_components.remove(&key);
        self.generated_components.insert(key);

        Ok(component_name)
    }

    fn primitive_schema(&self, primitive: &Primitive) -> Schema {
        match primitive {
            Primitive::i8 => self.integer_schema(Some(i8::MIN.into()), Some(i8::MAX.into()), None),
            Primitive::i16 => {
                self.integer_schema(Some(i16::MIN.into()), Some(i16::MAX.into()), None)
            }
            Primitive::i32 => self.integer_schema(None, None, Some(IntegerFormat::Int32)),
            Primitive::i64 => self.integer_schema(None, None, Some(IntegerFormat::Int64)),
            Primitive::i128 | Primitive::isize => self.integer_schema(None, None, None),
            Primitive::u8 => self.integer_schema(Some(0), Some(u8::MAX.into()), None),
            Primitive::u16 => self.integer_schema(Some(0), Some(u16::MAX.into()), None),
            Primitive::u32 => self.integer_schema(Some(0), None, Some(IntegerFormat::Int32)),
            Primitive::u64 | Primitive::u128 | Primitive::usize => {
                self.integer_schema(Some(0), None, None)
            }
            Primitive::f16 | Primitive::f32 => self.number_schema(Some(NumberFormat::Float)),
            Primitive::f64 | Primitive::f128 => self.number_schema(Some(NumberFormat::Double)),
            Primitive::bool => Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::Boolean(BooleanType::default())),
            },
            Primitive::str => Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType::default())),
            },
            Primitive::char => Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType {
                    min_length: Some(1),
                    max_length: Some(1),
                    ..Default::default()
                })),
            },
        }
    }

    fn integer_schema(
        &self,
        minimum: Option<i64>,
        maximum: Option<i64>,
        format: Option<IntegerFormat>,
    ) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
                minimum,
                maximum,
                format: format
                    .map(|v| {
                        Some(match v {
                            IntegerFormat::Int32 => "int32".to_string(),
                            IntegerFormat::Int64 => "int64".to_string(),
                        })
                        .into()
                    })
                    .unwrap_or_default(),
                ..Default::default()
            })),
        }
    }

    fn number_schema(&self, format: Option<NumberFormat>) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::Number(NumberType {
                format: format
                    .map(|v| {
                        Some(match v {
                            NumberFormat::Float => "float".to_string(),
                            NumberFormat::Double => "double".to_string(),
                        })
                        .into()
                    })
                    .unwrap_or_default(),
                ..Default::default()
            })),
        }
    }

    fn ref_schema(&self, reference: String) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::AllOf {
                all_of: vec![ReferenceOr::ref_(&reference)],
            },
        }
    }

    fn dynamic_ref_schema(&self, anchor: &str) -> Schema {
        let mut schema = self.any_schema();
        schema
            .schema_data
            .extensions
            .insert("$dynamicRef".into(), Value::String(format!("#{anchor}")));
        schema
    }

    fn any_schema(&self) -> Schema {
        Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Any(AnySchema::default()),
        }
    }

    fn base_component_name(&self, ndt: &NamedDataType) -> String {
        let base_name = ndt.name().to_string();
        if self
            .duplicate_name_counts
            .get(&base_name)
            .copied()
            .unwrap_or_default()
            <= 1
        {
            return sanitize_component_name(&base_name);
        }

        sanitize_component_name(&format!(
            "{}__{}",
            ndt.module_path().replace("::", "__"),
            ndt.name()
        ))
    }

    fn variant_name(&self, original: &Cow<'static, str>, variant: &Variant) -> String {
        variant
            .attributes()
            .get_named_as::<String>(SERDE_VARIANT_RENAME_SERIALIZE)
            .cloned()
            .unwrap_or_else(|| original.to_string())
    }

    fn field_name(&self, original: &Cow<'static, str>, field: &Field) -> String {
        field
            .attributes()
            .get_named_as::<String>(SERDE_FIELD_RENAME_SERIALIZE)
            .cloned()
            .unwrap_or_else(|| original.to_string())
    }

    fn field_is_flattened(&self, field: &Field) -> bool {
        field.flatten() || field.attributes().contains_key(SERDE_FIELD_FLATTEN)
    }

    fn is_valid_map_key(&self, dt: &DataType, generics: &[(GenericReference, DataType)]) -> bool {
        match dt {
            DataType::Primitive(Primitive::str | Primitive::char) => true,
            DataType::Reference(Reference::Generic(g)) => generics
                .iter()
                .find(|(candidate, _)| candidate == g)
                .is_some_and(|(_, resolved)| self.is_valid_map_key(resolved, generics)),
            DataType::Reference(Reference::Named(r)) => r.get(self.types).is_some_and(|ndt| {
                self.is_valid_map_key(ndt.ty(), &merge_generics(generics, r.generics()))
            }),
            DataType::Enum(enm) => enm.is_string_enum(),
            _ => false,
        }
    }

    fn schema_data_for_docs(
        &self,
        docs: &Cow<'static, str>,
        deprecated: Option<&Deprecated>,
    ) -> SchemaData {
        SchemaData {
            description: (!docs.is_empty()).then(|| docs.to_string()),
            deprecated: deprecated.is_some(),
            ..Default::default()
        }
    }

    fn hash_generic_reference(&self, generic: &GenericReference) -> u64 {
        stable_hash(&format!("{generic:?}"))
    }
}

#[derive(Default)]
struct ContainerAttrs {
    tag: Option<String>,
    content: Option<String>,
    untagged: bool,
}

impl ContainerAttrs {
    fn from_enum(enm: &Enum) -> Self {
        Self {
            tag: enm
                .attributes()
                .get_named_as::<String>(SERDE_CONTAINER_TAG)
                .cloned(),
            content: enm
                .attributes()
                .get_named_as::<String>(SERDE_CONTAINER_CONTENT)
                .cloned(),
            untagged: enm.attributes().contains_key(SERDE_CONTAINER_UNTAGGED),
        }
    }
}

trait SchemaExt {
    fn with_generic_arguments(self, schemas: Vec<Schema>) -> Self;
}

impl SchemaExt for Schema {
    fn with_generic_arguments(mut self, schemas: Vec<Schema>) -> Self {
        if !schemas.is_empty() {
            self.schema_data
                .extensions
                .insert("x-specta-generics".into(), json!(schemas));
        }
        self
    }
}

fn merge_generics(
    parent: &[(GenericReference, DataType)],
    child: &[(GenericReference, DataType)],
) -> Vec<(GenericReference, DataType)> {
    let unshadowed_parent = parent
        .iter()
        .filter(|(parent_generic, _)| {
            !child
                .iter()
                .any(|(child_generic, _)| child_generic == parent_generic)
        })
        .cloned();

    child
        .iter()
        .map(|(generic, dt)| (generic.clone(), resolve_generics_in_datatype(dt, parent)))
        .chain(unshadowed_parent)
        .collect()
}

fn resolve_generics_in_datatype(
    dt: &DataType,
    generics: &[(GenericReference, DataType)],
) -> DataType {
    match dt {
        DataType::Primitive(_) => dt.clone(),
        DataType::List(list) => {
            let mut out = list.clone();
            out.set_ty(resolve_generics_in_datatype(list.ty(), generics));
            DataType::List(out)
        }
        DataType::Map(map) => {
            let mut out = map.clone();
            out.set_key_ty(resolve_generics_in_datatype(map.key_ty(), generics));
            out.set_value_ty(resolve_generics_in_datatype(map.value_ty(), generics));
            DataType::Map(out)
        }
        DataType::Struct(strct) => {
            let mut out = strct.clone();
            match out.fields_mut() {
                Fields::Unit => {}
                Fields::Unnamed(unnamed) => {
                    for field in unnamed.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = resolve_generics_in_datatype(ty, generics);
                        }
                    }
                }
                Fields::Named(named) => {
                    for (_, field) in named.fields_mut() {
                        if let Some(ty) = field.ty_mut() {
                            *ty = resolve_generics_in_datatype(ty, generics);
                        }
                    }
                }
            }
            DataType::Struct(out)
        }
        DataType::Enum(enm) => {
            let mut out = enm.clone();
            for (_, variant) in out.variants_mut() {
                match variant.fields_mut() {
                    Fields::Unit => {}
                    Fields::Unnamed(unnamed) => {
                        for field in unnamed.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve_generics_in_datatype(ty, generics);
                            }
                        }
                    }
                    Fields::Named(named) => {
                        for (_, field) in named.fields_mut() {
                            if let Some(ty) = field.ty_mut() {
                                *ty = resolve_generics_in_datatype(ty, generics);
                            }
                        }
                    }
                }
            }
            DataType::Enum(out)
        }
        DataType::Tuple(tuple) => {
            let mut out = tuple.clone();
            for element in out.elements_mut() {
                *element = resolve_generics_in_datatype(element, generics);
            }
            DataType::Tuple(out)
        }
        DataType::Nullable(inner) => {
            DataType::Nullable(Box::new(resolve_generics_in_datatype(inner, generics)))
        }
        DataType::Reference(Reference::Generic(generic)) => generics
            .iter()
            .find(|(candidate, _)| candidate == generic)
            .map(|(_, dt)| dt.clone())
            .unwrap_or_else(|| dt.clone()),
        DataType::Reference(Reference::Named(_reference)) => dt.clone(),
        DataType::Reference(Reference::Opaque(_)) => dt.clone(),
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn sanitize_component_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "Schema".into()
    } else {
        out
    }
}
