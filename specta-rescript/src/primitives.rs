use std::borrow::Cow;

use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Fields, Generic, NamedDataType, NamedReferenceType, Primitive,
        Reference,
    },
};

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Naming helpers
// ---------------------------------------------------------------------------

/// Convert a Rust type name (PascalCase) to a ReScript type name.
/// ReScript requires type names to start with a lowercase letter.
/// Strategy: lowercase only the first character to preserve the rest.
/// e.g. "MyType" -> "myType", "UUID" -> "uUID"
pub(crate) fn type_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        _ => String::new(),
    }
}

pub(crate) fn is_valid_type_name(name: &str) -> bool {
    is_valid_identifier(name, |ch| matches!(ch, 'a'..='z' | '_'))
}

fn is_valid_variant_constructor(name: &str) -> bool {
    is_valid_identifier(name, |ch| ch.is_ascii_uppercase())
}

fn is_valid_polymorphic_variant(name: &str) -> bool {
    is_valid_identifier(name, |ch| ch.is_ascii_alphabetic())
}

fn is_valid_identifier(name: &str, valid_start: impl FnOnce(char) -> bool) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(valid_start)
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '\''))
}

fn is_string_map_key(dt: &DataType) -> bool {
    match dt {
        DataType::Primitive(Primitive::str) => true,
        DataType::Reference(Reference::Named(reference)) => match &reference.inner {
            NamedReferenceType::Inline { dt, .. } => is_string_map_key(dt),
            _ => false,
        },
        _ => false,
    }
}

/// Convert a Rust generic parameter name to ReScript style.
/// ReScript generic type parameters use an apostrophe prefix.
/// e.g. "T" -> "'t", "A" -> "'a"
fn generic_param(name: &str) -> String {
    format!("'{}", name.to_lowercase())
}

// ---------------------------------------------------------------------------
// Generic / field helpers
// ---------------------------------------------------------------------------

/// Wrap a non-empty list of rendered type params in angle brackets.
/// Returns `""` when `params` is empty, `"<a, b>"` otherwise.
fn wrap_generics(params: Vec<String>) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!("<{}>", params.join(", "))
    }
}

