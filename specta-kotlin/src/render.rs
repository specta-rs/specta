use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use specta::{
    Format, Types,
    datatype::{
        DataType, Enum, Field, Fields, Generic, NamedDataType, NamedReferenceType, Primitive,
        Reference, Struct, Variant,
    },
};

use crate::{
    Error, Kotlin, Layout, NamingConvention, Serialization, UnknownType,
    reserved_names::{GENERIC_MODIFIERS, RESERVED, ROOT_NAMESPACES},
};

pub(crate) enum Selection<'a> {
    All,
    One(&'a NamedDataType),
    RawOnly,
}

pub(crate) fn render_file(
    kotlin: &Kotlin,
    types: &Types,
    format: Option<&dyn Format>,
    selection: Selection<'_>,
) -> Result<String, Error> {
    let mut out = String::new();
    if !kotlin.header.is_empty() {
        out.push_str(kotlin.header.trim_end());
        out.push_str("\n\n");
    }
    if kotlin.serialization == Serialization::Kotlinx {
        out.push_str(
            "@file:kotlin.OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)\n\n",
        );
    }
    if let Some(package) = &kotlin.package {
        out.push_str("package ");
        out.push_str(&package_name(package)?);
        out.push_str("\n\n");
    }
    match selection {
        Selection::All => {
            let mut names = BTreeMap::new();
            for ndt in types.iter_sorted().filter(|ndt| ndt.ty.is_some()) {
                let name = named_type_identifier(kotlin, ndt, &ndt.name)?;
                if names.insert(name.clone(), ndt.name.as_ref()).is_some() {
                    return Err(Error::DuplicateTypeName { name });
                }
                render_named(&mut out, kotlin, format, types, ndt)?;
                out.push_str("\n\n");
            }
            render_raw(&mut out, kotlin);
        }
        Selection::One(ndt) => render_named(&mut out, kotlin, format, types, ndt)?,
        Selection::RawOnly => render_raw(&mut out, kotlin),
    }

    Ok(out.trim_end().to_owned() + "\n")
}

fn render_raw(out: &mut String, kotlin: &Kotlin) {
    for (index, raw) in kotlin.raw().iter().enumerate() {
        if index != 0 || !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(raw.trim());
        out.push('\n');
    }
}

pub(crate) fn filename(kotlin: &Kotlin, name: &str) -> Result<String, Error> {
    let converted = convert_name(kotlin.naming, name, NameKind::Type);
    validate_identifier(&converted, name)?;
    Ok(converted)
}

fn render_named(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    ndt: &NamedDataType,
) -> Result<(), Error> {
    let Some(original) = &ndt.ty else {
        return Ok(());
    };
    let ty = map_datatype(format, types, original)?;
    render_kdoc(out, "", &ndt.docs);
    render_deprecated(out, "", ndt.deprecated.as_ref());

    let name = named_type_identifier(kotlin, ndt, &ndt.name)?;
    let generics = ndt
        .generics
        .iter()
        .map(|generic| generic_identifier(&generic.name, &ndt.name))
        .collect::<Result<Vec<_>, _>>()?;
    ensure_unique(
        ndt.name.as_ref(),
        std::iter::once(name.as_str()).chain(generics.iter().map(String::as_str)),
    )?;
    let generic_scope = ndt
        .generics
        .iter()
        .map(|generic| generic.reference())
        .collect::<Vec<_>>();

    match &ty {
        DataType::Struct(strct) => render_struct(
            out,
            kotlin,
            format,
            types,
            &name,
            &generics,
            &generic_scope,
            strct,
            &ndt.name,
        ),
        DataType::Enum(enm) => render_enum(
            out,
            kotlin,
            format,
            types,
            &name,
            &generics,
            &generic_scope,
            enm,
            &ndt.name,
        ),
        DataType::Generic(generic) => {
            if kotlin.serialization == Serialization::Kotlinx && kotlin.mutable_properties {
                return Err(Error::UnsupportedType {
                    path: ndt.name.to_string(),
                    reason: "mutable Kotlinx generic wrappers cannot preserve scalar encoding",
                });
            }
            annotation(out, "", kotlin, "Serializable", None);
            if kotlin.mutable_properties {
                out.push_str("public data class ");
            } else {
                out.push_str("@kotlin.jvm.JvmInline\npublic value class ");
            }
            out.push_str(&name);
            out.push_str(&generic_declaration(&generics));
            out.push_str("(public ");
            out.push_str(property_keyword(kotlin));
            out.push_str(" value: ");
            out.push_str(&datatype(
                kotlin,
                format,
                types,
                &DataType::Generic(generic.clone()),
                &generic_scope,
                &ndt.name,
            )?);
            out.push(')');
            Ok(())
        }
        _ => {
            if contains_named_reference(format, types, &ty, ndt)? {
                return Err(Error::UnsupportedType {
                    path: ndt.name.to_string(),
                    reason: "recursive Kotlin typealiases are not supported",
                });
            }
            out.push_str("public typealias ");
            out.push_str(&name);
            out.push_str(&generic_declaration(&generics));
            out.push_str(" = ");
            out.push_str(&datatype(
                kotlin,
                format,
                types,
                &ty,
                &generic_scope,
                &ndt.name,
            )?);
            Ok(())
        }
    }
}

fn contains_named_reference(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    target: &NamedDataType,
) -> Result<bool, Error> {
    contains_named_reference_inner(
        format,
        types,
        dt,
        target,
        &mut vec![target as *const NamedDataType],
    )
}

