//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are advanced features and should generally be avoided in end-user applications.

use std::{
    borrow::{Borrow, Cow},
    fmt::Write as _,
    iter,
};

use specta::{
    datatype::{
        reference::Reference, DataType, EnumRepr, EnumType, Field, Fields, List, LiteralType, Map,
        NamedDataType, PrimitiveType, StructType, TupleType,
    },
    TypeCollection,
};

use crate::{reserved_names::*, BigIntExportBehavior, CommentFormatterArgs, Error, Typescript};

/// Generate an `export Type = ...` Typescript string for a specific [`DataType`].
///
/// This method leaves the following up to the implementor:
///  - Ensuring all referenced types are exported
///  - Handling multiple type with overlapping names
///  - Transforming the type for your serialization format (Eg. Serde)
///
pub fn export(
    ts: &Typescript,
    types: &TypeCollection,
    dt: &NamedDataType,
) -> Result<String, Error> {
    validate_name(dt.name(), &vec![])?;

    let generics = dt
        .inner
        .generics()
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|g| {
            iter::once("<")
                .chain(intersperse(g.into_iter().map(|g| g.borrow()), ", "))
                .chain(iter::once(">"))
        })
        .into_iter()
        .flatten();

    let s = iter::empty()
        .chain(["export type ", dt.name()])
        .chain(generics)
        .chain([" = "])
        .collect::<String>();

    // TODO: Collecting directly into `result` insetad of allocating `s`?
    let mut result = ts
        .comment_exporter
        .map(|v| {
            v(CommentFormatterArgs {
                docs: dt.docs(),
                deprecated: dt.deprecated(),
            })
        })
        .unwrap_or_default();
    result.push_str(&s);

    datatype(
        &mut result,
        ts,
        types,
        &dt.inner,
        vec![dt.name().clone()],
        false,
    )?;
    result.push_str(";");

    Ok(result)
}

/// Generate an inlined Typescript string for a specific [`DataType`].
///
/// This methods leaves all the same things as the [`export`] method up to the user.
///
pub fn inline(ts: &Typescript, types: &TypeCollection, dt: &DataType) -> Result<String, Error> {
    let mut s = String::new();
    datatype(&mut s, ts, types, dt, vec![], false)?;
    Ok(s)
}

// /// Generate an `export Type = ...` Typescript string for a specific [`DataType`].
// ///
// /// Similar to [`export`] but works on a [`FunctionResultVariant`].
// pub fn export_func(ts: &Typescript, types: &TypeCollection, dt: FunctionResultVariant) -> Result<String, ExportError> {
//     todo!();
// }

fn datatype(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    dt: &DataType,
    mut location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    // TODO: Validating the variant from `dt` can be flattened

    match dt {
        DataType::Any => s.push_str("any"),
        DataType::Unknown => s.push_str("unknown"),
        DataType::Primitive(p) => s.push_str(primitive_dt(&ts.bigint, p, location)?),
        DataType::Literal(l) => literal_dt(s, l),
        DataType::List(l) => list_dt(s, ts, types, l, location, flattening)?,
        DataType::Map(m) => map_dt(s, ts, types, m, location, flattening)?,
        DataType::Nullable(t) => {
            datatype(s, ts, types, &*t, location, flattening)?;
            let or_null = " | null";
            if !s.ends_with(or_null) {
                s.push_str(or_null);
            }
        }
        DataType::Struct(st) => {
            location.push(st.name().clone());
            fields_dt(s, ts, types, st.name(), &st.fields(), location, flattening)?
        }
        DataType::Enum(e) => enum_dt(s, ts, types, e, location, flattening)?,
        DataType::Tuple(t) => tuple_dt(s, ts, types, t, location, flattening)?,
        DataType::Reference(r) => reference_dt(s, ts, types, r, location, flattening)?,
        DataType::Generic(g) => s.push_str(g.borrow()),
    };

    Ok(())
}

fn primitive_dt(
    b: &BigIntExportBehavior,
    p: &PrimitiveType,
    location: Vec<Cow<'static, str>>,
) -> Result<&'static str, Error> {
    use PrimitiveType::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 | f32 | f64 => "number",
        usize | isize | i64 | u64 | i128 | u128 => match b {
            BigIntExportBehavior::String => "string",
            BigIntExportBehavior::Number => "number",
            BigIntExportBehavior::BigInt => "bigint",
            BigIntExportBehavior::Fail => {
                return Err(Error::BigIntForbidden {
                    path: location.join("."),
                })
            }
        },
        PrimitiveType::bool => "boolean",
        String | char => "string",
    })
}