/// The scope maps each generic parameter to its ReScript name.
/// Derived from `NamedDataType::generics`.
type Scope<'a> = &'a [(Generic, Cow<'static, str>)];

fn build_scope(ndt: &NamedDataType) -> Vec<(Generic, Cow<'static, str>)> {
    ndt.generics
        .iter()
        .map(|gd| (gd.reference(), gd.name.clone()))
        .collect()
}

/// Renders the active (non-None) field types of an unnamed fields list.
/// Returns `None` if there are no active fields, `Some(rendered_types)` otherwise.
fn unnamed_field_types(
    types: &Types,
    scope: Scope<'_>,
    uf: &specta::datatype::UnnamedFields,
) -> Result<Option<Vec<String>>> {
    let active: Vec<_> = uf.fields.iter().filter_map(|f| f.ty.as_ref()).collect();
    if active.is_empty() {
        return Ok(None);
    }
    active
        .iter()
        .map(|ty| datatype_to_rescript(types, scope, ty))
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

// ---------------------------------------------------------------------------
// Result detection
// ---------------------------------------------------------------------------

/// Detect whether an Enum represents Rust's `Result<T, E>`.
///
/// Criteria: exactly two non-skipped variants named "Ok" and "Err",
/// each with exactly one non-skipped unnamed field.
///
/// When true, we emit `result<ok_ty, err_ty>` using ReScript's built-in
/// result type. Note that ReScript's result uses `Ok`/`Error` constructors
/// while Rust uses `Ok`/`Err` — the user's serialization layer handles that.
fn is_result_enum(e: &specta::datatype::Enum) -> bool {
    let non_skipped: Vec<_> = e.variants.iter().filter(|(_, v)| !v.skip).collect();
    let [a, b] = non_skipped.as_slice() else {
        return false;
    };
    a.0 == "Ok"
        && b.0 == "Err"
        && [a, b].iter().all(|v| {
            matches!(&v.1.fields, Fields::Unnamed(uf) if {
                uf.fields.iter().filter(|f| f.ty.is_some()).count() == 1
            })
        })
}

/// Extract the single active field type from a variant assumed to have exactly
/// one unnamed field. Panics if the invariant is violated (only call after
/// `is_result_enum` confirmed the shape).
fn single_unnamed_field_type(variant: &specta::datatype::Variant) -> &DataType {
    match &variant.fields {
        Fields::Unnamed(uf) => uf
            .fields
            .iter()
            .find_map(|f| f.ty.as_ref())
            .expect("is_result_enum guarantees one active field"),
        _ => unreachable!("is_result_enum guarantees unnamed fields"),
    }
}

// ---------------------------------------------------------------------------
// Primitive mapping
// ---------------------------------------------------------------------------

fn primitive_to_rescript(p: &Primitive) -> Result<String> {
    match p {
        Primitive::i8
        | Primitive::i16
        | Primitive::i32
        | Primitive::i64
        | Primitive::isize
        | Primitive::u8
        | Primitive::u16
        | Primitive::u32
        | Primitive::u64
        | Primitive::usize => Ok("int".to_string()),
        Primitive::f32 | Primitive::f64 => Ok("float".to_string()),
        Primitive::char | Primitive::str => Ok("string".to_string()),
        Primitive::bool => Ok("bool".to_string()),
        Primitive::i128 | Primitive::u128 => Err(Error::UnsupportedType(
            "ReScript does not support 128-bit integers (i128/u128)".to_string(),
        )),
        Primitive::f16 | Primitive::f128 => Err(Error::UnsupportedType(
            "ReScript does not support f16/f128".to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Core recursive type renderer
// ---------------------------------------------------------------------------

/// Render a `DataType` as an inline ReScript type expression.
/// Used for field types, generic arguments, and type alias bodies.
pub fn datatype_to_rescript(types: &Types, scope: Scope<'_>, dt: &DataType) -> Result<String> {
    match dt {
        DataType::Primitive(p) => primitive_to_rescript(p),
        DataType::Nullable(inner) => Ok(format!(
            "option<{}>",
            datatype_to_rescript(types, scope, inner)?
        )),
        DataType::List(l) => Ok(format!(
            "array<{}>",
            datatype_to_rescript(types, scope, &l.ty)?
        )),
        DataType::Struct(_) => Err(Error::UnsupportedType(
            "Inline anonymous structs are not supported; use a named type".to_string(),
        )),
        DataType::Reference(r) => reference_to_rescript(types, scope, r),
        DataType::Generic(g) => {
            let name = scope
                .iter()
                .find(|(gr, _)| gr == g)
                .map(|(_, name)| name.as_ref())
                .unwrap_or("unknown");
            Ok(generic_param(name))
        }
        DataType::Map(m) => {
            if !is_string_map_key(m.key_ty()) {
                return Err(Error::UnsupportedType(
                    "ReScript dict keys must be strings".to_string(),
                ));
            }
            Ok(format!(
                "dict<{}>",
                datatype_to_rescript(types, scope, m.value_ty())?
            ))
        }
        DataType::Enum(e) => {
            if is_result_enum(e) {
                let variants: Vec<_> = e.variants.iter().filter(|(_, v)| !v.skip).collect();
                let ok_ty = single_unnamed_field_type(&variants[0].1);
                let err_ty = single_unnamed_field_type(&variants[1].1);
                let ok_str = datatype_to_rescript(types, scope, ok_ty)?;
                let err_str = datatype_to_rescript(types, scope, err_ty)?;
                return Ok(format!("result<{}, {}>", ok_str, err_str));
            }
            let all_unit = e
                .variants
                .iter()
                .filter(|(_, v)| !v.skip)
                .all(|(_, v)| matches!(v.fields, Fields::Unit));
            if all_unit {
                let variants: Vec<String> = e
                    .variants
                    .iter()
                    .filter(|(_, v)| !v.skip)
                    .map(|(name, _)| {
                        if !is_valid_polymorphic_variant(name) {
                            return Err(Error::InvalidPolymorphicVariant(name.to_string()));
                        }
                        Ok(format!("#{}", name))
                    })
                    .collect::<Result<_>>()?;
                return Ok(format!("[{}]", variants.join(" | ")));
            }
            Err(Error::UnsupportedType(
                "Cannot inline a non-trivial enum; use a named type".to_string(),
            ))
        }
        DataType::Tuple(t) => {
            let elems = &t.elements;
            match elems.len() {
                0 => Ok("unit".to_string()),
                1 => Err(Error::UnsupportedType(
                    "ReScript does not support one-element tuples".to_string(),
                )),
                _ => {
                    let parts = elems
                        .iter()
                        .map(|e| datatype_to_rescript(types, scope, e))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(format!("({})", parts.join(", ")))
                }
            }
        }
        DataType::Intersection(_) => Err(Error::UnsupportedType(
            "Intersection types are not supported by the ReScript exporter".to_string(),
        )),
    }
}

fn render_unnamed_fields(
    types: &Types,
    scope: Scope<'_>,
    uf: &specta::datatype::UnnamedFields,
) -> Result<String> {
    match unnamed_field_types(types, scope, uf)? {
        Some(parts) if parts.len() == 1 => Ok(parts
            .into_iter()
            .next()
            .expect("one field was checked above")),
        Some(parts) => Ok(format!("({})", parts.join(", "))),
        _ => Ok("unit".to_string()),
    }
}

fn reference_to_rescript(types: &Types, scope: Scope<'_>, r: &Reference) -> Result<String> {
    match r {
        Reference::Named(n) => match &n.inner {
            NamedReferenceType::Inline { dt, .. } => datatype_to_rescript(types, scope, dt),
            NamedReferenceType::Reference { generics, .. } => {
                let ndt = types.get(n).ok_or_else(|| {
                    Error::InvalidType("Reference to unknown named type".to_string())
                })?;
                let base_name = type_name(&ndt.name);
                let rendered_generics = generics
                    .iter()
                    .map(|(_, dt)| datatype_to_rescript(types, scope, dt))
                    .collect::<Result<Vec<_>>>()?;
                Ok(format!("{}{}", base_name, wrap_generics(rendered_generics)))
            }
            NamedReferenceType::Recursive(recursive) => {
                let ndt = types.get(n);
                let name = ndt.map(|n| n.name.as_ref()).unwrap_or("unknown");
                let generics = recursive
                    .generics()
                    .iter()
                    .map(|(_, dt)| datatype_to_rescript(types, scope, dt))
                    .collect::<Result<Vec<_>>>()?;
                Ok(format!("{}{}", type_name(name), wrap_generics(generics)))
            }
        },
        Reference::Opaque(o) => Err(Error::UnsupportedType(format!(
            "Opaque reference '{}' is not supported by the ReScript exporter",
            o.type_name()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Struct body renderer (for named top-level types)
// ---------------------------------------------------------------------------

fn render_struct_body(
    types: &Types,
    scope: Scope<'_>,
    s: &specta::datatype::Struct,
) -> Result<String> {
    match &s.fields {
        Fields::Unit => Ok("unit".to_string()),
        Fields::Unnamed(uf) => render_unnamed_fields(types, scope, uf),
        Fields::Named(nf) => {
            let parts = render_named_fields(types, scope, nf)?;
            if parts.is_empty() {
                Ok("unit".to_string())
            } else {
                Ok(format!("{{\n{},\n}}", parts.join(",\n")))
            }
        }
    }
}

fn render_named_fields(
    types: &Types,
    scope: Scope<'_>,
    nf: &specta::datatype::NamedFields,
) -> Result<Vec<String>> {
    nf.fields
        .iter()
        .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, field, ty)))
        .map(|(name, field, ty)| {
            if !is_valid_record_label(name) {
                return Err(Error::InvalidRecordLabel(name.to_string()));
            }
            let ty_str = datatype_to_rescript(types, scope, ty)?;
            let final_ty = if field.optional {
                format!("option<{}>", ty_str)
            } else {
                ty_str
            };
            let mut out = render_docs(&field.docs, "  ");
            if let Some(dep) = &field.deprecated {
                out.push_str(&render_deprecated(dep, "  "));
            }
            out.push_str(&format!("  {}: {}", name, final_ty));
            Ok(out)
        })
        .collect()
}

fn is_valid_record_label(name: &str) -> bool {
    is_valid_type_name(name)
}

/// Format a `Deprecated` into a human-readable note string.
fn deprecated_msg(dep: &Deprecated) -> &str {
    dep.note.as_deref().unwrap_or("deprecated")
}

fn render_docs(docs: &str, indent: &str) -> String {
    if docs.is_empty() {
        return String::new();
    }

    let mut out = format!("{indent}/**\n");
    for line in docs.lines() {
        let line = line.trim().replace("*/", "* /");
        if line.is_empty() {
            out.push_str(&format!("{indent} *\n"));
        } else {
            out.push_str(&format!("{indent} * {line}\n"));
        }
    }
    out.push_str(&format!("{indent} */\n"));
    out
}

fn render_deprecated(deprecated: &Deprecated, indent: &str) -> String {
    format!(
        "{indent}/** @deprecated {} */\n",
        deprecated_msg(deprecated).replace("*/", "* /")
    )
}

// ---------------------------------------------------------------------------
// Enum variant renderer
// ---------------------------------------------------------------------------

/// Render all variants of a data-carrying enum.
///
/// Returns `(auxiliary_types, variant_lines)` where:
/// - `auxiliary_types`: auxiliary `type` declarations for variants with named fields
/// - `variant_lines`: one string per variant, e.g. `"Circle(float)"` or `"Idle"`
fn render_enum_variants(
    types: &Types,
    scope: Scope<'_>,
    e: &specta::datatype::Enum,
    enum_rescript_name: &str,
    generics_decl: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut auxiliary: Vec<String> = Vec::new();
    let mut variant_lines: Vec<String> = Vec::new();

    for (variant_name, variant) in &e.variants {
        if variant.skip {
            continue;
        }
        if !is_valid_variant_constructor(variant_name) {
            return Err(Error::InvalidVariantConstructor(variant_name.to_string()));
        }

        let mut prefix = render_docs(&variant.docs, "  ");
        if let Some(dep) = &variant.deprecated {
            prefix.push_str(&render_deprecated(dep, "  "));
        }

        let line = match &variant.fields {
            Fields::Unit => variant_name.to_string(),

            Fields::Unnamed(uf) => match unnamed_field_types(types, scope, uf)? {
                Some(parts) => format!("{}({})", variant_name, parts.join(", ")),
                _ => variant_name.to_string(),
            },

            Fields::Named(nf) => {
                let field_parts = render_named_fields(types, scope, nf)?;
                if field_parts.is_empty() {
                    variant_lines.push(format!("{}{}", prefix, variant_name));
                    continue;
                }

                // Generate an auxiliary record type: `{enumName}{VariantName}Fields`
                // The enum name is already lowercased; variant name stays PascalCase.
                let aux_name = format!("{}{}Fields", enum_rescript_name, variant_name);

                let aux_decl = format!(
                    "type {}{} = {{\n{},\n}}",
                    aux_name,
                    generics_decl,
                    field_parts.join(",\n")
                );
                auxiliary.push(aux_decl);

                // Reference the aux type in the variant (with generics if present)
                format!("{}({}{})", variant_name, aux_name, generics_decl)
            }
        };
        variant_lines.push(format!("{}{}", prefix, line));
    }

    Ok((auxiliary, variant_lines))
}

// ---------------------------------------------------------------------------
// Top-level named type exporter
// ---------------------------------------------------------------------------

/// Export a single named type to ReScript.
///
/// May return multiple `type` declarations (separated by newlines) when an
/// enum has variants with named fields that need auxiliary record types.
pub fn export_type(types: &Types, dt: &NamedDataType) -> Result<String> {
    let Some(ty) = &dt.ty else {
        return Ok(String::new());
    };

    let mut out = String::new();

    out.push_str(&render_docs(&dt.docs, ""));

    // Deprecated comment
    if let Some(dep) = &dt.deprecated {
        out.push_str(&render_deprecated(dep, ""));
    }

    let rescript_name = type_name(&dt.name);

    // Derive scope from the generics of this named type.
    let scope = build_scope(dt);

    // Generic parameter list: e.g. `<'a, 'b>`
    let generics_decl = wrap_generics(scope.iter().map(|(_, name)| generic_param(name)).collect());

    match ty {
        DataType::Struct(s) => out.push_str(&format!(
            "type {}{} = {}\n",
            rescript_name,
            generics_decl,
            render_struct_body(types, &scope, s)?
        )),
        DataType::Enum(e) => {
            // Result detection runs first
            if is_result_enum(e) {
                if rescript_name == "result" {
                    return Ok(String::new());
                }
                let body = datatype_to_rescript(types, &scope, ty)?;
                out.push_str(&format!(
                    "type {}{} = {}\n",
                    rescript_name, generics_decl, body
                ));
                return Ok(out);
            }

            // All unit variants -> polymorphic variants
            let all_unit = e
                .variants
                .iter()
                .filter(|(_, v)| !v.skip)
                .all(|(_, v)| matches!(v.fields, Fields::Unit));

            if all_unit {
                for (name, _) in e.variants.iter().filter(|(_, variant)| !variant.skip) {
                    if !is_valid_polymorphic_variant(name) {
                        return Err(Error::InvalidPolymorphicVariant(name.to_string()));
                    }
                }
                let has_metadata = e.variants.iter().any(|(_, variant)| {
                    !variant.skip && (!variant.docs.is_empty() || variant.deprecated.is_some())
                });
                if has_metadata {
                    out.push_str(&format!("type {}{} = [\n", rescript_name, generics_decl));
                    for (name, variant) in e.variants.iter().filter(|(_, variant)| !variant.skip) {
                        out.push_str(&render_docs(&variant.docs, "  "));
                        if let Some(dep) = &variant.deprecated {
                            out.push_str(&render_deprecated(dep, "  "));
                        }
                        out.push_str(&format!("  | #{}\n", name));
                    }
                    out.push_str("]\n");
                } else {
                    let variants = e
                        .variants
                        .iter()
                        .filter(|(_, variant)| !variant.skip)
                        .map(|(name, _)| format!("#{}", name))
                        .collect::<Vec<_>>();
                    out.push_str(&format!(
                        "type {}{} = [{}]\n",
                        rescript_name,
                        generics_decl,
                        variants.join(" | ")
                    ));
                }
            } else {
                // Data variants — may need auxiliary record types
                let (auxiliary, variant_lines) =
                    render_enum_variants(types, &scope, e, &rescript_name, &generics_decl)?;

                out.extend(auxiliary.iter().map(|aux| format!("{}\n", aux)));
                out.push_str(&format!("type {}{} =\n", rescript_name, generics_decl));
                out.extend(variant_lines.iter().map(|l| {
                    if l.starts_with("  /**") {
                        format!(
                            "{}\n  | {}\n",
                            l.rsplit_once('\n').map_or("", |v| v.0),
                            l.rsplit_once('\n').map_or(l.as_str(), |v| v.1)
                        )
                    } else {
                        format!("  | {}\n", l)
                    }
                }));
            }
        }

        other => {
            let body = datatype_to_rescript(types, &scope, other)?;
            out.push_str(&format!(
                "type {}{} = {}\n",
                rescript_name, generics_decl, body
            ));
        }
    }

    Ok(out)
}