fn contains_named_reference_inner(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    target: &NamedDataType,
    visited: &mut Vec<*const NamedDataType>,
) -> Result<bool, Error> {
    match dt {
        DataType::Primitive(_)
        | DataType::Generic(_)
        | DataType::Reference(Reference::Opaque(_)) => Ok(false),
        DataType::List(list) => {
            contains_named_reference_inner(format, types, &list.ty, target, visited)
        }
        DataType::Map(map) => {
            Ok(
                contains_named_reference_inner(format, types, map.key_ty(), target, visited)?
                    || contains_named_reference_inner(
                        format,
                        types,
                        map.value_ty(),
                        target,
                        visited,
                    )?,
            )
        }
        DataType::Nullable(inner) => {
            contains_named_reference_inner(format, types, inner, target, visited)
        }
        DataType::Tuple(tuple) => {
            for ty in &tuple.elements {
                if contains_named_reference_inner(format, types, ty, target, visited)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DataType::Struct(strct) => {
            fields_contain_named_reference(format, types, &strct.fields, target, visited)
        }
        DataType::Enum(enm) => {
            for (_, variant) in &enm.variants {
                if fields_contain_named_reference(format, types, &variant.fields, target, visited)?
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DataType::Reference(Reference::Named(reference)) => {
            match &reference.inner {
                NamedReferenceType::Reference { generics, .. } => {
                    for (_, ty) in generics {
                        if contains_named_reference_inner(format, types, ty, target, visited)? {
                            return Ok(true);
                        }
                    }
                }
                NamedReferenceType::Inline { dt, .. } => {
                    return contains_named_reference_inner(format, types, dt, target, visited);
                }
                NamedReferenceType::Recursive(_) => return Ok(false),
            }

            let Some(ndt) = types.get(reference) else {
                return Ok(false);
            };
            if ndt == target {
                return Ok(true);
            }
            let pointer = ndt as *const NamedDataType;
            if visited.contains(&pointer) {
                return Ok(false);
            }
            let Some(original) = &ndt.ty else {
                return Ok(false);
            };
            let mapped = map_datatype(format, types, original)?;
            if matches!(mapped, DataType::Struct(_) | DataType::Enum(_)) {
                return Ok(false);
            }
            visited.push(pointer);
            let result = contains_named_reference_inner(format, types, &mapped, target, visited);
            visited.pop();
            result
        }
        DataType::Intersection(types_) => {
            for ty in types_ {
                if contains_named_reference_inner(format, types, ty, target, visited)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn fields_contain_named_reference(
    format: Option<&dyn Format>,
    types: &Types,
    fields: &Fields,
    target: &NamedDataType,
    visited: &mut Vec<*const NamedDataType>,
) -> Result<bool, Error> {
    let fields = match fields {
        Fields::Named(fields) => fields
            .fields
            .iter()
            .map(|(_, field)| field)
            .collect::<Vec<_>>(),
        Fields::Unnamed(fields) => fields.fields.iter().collect::<Vec<_>>(),
        Fields::Unit => return Ok(false),
    };
    for field in fields {
        if let Some(ty) = &field.ty
            && contains_named_reference_inner(format, types, ty, target, visited)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[allow(clippy::too_many_arguments)]
fn render_struct(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    name: &str,
    generics: &[String],
    generic_scope: &[Generic],
    strct: &Struct,
    path: &str,
) -> Result<(), Error> {
    if kotlin.serialization == Serialization::Kotlinx {
        match &strct.fields {
            Fields::Unit => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx object serialization does not preserve unit-struct null encoding",
                });
            }
            Fields::Unnamed(fields)
                if fields.fields.len() != 1 || fields.fields[0].ty.is_none() =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx data classes do not preserve tuple-struct array encoding",
                });
            }
            Fields::Unnamed(_) if kotlin.mutable_properties => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "mutable Kotlinx newtypes cannot preserve scalar encoding",
                });
            }
            Fields::Unnamed(_) | Fields::Named(_) => {}
        }
    }
    annotation(out, "", kotlin, "Serializable", None);
    match &strct.fields {
        Fields::Unit => {
            if generics.is_empty() {
                out.push_str("public data object ");
                out.push_str(name);
            } else {
                out.push_str("public class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
            }
        }
        Fields::Named(fields) if fields.fields.iter().all(|(_, field)| field.ty.is_none()) => {
            out.push_str("public class ");
            out.push_str(name);
            out.push_str(&generic_declaration(generics));
        }
        Fields::Named(fields) => {
            ensure_unique(
                path,
                fields
                    .fields
                    .iter()
                    .filter(|(_, field)| field.ty.is_some())
                    .map(|(name, _)| {
                        safe_member_name(
                            &convert_name(kotlin.naming, name, NameKind::Property),
                            "field",
                        )
                    }),
            )?;
            out.push_str("public data class ");
            out.push_str(name);
            out.push_str(&generic_declaration(generics));
            out.push_str("(\n");
            let fields = fields
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            for (index, (field_name, field)) in fields.iter().enumerate() {
                render_property(
                    out,
                    kotlin,
                    format,
                    types,
                    generic_scope,
                    field_name,
                    field,
                    &format!("{path}.{field_name}"),
                    1,
                    None,
                )?;
                if index + 1 != fields.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push(')');
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.len() == 1 && !kotlin.mutable_properties {
                out.push_str("@kotlin.jvm.JvmInline\npublic value class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(public ");
                out.push_str(property_keyword(kotlin));
                out.push_str(" value: ");
                let (index, field) = fields[0];
                out.push_str(&field_datatype(
                    kotlin,
                    format,
                    types,
                    generic_scope,
                    field,
                    &format!("{path}.{index}"),
                    true,
                    None,
                )?);
                if field.optional {
                    out.push_str(" = null");
                }
                out.push(')');
            } else if fields.is_empty() {
                out.push_str("public class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
            } else {
                out.push_str("public data class ");
                out.push_str(name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                for (position, (index, field)) in fields.iter().enumerate() {
                    let field_name = format!("field{index}");
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        &field_name,
                        field,
                        &format!("{path}.{index}"),
                        1,
                        None,
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push(')');
            }
        }
    }
    Ok(())
}

/// How Serde represents an enum on the wire.
enum EnumRepr<'a> {
    /// A plain externally tagged enum: `"Variant"` or `{"Variant": payload}`.
    External,
    /// `specta-serde` lowered the tag into every variant, covering Serde's internal and
    /// adjacent representations.
    Tagged { tag_key: Cow<'a, str> },
    /// `#[serde(untagged)]`: the payload is the value with no discriminator.
    Untagged,
    /// A rewritten representation with no uniform tag (per-variant `#[serde(untagged)]`, a
    /// `#[serde(other)]` catch-all, ...).
    Custom,
}

/// One variant after its Serde representation has been resolved.
struct VariantShape<'a> {
    /// Rust variant name.
    name: &'a str,
    /// Wire tag for the variant, already carrying `serde`'s renames.
    wire: Cow<'a, str>,
    /// Payload fields, with any lowered tag field removed.
    fields: Fields,
}

struct EnumShape<'a> {
    repr: EnumRepr<'a>,
    variants: Vec<VariantShape<'a>>,
}

struct TaggedVariant<'a> {
    tag_key: Cow<'a, str>,
    wire: Cow<'a, str>,
    payload: Fields,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PayloadShape {
    Unit,
    Newtype,
    Struct,
    Tuple,
}

fn enum_shape<'a>(
    format: Option<&dyn Format>,
    types: &Types,
    enm: &'a Enum,
    variants: &[(&'a Cow<'a, str>, &'a Variant)],
    path: &str,
) -> Result<EnumShape<'a>, Error> {
    let rewritten = enm
        .attributes
        .contains_key("specta_serde:enum_repr_rewritten");
    let untagged = enm.attributes.contains_key("serde:container:untagged");

    if untagged {
        return Ok(EnumShape {
            repr: EnumRepr::Untagged,
            variants: variants
                .iter()
                .copied()
                .map(|(name, variant)| VariantShape {
                    name,
                    wire: Cow::Borrowed(name.as_ref()),
                    fields: normalized_variant_fields(name, &variant.fields, rewritten, true),
                })
                .collect(),
        });
    }

    if rewritten {
        let mut tag_key: Option<Cow<'a, str>> = None;
        let mut shapes = Vec::with_capacity(variants.len());
        let mut uniform = true;
        let mut tagged_any = false;
        for (name, variant) in variants.iter().copied() {
            let Some(tagged) = tagged_variant(variant, types, format, path)? else {
                uniform = false;
                continue;
            };
            tagged_any = true;
            match &tag_key {
                Some(key) if *key != tagged.tag_key => {
                    uniform = false;
                }
                Some(_) => {}
                None => tag_key = Some(tagged.tag_key.clone()),
            }
            shapes.push(VariantShape {
                name: name.as_ref(),
                wire: tagged.wire,
                fields: tagged.payload,
            });
        }
        if uniform && let Some(tag_key) = tag_key {
            return Ok(EnumShape {
                repr: EnumRepr::Tagged { tag_key },
                variants: shapes,
            });
        }
        // `specta-serde` also marks a plain external enum it only renamed, so an absent tag
        // means there is nothing to lower rather than an unmodelled representation.
        let repr = if tagged_any {
            EnumRepr::Custom
        } else {
            EnumRepr::External
        };
        return Ok(EnumShape {
            repr,
            variants: variants
                .iter()
                .copied()
                .map(|(name, variant)| VariantShape {
                    name,
                    wire: Cow::Borrowed(name.as_ref()),
                    fields: normalized_variant_fields(name, &variant.fields, rewritten, untagged),
                })
                .collect(),
        });
    }

    Ok(EnumShape {
        repr: EnumRepr::External,
        variants: variants
            .iter()
            .copied()
            .map(|(name, variant)| VariantShape {
                name,
                wire: Cow::Borrowed(name.as_ref()),
                fields: normalized_variant_fields(name, &variant.fields, false, false),
            })
            .collect(),
    })
}

/// Recover the tag `specta-serde` lowered into an internally or adjacently tagged variant.
///
/// Both representations end up as a named tag field holding a single-variant unit enum (a
/// string literal) followed by the payload fields. Internal tagging flattens a struct
/// payload into those fields while adjacent tagging nests it under the `content` key, but
/// flattening the remaining fields is the correct wire shape for both: the field already
/// carries the content key.
fn tagged_variant<'a>(
    variant: &'a Variant,
    types: &Types,
    format: Option<&dyn Format>,
    path: &str,
) -> Result<Option<TaggedVariant<'a>>, Error> {
    match &variant.fields {
        Fields::Named(fields) => {
            let Some((tag_key, wire)) = fields.fields.iter().find_map(|(name, field)| {
                literal_tag(field).map(|wire| (name.clone(), Cow::Borrowed(wire)))
            }) else {
                return Ok(None);
            };
            Ok(Some(TaggedVariant {
                tag_key: tag_key.clone(),
                wire,
                payload: named_fields(
                    fields
                        .fields
                        .iter()
                        .filter(|(name, _)| name != &tag_key)
                        .cloned(),
                ),
            }))
        }
        // An internally tagged newtype variant is lowered to the tag struct intersected
        // with the payload. Hoisting the payload's fields reproduces Serde's flattening.
        Fields::Unnamed(fields) => {
            let [field] = fields.fields.as_slice() else {
                return Ok(None);
            };
            let Some(DataType::Intersection(parts)) = &field.ty else {
                return Ok(None);
            };
            let mut tag_key = None;
            let mut wire = None;
            let mut payload = Vec::new();
            for part in parts {
                let Some(fields) = resolved_named_fields(types, format, part, path, 0)? else {
                    return Ok(None);
                };
                for (name, field) in fields {
                    if let Some(value) = literal_tag(&field) {
                        if tag_key.is_some() {
                            return Ok(None);
                        }
                        wire = Some(Cow::Owned(value.to_owned()));
                        tag_key = Some(name);
                    } else {
                        payload.push((name, field));
                    }
                }
            }
            let (Some(tag_key), Some(wire)) = (tag_key, wire) else {
                return Ok(None);
            };
            Ok(Some(TaggedVariant {
                tag_key,
                wire,
                payload: named_fields(payload),
            }))
        }
        Fields::Unit => Ok(None),
    }
}

/// A field whose type is a single-variant unit enum carries the Serde tag as a literal.
fn literal_tag(field: &Field) -> Option<&str> {
    let DataType::Enum(enm) = field.ty.as_ref()? else {
        return None;
    };
    if !enm
        .attributes
        .contains_key("specta_serde:enum_repr_rewritten")
    {
        return None;
    }
    let [(name, variant)] = enm.variants.as_slice() else {
        return None;
    };
    is_unit_fields(&variant.fields).then_some(name.as_ref())
}

/// Resolve a datatype that structurally contributes named fields, following inline and
/// named references and substituting generic arguments.
fn resolved_named_fields(
    types: &Types,
    format: Option<&dyn Format>,
    dt: &DataType,
    path: &str,
    depth: usize,
) -> Result<Option<Vec<(Cow<'static, str>, Field)>>, Error> {
    if depth > 16 {
        return Ok(None);
    }
    let dt = map_datatype(format, types, dt)?;
    match &dt {
        DataType::Struct(strct) => match &strct.fields {
            Fields::Named(fields) => Ok(Some(fields.fields.clone())),
            _ => Ok(None),
        },
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => {
                resolved_named_fields(types, format, dt, path, depth + 1)
            }
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference {
                        path: path.to_owned(),
                    })?;
                let Some(original) = &ndt.ty else {
                    return Ok(None);
                };
                let mut ty = map_datatype(format, types, original)?;
                let resolved = resolved_reference_generics(ndt, generics, ndt.name.as_ref())?;
                if !resolved.is_empty() {
                    let substitutions = ndt
                        .generics
                        .iter()
                        .map(|generic| generic.reference())
                        .zip(resolved)
                        .collect::<Vec<_>>();
                    substitute_generics(&mut ty, &substitutions);
                }
                resolved_named_fields(types, format, &ty, path, depth + 1)
            }
            NamedReferenceType::Recursive(_) => Ok(None),
        },
        _ => Ok(None),
    }
}

