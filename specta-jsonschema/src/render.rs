use std::{borrow::Cow, collections::BTreeMap};

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

type Generics = BTreeMap<Cow<'static, str>, DataType>;

pub(crate) struct Renderer<'a> {
    schema_version: SchemaVersion,
    types: &'a Types,
    definitions: Map<String, Value>,
    in_progress: Vec<String>,
    name_counts: BTreeMap<Cow<'static, str>, usize>,
    definition_owners: BTreeMap<String, (NamedDataType, Generics)>,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(schema_version: SchemaVersion, types: &'a Types) -> Self {
        let mut name_counts = BTreeMap::new();
        for ndt in types.into_sorted_iter() {
            *name_counts.entry(ndt.name.clone()).or_default() += 1;
        }

        Self {
            schema_version,
            types,
            definitions: Map::new(),
            in_progress: Vec::new(),
            name_counts,
            definition_owners: BTreeMap::new(),
        }
    }

    pub(crate) fn render_definitions(
        mut self,
        roots: &[DataType],
    ) -> Result<Map<String, Value>, Error> {
        for root in roots {
            self.render_datatype(root, &Generics::new(), "registered root", 0)?;
        }

        for ndt in self.types.into_sorted_iter() {
            if ndt.ty.is_none() {
                continue;
            }

            let generics = ndt
                .generics
                .iter()
                .filter_map(|generic| {
                    generic
                        .default
                        .clone()
                        .map(|default| (generic.name.clone(), default))
                })
                .collect::<Generics>();
            // JSON Schema and OpenAPI have no generic declarations. Concrete
            // instantiations are materialized while rendering references; an
            // unconstrained template would only create a misleading `{}` hole.
            if generics.len() != ndt.generics.len() {
                continue;
            }
            let key = self.definition_key(ndt, &generics);
            self.ensure_definition(ndt, key, generics, "$defs")?;
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
        if let Some((owner, owner_generics)) = self.definition_owners.get(&key) {
            if owner != ndt || owner_generics != &generics {
                return Err(Error::DuplicateDefinitionName {
                    name: key,
                    first: type_path(owner),
                    second: type_path(ndt),
                });
            }
        } else {
            self.definition_owners
                .insert(key.clone(), (ndt.clone(), generics.clone()));
        }

        if self.definitions.contains_key(&key) || self.in_progress.contains(&key) {
            return Ok(());
        }

        let Some(ty) = &ndt.ty else {
            self.definitions.insert(key, Value::Object(Map::new()));
            return Ok(());
        };

        self.in_progress.push(key.clone());
        let mut schema = self.render_datatype(ty, &generics, path, 0)?;
        self.apply_metadata(
            &mut schema,
            Some(ndt.name.as_ref()),
            &ndt.docs,
            ndt.deprecated.as_ref(),
        );
        self.in_progress.pop();

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
            DataType::Generic(generic) => generics
                .get(generic.name())
                .cloned()
                .map(|ty| self.render_datatype(&ty, generics, path, depth + 1))
                .unwrap_or_else(|| Ok(Value::Object(Map::new()))),
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
        let mut schema = Map::new();
        schema.insert(
            "allOf".to_string(),
            Value::Array(
                types
                    .iter()
                    .map(|ty| self.render_datatype_for_intersection(ty, generics, path, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
        );
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
        let mut generics = ndt
            .generics
            .iter()
            .filter_map(|generic| {
                generic
                    .default
                    .clone()
                    .map(|default| (generic.name.clone(), default))
            })
            .collect::<Generics>();

        for (generic, ty) in reference_generics {
            generics.insert(
                generic.name().clone(),
                substitute_generics(ty, parent_generics),
            );
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
        if !is_valid_map_key(map.key_ty(), generics) {
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
            DataType::Generic(generic) => generics
                .get(generic.name())
                .map(|ty| self.map_key_schema(ty, generics, path, depth))
                .transpose()?
                .flatten(),
            DataType::Reference(Reference::Named(reference)) => {
                if self.is_unconstrained_string_map_key(
                    &DataType::Reference(Reference::Named(reference.clone())),
                    generics,
                    0,
                ) {
                    return Ok(None);
                }
                let schema = self.render_named_reference(reference, generics, path, depth)?;
                (!is_unconstrained_string_schema(&schema)).then_some(schema)
            }
            DataType::Nullable(inner) => self.map_key_schema(inner, generics, path, depth)?,
            _ => None,
        })
    }

    fn is_unconstrained_string_map_key(
        &self,
        dt: &DataType,
        generics: &Generics,
        depth: usize,
    ) -> bool {
        if depth > INLINE_RECURSION_LIMIT {
            return false;
        }

        match dt {
            DataType::Primitive(Primitive::str) => true,
            DataType::Generic(generic) => generics
                .get(generic.name())
                .is_some_and(|ty| self.is_unconstrained_string_map_key(ty, generics, depth + 1)),
            DataType::Struct(strct) => match &strct.fields {
                Fields::Unnamed(fields) if fields.fields.len() == 1 => {
                    fields.fields[0].ty.as_ref().is_some_and(|ty| {
                        self.is_unconstrained_string_map_key(ty, generics, depth + 1)
                    })
                }
                _ => false,
            },
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                NamedReferenceType::Inline { dt, .. } => {
                    self.is_unconstrained_string_map_key(dt, generics, depth + 1)
                }
                NamedReferenceType::Reference {
                    generics: reference_generics,
                    ..
                } => self.types.get(reference).is_some_and(|ndt| {
                    let generics = self.resolve_generics(ndt, reference_generics, generics);
                    ndt.ty.as_ref().is_some_and(|ty| {
                        self.is_unconstrained_string_map_key(ty, &generics, depth + 1)
                    })
                }),
                NamedReferenceType::Recursive(_) => false,
            },
            _ => false,
        }
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
                if let [field] = fields.fields.as_slice() {
                    return field.ty.as_ref().map_or_else(
                        || Ok(Value::Object(Map::new())),
                        |ty| {
                            let mut schema = self.render_datatype(ty, generics, path, depth + 1)?;
                            self.apply_metadata(
                                &mut schema,
                                None,
                                &field.docs,
                                field.deprecated.as_ref(),
                            );
                            Ok(schema)
                        },
                    );
                }
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
                if deny_unknown_fields {
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
        let untagged = enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED)
            || enm
                .variants
                .iter()
                .any(|(_, variant)| variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED));
        let variants = enm
            .variants
            .iter()
            .filter(|(_, variant)| !variant.skip)
            .map(|(name, variant)| {
                self.render_variant(
                    name.as_ref(),
                    variant,
                    enm.attributes.contains_key(SERDE_CONTAINER_UNTAGGED),
                    generics,
                    path,
                    depth + 1,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        match variants.as_slice() {
            [] => Ok(Value::Bool(false)),
            [variant] => Ok(variant.clone()),
            _ if untagged => Ok(object([("anyOf", Value::Array(variants))])),
            _ => Ok(object([("oneOf", Value::Array(variants))])),
        }
    }

    fn render_variant(
        &mut self,
        name: &str,
        variant: &Variant,
        untagged: bool,
        generics: &Generics,
        path: &str,
        depth: usize,
    ) -> Result<Value, Error> {
        if untagged || variant.attributes.contains_key(SERDE_VARIANT_UNTAGGED) {
            return self.render_untagged_variant(variant, generics, path, depth);
        }

        let schema = match &variant.fields {
            Fields::Unit => object([("const", string(name))]),
            Fields::Unnamed(fields) => {
                if let [field] = fields.fields.as_slice()
                    && let Some(ty) = &field.ty
                {
                    let payload = self.render_datatype(ty, generics, path, depth + 1)?;
                    if !schema_has_const(&payload, name) {
                        return Ok(payload);
                    }
                }

                let payload =
                    self.render_unnamed_fields(&fields.fields, generics, path, depth + 1)?;
                if schema_has_const(&payload, name) {
                    object([("const", string(name))])
                } else {
                    self.external_variant_schema(name, payload)
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
                if schema_has_const(&payload, name) || schema_has_property(&payload, name) {
                    payload
                } else {
                    self.external_variant_schema(name, payload)
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

        if let Some(title) = title {
            schema.insert("title".to_string(), string(title));
        }
        if !docs.is_empty() {
            schema.insert("description".to_string(), string(docs));
        }
        if deprecated.is_some() {
            schema.insert("deprecated".to_string(), Value::Bool(true));
        }
    }

    fn ref_path(&self, key: &str) -> String {
        format!(
            "#/{}/{}",
            self.schema_version.definitions_key(),
            escape_json_pointer(key)
        )
    }

    fn definition_key(&self, ndt: &NamedDataType, generics: &Generics) -> String {
        let mut key = if self.name_counts.get(&ndt.name).copied().unwrap_or_default() > 1
            && !ndt.module_path.is_empty()
        {
            format!(
                "{}_{}",
                sanitise_name(&ndt.module_path.replace("::", "_")),
                sanitise_name(&ndt.name)
            )
        } else {
            sanitise_name(&ndt.name)
        };

        if !generics.is_empty() {
            for ty in generics.values() {
                key.push('_');
                key.push_str(&self.datatype_key(ty));
            }
        }

        key
    }

    fn datatype_key(&self, dt: &DataType) -> String {
        let key = match dt {
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
        };

        sanitise_name(&key)
    }
}

fn is_unconstrained_string_schema(schema: &Value) -> bool {
    schema.as_object().is_some_and(|schema| {
        schema.get("type").and_then(Value::as_str) == Some("string") && schema.len() == 1
    })
}

fn type_path(ndt: &NamedDataType) -> String {
    if ndt.module_path.is_empty() {
        ndt.name.to_string()
    } else {
        format!("{}::{}", ndt.module_path, ndt.name)
    }
}

fn substitute_generics(dt: &DataType, generics: &Generics) -> DataType {
    match dt {
        DataType::Generic(generic) => generics
            .get(generic.name())
            .cloned()
            .unwrap_or_else(|| dt.clone()),
        DataType::List(list) => {
            let mut list_ = List::new(substitute_generics(&list.ty, generics));
            list_.length = list.length;
            list_.unique = list.unique;
            DataType::List(list_)
        }
        DataType::Map(map) => DataType::Map(SpectaMap::new(
            substitute_generics(map.key_ty(), generics),
            substitute_generics(map.value_ty(), generics),
        )),
        DataType::Nullable(inner) => {
            DataType::Nullable(Box::new(substitute_generics(inner, generics)))
        }
        DataType::Tuple(tuple) => DataType::Tuple(Tuple::new(
            tuple
                .elements
                .iter()
                .map(|ty| substitute_generics(ty, generics))
                .collect(),
        )),
        DataType::Intersection(types) => DataType::Intersection(
            types
                .iter()
                .map(|ty| substitute_generics(ty, generics))
                .collect(),
        ),
        _ => dt.clone(),
    }
}

fn is_valid_map_key(dt: &DataType, generics: &Generics) -> bool {
    match dt {
        DataType::Primitive(_)
        | DataType::Nullable(_)
        | DataType::Reference(Reference::Named(_))
        | DataType::Reference(Reference::Opaque(_)) => true,
        DataType::Generic(generic) => generics
            .get(generic.name())
            .is_none_or(|ty| is_valid_map_key(ty, generics)),
        DataType::List(_)
        | DataType::Map(_)
        | DataType::Struct(_)
        | DataType::Enum(_)
        | DataType::Tuple(_)
        | DataType::Intersection(_) => false,
    }
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
        Primitive::i8 => integer(Some(i8::MIN.into()), Some(i8::MAX.into()), Some("int32")),
        Primitive::i16 => integer(Some(i16::MIN.into()), Some(i16::MAX.into()), Some("int32")),
        Primitive::i32 => integer(Some(i32::MIN.into()), Some(i32::MAX.into()), Some("int32")),
        Primitive::i64 => integer(Some(i64::MIN), Some(i64::MAX), Some("int64")),
        Primitive::i128 => integer(None, None, Some("int128")),
        Primitive::isize => integer(None, None, Some("isize")),
        Primitive::u8 => integer(Some(0), Some(u8::MAX.into()), Some("int32")),
        Primitive::u16 => integer(Some(0), Some(u16::MAX.into()), Some("int32")),
        Primitive::u32 => integer(Some(0), Some(u32::MAX.into()), Some("int64")),
        Primitive::u64 => integer(Some(0), None, Some("uint64")),
        Primitive::u128 => integer(Some(0), None, Some("uint128")),
        Primitive::usize => integer(Some(0), None, Some("usize")),
        Primitive::f16 | Primitive::f32 => number_schema(Some("float")),
        Primitive::f64 => number_schema(Some("double")),
        Primitive::f128 => number_schema(Some("float128")),
    }
}

fn integer(minimum: Option<i64>, maximum: Option<i64>, format: Option<&str>) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), string("integer"));
    if let Some(minimum) = minimum {
        schema.insert("minimum".to_string(), Value::Number(minimum.into()));
    }
    if let Some(maximum) = maximum {
        schema.insert("maximum".to_string(), Value::Number(maximum.into()));
    }
    if let Some(format) = format {
        schema.insert("format".to_string(), string(format));
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

fn sanitise_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() {
        "Type".to_string()
    } else {
        out
    }
}

fn escape_json_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
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
