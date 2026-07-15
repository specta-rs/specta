use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use serde_json::{Map, Number, Value};
use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, Generic, List, Map as SpectaMap, NamedDataType,
        NamedReference, NamedReferenceType, Primitive, Reference, Struct, Tuple, Variant,
    },
};

use crate::{Error, SchemaVersion};

const INLINE_RECURSION_LIMIT: usize = 128;
const SERDE_CONTAINER_UNTAGGED: &str = "serde:container:untagged";
const SERDE_VARIANT_UNTAGGED: &str = "serde:variant:untagged";
const SERDE_ENUM_REPR_REWRITTEN: &str = "specta_serde:enum_repr_rewritten";

type Generics = BTreeMap<Cow<'static, str>, DataType>;

#[derive(Clone, PartialEq)]
struct DefinitionSource {
    type_path: String,
    file: &'static str,
    line: u32,
    column: u32,
    generics: Generics,
}

pub(crate) struct Renderer<'a> {
    schema_version: SchemaVersion,
    types: &'a Types,
    definitions: Map<String, Value>,
    definition_sources: BTreeMap<String, DefinitionSource>,
    in_progress: Vec<String>,
    in_progress_types: Vec<(String, &'static str, u32, u32)>,
    name_counts: BTreeMap<Cow<'static, str>, usize>,
    allow_additional_properties: bool,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        schema_version: SchemaVersion,
        types: &'a Types,
        allow_additional_properties: bool,
    ) -> Self {
        let mut name_counts = BTreeMap::new();
        for ndt in types.into_sorted_iter() {
            *name_counts.entry(ndt.name.clone()).or_default() += 1;
        }

        Self {
            schema_version,
            types,
            definitions: Map::new(),
            definition_sources: BTreeMap::new(),
            in_progress: Vec::new(),
            in_progress_types: Vec::new(),
            name_counts,
            allow_additional_properties,
        }
    }

    pub(crate) fn render_definitions(mut self) -> Result<Map<String, Value>, Error> {
        for ndt in self.types.into_sorted_iter() {
            if ndt.ty.is_none() {
                continue;
            }

            let generics = default_generics(ndt);
            let key = self.definition_key(ndt, &generics);
            let path = format!("{}.{}", self.schema_version.definitions_key(), key);
            self.ensure_definition(ndt, key, generics, &path)?;
        }

        Ok(self.definitions)
    }

    fn ensure_definition(
        &mut self,
        ndt: &NamedDataType,
        key: String,
        generics: Generics,
        path: &str,
    ) -> Result<(), Error> {
        let source = DefinitionSource {
            type_path: rust_type_path(ndt),
            file: ndt.location.file(),
            line: ndt.location.line(),
            column: ndt.location.column(),
            generics: generics.clone(),
        };
        if let Some(first) = self.definition_sources.get(&key) {
            if first != &source {
                return Err(Error::DuplicateDefinitionName {
                    key,
                    first: first.type_path.clone(),
                    second: source.type_path,
                });
            }
        } else {
            self.definition_sources.insert(key.clone(), source.clone());
        }

        if self.definitions.contains_key(&key) || self.in_progress.contains(&key) {
            return Ok(());
        }

        let source_id = (
            source.type_path.clone(),
            source.file,
            source.line,
            source.column,
        );
        if self.in_progress_types.contains(&source_id) {
            return Err(Error::ExpandingRecursiveGeneric {
                path: path.to_string(),
                type_path: source.type_path,
            });
        }

        let Some(ty) = &ndt.ty else {
            self.definitions.insert(key, Value::Object(Map::new()));
            return Ok(());
        };

        self.in_progress.push(key.clone());
        self.in_progress_types.push(source_id);
        let schema = self.render_datatype(ty, &generics, path, 0);
        self.in_progress_types.pop();
        self.in_progress.pop();
        let mut schema = schema?;
        self.apply_metadata(
            &mut schema,
            Some(ndt.name.as_ref()),
            &ndt.docs,
            ndt.deprecated.as_ref(),
        );
        self.definitions.insert(key, schema);
        Ok(())
    }

    fn render_datatype(
        &mut self,
        dt: &DataType,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        if depth > INLINE_RECURSION_LIMIT {
            return Err(Error::InlineRecursionLimitExceeded { path: path.into() });
        }

        match dt {
            DataType::Primitive(primitive) => Ok(primitive_schema(primitive)),
            DataType::List(list) => self.render_list(list, generics, path, depth),
            DataType::Map(map) => self.render_map(map, generics, path, depth),
            DataType::Struct(strct) => self.render_struct(strct, generics, path, depth),
            DataType::Enum(enm) => self.render_enum(enm, generics, path, depth),
            DataType::Tuple(tuple) => self.render_tuple(tuple, generics, path, depth),
            DataType::Nullable(inner) => Ok(object([(
                "anyOf",
                Value::Array(vec![
                    self.render_datatype(inner, generics, path, depth + 1)?,
                    object([("type", string("null"))]),
                ]),
            )])),
            DataType::Intersection(types) => self.render_intersection(types, generics, path, depth),
            DataType::Generic(generic) => match generics.get(generic.name()) {
                Some(ty) if ty != dt => self.render_datatype(ty, generics, path, depth + 1),
                _ => Ok(Value::Object(Map::new())),
            },
            DataType::Reference(reference) => {
                self.render_reference(reference, generics, path, depth)
            }
        }
    }

    fn render_intersection(
        &mut self,
        types: &[DataType],
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        if types.is_empty() {
            return Ok(Value::Object(Map::new()));
        }

        let parts = types
            .iter()
            .map(|ty| self.render_datatype_for_intersection(ty, generics, path, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(schema) = merge_object_intersection(&parts, self.allow_additional_properties) {
            return Ok(schema);
        }

        let mut schema = Map::new();
        schema.insert("allOf".to_string(), Value::Array(parts));
        if self.schema_version.supports_unevaluated_properties() {
            schema.insert("unevaluatedProperties".to_string(), Value::Bool(false));
        }

        Ok(Value::Object(schema))
    }

    fn render_datatype_for_intersection(
        &mut self,
        dt: &DataType,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        match dt {
            DataType::Struct(strct) => {
                self.render_fields(&strct.fields, generics, path, depth, false)
            }
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Reference {
                    generics: reference_generics,
                    ..
                } => {
                    let ndt = self
                        .types
                        .get(reference)
                        .ok_or_else(|| Error::dangling(path, format!("{reference:?}")))?;
                    let resolved_generics =
                        self.resolve_generics(ndt, reference_generics, generics);
                    match &ndt.ty {
                        Some(DataType::Struct(strct)) => self.render_fields(
                            &strct.fields,
                            &resolved_generics,
                            path,
                            depth,
                            false,
                        ),
                        _ => self.render_named_reference(reference, generics, path, depth),
                    }
                }
                _ => self.render_named_reference(reference, generics, path, depth),
            },
            _ => self.render_datatype(dt, generics, path, depth),
        }
    }

    fn render_reference(
        &mut self,
        reference: &Reference,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        match reference {
            Reference::Named(reference) => {
                self.render_named_reference(reference, generics, path, depth)
            }
            Reference::Opaque(reference) => Err(Error::UnsupportedOpaqueReference {
                path: path.into(),
                reference: reference.clone(),
            }),
        }
    }

    fn render_named_reference(
        &mut self,
        reference: &NamedReference,
        parent_generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                self.render_datatype(dt, parent_generics, path, depth + 1)
            }
            NamedReferenceType::Recursive(cycle) => Err(Error::InfiniteRecursiveInlineType {
                path: path.into(),
                cycle: cycle.clone(),
            }),
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = self
                    .types
                    .get(reference)
                    .ok_or_else(|| Error::dangling(path, format!("{reference:?}")))?;
                let generics = self.resolve_generics(ndt, generics, parent_generics);
                let key = self.definition_key(ndt, &generics);
                self.ensure_definition(ndt, key.clone(), generics, path)?;
                Ok(object([("$ref", string(self.ref_path(&key)))]))
            }
        }
    }

    fn resolve_generics(
        &self,
        ndt: &NamedDataType,
        reference_generics: &[(Generic, DataType)],
        parent_generics: &Generics,
    ) -> Generics {
        let mut generics = default_generics(ndt);

        for (generic, ty) in reference_generics {
            generics.insert(
                generic.name().clone(),
                substitute_generics(ty, parent_generics),
            );
        }

        let resolved = generics.clone();
        for ty in generics.values_mut() {
            *ty = substitute_generics(ty, &resolved);
        }

        generics
    }

    fn render_list(
        &mut self,
        list: &List,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        let mut schema = Map::new();
        schema.insert("type".to_string(), string("array"));
        schema.insert(
            "items".to_string(),
            self.render_datatype(&list.ty, generics, path, depth + 1)?,
        );
        if let Some(length) = list.length {
            schema.insert("minItems".to_string(), number(length));
            schema.insert("maxItems".to_string(), number(length));
        }
        if list.unique {
            schema.insert("uniqueItems".to_string(), Value::Bool(true));
        }
        Ok(Value::Object(schema))
    }

    fn render_map(
        &mut self,
        map: &SpectaMap,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        if !self.is_valid_map_key(map.key_ty(), generics, 0) {
            return Err(Error::InvalidMapKey {
                path: path.into(),
                reason: "JSON object keys must be scalar string-like values".into(),
            });
        }

        let mut schema = Map::new();
        schema.insert("type".to_string(), string("object"));
        schema.insert(
            "additionalProperties".to_string(),
            self.render_datatype(map.value_ty(), generics, path, depth + 1)?,
        );
        if let Some(property_names) =
            self.map_key_schema(map.key_ty(), generics, path, depth + 1)?
        {
            schema.insert("propertyNames".to_string(), property_names);
        }

        Ok(Value::Object(schema))
    }

    fn is_valid_map_key(&self, dt: &DataType, generics: &Generics, depth: usize) -> bool {
        if depth > INLINE_RECURSION_LIMIT {
            return false;
        }

        match dt {
            DataType::Primitive(Primitive::f16 | Primitive::f128) => false,
            DataType::Primitive(_) => true,
            DataType::Generic(generic) => generics
                .get(generic.name())
                .is_none_or(|ty| self.is_valid_map_key(ty, generics, depth + 1)),
            DataType::Nullable(_) => false,
            DataType::Struct(strct) => match &strct.fields {
                Fields::Unnamed(fields) if fields.fields.len() == 1 => fields.fields[0]
                    .ty
                    .as_ref()
                    .is_some_and(|ty| self.is_valid_map_key(ty, generics, depth + 1)),
                _ => false,
            },
            DataType::Enum(enm) => enm
                .variants
                .iter()
                .filter(|(_, variant)| !variant.skip)
                .all(|(_, variant)| match &variant.fields {
                    Fields::Unit => !enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED),
                    Fields::Unnamed(fields) => {
                        let mut fields = fields.fields.iter().filter_map(|field| field.ty.as_ref());
                        fields.next().is_some_and(|ty| {
                            fields.next().is_none()
                                && self.is_valid_map_key(ty, generics, depth + 1)
                        })
                    }
                    Fields::Named(_) => false,
                }),
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    self.is_valid_map_key(dt, generics, depth + 1)
                }
                NamedReferenceType::Reference {
                    generics: reference_generics,
                    ..
                } => self.types.get(reference).is_some_and(|ndt| {
                    let resolved = self.resolve_generics(ndt, reference_generics, generics);
                    ndt.ty
                        .as_ref()
                        .is_some_and(|ty| self.is_valid_map_key(ty, &resolved, depth + 1))
                }),
                NamedReferenceType::Recursive(_) => false,
            },
            DataType::List(_)
            | DataType::Map(_)
            | DataType::Tuple(_)
            | DataType::Intersection(_)
            | DataType::Reference(Reference::Opaque(_)) => false,
        }
    }

    fn map_key_schema(
        &mut self,
        dt: &DataType,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Option<Value>, Error> {
        Ok(match dt {
            DataType::Primitive(Primitive::char) => Some(primitive_schema(&Primitive::char)),
            DataType::Primitive(Primitive::str) => None,
            DataType::Primitive(Primitive::bool) => Some(object([(
                "enum",
                Value::Array(vec![string("false"), string("true")]),
            )])),
            DataType::Primitive(
                Primitive::i8
                | Primitive::i16
                | Primitive::i32
                | Primitive::i64
                | Primitive::i128
                | Primitive::isize,
            ) => Some(pattern_schema("^-?(0|[1-9][0-9]*)$")),
            DataType::Primitive(
                Primitive::u8
                | Primitive::u16
                | Primitive::u32
                | Primitive::u64
                | Primitive::u128
                | Primitive::usize,
            ) => Some(pattern_schema("^(0|[1-9][0-9]*)$")),
            DataType::Primitive(Primitive::f32 | Primitive::f64) => Some(pattern_schema(
                "^-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?$",
            )),
            DataType::Primitive(Primitive::f16 | Primitive::f128) => None,
            DataType::Generic(generic) => generics
                .get(generic.name())
                .filter(|ty| *ty != dt)
                .map(|ty| self.map_key_schema(ty, generics, path, depth + 1))
                .transpose()?
                .flatten(),
            DataType::Struct(strct) => match &strct.fields {
                Fields::Unnamed(fields) => fields
                    .fields
                    .iter()
                    .filter_map(|field| field.ty.as_ref())
                    .next()
                    .map(|ty| self.map_key_schema(ty, generics, path, depth + 1))
                    .transpose()?
                    .flatten(),
                Fields::Unit | Fields::Named(_) => None,
            },
            DataType::Enum(enm) if enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED) => {
                let mut schemas = Vec::new();
                for (_, variant) in enm.variants.iter().filter(|(_, variant)| !variant.skip) {
                    let Fields::Unnamed(fields) = &variant.fields else {
                        continue;
                    };
                    let Some(ty) = fields.fields.iter().find_map(|field| field.ty.as_ref()) else {
                        continue;
                    };
                    let Some(schema) = self.map_key_schema(ty, generics, path, depth + 1)? else {
                        return Ok(None);
                    };
                    schemas.push(schema);
                }
                match schemas.as_slice() {
                    [] => None,
                    [schema] => Some(schema.clone()),
                    _ => Some(object([("anyOf", Value::Array(schemas))])),
                }
            }
            DataType::Enum(_) => Some(self.render_datatype(dt, generics, path, depth + 1)?),
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    self.map_key_schema(dt, generics, path, depth + 1)?
                }
                NamedReferenceType::Reference {
                    generics: reference_generics,
                    ..
                } => {
                    let ndt = self
                        .types
                        .get(reference)
                        .ok_or_else(|| Error::dangling(path, format!("{reference:?}")))?;
                    let resolved = self.resolve_generics(ndt, reference_generics, generics);
                    ndt.ty
                        .as_ref()
                        .map(|ty| self.map_key_schema(ty, &resolved, path, depth + 1))
                        .transpose()?
                        .flatten()
                }
                NamedReferenceType::Recursive(_) => None,
            },
            DataType::Nullable(_) => None,
            _ => None,
        })
    }

    fn render_struct(
        &mut self,
        strct: &Struct,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        self.render_fields(&strct.fields, generics, path, depth, true)
    }

    fn render_fields(
        &mut self,
        fields: &Fields,
        generics: &Generics,
        path: &str,
        depth: usize,
        deny_unknown_fields: bool,
    ) -> Result<Value, Error> {
        match fields {
            Fields::Unit => Ok(object([("type", string("null"))])),
            Fields::Unnamed(fields) => {
                self.render_unnamed_fields(&fields.fields, generics, path, depth)
            }
            Fields::Named(fields) => {
                let mut properties = Map::new();
                let mut required = Vec::new();

                for (name, field) in &fields.fields {
                    let Some(ty) = &field.ty else {
                        continue;
                    };

                    let field_path = format!("{path}.{name}");
                    let mut schema = self.render_datatype(ty, generics, &field_path, depth + 1)?;
                    self.apply_metadata(&mut schema, None, &field.docs, field.deprecated.as_ref());
                    properties.insert(name.to_string(), schema);
                    if !field.optional {
                        required.push(string(name.as_ref()));
                    }
                }

                let mut schema = Map::new();
                schema.insert("type".to_string(), string("object"));
                schema.insert("properties".to_string(), Value::Object(properties));
                if !required.is_empty() {
                    schema.insert("required".to_string(), Value::Array(required));
                }
                if deny_unknown_fields && !self.allow_additional_properties {
                    schema.insert("additionalProperties".to_string(), Value::Bool(false));
                }
                Ok(Value::Object(schema))
            }
        }
    }

    fn render_unnamed_fields(
        &mut self,
        fields: &[Field],
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        // Skipped `ty: None` slots (kept by skip-reduced tuples to preserve
        // their declared arity) are off-wire — serde emits only the live
        // elements — so both the prefix items and the size bounds are
        // computed over live fields only.
        let live = fields
            .iter()
            .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
            .collect::<Vec<_>>();
        let items = live
            .iter()
            .enumerate()
            .map(|(idx, (_, ty))| {
                self.render_datatype(ty, generics, &format!("{path}.{idx}"), depth + 1)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if fields.len() == 1 {
            return Ok(items.into_iter().next().unwrap_or(Value::Bool(false)));
        }
        // serde accepts sequences truncated anywhere inside the trailing run
        // of defaulted (`optional`) elements, so those don't count toward
        // `minItems`. A single declared field is a newtype (bare wire value,
        // nothing to omit), so its `optional` flag is ignored — mirroring
        // the TypeScript renderer.
        let mut min_items = items.len();
        if fields.len() > 1 {
            min_items = 0;
            for (idx, (field, _)) in live.iter().enumerate() {
                if !field.optional {
                    min_items = idx + 1;
                }
            }
        }
        Ok(self.tuple_schema(items, min_items))
    }

    fn render_tuple(
        &mut self,
        tuple: &Tuple,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        if tuple.elements.is_empty() {
            return Ok(object([("type", string("null"))]));
        }

        let items = tuple
            .elements
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                self.render_datatype(ty, generics, &format!("{path}.{idx}"), depth + 1)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let min_items = items.len();
        Ok(self.tuple_schema(items, min_items))
    }

    fn tuple_schema(&self, items: Vec<Value>, min_items: usize) -> Value {
        let mut schema = Map::new();
        schema.insert("type".to_string(), string("array"));
        if self.schema_version.uses_prefix_items() {
            schema.insert("prefixItems".to_string(), Value::Array(items.clone()));
            schema.insert("items".to_string(), Value::Bool(false));
        } else {
            schema.insert("items".to_string(), Value::Array(items.clone()));
            schema.insert("additionalItems".to_string(), Value::Bool(false));
        }
        schema.insert("minItems".to_string(), number(min_items));
        schema.insert("maxItems".to_string(), number(items.len()));
        Value::Object(schema)
    }

    fn render_enum(
        &mut self,
        enm: &Enum,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        let untagged = enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED);
        let rewritten = enm.attributes.contains_key(SERDE_ENUM_REPR_REWRITTEN);
        let variants = enm
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .map(|(name, variant)| {
                self.render_variant(
                    name.as_ref(),
                    variant,
                    untagged,
                    rewritten,
                    generics,
                    path,
                    depth + 1,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let any_of = self.allow_additional_properties
            || untagged
            || enm.variants.iter().any(|(_, variant)| {
                !variant.skip && variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED)
            });

        match variants.as_slice() {
            [] => Ok(Value::Bool(false)),
            [variant] => Ok(variant.clone()),
            _ if any_of => Ok(object([("anyOf", Value::Array(variants))])),
            _ => Ok(object([("oneOf", Value::Array(variants))])),
        }
    }

    fn render_variant(
        &mut self,
        name: &str,
        variant: &Variant,
        untagged: bool,
        rewritten: bool,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        let schema = if untagged || variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED) {
            self.render_untagged_variant(variant, generics, path, depth)?
        } else {
            match &variant.fields {
                Fields::Unit => object([("const", string(name))]),
                Fields::Unnamed(fields) => {
                    let rewritten_payload = if rewritten
                        && let [field] = fields.fields.as_slice()
                        && let Some(ty) = &field.ty
                    {
                        let payload = self.render_datatype(ty, generics, path, depth + 1)?;
                        (!schema_has_const(&payload, name)).then_some(payload)
                    } else {
                        None
                    };

                    if let Some(payload) = rewritten_payload {
                        payload
                    } else {
                        let payload =
                            self.render_unnamed_fields(&fields.fields, generics, path, depth + 1)?;
                        if rewritten && schema_has_const(&payload, name) {
                            object([("const", string(name))])
                        } else {
                            self.external_variant_schema(name, payload)
                        }
                    }
                }
                Fields::Named(fields) => {
                    let payload = self.render_fields(
                        &Fields::Named(fields.clone()),
                        generics,
                        path,
                        depth + 1,
                        true,
                    )?;
                    if rewritten
                        && (schema_has_const(&payload, name) || schema_has_property(&payload, name))
                    {
                        payload
                    } else {
                        self.external_variant_schema(name, payload)
                    }
                }
            }
        };

        let mut schema = schema;
        self.apply_metadata(
            &mut schema,
            None,
            &variant.docs,
            variant.deprecated.as_ref(),
        );
        Ok(schema)
    }

    fn render_untagged_variant(
        &mut self,
        variant: &Variant,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        match &variant.fields {
            Fields::Unit => Ok(object([("type", string("null"))])),
            Fields::Unnamed(fields) => {
                if let [field] = fields.fields.as_slice() {
                    return field.ty.as_ref().map_or_else(
                        || Ok(Value::Object(Map::new())),
                        |ty| self.render_datatype(ty, generics, path, depth + 1),
                    );
                }

                self.render_unnamed_fields(&fields.fields, generics, path, depth + 1)
            }
            Fields::Named(fields) => self.render_fields(
                &Fields::Named(fields.clone()),
                generics,
                path,
                depth + 1,
                true,
            ),
        }
    }

    fn external_variant_schema(&self, name: &str, payload: Value) -> Value {
        let mut properties = Map::new();
        properties.insert(name.to_string(), payload);
        object([
            ("type", string("object")),
            ("required", Value::Array(vec![string(name)])),
            ("properties", Value::Object(properties)),
            ("additionalProperties", Value::Bool(false)),
        ])
    }

    fn apply_metadata(
        &self,
        schema: &mut Value,
        title: Option<&str>,
        docs: &str,
        deprecated: Option<&Deprecated>,
    ) {
        let Value::Object(schema) = schema else {
            return;
        };

        if matches!(self.schema_version, SchemaVersion::Draft7)
            && schema.contains_key("$ref")
            && (title.is_some() || !docs.is_empty() || deprecated.is_some())
        {
            let reference = std::mem::take(schema);
            schema.insert(
                "allOf".to_string(),
                Value::Array(vec![Value::Object(reference)]),
            );
        }

        if let Some(title) = title {
            schema.insert("title".to_string(), string(title));
        }
        let description = metadata_description(docs, deprecated);
        if !description.is_empty() {
            schema.insert("description".to_string(), string(description));
        }
        if deprecated.is_some() {
            schema.insert("deprecated".to_string(), Value::Bool(true));
        }
    }

    fn ref_path(&self, key: &str) -> String {
        format!(
            "#/{}/{}",
            self.schema_version.definitions_key(),
            encode_ref_token(key)
        )
    }

    fn definition_key(&self, ndt: &NamedDataType, generics: &Generics) -> String {
        let mut key = if self.name_counts.get(&ndt.name).copied().unwrap_or_default() > 1
            && !ndt.module_path.is_empty()
        {
            format!("{}::{}", ndt.module_path, ndt.name)
        } else {
            ndt.name.to_string()
        };

        if !generics.is_empty() {
            key.push('<');
            key.push_str(
                &ndt.generics
                    .iter()
                    .filter_map(|generic| generics.get(&generic.name))
                    .map(|ty| self.datatype_key(ty))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            key.push('>');
        }

        key
    }

    fn datatype_key(&self, dt: &DataType) -> String {
        match dt {
            DataType::Primitive(primitive) => format!("{primitive:?}"),
            DataType::Generic(generic) => generic.name().to_string(),
            DataType::List(list) => format!("Array_{}", self.datatype_key(&list.ty)),
            DataType::Map(map) => format!(
                "Map_{}_{}",
                self.datatype_key(map.key_ty()),
                self.datatype_key(map.value_ty())
            ),
            DataType::Nullable(inner) => format!("Nullable_{}", self.datatype_key(inner)),
            DataType::Tuple(tuple) => format!(
                "Tuple_{}",
                tuple
                    .elements
                    .iter()
                    .map(|ty| self.datatype_key(ty))
                    .collect::<Vec<_>>()
                    .join("_")
            ),
            DataType::Reference(Reference::Named(reference)) => self
                .types
                .get(reference)
                .map(|ndt| match &reference.inner {
                    NamedReferenceType::Reference { generics, .. } => {
                        let generics = self.resolve_generics(ndt, generics, &Generics::new());
                        self.definition_key(ndt, &generics)
                    }
                    _ => self.definition_key(ndt, &Generics::new()),
                })
                .unwrap_or_else(|| "Reference".to_string()),
            DataType::Reference(Reference::Opaque(reference)) => reference.type_name().to_string(),
            other => format!("{other:?}"),
        }
    }
}

fn substitute_generics(dt: &DataType, generics: &Generics) -> DataType {
    let mut dt = dt.clone();
    substitute_generics_mut(&mut dt, generics, &mut BTreeSet::new());
    dt
}

fn substitute_generics_mut(
    dt: &mut DataType,
    generics: &Generics,
    active: &mut BTreeSet<Cow<'static, str>>,
) {
    if let DataType::Generic(generic) = dt {
        let name = generic.name().clone();
        let inserted = active.insert(name.clone());
        if inserted && let Some(replacement) = generics.get(&name) {
            *dt = replacement.clone();
            substitute_generics_mut(dt, generics, active);
        }
        if inserted {
            active.remove(&name);
        }
        return;
    }

    match dt {
        DataType::List(list) => substitute_generics_mut(&mut list.ty, generics, active),
        DataType::Map(map) => {
            substitute_generics_mut(map.key_ty_mut(), generics, active);
            substitute_generics_mut(map.value_ty_mut(), generics, active);
        }
        DataType::Struct(strct) => substitute_fields(&mut strct.fields, generics, active),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_fields(&mut variant.fields, generics, active);
            }
        }
        DataType::Tuple(tuple) => {
            for ty in &mut tuple.elements {
                substitute_generics_mut(ty, generics, active);
            }
        }
        DataType::Nullable(inner) => substitute_generics_mut(inner, generics, active),
        DataType::Intersection(types) => {
            for ty in types {
                substitute_generics_mut(ty, generics, active);
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Inline { dt, .. } => substitute_generics_mut(dt, generics, active),
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => {
                for (_, ty) in reference_generics {
                    substitute_generics_mut(ty, generics, active);
                }
            }
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_fields(
    fields: &mut Fields,
    generics: &Generics,
    active: &mut BTreeSet<Cow<'static, str>>,
) {
    let fields: Vec<_> = match fields {
        Fields::Unit => return,
        Fields::Unnamed(fields) => fields
            .fields
            .iter_mut()
            .map(|field| &mut field.ty)
            .collect(),
        Fields::Named(fields) => fields
            .fields
            .iter_mut()
            .map(|(_, field)| &mut field.ty)
            .collect(),
    };
    for ty in fields.into_iter().flatten() {
        substitute_generics_mut(ty, generics, active);
    }
}

fn default_generics(ndt: &NamedDataType) -> Generics {
    let mut generics = Generics::new();
    for generic in ndt.generics.iter() {
        if let Some(default) = &generic.default {
            generics.insert(
                generic.name.clone(),
                substitute_generics(default, &generics),
            );
        }
    }
    generics
}

fn primitive_schema(primitive: &Primitive) -> Value {
    match primitive {
        Primitive::bool => object([("type", string("boolean"))]),
        Primitive::str => object([("type", string("string"))]),
        Primitive::char => object([
            ("type", string("string")),
            ("minLength", number(1)),
            ("maxLength", number(1)),
        ]),
        Primitive::i8 => integer(Some(i8::MIN.into()), Some(i8::MAX.into())),
        Primitive::i16 => integer(Some(i16::MIN.into()), Some(i16::MAX.into())),
        Primitive::i32 => integer(Some(i32::MIN.into()), Some(i32::MAX.into())),
        Primitive::i64 => integer(Some(i64::MIN.into()), Some(i64::MAX.into())),
        Primitive::i128 | Primitive::isize => integer(None, None),
        Primitive::u8 => integer(Some(0.into()), Some(u8::MAX.into())),
        Primitive::u16 => integer(Some(0.into()), Some(u16::MAX.into())),
        Primitive::u32 => integer(Some(0.into()), Some(u32::MAX.into())),
        Primitive::u64 => integer(Some(0.into()), Some(u64::MAX.into())),
        Primitive::u128 | Primitive::usize => integer(Some(0.into()), None),
        Primitive::f16 | Primitive::f32 | Primitive::f64 | Primitive::f128 => number_schema(None),
    }
}

fn integer(minimum: Option<Number>, maximum: Option<Number>) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), string("integer"));
    if let Some(minimum) = minimum {
        schema.insert("minimum".to_string(), Value::Number(minimum));
    }
    if let Some(maximum) = maximum {
        schema.insert("maximum".to_string(), Value::Number(maximum));
    }
    Value::Object(schema)
}

fn number_schema(format: Option<&str>) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), string("number"));
    if let Some(format) = format {
        schema.insert("format".to_string(), string(format));
    }
    Value::Object(schema)
}