fn literal_dt(s: &mut String, l: &LiteralType) {
    use LiteralType::*;

    match l {
        i8(v) => write!(s, "{v}"),
        i16(v) => write!(s, "{v}"),
        i32(v) => write!(s, "{v}"),
        u8(v) => write!(s, "{v}"),
        u16(v) => write!(s, "{v}"),
        u32(v) => write!(s, "{v}"),
        f32(v) => write!(s, "{v}"),
        f64(v) => write!(s, "{v}"),
        bool(v) => write!(s, "{v}"),
        String(v) => write!(s, "\"{v}\""),
        char(v) => write!(s, "\"{v}\""),
        None => write!(s, "null"),
        // We panic because this is a bug in Specta.
        v => unreachable!("attempted to export unsupported LiteralType variant {v:?}"),
    }
    .expect("writing to a string is an infallible operation");
}

fn list_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    l: &List,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    // We use `T[]` instead of `Array<T>` to avoid issues with circular references.

    let mut result = String::new();
    datatype(&mut result, ts, types, &l.ty(), location, flattening)?;
    let result = if (result.contains(' ') && !result.ends_with('}'))
        // This is to do with maintaining order of operations.
        // Eg `{} | {}` must be wrapped in parens like `({} | {})[]` but `{}` doesn't cause `{}[]` is valid
        || (result.contains(' ') && (result.contains('&') || result.contains('|')))
    {
        format!("({result})")
    } else {
        result
    };

    match l.length() {
        Some(len) => {
            s.push_str("[");
            iter_with_sep(
                s,
                0..len,
                |s, _| {
                    s.push_str(&result);
                    Ok(())
                },
                ", ",
            )?;
            s.push_str("]");
        }
        None => {
            s.push_str(&result);
            s.push_str("[]");
        }
    }

    Ok(())
}

fn map_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    m: &Map,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
    // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
    s.push_str("Partial<{ [key in ");
    datatype(s, ts, types, m.key_ty(), location.clone(), flattening)?;
    s.push_str("]: ");
    datatype(s, ts, types, m.value_ty(), location, flattening)?;
    s.push_str(" }>");
    Ok(())
}

fn enum_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    e: &EnumType,
    mut location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    assert!(!flattening, "todo: support for flattening enums"); // TODO

    location.push(e.name().clone());

    let mut _ts = None;
    if e.skip_bigint_checks() {
        _ts = Some(Typescript {
            bigint: BigIntExportBehavior::Number,
            ..ts.clone()
        });
        _ts.as_ref().expect("set above")
    } else {
        ts
    };

    let variants = e.variants().iter().filter(|(_, variant)| !variant.skip());

    if variants.clone().next().is_none()
    /* is_empty */
    {
        s.push_str("never");
        return Ok(());
    }

    let mut variants = variants
        .into_iter()
        .map(|(variant_name, variant)| {
            let mut s = String::new();
            let mut location = location.clone();
            location.push(variant_name.clone());

            // TODO
            // variant.deprecated()
            // variant.docs()

            match &e.repr() {
                EnumRepr::Untagged => {
                    fields_dt(&mut s, ts, types, variant_name, variant.fields(), location, flattening)?;
                },
                EnumRepr::External => match variant.fields() {
                    Fields::Unit => {
                        s.push_str("\"");
                        s.push_str(variant_name);
                        s.push_str("\"");
                    },
                    Fields::Unnamed(n) if n.fields().into_iter().filter(|f| f.ty().is_some()).next().is_none() /* is_empty */ => {
                        // We detect `#[specta(skip)]` by checking if the unfiltered fields are also empty.
                        if n.fields().is_empty() {
                            s.push_str("[]");
                        } else {
                            s.push_str("\"");
                            s.push_str(variant_name);
                            s.push_str("\"");
                        }
                    }
                    _ => {
                        s.push_str("{ ");
                        s.push_str(&escape_key(variant_name));
                        s.push_str(": ");
                        fields_dt(&mut s, ts, types, variant_name, variant.fields(), location, flattening)?;
                        s.push_str(" }");
                    }
                }
                EnumRepr::Internal { tag } => {
                    // TODO: Unconditionally wrapping in `(` kinda sucks.
                    write!(s, "({{ {}: \"{}\"", escape_key(tag), variant_name).expect("infallible");

                    match variant.fields() {
                        Fields::Unit => {
                            s.push_str(" }");
                        },
                        // Fields::Unnamed(f) => {
                        //     let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

                        //     // if fields.len

                        //     // TODO: Having no fields are skipping is valid
                        //     // TODO: Having more than 1 field is invalid

                        //     // TODO: Check if the field's type is object-like and can be merged.

                        //     todo!();
                        // }
                        f => {
                            s.push_str("; ");
                            flattened_fields_dt(&mut s, ts, types, variant_name, f, location, flattening)?;
                            s.push_str(" }");
                        }
                    }
                    s.push_str(")");
                }
                EnumRepr::Adjacent { tag, content } => {
                    write!(s, "{{ {}: \"{}\"", escape_key(tag), variant_name).expect("infallible");

                    match variant.fields() {
                        Fields::Unit => {},
                        f => {
                            write!(s, "; {}: ", escape_key(content)).expect("infallible");
                            fields_dt(&mut s, ts, types, variant_name, f, location ,flattening)?;
                        }
                    }

                    s.push_str(" }");
                }
            }

            Ok(s)
        })
        .collect::<Result<Vec<String>, Error>>()?;

    // TODO: Instead of deduplicating on the string, we should do it in the AST.
    // This would avoid the intermediate `String` allocations and be more reliable.
    variants.dedup();

    iter_with_sep(
        s,
        variants,
        |s, v| {
            s.push_str(&v);
            Ok(())
        },
        " | ",
    )?;

    Ok(())
}