fn named_fields(fields: impl IntoIterator<Item = (Cow<'static, str>, Field)>) -> Fields {
    let mut builder = Struct::named();
    for (name, field) in fields {
        builder = builder.field(name, field);
    }
    let DataType::Struct(strct) = builder.build() else {
        unreachable!("StructBuilder always creates a struct")
    };
    strct.fields
}

#[allow(clippy::too_many_arguments)]
fn render_enum(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    name: &str,
    generics: &[String],
    generic_scope: &[Generic],
    enm: &Enum,
    path: &str,
) -> Result<(), Error> {
    let variants = enm
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .map(|(name, variant)| (name, variant))
        .collect::<Vec<_>>();
    let shape = enum_shape(format, types, enm, &variants, path)?;
    let mut used_names = std::iter::once(name.to_owned())
        .chain(generics.iter().cloned())
        .map(|name| semantic_identifier(&name).to_owned())
        .collect::<BTreeSet<_>>();
    let mut base_names = BTreeSet::new();
    let variant_names = variants
        .iter()
        .map(|(original, _)| {
            let base = safe_member_name(&convert_variant_name(kotlin.naming, original), "Variant");
            let base_identifier = identifier(&base, &format!("{path}.{original}"))?;
            let base_identifier = semantic_identifier(&base_identifier).to_owned();
            if !base_names.insert(base_identifier) {
                return Err(Error::DuplicateIdentifier {
                    path: path.into(),
                    name: base,
                });
            }
            let mut resolved = base;
            loop {
                let identifier = identifier(&resolved, &format!("{path}.{original}"))?;
                let identifier = semantic_identifier(&identifier).to_owned();
                if !used_names.contains(&identifier)
                    && !ROOT_NAMESPACES.contains(&resolved.as_str())
                {
                    used_names.insert(identifier);
                    break;
                }
                resolved.push_str("Variant");
            }
            Ok(resolved)
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Generated helpers and serializers share the file with every other declaration, so
    // avoid names that would shadow one of them.
    used_names.extend(
        types
            .iter_sorted()
            .filter(|ndt| ndt.ty.is_some())
            .map(|ndt| named_type_identifier(kotlin, ndt, &ndt.name))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|name| semantic_identifier(name).to_owned()),
    );
    let kotlinx = kotlin.serialization == Serialization::Kotlinx;

    if kotlinx {
        match &shape.repr {
            EnumRepr::Untagged => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx cannot tell untagged enum variants apart by shape",
                });
            }
            EnumRepr::Custom => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "this Serde enum representation has no generated Kotlinx serializer",
                });
            }
            EnumRepr::External | EnumRepr::Tagged { .. } => {}
        }
        // A custom serializer would have to be parameterised by every instantiation's
        // serializers, which standard Kotlinx annotations cannot express.
        if !generics.is_empty() {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "generic enums need a per-instantiation Kotlinx serializer",
            });
        }
    }

    let unit_only = generics.is_empty()
        && shape
            .variants
            .iter()
            .all(|variant| is_unit_fields(&variant.fields));

    // A plain externally tagged all-unit enum encodes exactly like a Kotlinx string enum.
    // Internal and adjacent tagging still wrap the tag in an object, so those need the
    // serializer even though every variant carries no payload.
    let string_enum = unit_only && (!kotlinx || matches!(shape.repr, EnumRepr::External));
    if string_enum && !(kotlinx && kotlin.unknown_variants) {
        return render_enum_class(out, kotlin, &shape, &variants, &variant_names, name, path);
    }

    let serializer_name = if kotlinx {
        Some(unique_name(
            &mut used_names,
            &format!("{}Serializer", semantic_identifier(name)),
            path,
        )?)
    } else {
        None
    };
    let unknown_name = if kotlinx && kotlin.unknown_variants {
        Some(unique_name(&mut used_names, "Unknown", path)?)
    } else {
        None
    };

    if let Some(serializer_name) = &serializer_name {
        annotation_raw(
            out,
            "",
            &format!("@kotlinx.serialization.Serializable(with = {serializer_name}::class)"),
        );
    }
    out.push_str("public sealed interface ");
    out.push_str(name);
    out.push_str(&generic_declaration(generics));
    out.push_str(" {\n");
    let mut helpers = Vec::new();
    for ((shape_variant, converted), (_, variant)) in
        shape.variants.iter().zip(&variant_names).zip(&variants)
    {
        render_variant(
            out,
            kotlin,
            format,
            types,
            name,
            generics,
            generic_scope,
            shape_variant,
            variant,
            converted,
            &shape.repr,
            path,
            &mut helpers,
            &mut used_names,
        )?;
        out.push('\n');
    }
    if let Some(unknown_name) = &unknown_name {
        render_unknown_variant(out, kotlin, name, unknown_name);
        out.push('\n');
    }
    out.push('}');
    for helper in helpers {
        out.push_str("\n\n");
        out.push_str(&helper);
    }
    if let Some(serializer_name) = &serializer_name {
        out.push_str("\n\n");
        return render_serializer(
            out,
            kotlin,
            name,
            serializer_name,
            unknown_name.as_deref(),
            &shape.repr,
            &shape.variants,
            &variant_names,
            path,
        );
    }
    Ok(())
}

