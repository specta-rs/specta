use specta::{
    TypeCollection,
    datatype::{
        DataType, DeprecatedType, Fields, NamedDataType, Primitive, Reference, skip_fields_named,
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
fn type_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        _ => String::new(),
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

/// Renders the active (non-None) field types of an unnamed fields list.
/// Returns `None` if there are no active fields, `Some(rendered_types)` otherwise.
fn unnamed_field_types(
    types: &TypeCollection,
    uf: &specta::datatype::UnnamedFields,
) -> Result<Option<Vec<String>>> {
    let active: Vec<_> = uf.fields().iter().filter_map(|f| f.ty()).collect();
    if active.is_empty() {
        return Ok(None);
    }
    active
        .iter()
        .map(|ty| datatype_to_rescript(types, ty))
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
    let non_skipped: Vec<_> = e.variants().iter().filter(|(_, v)| !v.skip()).collect();
    let [a, b] = non_skipped.as_slice() else {
        return false;
    };
    a.0 == "Ok"
        && b.0 == "Err"
        && [a, b].iter().all(|v| {
            matches!(v.1.fields(), Fields::Unnamed(uf) if {
                uf.fields().iter().filter(|f| f.ty().is_some()).count() == 1
            })
        })
}

/// Extract the single active field type from a variant assumed to have exactly
/// one unnamed field. Panics if the invariant is violated (only call after
/// `is_result_enum` confirmed the shape).
fn single_unnamed_field_type(variant: &specta::datatype::EnumVariant) -> &DataType {
    match variant.fields() {
        Fields::Unnamed(uf) => uf
            .fields()
            .iter()
            .find_map(|f| f.ty())
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
        Primitive::char | Primitive::String => Ok("string".to_string()),
        Primitive::bool => Ok("bool".to_string()),
        Primitive::i128 | Primitive::u128 => Err(Error::UnsupportedType(
            "ReScript does not support 128-bit integers (i128/u128)".to_string(),
        )),
        Primitive::f16 => Err(Error::UnsupportedType(
            "ReScript does not support f16".to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Core recursive type renderer
// ---------------------------------------------------------------------------

/// Render a `DataType` as an inline ReScript type expression.
/// Used for field types, generic arguments, and type alias bodies.
pub fn datatype_to_rescript(types: &TypeCollection, dt: &DataType) -> Result<String> {
    match dt {
        DataType::Primitive(p) => primitive_to_rescript(p),
        DataType::Nullable(inner) => Ok(format!("option<{}>", datatype_to_rescript(types, inner)?)),
        DataType::List(l) => Ok(format!("array<{}>", datatype_to_rescript(types, l.ty())?)),
        DataType::Struct(_) => Err(Error::UnsupportedType(
            "Inline anonymous structs are not supported; use a named type".to_string(),
        )),
        DataType::Reference(r) => reference_to_rescript(types, r),
        DataType::Generic(g) => Ok(generic_param(&g.to_string())),
        DataType::Map(m) => {
            // ReScript's `dict<v>` only supports string keys.
            match m.key_ty() {
                DataType::Primitive(Primitive::String | Primitive::char) =>
                    Ok(format!("dict<{}>", datatype_to_rescript(types, m.value_ty())?)),
                _ => Err(Error::InvalidType(
                    "ReScript dict only supports string keys; non-string map keys are not supported"
                        .to_string(),
                )),
            }
        }
        DataType::Enum(e) => {
            if is_result_enum(e) {
                let variants: Vec<_> = e.variants().iter().filter(|(_, v)| !v.skip()).collect();
                let ok_ty = single_unnamed_field_type(&variants[0].1);
                let err_ty = single_unnamed_field_type(&variants[1].1);
                let ok_str = datatype_to_rescript(types, ok_ty)?;
                let err_str = datatype_to_rescript(types, err_ty)?;
                return Ok(format!("result<{}, {}>", ok_str, err_str));
            }
            if e.is_string_enum() {
                let variants: Vec<String> = e
                    .variants()
                    .iter()
                    .filter(|(_, v)| !v.skip())
                    .map(|(name, _)| format!("#{}", name))
                    .collect();
                return Ok(format!("[ {} ]", variants.join(" | ")));
            }
            Err(Error::UnsupportedType(
                "Cannot inline a non-trivial enum; use a named type".to_string(),
            ))
        }
        DataType::Tuple(t) => {
            let elems = t.elements();
            match elems.len() {
                0 => Ok("unit".to_string()),
                1 => datatype_to_rescript(types, &elems[0]),
                _ => {
                    let parts = elems
                        .iter()
                        .map(|e| datatype_to_rescript(types, e))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(format!("({})", parts.join(", ")))
                }
            }
        }
    }
}

fn render_unnamed_fields(
    types: &TypeCollection,
    uf: &specta::datatype::UnnamedFields,
) -> Result<String> {
    match unnamed_field_types(types, uf)? {
        Some(parts) if parts.len() == 1 => Ok(parts.into_iter().next().unwrap()),
        Some(parts) => Ok(format!("({})", parts.join(", "))),
        _ => Ok("unit".to_string()),
    }
}

fn reference_to_rescript(types: &TypeCollection, r: &Reference) -> Result<String> {
    match r {
        Reference::Named(n) => {
            let dt = n
                .get(types)
                .ok_or_else(|| Error::InvalidType("Reference to unknown named type".to_string()))?;

            let base_name = type_name(dt.name());

            let generics = n
                .generics()
                .iter()
                .map(|(_, dt)| datatype_to_rescript(types, dt))
                .collect::<Result<Vec<_>>>()?;
            Ok(format!("{}{}", base_name, wrap_generics(generics)))
        }
        Reference::Opaque(o) => Err(Error::UnsupportedType(format!(
            "Opaque reference '{}' is not supported by the ReScript exporter",
            o.type_name()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Struct body renderer (for named top-level types)
// ---------------------------------------------------------------------------

fn render_struct_body(types: &TypeCollection, s: &specta::datatype::Struct) -> Result<String> {
    match s.fields() {
        Fields::Unit => Ok("unit".to_string()),
        Fields::Unnamed(uf) => render_unnamed_fields(types, uf),
        Fields::Named(nf) => {
            let parts = render_named_fields(types, nf)?;
            if parts.is_empty() {
                Ok("unit".to_string())
            } else {
                Ok(format!("{{\n{},\n}}", parts.join(",\n")))
            }
        }
    }
}

fn render_named_fields(
    types: &TypeCollection,
    nf: &specta::datatype::NamedFields,
) -> Result<Vec<String>> {
    skip_fields_named(nf.fields())
        .map(|(name, (field, ty))| {
            let ty_str = datatype_to_rescript(types, ty)?;
            let final_ty = if field.optional() {
                format!("option<{}>", ty_str)
            } else {
                ty_str
            };
            Ok(format!("  {}: {}", name, final_ty))
        })
        .collect()
}

/// Format a `DeprecatedType` into a human-readable note string.
fn deprecated_msg(dep: &DeprecatedType) -> &str {
    match dep {
        DeprecatedType::DeprecatedWithSince { note, .. } if !note.is_empty() => note,
        _ => "deprecated",
    }
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
    types: &TypeCollection,
    e: &specta::datatype::Enum,
    enum_rescript_name: &str,
    generics_decl: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut auxiliary: Vec<String> = Vec::new();
    let mut variant_lines: Vec<String> = Vec::new();

    for (variant_name, variant) in e.variants() {
        if variant.skip() {
            continue;
        }

        match variant.fields() {
            Fields::Unit => {
                variant_lines.push(variant_name.to_string());
            }

            Fields::Unnamed(uf) => {
                let line = match unnamed_field_types(types, uf)? {
                    Some(parts) => format!("{}({})", variant_name, parts.join(", ")),
                    _ => variant_name.to_string(),
                };
                variant_lines.push(line);
            }

            Fields::Named(nf) if nf.fields().is_empty() => {
                variant_lines.push(variant_name.to_string());
            }

            Fields::Named(nf) => {
                // Generate an auxiliary record type: `{enumName}{VariantName}Fields`
                // The enum name is already lowercased; variant name stays PascalCase.
                let aux_name = format!("{}{}Fields", enum_rescript_name, variant_name);

                let field_parts = render_named_fields(types, nf)?;

                let aux_decl = format!(
                    "type {}{} = {{\n{},\n}}",
                    aux_name,
                    generics_decl,
                    field_parts.join(",\n")
                );
                auxiliary.push(aux_decl);

                // Reference the aux type in the variant (with generics if present)
                variant_lines.push(format!("{}({}{})", variant_name, aux_name, generics_decl));
            }
        }
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
pub fn export_type(types: &TypeCollection, dt: &NamedDataType) -> Result<String> {
    let mut out = String::new();

    // Doc comment
    out.extend(
        dt.docs()
            .lines()
            .map(|l| format!("// {}\n", l.trim_start())),
    );

    // Deprecated comment
    if let Some(dep) = dt.deprecated() {
        out.push_str(&format!("// @deprecated {}\n", deprecated_msg(dep)));
    }

    let rescript_name = type_name(dt.name());

    // Generic parameter list: e.g. `<'a, 'b>`
    let generics_decl = wrap_generics(
        dt.generics()
            .iter()
            .map(|g| generic_param(&g.to_string()))
            .collect(),
    );

    match dt.ty() {
        DataType::Struct(s) => out.push_str(&format!(
            "type {}{} = {}\n",
            rescript_name,
            generics_decl,
            render_struct_body(types, s)?
        )),
        DataType::Enum(e) => {
            // Result detection runs first
            if is_result_enum(e) {
                let body = datatype_to_rescript(types, dt.ty())?;
                out.push_str(&format!(
                    "type {}{} = {}\n",
                    rescript_name, generics_decl, body
                ));
                return Ok(out);
            }

            // All unit variants -> polymorphic variants
            let all_unit = e
                .variants()
                .iter()
                .filter(|(_, v)| !v.skip())
                .all(|(_, v)| matches!(v.fields(), Fields::Unit));

            if all_unit {
                let variants: Vec<String> = e
                    .variants()
                    .iter()
                    .filter(|(_, v)| !v.skip())
                    .map(|(name, _)| format!("#{}", name))
                    .collect();
                out.push_str(&format!(
                    "type {}{} = [ {} ]\n",
                    rescript_name,
                    generics_decl,
                    variants.join(" | ")
                ));
            } else {
                // Data variants — may need auxiliary record types
                let (auxiliary, variant_lines) =
                    render_enum_variants(types, e, &rescript_name, &generics_decl)?;

                out.extend(auxiliary.iter().map(|aux| format!("{}\n", aux)));
                out.push_str(&format!("type {}{} =\n", rescript_name, generics_decl));
                out.extend(variant_lines.iter().map(|l| format!("  | {}\n", l)));
            }
        }

        other => {
            let body = datatype_to_rescript(types, other)?;
            out.push_str(&format!(
                "type {}{} = {}\n",
                rescript_name, generics_decl, body
            ));
        }
    }

    Ok(out)
}