fn fields_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    name: &Cow<'static, str>,
    f: &Fields,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    assert!(!flattening, "todo: support for flattening enums"); // TODO

    match f {
        Fields::Unit => s.push_str("null"),
        Fields::Unnamed(f) => {
            let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

            // A single field usually becomes `T`.
            // but when `#[serde(skip)]` is used it should be `[T]`.
            if fields.clone().count() == 1 && f.fields.len() == 1 {
                return field_dt(
                    s,
                    ts,
                    types,
                    None,
                    fields.next().expect("checked above"),
                    location,
                    flattening,
                );
            }

            s.push_str("[");
            iter_with_sep(
                s,
                fields.enumerate(),
                |s, (i, f)| {
                    let mut location = location.clone();
                    location.push(i.to_string().into());

                    field_dt(s, ts, types, None, f, location, flattening)
                },
                ", ",
            )?;
            s.push_str("]");
        }
        Fields::Named(f) => {
            let fields = f.fields().into_iter().filter(|(_, f)| f.ty().is_some());
            if fields.clone().next().is_none()
            /* is_empty */
            {
                if let Some(tag) = f.tag() {
                    write!(s, "{{ {}: \"{name}\" }}", escape_key(tag)).expect("infallible");
                } else {
                    s.push_str("Record<string, never>");
                }

                return Ok(());
            }

            s.push_str("{ ");
            if let Some(tag) = &f.tag() {
                write!(s, "{}: \"{name}\"; ", escape_key(tag)).expect("infallible");
            }

            iter_with_sep(
                s,
                fields,
                |s, (key, f)| {
                    let mut location = location.clone();
                    location.push(key.clone());

                    field_dt(s, ts, types, Some(key), f, location, flattening)
                },
                "; ",
            )?;
            s.push_str(" }");
        }
    }
    Ok(())
}

// TODO: Remove this to avoid so much duplicate logic
fn flattened_fields_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    name: &Cow<'static, str>,
    f: &Fields,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    match f {
        Fields::Unit => todo!(), // s.push_str("null"),
        Fields::Unnamed(f) => {
            // TODO: Validate flattening?

            let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

            // A single field usually becomes `T`.
            // but when `#[serde(skip)]` is used it should be `[T]`.
            if fields.clone().count() == 1 && f.fields.len() == 1 {
                return field_dt(
                    s,
                    ts,
                    types,
                    None,
                    fields.next().expect("checked above"),
                    location,
                    flattening,
                );
            }

            s.push_str("[");
            iter_with_sep(
                s,
                fields.enumerate(),
                |s, (i, f)| {
                    let mut location = location.clone();
                    location.push(i.to_string().into());

                    field_dt(s, ts, types, None, f, location, flattening)
                },
                ", ",
            )?;
            s.push_str("]");
        }
        Fields::Named(f) => {
            let fields = f.fields().into_iter().filter(|(_, f)| f.ty().is_some());
            if fields.clone().next().is_none()
            /* is_empty */
            {
                if let Some(tag) = f.tag() {
                    write!(s, "{{ {}: \"{name}\" }}", escape_key(tag)).expect("infallible");
                } else {
                    s.push_str("Record<string, never>");
                }

                return Ok(());
            }

            // s.push_str("{ "); // TODO
            if let Some(tag) = &f.tag() {
                write!(s, "{}: \"{name}\"; ", escape_key(tag)).expect("infallible");
            }

            iter_with_sep(
                s,
                fields,
                |s, (key, f)| {
                    let mut location = location.clone();
                    location.push(key.clone());

                    field_dt(s, ts, types, Some(key), f, location, flattening)
                },
                "; ",
            )?;
            // s.push_str(" }"); // TODO
        }
    }
    Ok(())
}