fn render_enum_class(
    out: &mut String,
    kotlin: &Kotlin,
    shape: &EnumShape<'_>,
    variants: &[(&Cow<'_, str>, &Variant)],
    variant_names: &[String],
    name: &str,
    path: &str,
) -> Result<(), Error> {
    let kotlinx = kotlin.serialization == Serialization::Kotlinx;
    // Plain declarations have no serialization framework to read `@SerialName`, so record
    // the wire spelling on the enum itself.
    let record_wire = kotlin.serialization == Serialization::None
        && !matches!(shape.repr, EnumRepr::Untagged)
        && shape
            .variants
            .iter()
            .zip(variant_names)
            .any(|(variant, converted)| semantic_identifier(converted) != variant.wire);
    annotation(out, "", kotlin, "Serializable", None);
    out.push_str("public enum class ");
    out.push_str(name);
    if record_wire {
        out.push_str("(public val wireValue: kotlin.String)");
    }
    out.push_str(" {\n");
    for (index, (variant, converted)) in variants.iter().zip(variant_names).enumerate() {
        let shape_variant = &shape.variants[index];
        let indent = kotlin.indentation(1);
        render_kdoc(out, &indent, &variant.1.docs);
        render_deprecated(out, &indent, variant.1.deprecated.as_ref());
        let converted_identifier =
            identifier(converted, &format!("{path}.{}", shape_variant.name))?;
        if kotlinx && semantic_identifier(&converted_identifier) != shape_variant.wire {
            annotation(
                out,
                &indent,
                kotlin,
                "SerialName",
                Some(&shape_variant.wire),
            );
        }
        out.push_str(&indent);
        out.push_str(&converted_identifier);
        if record_wire {
            out.push_str("(\"");
            out.push_str(&escape_string(&shape_variant.wire));
            out.push_str("\")");
        }
        if index + 1 != variants.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push('}');
    Ok(())
}

fn render_unknown_variant(out: &mut String, kotlin: &Kotlin, parent: &str, name: &str) {
    let indent = kotlin.indentation(1);
    annotation(out, &indent, kotlin, "Serializable", None);
    out.push_str(&indent);
    out.push_str("public data class ");
    out.push_str(name);
    out.push_str("(\n");
    out.push_str(&kotlin.indentation(2));
    out.push_str("public val tag: kotlin.String,\n");
    out.push_str(&kotlin.indentation(2));
    out.push_str("public val content: kotlinx.serialization.json.JsonElement? = null,\n");
    out.push_str(&indent);
    out.push_str(") : ");
    out.push_str(parent);
}

#[allow(clippy::too_many_arguments)]
fn render_variant(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    parent: &str,
    generics: &[String],
    generic_scope: &[Generic],
    shape: &VariantShape<'_>,
    variant: &Variant,
    converted: &str,
    repr: &EnumRepr<'_>,
    path: &str,
    helpers: &mut Vec<String>,
    used_names: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let indent = kotlin.indentation(1);
    render_kdoc(out, &indent, &variant.docs);
    render_deprecated(out, &indent, variant.deprecated.as_ref());
    annotation(out, &indent, kotlin, "Serializable", None);
    let serial_name = match repr {
        EnumRepr::Tagged { .. } => shape.wire.as_ref(),
        _ => shape.name,
    };
    if converted != serial_name {
        annotation(out, &indent, kotlin, "SerialName", Some(serial_name));
    }
    let variant_name = identifier(converted, &format!("{path}.{}", shape.name))?;
    out.push_str(&indent);
    let parent_type = format!("{parent}{}", generic_usage(generics));

    match &shape.fields {
        Fields::Unit => {
            if generics.is_empty() {
                out.push_str("public data object ");
                out.push_str(&variant_name);
            } else {
                out.push_str("public class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str(" : ");
                out.push_str(&parent_type);
                return Ok(());
            }
            out.push_str(" : ");
            out.push_str(&parent_type);
        }
        Fields::Unnamed(fields) => {
            let fields = fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.is_empty() {
                if generics.is_empty() {
                    out.push_str("public data object ");
                } else {
                    out.push_str("public class ");
                }
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str(" : ");
                out.push_str(&parent_type);
            } else {
                out.push_str("public data class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                let mut overrides = Vec::with_capacity(fields.len());
                for (index, field) in &fields {
                    let property = if fields.len() == 1 {
                        "value".to_owned()
                    } else {
                        format!("field{index}")
                    };
                    let hint = if fields.len() == 1 {
                        format!(
                            "{}{}Data",
                            semantic_identifier(parent),
                            pascal_case(shape.name)
                        )
                    } else {
                        format!(
                            "{}{}{}Data",
                            semantic_identifier(parent),
                            pascal_case(shape.name),
                            pascal_case(&property)
                        )
                    };
                    overrides.push(hoist_inline_struct(
                        helpers,
                        kotlin,
                        format,
                        types,
                        generics,
                        generic_scope,
                        &hint,
                        field,
                        path,
                        used_names,
                    )?);
                }
                for (position, (index, field)) in fields.iter().enumerate() {
                    let property = if fields.len() == 1 {
                        "value".to_owned()
                    } else {
                        format!("field{index}")
                    };
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        &property,
                        field,
                        &format!("{path}.{}.{index}", shape.name),
                        2,
                        overrides[position].as_deref(),
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&indent);
                out.push_str(") : ");
                out.push_str(&parent_type);
            }
        }
        Fields::Named(fields) => {
            ensure_unique(
                &format!("{path}.{}", shape.name),
                fields
                    .fields
                    .iter()
                    .filter(|(_, field)| field.ty.is_some())
                    .map(|(name, _)| {
                        safe_member_name(
                            &convert_name(kotlin.naming, name, NameKind::Property),
                            "field",
                        )
                    }),
            )?;
            let fields = fields
                .fields
                .iter()
                .filter(|(_, field)| field.ty.is_some())
                .collect::<Vec<_>>();
            if fields.is_empty() {
                if generics.is_empty() {
                    out.push_str("public data object ");
                } else {
                    out.push_str("public class ");
                }
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str(" : ");
                out.push_str(&parent_type);
            } else {
                out.push_str("public data class ");
                out.push_str(&variant_name);
                out.push_str(&generic_declaration(generics));
                out.push_str("(\n");
                let mut overrides = Vec::with_capacity(fields.len());
                for (field_name, field) in &fields {
                    let hint = if fields.len() == 1 {
                        format!(
                            "{}{}Data",
                            semantic_identifier(parent),
                            pascal_case(shape.name)
                        )
                    } else {
                        format!(
                            "{}{}{}Data",
                            semantic_identifier(parent),
                            pascal_case(shape.name),
                            pascal_case(field_name)
                        )
                    };
                    overrides.push(hoist_inline_struct(
                        helpers,
                        kotlin,
                        format,
                        types,
                        generics,
                        generic_scope,
                        &hint,
                        field,
                        path,
                        used_names,
                    )?);
                }
                for (position, (field_name, field)) in fields.iter().enumerate() {
                    render_property(
                        out,
                        kotlin,
                        format,
                        types,
                        generic_scope,
                        field_name,
                        field,
                        &format!("{path}.{}.{field_name}", shape.name),
                        2,
                        overrides[position].as_deref(),
                    )?;
                    if position + 1 != fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&indent);
                out.push_str(") : ");
                out.push_str(&parent_type);
            }
        }
    }
    Ok(())
}

/// Hoist an inline anonymous struct into a named data class, since Kotlin has no structural
/// types and `Map<String, Any?>` would lose the payload's types.
#[allow(clippy::too_many_arguments)]
fn hoist_inline_struct(
    helpers: &mut Vec<String>,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    generics: &[String],
    generic_scope: &[Generic],
    hint: &str,
    field: &Field,
    path: &str,
    used_names: &mut BTreeSet<String>,
) -> Result<Option<String>, Error> {
    let Some(ty) = &field.ty else {
        return Ok(None);
    };
    let (strct, nullable) = match ty {
        DataType::Struct(strct) => (strct, false),
        DataType::Nullable(inner) => match inner.as_ref() {
            DataType::Struct(strct) => (strct, true),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let Fields::Named(fields) = &strct.fields else {
        return Ok(None);
    };
    if fields.fields.iter().all(|(_, field)| field.ty.is_none()) {
        return Ok(None);
    }

    let name = unique_name(used_names, hint, path)?;
    let mut helper = String::new();
    render_inline_data_class(
        &mut helper,
        helpers,
        kotlin,
        format,
        types,
        &name,
        generics,
        generic_scope,
        fields,
        path,
        used_names,
    )?;
    helpers.push(helper);
    Ok(Some(if nullable { nullable_type(name) } else { name }))
}

#[allow(clippy::too_many_arguments)]
fn render_inline_data_class(
    out: &mut String,
    helpers: &mut Vec<String>,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    name: &str,
    generics: &[String],
    generic_scope: &[Generic],
    fields: &specta::datatype::NamedFields,
    path: &str,
    used_names: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let fields = fields
        .fields
        .iter()
        .filter(|(_, field)| field.ty.is_some())
        .collect::<Vec<_>>();
    ensure_unique(
        path,
        fields.iter().map(|(name, _)| {
            safe_member_name(
                &convert_name(kotlin.naming, name, NameKind::Property),
                "field",
            )
        }),
    )?;
    annotation(out, "", kotlin, "Serializable", None);
    out.push_str("public data class ");
    out.push_str(name);
    out.push_str(&generic_declaration(generics));
    out.push_str("(\n");
    let mut overrides = Vec::with_capacity(fields.len());
    for (field_name, field) in &fields {
        let hint = format!(
            "{}{}Data",
            semantic_identifier(name),
            pascal_case(field_name)
        );
        overrides.push(hoist_inline_struct(
            helpers,
            kotlin,
            format,
            types,
            generics,
            generic_scope,
            &hint,
            field,
            path,
            used_names,
        )?);
    }
    for (position, (field_name, field)) in fields.iter().enumerate() {
        render_property(
            out,
            kotlin,
            format,
            types,
            generic_scope,
            field_name,
            field,
            &format!("{path}.{field_name}"),
            1,
            overrides[position].as_deref(),
        )?;
        if position + 1 != fields.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(')');
    Ok(())
}

fn payload_shape(fields: &Fields, repr: &EnumRepr<'_>) -> PayloadShape {
    let live = match fields {
        Fields::Unit => 0,
        Fields::Unnamed(fields) => fields
            .fields
            .iter()
            .filter(|field| field.ty.is_some())
            .count(),
        Fields::Named(fields) => fields
            .fields
            .iter()
            .filter(|(_, field)| field.ty.is_some())
            .count(),
    };
    match repr {
        EnumRepr::Tagged { .. } => {
            if live == 0 {
                PayloadShape::Unit
            } else {
                PayloadShape::Struct
            }
        }
        _ => match fields {
            Fields::Unnamed(_) if live > 1 => PayloadShape::Tuple,
            Fields::Unnamed(_) if live == 1 => PayloadShape::Newtype,
            Fields::Unit => PayloadShape::Unit,
            _ => PayloadShape::Struct,
        },
    }
}

/// Generate the custom `KSerializer` that reproduces Serde's enum encoding.
///
/// It works on JSON trees rather than keys so the same routine covers external, internal and
/// adjacent tagging: everything but the tag is encoded by the variant class's own generated
/// serializer, which keeps field renames, defaults and nested custom serializers correct.
#[allow(clippy::too_many_arguments)]
fn render_serializer(
    out: &mut String,
    kotlin: &Kotlin,
    name: &str,
    serializer_name: &str,
    unknown_name: Option<&str>,
    repr: &EnumRepr<'_>,
    variants: &[VariantShape<'_>],
    variant_names: &[String],
    path: &str,
) -> Result<(), Error> {
    let mut shapes = Vec::with_capacity(variants.len());
    for (variant, converted) in variants.iter().zip(variant_names) {
        let payload = payload_shape(&variant.fields, repr);
        if payload == PayloadShape::Tuple {
            return Err(Error::UnsupportedType {
                path: format!("{path}.{}", variant.name),
                reason: "Kotlinx custom enum serializers do not support tuple variants",
            });
        }
        let class = format!(
            "{name}.{}",
            identifier(converted, &format!("{path}.{}", variant.name))?
        );
        shapes.push((variant, class, payload));
    }
    let has_units = shapes
        .iter()
        .any(|(_, _, payload)| *payload == PayloadShape::Unit);
    let needs_format = shapes
        .iter()
        .any(|(_, _, payload)| *payload != PayloadShape::Unit);
    let unknown_class = unknown_name.map(|unknown| format!("{name}.{unknown}"));
    let tagged = match repr {
        EnumRepr::Tagged { tag_key } => Some(escape_string(tag_key)),
        _ => None,
    };

    let mut line = |depth: usize, text: &str| {
        out.push_str(&kotlin.indentation(depth));
        out.push_str(text);
        out.push('\n');
    };

    line(
        0,
        &format!("public object {serializer_name} : kotlinx.serialization.KSerializer<{name}> {{"),
    );
    line(
        1,
        "override val descriptor: kotlinx.serialization.descriptors.SerialDescriptor =",
    );
    line(
        2,
        &format!(
            "kotlinx.serialization.descriptors.buildClassSerialDescriptor(\"{}\")",
            escape_string(name)
        ),
    );
    line(0, "");

    // serialize
    line(1, "override fun serialize(");
    line(2, "encoder: kotlinx.serialization.encoding.Encoder,");
    line(2, &format!("value: {name},"));
    line(1, ") {");
    line(
        2,
        "val json = encoder as? kotlinx.serialization.json.JsonEncoder",
    );
    line(
        3,
        &format!(
            "?: throw kotlinx.serialization.SerializationException(\"{name} can only be serialized as JSON\")"
        ),
    );
    if needs_format {
        line(2, "val format = json.json");
    }
    line(2, "json.encodeJsonElement(");
    line(3, "when (value) {");
    for (variant, class, payload) in &shapes {
        let tag = escape_string(&variant.wire);
        match (tagged.as_deref(), payload) {
            (Some(tag_key), PayloadShape::Unit) => {
                line(
                    4,
                    &format!("is {class} -> kotlinx.serialization.json.buildJsonObject {{"),
                );
                line(
                    5,
                    &format!(
                        "put(\"{tag_key}\", kotlinx.serialization.json.JsonPrimitive(\"{tag}\"))"
                    ),
                );
                line(4, "}");
            }
            (Some(tag_key), _) => {
                line(
                    4,
                    &format!("is {class} -> kotlinx.serialization.json.buildJsonObject {{"),
                );
                line(
                    5,
                    &format!(
                        "put(\"{tag_key}\", kotlinx.serialization.json.JsonPrimitive(\"{tag}\"))"
                    ),
                );
                line(
                    5,
                    &format!(
                        "val payload = format.encodeToJsonElement({class}.serializer(), value) as kotlinx.serialization.json.JsonObject"
                    ),
                );
                line(5, "payload.forEach { (key, element) -> put(key, element) }");
                line(4, "}");
            }
            (None, PayloadShape::Unit) => {
                line(
                    4,
                    &format!("is {class} -> kotlinx.serialization.json.JsonPrimitive(\"{tag}\")"),
                );
            }
            (None, PayloadShape::Newtype) => {
                line(
                    4,
                    &format!("is {class} -> kotlinx.serialization.json.buildJsonObject {{"),
                );
                line(
                    5,
                    &format!(
                        "put(\"{tag}\", (format.encodeToJsonElement({class}.serializer(), value) as kotlinx.serialization.json.JsonObject)[\"value\"] ?: kotlinx.serialization.json.JsonNull)"
                    ),
                );
                line(4, "}");
            }
            (None, _) => {
                line(
                    4,
                    &format!("is {class} -> kotlinx.serialization.json.buildJsonObject {{"),
                );
                line(
                    5,
                    &format!(
                        "put(\"{tag}\", format.encodeToJsonElement({class}.serializer(), value))"
                    ),
                );
                line(4, "}");
            }
        }
    }
    if let Some(unknown_class) = &unknown_class {
        match &tagged {
            Some(tag_key) => {
                line(
                    4,
                    &format!("is {unknown_class} -> kotlinx.serialization.json.buildJsonObject {{"),
                );
                line(
                    5,
                    &format!(
                        "put(\"{tag_key}\", kotlinx.serialization.json.JsonPrimitive(value.tag))"
                    ),
                );
                line(
                    5,
                    "(value.content as? kotlinx.serialization.json.JsonObject)?.forEach { (key, element) -> put(key, element) }",
                );
                line(4, "}");
            }
            None => {
                line(
                    4,
                    &format!("is {unknown_class} -> if (value.content != null) {{"),
                );
                line(
                    5,
                    "kotlinx.serialization.json.buildJsonObject { put(value.tag, value.content) }",
                );
                line(4, "} else {");
                line(5, "kotlinx.serialization.json.JsonPrimitive(value.tag)");
                line(4, "}");
            }
        }
    }
    if shapes.is_empty() {
        line(
            4,
            &format!(
                "else -> throw kotlinx.serialization.SerializationException(\"Unsupported {name} variant\")"
            ),
        );
    }
    line(3, "}");
    line(2, ")");
    line(1, "}");
    line(0, "");

    // deserialize
    line(
        1,
        &format!(
            "override fun deserialize(decoder: kotlinx.serialization.encoding.Decoder): {name} {{"
        ),
    );
    line(
        2,
        "val json = decoder as? kotlinx.serialization.json.JsonDecoder",
    );
    line(
        3,
        &format!(
            "?: throw kotlinx.serialization.SerializationException(\"{name} can only be deserialized from JSON\")"
        ),
    );
    if needs_format {
        line(2, "val format = json.json");
    }
    if let Some(tag_key) = &tagged {
        line(2, "val element = json.decodeJsonElement()");
        line(
            2,
            "if (element !is kotlinx.serialization.json.JsonObject) {",
        );
        line(
            3,
            &format!(
                "throw kotlinx.serialization.SerializationException(\"{name} must be encoded as a JSON object\")"
            ),
        );
        line(2, "}");
        line(
            2,
            &format!(
                "val tag = (element[\"{tag_key}\"] as? kotlinx.serialization.json.JsonPrimitive)?.takeIf {{ it.isString }}?.content"
            ),
        );
        line(
            3,
            &format!(
                "?: throw kotlinx.serialization.SerializationException(\"{name} is missing its string tag field \\\"{tag_key}\\\"\")"
            ),
        );
        line(
            2,
            &format!(
                "val payload = kotlinx.serialization.json.JsonObject(element - \"{tag_key}\")"
            ),
        );
        line(2, "return when (tag) {");
        for (variant, class, payload) in &shapes {
            let wire = escape_string(&variant.wire);
            match payload {
                PayloadShape::Unit => line(3, &format!("\"{wire}\" -> {class}")),
                _ => line(
                    3,
                    &format!(
                        "\"{wire}\" -> format.decodeFromJsonElement({class}.serializer(), payload)"
                    ),
                ),
            }
        }
        line(
            3,
            &match &unknown_class {
                Some(unknown_class) => format!(
                    "else -> {unknown_class}(tag, if (payload.isEmpty()) null else payload)"
                ),
                None => format!(
                    "else -> throw kotlinx.serialization.SerializationException(\"Unknown {name} tag: \" + tag)"
                ),
            },
        );
        line(2, "}");
    } else {
        line(2, "val element = json.decodeJsonElement()");
        if has_units {
            line(
                2,
                "val primitive = element as? kotlinx.serialization.json.JsonPrimitive",
            );
            line(2, "if (primitive != null && primitive.isString) {");
            line(3, "return when (primitive.content) {");
            for (variant, class, payload) in &shapes {
                if *payload == PayloadShape::Unit {
                    line(
                        4,
                        &format!("\"{}\" -> {class}", escape_string(&variant.wire)),
                    );
                }
            }
            line(
                4,
                &match &unknown_class {
                    Some(unknown_class) => {
                        format!("else -> {unknown_class}(primitive.content, null)")
                    }
                    None => format!(
                        "else -> throw kotlinx.serialization.SerializationException(\"Unknown {name} tag: \" + primitive.content)"
                    ),
                },
            );
            line(3, "}");
            line(2, "}");
        }
        line(
            2,
            "if (element !is kotlinx.serialization.json.JsonObject) {",
        );
        line(
            3,
            &format!(
                "throw kotlinx.serialization.SerializationException(\"{name} must be encoded as a JSON object or string\")"
            ),
        );
        line(2, "}");
        line(2, "val entry = element.entries.singleOrNull()");
        line(
            3,
            &format!(
                "?: throw kotlinx.serialization.SerializationException(\"{name} must be encoded as an object with exactly one key\")"
            ),
        );
        let unknown = match &unknown_class {
            Some(unknown_class) => format!("{unknown_class}(entry.key, entry.value)"),
            None => format!(
                "throw kotlinx.serialization.SerializationException(\"Unknown {name} tag: \" + entry.key)"
            ),
        };
        if shapes
            .iter()
            .all(|(_, _, payload)| *payload == PayloadShape::Unit)
        {
            line(2, &format!("return {unknown}"));
        } else {
            line(2, "return when (entry.key) {");
            for (variant, class, payload) in &shapes {
                let wire = escape_string(&variant.wire);
                match payload {
                    PayloadShape::Unit => {}
                    PayloadShape::Newtype => line(
                        3,
                        &format!(
                            "\"{wire}\" -> format.decodeFromJsonElement({class}.serializer(), kotlinx.serialization.json.JsonObject(mapOf(\"value\" to entry.value)))"
                        ),
                    ),
                    _ => line(
                        3,
                        &format!(
                            "\"{wire}\" -> format.decodeFromJsonElement({class}.serializer(), entry.value)"
                        ),
                    ),
                }
            }
            line(3, &format!("else -> {unknown}"));
            line(2, "}");
        }
    }
    line(1, "}");
    out.push('}');
    Ok(())
}

fn unique_name(used: &mut BTreeSet<String>, hint: &str, path: &str) -> Result<String, Error> {
    let mut candidate = hint.to_owned();
    let mut index = 1;
    loop {
        let name = identifier(&candidate, path)?;
        if used.insert(semantic_identifier(&name).to_owned()) {
            return Ok(name);
        }
        candidate = format!("{hint}{index}");
        index += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_property(
    out: &mut String,
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    generic_scope: &[Generic],
    original_name: &str,
    field: &Field,
    path: &str,
    depth: usize,
    override_ty: Option<&str>,
) -> Result<(), Error> {
    let indent = kotlin.indentation(depth);
    let rendered = field_datatype(
        kotlin,
        format,
        types,
        generic_scope,
        field,
        path,
        false,
        override_ty,
    )?;
    let nullable = field
        .ty
        .as_ref()
        .map(|ty| datatype_nullability(format, types, ty, path, &mut Vec::new()))
        .transpose()?
        .is_some_and(|nullability| nullability == Nullability::Nullable);
    render_kdoc(out, &indent, &field.docs);
    render_deprecated(out, &indent, field.deprecated.as_ref());
    let converted = safe_member_name(
        &convert_name(kotlin.naming, original_name, NameKind::Property),
        "field",
    );
    if converted != original_name {
        annotation(out, &indent, kotlin, "SerialName", Some(original_name));
    }
    if kotlin.serialization == Serialization::Kotlinx
        && nullable
        && !field
            .attributes
            .contains_key("specta_serde:serialize_omission")
    {
        // `Field::optional` also covers fields Serde only accepts as missing (`#[serde(default)]`
        // or a nullable adjacent payload). Serde still writes those as `null`, so only a
        // `skip_serializing_if` omission may fall back to the Kotlin default.
        annotation(out, &indent, kotlin, "EncodeDefault", None);
    }
    out.push_str(&indent);
    out.push_str("public ");
    out.push_str(property_keyword(kotlin));
    out.push(' ');
    out.push_str(&identifier(&converted, path)?);
    out.push_str(": ");
    out.push_str(&rendered);
    if field.optional || (kotlin.serialization == Serialization::Kotlinx && nullable) {
        out.push_str(" = null");
    }
    Ok(())
}

fn field_datatype(
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    generic_scope: &[Generic],
    field: &Field,
    path: &str,
    allow_generic: bool,
    override_ty: Option<&str>,
) -> Result<String, Error> {
    let ty = field.ty.as_ref().ok_or_else(|| Error::UnsupportedType {
        path: path.to_owned(),
        reason: "skipped field cannot be rendered",
    })?;
    let mut rendered = match override_ty {
        Some(rendered) => rendered.to_owned(),
        None => datatype(kotlin, format, types, ty, generic_scope, path)?,
    };
    let nullability = datatype_nullability(format, types, ty, path, &mut Vec::new())?;
    if kotlin.serialization == Serialization::Kotlinx
        && nullability == Nullability::Generic
        && !allow_generic
    {
        return Err(Error::UnsupportedType {
            path: path.into(),
            reason: "Kotlinx missing-field behavior cannot be inferred for an unconstrained generic",
        });
    }
    let nullable = nullability == Nullability::Nullable;
    if kotlin.serialization == Serialization::Kotlinx && field.optional && !nullable {
        return Err(Error::UnsupportedType {
            path: path.into(),
            reason: "optional non-null fields require a wire-specific Kotlin default",
        });
    }
    if field.optional && !nullable {
        rendered = nullable_type(rendered);
    }
    Ok(rendered)
}

fn nullable_type(rendered: String) -> String {
    if rendered.ends_with('?') {
        rendered
    } else {
        format!("{rendered}?")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Nullability {
    Nullable,
    NonNullable,
    Generic,
}

fn datatype_nullability(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    path: &str,
    visited: &mut Vec<*const NamedDataType>,
) -> Result<Nullability, Error> {
    let mapped = map_datatype(format, types, dt)?;
    datatype_nullability_mapped(format, types, &mapped, path, visited)
}

fn datatype_nullability_mapped(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    path: &str,
    visited: &mut Vec<*const NamedDataType>,
) -> Result<Nullability, Error> {
    let DataType::Reference(Reference::Named(reference)) = dt else {
        return Ok(match dt {
            DataType::Nullable(_) => Nullability::Nullable,
            DataType::Generic(_) => Nullability::Generic,
            _ => Nullability::NonNullable,
        });
    };

    match &reference.inner {
        NamedReferenceType::Inline { dt, .. } => {
            return datatype_nullability(format, types, dt, path, visited);
        }
        NamedReferenceType::Recursive(_) => return Ok(Nullability::NonNullable),
        NamedReferenceType::Reference { .. } => {}
    }

    let ndt = types
        .get(reference)
        .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
    let pointer = ndt as *const NamedDataType;
    if visited.contains(&pointer) {
        return Ok(Nullability::NonNullable);
    }
    let Some(original) = &ndt.ty else {
        return Ok(Nullability::NonNullable);
    };
    let mut aliased = map_datatype(format, types, original)?;
    if matches!(
        aliased,
        DataType::Struct(_) | DataType::Enum(_) | DataType::Generic(_)
    ) {
        return Ok(Nullability::NonNullable);
    }

    if let NamedReferenceType::Reference { generics, .. } = &reference.inner {
        let resolved = resolved_reference_generics(ndt, generics, path)?;
        let substitutions = ndt
            .generics
            .iter()
            .map(|definition| definition.reference())
            .zip(resolved)
            .collect::<Vec<_>>();
        substitute_generics(&mut aliased, &substitutions);
    }

    visited.push(pointer);
    let nullable = datatype_nullability_mapped(format, types, &aliased, path, visited);
    visited.pop();
    nullable
}

fn datatype(
    kotlin: &Kotlin,
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
    generic_scope: &[Generic],
    path: &str,
) -> Result<String, Error> {
    let dt = map_datatype(format, types, dt)?;
    Ok(match &dt {
        DataType::Primitive(Primitive::i128 | Primitive::u128 | Primitive::f128)
            if kotlin.serialization == Serialization::Kotlinx =>
        {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "wide number has no built-in Kotlinx serializer",
            });
        }
        DataType::Primitive(Primitive::char) if kotlin.serialization == Serialization::Kotlinx => {
            "kotlin.String".to_owned()
        }
        DataType::Primitive(primitive) => primitive_name(primitive).to_owned(),
        DataType::List(list) => format!(
            "kotlin.collections.List<{}>",
            datatype(kotlin, format, types, &list.ty, generic_scope, path)?
        ),
        DataType::Map(map) => format!(
            "kotlin.collections.Map<{}, {}>",
            datatype(kotlin, format, types, map.key_ty(), generic_scope, path)?,
            datatype(kotlin, format, types, map.value_ty(), generic_scope, path)?
        ),
        DataType::Nullable(inner) => {
            let inner = datatype(kotlin, format, types, inner, generic_scope, path)?;
            nullable_type(inner)
        }
        DataType::Tuple(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "Kotlin Pair/Triple serialization does not preserve tuple array encoding",
            });
        }
        DataType::Tuple(tuple) => match tuple.elements.as_slice() {
            [] => "kotlin.Unit".to_owned(),
            [single] => datatype(kotlin, format, types, single, generic_scope, path)?,
            [first, second] => format!(
                "kotlin.Pair<{}, {}>",
                datatype(kotlin, format, types, first, generic_scope, path)?,
                datatype(kotlin, format, types, second, generic_scope, path)?
            ),
            [first, second, third] => format!(
                "kotlin.Triple<{}, {}, {}>",
                datatype(kotlin, format, types, first, generic_scope, path)?,
                datatype(kotlin, format, types, second, generic_scope, path)?,
                datatype(kotlin, format, types, third, generic_scope, path)?
            ),
            _ => "kotlin.collections.List<kotlin.Any?>".to_owned(),
        },
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = types
                    .get(reference)
                    .ok_or_else(|| Error::DanglingReference { path: path.into() })?;
                if ndt.ty.is_none() {
                    return Err(Error::UnsupportedType {
                        path: path.into(),
                        reason: "referenced named type does not have an exportable definition",
                    });
                }
                let name = named_type_identifier(kotlin, ndt, path)?;
                let generics = resolved_reference_generics(ndt, generics, path)?
                    .iter()
                    .map(|generic| datatype(kotlin, format, types, generic, generic_scope, path))
                    .collect::<Result<Vec<_>, _>>()?;
                format!("{name}{}", generic_usage(&generics))
            }
            NamedReferenceType::Inline { dt, .. } => {
                datatype(kotlin, format, types, dt, generic_scope, path)?
            }
            NamedReferenceType::Recursive(_) => {
                return Err(Error::RecursiveInlineType { path: path.into() });
            }
        },
        DataType::Reference(Reference::Opaque(reference)) => match kotlin.unknown_types {
            UnknownType::Any if kotlin.serialization == Serialization::None => {
                "kotlin.Any?".to_owned()
            }
            UnknownType::Any | UnknownType::Error => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: reference.type_name(),
                });
            }
        },
        DataType::Generic(generic) => generic_scope
            .iter()
            .find(|candidate| *candidate == generic)
            .map(|generic| generic_identifier(generic.name(), path))
            .transpose()?
            .ok_or_else(|| Error::UnsupportedType {
                path: path.into(),
                reason: "generic reference is not declared by the containing type",
            })?,
        // Kotlin has no structural type expressions. Preserve the useful outer shape for
        // inline containers while falling back to `Any?` for their heterogeneous values.
        DataType::Struct(strct) => match &strct.fields {
            Fields::Unit if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx object serialization does not preserve unit-struct null encoding",
                });
            }
            Fields::Unit => "kotlin.Unit".to_owned(),
            Fields::Unnamed(fields)
                if kotlin.serialization == Serialization::Kotlinx
                    && (fields.fields.len() != 1 || fields.fields[0].ty.is_none()) =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "Kotlinx data classes do not preserve tuple-struct array encoding",
                });
            }
            Fields::Unnamed(_)
                if kotlin.serialization == Serialization::Kotlinx && kotlin.mutable_properties =>
            {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "mutable Kotlinx newtypes cannot preserve scalar encoding",
                });
            }
            Fields::Unnamed(fields) if fields.fields.len() == 1 => fields.fields[0]
                .ty
                .as_ref()
                .map(|ty| datatype(kotlin, format, types, ty, generic_scope, path))
                .transpose()?
                .unwrap_or_else(|| "kotlin.Unit".to_owned()),
            Fields::Unnamed(_) if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "heterogeneous inline tuple has no inferred Kotlinx serializer",
                });
            }
            Fields::Unnamed(_) => "kotlin.collections.List<kotlin.Any?>".to_owned(),
            Fields::Named(_) if kotlin.serialization == Serialization::Kotlinx => {
                return Err(Error::UnsupportedType {
                    path: path.into(),
                    reason: "anonymous structural type has no inferred Kotlinx serializer",
                });
            }
            Fields::Named(_) => "kotlin.collections.Map<kotlin.String, kotlin.Any?>".to_owned(),
        },
        DataType::Enum(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "structural union has no inferred Kotlinx serializer",
            });
        }
        // An enum with no variants is uninhabited, which `kotlin.Never` describes. This
        // has to come before the unit-only arm below, whose `all(..)` is vacuously true
        // for an empty variant list and would otherwise report it as a string enum.
        DataType::Enum(enm) if enm.variants.is_empty() => "kotlin.Never".to_owned(),
        DataType::Enum(enm)
            if enm
                .variants
                .iter()
                .all(|(_, variant)| is_unit_fields(&variant.fields)) =>
        {
            "kotlin.String".to_owned()
        }
        DataType::Intersection(_) if kotlin.serialization == Serialization::Kotlinx => {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "structural union has no inferred Kotlinx serializer",
            });
        }
        DataType::Enum(_) | DataType::Intersection(_) => "kotlin.Any?".to_owned(),
    })
}