fn pattern_schema(pattern: &str) -> Value {
    object([("type", string("string")), ("pattern", string(pattern))])
}

fn object<const N: usize>(entries: [(&str, Value); N]) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn string(value: impl AsRef<str>) -> Value {
    Value::String(value.as_ref().to_string())
}

fn number(value: usize) -> Value {
    Value::Number(Number::from(value))
}

fn rust_type_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn metadata_description(docs: &str, deprecated: Option<&Deprecated>) -> String {
    let mut description = docs.to_string();
    let Some(deprecated) = deprecated else {
        return description;
    };
    let Some(note) = &deprecated.note else {
        return description;
    };

    if !description.is_empty() && !description.ends_with('\n') {
        description.push('\n');
    }
    description.push_str("Deprecated");
    if let Some(since) = &deprecated.since {
        description.push_str(" since ");
        description.push_str(since);
    }
    description.push_str(": ");
    description.push_str(note);
    description
}

fn merge_object_intersection(parts: &[Value], allow_additional_properties: bool) -> Option<Value> {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for part in parts {
        let part = part.as_object()?;
        if part.get("type").and_then(Value::as_str) != Some("object") {
            return None;
        }

        for (name, property) in part.get("properties")?.as_object()? {
            if let Some(existing) = properties.remove(name) {
                properties.insert(
                    name.clone(),
                    object([("allOf", Value::Array(vec![existing, property.clone()]))]),
                );
            } else {
                properties.insert(name.clone(), property.clone());
            }
        }

        if let Some(part_required) = part.get("required").and_then(Value::as_array) {
            for name in part_required {
                if !required.contains(name) {
                    required.push(name.clone());
                }
            }
        }
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), string("object"));
    schema.insert("properties".to_string(), Value::Object(properties));
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    if !allow_additional_properties {
        schema.insert("additionalProperties".to_string(), Value::Bool(false));
    }
    Some(Value::Object(schema))
}

pub(crate) fn encode_ref_token(value: &str) -> String {
    let pointer = value.replace('~', "~0").replace('/', "~1");
    let mut encoded = String::with_capacity(pointer.len());
    for byte in pointer.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn schema_has_const(schema: &Value, value: &str) -> bool {
    match schema {
        Value::Object(schema) => {
            schema
                .get("const")
                .and_then(Value::as_str)
                .is_some_and(|const_| const_ == value)
                || schema
                    .values()
                    .any(|value_| schema_has_const(value_, value))
        }
        Value::Array(values) => values.iter().any(|value_| schema_has_const(value_, value)),
        _ => false,
    }
}

fn schema_has_property(schema: &Value, property: &str) -> bool {
    schema
        .as_object()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key(property))
}