fn field_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    key: Option<&Cow<'static, str>>,
    f: &Field,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    let Some(ty) = f.ty() else {
        // These should be filtered out before getting here.
        unreachable!()
    };

    // TODO
    // field.deprecated(),
    // field.docs(),

    let ty = if f.inline() {
        specta::datatype::inline_dt(types, ty.clone())
    } else {
        ty.clone()
    };

    if !f.flatten() {
        if let Some(key) = key {
            s.push_str(&*escape_key(key));
            // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
            if f.optional() {
                s.push_str("?");
            }
            s.push_str(": ");
        }
    } else {
        // TODO: We need to validate the inner type can be flattened safely???

        //     data

        //     match ty {
        //         DataType::Any => todo!(),
        //         DataType::Unknown => todo!(),
        //         DataType::Primitive(primitive_type) => todo!(),
        //         DataType::Literal(literal_type) => todo!(),
        //         DataType::List(list) => todo!(),
        //         DataType::Map(map) => todo!(),
        //         DataType::Nullable(data_type) => todo!(),
        //         DataType::Struct(st) => {
        //             // location.push(st.name().clone()); // TODO
        //             flattened_fields_dt(s, ts, types, st.name(), &st.fields(), location)?
        //         }

        //         // flattened_fields_dt(s, ts, types, &ty, location)?,
        //         DataType::Enum(enum_type) => todo!(),
        //         DataType::Tuple(tuple_type) => todo!(),
        //         DataType::Reference(reference) => todo!(),
        //         DataType::Generic(generic_type) => todo!(),
        //     };
    }

    datatype(s, ts, types, &ty, location, flattening || f.flatten())?;

    // TODO: This is not always correct but is it ever correct?
    // If we can't use `?` (Eg. in a tuple) we manually join it.
    // if key.is_none() && f.optional() {
    //     s.push_str(" | undefined");
    // }

    Ok(())
}

fn tuple_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    t: &TupleType,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    match &t.elements()[..] {
        [] => s.push_str("null"),
        elems => {
            s.push_str("[");
            iter_with_sep(
                s,
                elems.into_iter().enumerate(),
                |s, (i, dt)| {
                    let mut location = location.clone();
                    location.push(i.to_string().into());

                    datatype(s, ts, types, &dt, location, flattening)
                },
                ", ",
            )?;
            s.push_str("]");
        }
    }
    Ok(())
}

fn reference_dt(
    s: &mut String,
    ts: &Typescript,
    types: &TypeCollection,
    r: &Reference,
    location: Vec<Cow<'static, str>>,
    flattening: bool,
) -> Result<(), Error> {
    let ndt = types
        .get(r.sid())
        // Should be impossible without a bug in Specta.
        .unwrap_or_else(|| panic!("Missing {:?} in `TypeCollection`", r.sid()));

    if r.inline() {
        todo!("inline reference!");
    }

    s.push_str(ndt.name());
    // TODO: We could possible break this out, the root `export` function also has to emit generics.
    match r.generics() {
        [] => {}
        generics => {
            s.push('<');
            // TODO: Should we push a location for which generic?
            iter_with_sep(
                s,
                generics,
                |s, dt| datatype(s, ts, types, &dt, location.clone(), flattening),
                ", ",
            )?;
            s.push('>');
        }
    }

    Ok(())
}

fn validate_name(
    ident: &Cow<'static, str>,
    location: &Vec<Cow<'static, str>>,
) -> Result<(), Error> {
    // TODO: Use a perfect hash-map for faster lookups?
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(Error::ForbiddenName {
            path: location.join("."),
            name,
        });
    }

    if ident.is_empty() {
        return Err(Error::InvalidName {
            path: location.join("."),
            name: ident.clone(),
        });
    }

    if let Some(first_char) = ident.chars().next() {
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(Error::InvalidName {
                path: location.join("."),
                name: ident.clone(),
            });
        }
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        return Err(Error::InvalidName {
            path: location.join("."),
            name: ident.clone(),
        });
    }

    Ok(())
}

fn escape_key(name: &Cow<'static, str>) -> Cow<'static, str> {
    let needs_escaping = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    if !needs_escaping {
        format!(r#""{name}""#).into()
    } else {
        name.clone()
    }
}

/// Iterate with separate and error handling
fn iter_with_sep<T>(
    s: &mut String,
    i: impl IntoIterator<Item = T>,
    mut item: impl FnMut(&mut String, T) -> Result<(), Error>,
    sep: &'static str,
) -> Result<(), Error> {
    for (i, e) in i.into_iter().enumerate() {
        if i != 0 {
            s.push_str(sep);
        }
        (item)(s, e)?;
    }
    Ok(())
}

// A smaller helper until this is stablised into the Rust standard library.
fn intersperse<T: Clone>(iter: impl Iterator<Item = T>, sep: T) -> impl Iterator<Item = T> {
    iter.enumerate().flat_map(move |(i, item)| {
        if i == 0 {
            vec![item]
        } else {
            vec![sep.clone(), item]
        }
    })
}