fn resolved_reference_generics(
    ndt: &NamedDataType,
    explicit: &[(Generic, DataType)],
    path: &str,
) -> Result<Vec<DataType>, Error> {
    for (index, (generic, _)) in explicit.iter().enumerate() {
        if !ndt
            .generics
            .iter()
            .any(|definition| definition.reference() == *generic)
        {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "named reference supplies an unknown generic argument",
            });
        }
        if explicit[..index]
            .iter()
            .any(|(candidate, _)| candidate == generic)
        {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "named reference supplies a duplicate generic argument",
            });
        }
    }

    let mut scoped = Vec::with_capacity(ndt.generics.len());
    let mut resolved = Vec::with_capacity(ndt.generics.len());

    for definition in ndt.generics.iter() {
        let ty = if let Some((_, ty)) = explicit
            .iter()
            .find(|(generic, _)| generic == &definition.reference())
        {
            ty.clone()
        } else if let Some(default) = &definition.default {
            let mut default = default.clone();
            substitute_generics(&mut default, &scoped);
            default
        } else {
            return Err(Error::UnsupportedType {
                path: path.into(),
                reason: "named reference is missing a required generic argument",
            });
        };

        scoped.push((definition.reference(), ty.clone()));
        resolved.push(ty);
    }

    Ok(resolved)
}

fn substitute_generics(dt: &mut DataType, generics: &[(Generic, DataType)]) {
    match dt {
        DataType::Generic(generic) => {
            if let Some((_, replacement)) =
                generics.iter().find(|(candidate, _)| candidate == generic)
            {
                *dt = replacement.clone();
            }
        }
        DataType::List(list) => substitute_generics(&mut list.ty, generics),
        DataType::Map(map) => {
            substitute_generics(map.key_ty_mut(), generics);
            substitute_generics(map.value_ty_mut(), generics);
        }
        DataType::Nullable(inner) => substitute_generics(inner, generics),
        DataType::Struct(strct) => substitute_field_generics(&mut strct.fields, generics),
        DataType::Enum(enm) => {
            for (_, variant) in &mut enm.variants {
                substitute_field_generics(&mut variant.fields, generics);
            }
        }
        DataType::Tuple(tuple) => {
            for element in &mut tuple.elements {
                substitute_generics(element, generics);
            }
        }
        DataType::Reference(Reference::Named(reference)) => match &mut reference.inner {
            NamedReferenceType::Reference {
                generics: reference_generics,
                ..
            } => {
                for (_, generic) in reference_generics {
                    substitute_generics(generic, generics);
                }
            }
            NamedReferenceType::Inline { dt, .. } => substitute_generics(dt, generics),
            NamedReferenceType::Recursive(_) => {}
        },
        DataType::Intersection(types) => {
            for ty in types {
                substitute_generics(ty, generics);
            }
        }
        DataType::Primitive(_) | DataType::Reference(Reference::Opaque(_)) => {}
    }
}

fn substitute_field_generics(fields: &mut Fields, generics: &[(Generic, DataType)]) {
    match fields {
        Fields::Named(fields) => {
            for (_, field) in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
        Fields::Unnamed(fields) => {
            for field in &mut fields.fields {
                if let Some(ty) = &mut field.ty {
                    substitute_generics(ty, generics);
                }
            }
        }
        Fields::Unit => {}
    }
}

fn primitive_name(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::i8 => "kotlin.Byte",
        Primitive::i16 => "kotlin.Short",
        Primitive::i32 => "kotlin.Int",
        Primitive::i64 | Primitive::isize => "kotlin.Long",
        Primitive::i128 => "java.math.BigInteger",
        Primitive::u8 => "kotlin.UByte",
        Primitive::u16 => "kotlin.UShort",
        Primitive::u32 => "kotlin.UInt",
        Primitive::u64 | Primitive::usize => "kotlin.ULong",
        Primitive::u128 => "java.math.BigInteger",
        Primitive::f16 | Primitive::f32 => "kotlin.Float",
        Primitive::f64 => "kotlin.Double",
        Primitive::f128 => "java.math.BigDecimal",
        Primitive::bool => "kotlin.Boolean",
        Primitive::char => "kotlin.Char",
        Primitive::str => "kotlin.String",
    }
}

fn map_datatype(
    format: Option<&dyn Format>,
    types: &Types,
    dt: &DataType,
) -> Result<DataType, Error> {
    let Some(format) = format else {
        return Ok(dt.clone());
    };
    format
        .map_type(types, dt)
        .map(Cow::into_owned)
        .map_err(|source| Error::format("datatype formatter failed", source))
}

fn is_unit_fields(fields: &Fields) -> bool {
    match fields {
        Fields::Unit => true,
        Fields::Unnamed(fields) => fields.fields.iter().all(|field| field.ty.is_none()),
        Fields::Named(fields) => fields.fields.iter().all(|(_, field)| field.ty.is_none()),
    }
}

fn normalized_variant_fields(
    variant_name: &str,
    fields: &Fields,
    serde_rewritten: bool,
    serde_untagged: bool,
) -> Fields {
    if !serde_rewritten {
        return fields.clone();
    }
    let payload = match fields {
        Fields::Unnamed(fields) if fields.fields.len() == 1 => fields.fields[0]
            .ty
            .as_ref()
            .map(|ty| (None, &fields.fields[0], ty)),
        Fields::Named(fields) if fields.fields.len() == 1 => {
            let (name, field) = &fields.fields[0];
            field.ty.as_ref().map(|ty| (Some(name.as_ref()), field, ty))
        }
        _ => None,
    };

    let Some((field_name, field, payload)) = payload else {
        return fields.clone();
    };
    if field_name.is_none() && !serde_untagged && literal_enum_value(payload) == Some(variant_name)
    {
        return Fields::Unit;
    }
    if field_name.is_some_and(|name| name.eq_ignore_ascii_case(variant_name)) {
        if let DataType::Struct(strct) = payload {
            return strct.fields.clone();
        }
        let mut builder = Struct::unnamed();
        builder.field_mut(field.clone());
        let DataType::Struct(strct) = builder.build() else {
            unreachable!("StructBuilder always creates a struct")
        };
        return strct.fields;
    }
    fields.clone()
}

fn literal_enum_value(dt: &DataType) -> Option<&str> {
    let DataType::Enum(enm) = dt else {
        return None;
    };
    let [(name, variant)] = enm.variants.as_slice() else {
        return None;
    };
    is_unit_fields(&variant.fields).then_some(name)
}

fn property_keyword(kotlin: &Kotlin) -> &'static str {
    if kotlin.mutable_properties {
        "var"
    } else {
        "val"
    }
}

fn generic_declaration(generics: &[String]) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    }
}

fn generic_usage(generics: &[String]) -> String {
    generic_declaration(generics)
}

fn annotation(
    out: &mut String,
    indent: &str,
    kotlin: &Kotlin,
    annotation: &str,
    argument: Option<&str>,
) {
    if kotlin.serialization != Serialization::Kotlinx {
        return;
    }
    out.push_str(indent);
    out.push('@');
    out.push_str("kotlinx.serialization.");
    out.push_str(annotation);
    if let Some(argument) = argument {
        out.push_str("(\"");
        out.push_str(&escape_string(argument));
        out.push_str("\")");
    }
    out.push('\n');
}

fn annotation_raw(out: &mut String, indent: &str, annotation: &str) {
    out.push_str(indent);
    out.push_str(annotation);
    out.push('\n');
}

fn render_kdoc(out: &mut String, indent: &str, docs: &str) {
    if docs.trim().is_empty() {
        return;
    }
    out.push_str(indent);
    out.push_str("/**\n");
    for line in docs.lines() {
        out.push_str(indent);
        out.push_str(" * ");
        out.push_str(&line.trim_start().replace("*/", "* /"));
        out.push('\n');
    }
    out.push_str(indent);
    out.push_str(" */\n");
}

fn render_deprecated(
    out: &mut String,
    indent: &str,
    deprecated: Option<&specta::datatype::Deprecated>,
) {
    let Some(deprecated) = deprecated else {
        return;
    };
    let note = deprecated
        .note
        .as_deref()
        .unwrap_or("This declaration is deprecated");
    out.push_str(indent);
    out.push_str("@kotlin.Deprecated(\"");
    out.push_str(&escape_string(note));
    out.push_str("\")\n");
}

#[derive(Clone, Copy)]
enum NameKind {
    Type,
    Property,
}

fn converted_identifier(
    kotlin: &Kotlin,
    name: &str,
    kind: NameKind,
    path: &str,
) -> Result<String, Error> {
    identifier(&convert_name(kotlin.naming, name, kind), path)
}

fn named_type_identifier(
    kotlin: &Kotlin,
    ndt: &NamedDataType,
    path: &str,
) -> Result<String, Error> {
    let name = if kotlin.layout == Layout::ModulePrefixedName && !ndt.module_path.is_empty() {
        let prefix = ndt
            .module_path
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(pascal_case)
            .collect::<String>();
        format!("{prefix}{}", ndt.name)
    } else {
        ndt.name.to_string()
    };
    let name = converted_identifier(kotlin, &name, NameKind::Type, path)?;
    if ROOT_NAMESPACES.contains(&name.as_str()) {
        return Err(Error::ReservedNamespace {
            path: path.into(),
            name,
        });
    }
    Ok(name)
}

fn convert_name(convention: NamingConvention, name: &str, kind: NameKind) -> String {
    match convention {
        NamingConvention::Preserve => name.to_owned(),
        NamingConvention::PascalCase if matches!(kind, NameKind::Type) => pascal_case(name),
        NamingConvention::PascalCase | NamingConvention::CamelCase => camel_case(name),
        NamingConvention::SnakeCase => snake_case(name),
    }
}

fn convert_variant_name(convention: NamingConvention, name: &str) -> String {
    match convention {
        NamingConvention::Preserve | NamingConvention::PascalCase => pascal_case(name),
        NamingConvention::CamelCase => camel_case(name),
        NamingConvention::SnakeCase => snake_case(name),
    }
}

fn identifier(name: &str, path: &str) -> Result<String, Error> {
    validate_identifier(name, path)?;
    let plain = name
        .chars()
        .enumerate()
        .all(|(index, c)| c == '_' || c.is_alphanumeric() && (index != 0 || !c.is_numeric()));
    if plain && !RESERVED.contains(&name) {
        Ok(name.to_owned())
    } else {
        Ok(format!("`{name}`"))
    }
}

fn generic_identifier(name: &str, path: &str) -> Result<String, Error> {
    if ROOT_NAMESPACES.contains(&name) {
        return Err(Error::ReservedNamespace {
            path: path.into(),
            name: name.into(),
        });
    }
    let identifier = identifier(name, path)?;
    if GENERIC_MODIFIERS.contains(&name) && !identifier.starts_with('`') {
        Ok(format!("`{identifier}`"))
    } else {
        Ok(identifier)
    }
}

fn validate_identifier(name: &str, path: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.chars().all(|c| c == '_')
        || name.chars().any(|c| {
            matches!(
                c,
                '\0' | '`' | '\n' | '\r' | '.' | ';' | '/' | '\\' | '[' | ']' | ':' | '<' | '>'
            )
        })
    {
        return Err(Error::InvalidIdentifier {
            path: path.into(),
            name: name.into(),
        });
    }
    Ok(())
}

fn safe_member_name(name: &str, prefix: &str) -> String {
    if validate_identifier(name, "generated member").is_ok() {
        return name.to_owned();
    }

    // FNV-1a gives invalid wire names stable, collision-resistant source identifiers without
    // leaking filesystem separators or other forbidden characters into generated Kotlin.
    let hash = name
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{prefix}_{hash:x}")
}

fn ensure_unique(
    path: &str,
    names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), Error> {
    let mut seen = std::collections::BTreeSet::new();
    for name in names {
        let name = semantic_identifier(name.as_ref());
        if !seen.insert(name.to_owned()) {
            return Err(Error::DuplicateIdentifier {
                path: path.into(),
                name: name.into(),
            });
        }
    }
    Ok(())
}

fn semantic_identifier(name: &str) -> &str {
    name.strip_prefix('`')
        .and_then(|name| name.strip_suffix('`'))
        .unwrap_or(name)
}

fn package_name(package: &str) -> Result<String, Error> {
    if package.split('.').next() == Some("kotlin") {
        return Err(Error::ReservedNamespace {
            path: "package".into(),
            name: "kotlin".into(),
        });
    }
    package
        .split('.')
        .map(|part| identifier(part, "package"))
        .collect::<Result<Vec<_>, _>>()
        .map(|segments| segments.join("."))
}

fn pascal_case(name: &str) -> String {
    let camel = camel_case(name);
    let mut chars = camel.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase = false;
    for (index, c) in name.chars().enumerate() {
        if matches!(c, '_' | '-' | ' ') {
            uppercase = true;
        } else if uppercase {
            out.extend(c.to_uppercase());
            uppercase = false;
        } else if index == 0 {
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (index, c) in name.chars().enumerate() {
        if matches!(c, '-' | ' ') {
            if !out.ends_with('_') {
                out.push('_');
            }
        } else if c.is_uppercase() {
            if index != 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn escape_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
