//! Primitives provide building blocks for Specta-based libraries.
//!
//! These are advanced features and should generally be avoided in end-user applications.

use std::{borrow::Borrow, fmt::Write as _, iter};

use specta::{datatype::{reference::Reference, DataType, EnumType, Field, Fields, FunctionResultVariant, List, LiteralType, Map, NamedDataType, PrimitiveType, StructType, TupleType}, TypeCollection};

use crate::{constants::*, utils::intersperse, BigIntExportBehavior, CommentFormatterArgs, ExportError, Typescript};

/// Generate an `export Type = ...` Typescript string for a specific [`DataType`].
///
/// This method leaves the following up to the implementor:
///  - Ensuring all referenced types are exported
///  - Handling multiple type with overlapping names
///  - Transforming the type for your serialization format (Eg. Serde)
///
pub fn export(ts: &Typescript, types: &TypeCollection, dt: &NamedDataType) -> Result<String, ExportError> {
    validate_name(dt.name())?;

    let generics = dt.inner.generics()
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|g| iter::once("<")
            .chain(intersperse(g.into_iter().map(|g| g.borrow()), ", "))
            .chain(iter::once(">"))
        )
        .into_iter()
        .flatten();

    let s = iter::empty()
        .chain([
            "export type ",
            dt.name(),
        ])
        .chain(generics)
        .chain([" = "])
        .collect::<String>();

    // TODO: Collecting directly into `result` insetad of allocating `s`?
    let mut result = ts
        .comment_exporter
        .map(|v| v(CommentFormatterArgs { docs: dt.docs(), deprecated: dt.deprecated() }))
        .unwrap_or_default();
    result.push_str(&s);

    datatype(&mut result, ts, types, &dt.inner)?;
    result.push_str(";");

    Ok(result)
}

/// Generate an inlined Typescript string for a specific [`DataType`].
///
/// This methods leaves all the same things as the [`export`] method up to the user.
///
pub fn inline(ts: &Typescript, types: &TypeCollection, dt: &DataType) -> Result<String, ExportError> {
    let mut s = String::new();
    datatype(&mut s, ts, types, dt)?;
    Ok(s)
}

/// Generate an `export Type = ...` Typescript string for a specific [`DataType`].
///
/// Similar to [`export`] but works on a [`FunctionResultVariant`].
pub fn export_func(ts: &Typescript, types: &TypeCollection, dt: FunctionResultVariant) -> Result<String, ExportError> {
    todo!();
}

fn datatype(s: &mut String, ts: &Typescript, types: &TypeCollection, dt: &DataType) -> Result<(), ExportError> {
    match dt {
        DataType::Any => s.push_str(ANY),
        DataType::Unknown => s.push_str(UNKNOWN),
        DataType::Primitive(p) => s.push_str(primitive_dt(&ts.bigint, p)?),
        DataType::Literal(l) => literal_dt(s, l),
        DataType::List(l) => list_dt(s, ts, types, l)?,
        DataType::Map(m) => map_dt(s, ts, types, m)?,
        DataType::Nullable(t) => {
            datatype(s, ts, types, &*t)?;
            let or_null = " | null";
            if !s.ends_with(or_null) {
                s.push_str(or_null);
            }
        }
        DataType::Struct(st) => fields_dt(s, ts, types, &st.fields())?,
        DataType::Enum(e) => enum_dt(s, ts, types, e)?,
        DataType::Tuple(t) => tuple_dt(s, ts, types, t)?,
        DataType::Reference(r) => reference_dt(s, ts, types, r)?,
        DataType::Generic(g) => s.push_str(g.borrow()),
    };


    Ok(())
}

fn primitive_dt(b: &BigIntExportBehavior, p: &PrimitiveType) -> Result<&'static str, ExportError> {
    use PrimitiveType::*;

    Ok(match p {
        i8 | i16 | i32 | u8 | u16 | u32 | f32 | f64 => NUMBER,
        usize | isize | i64 | u64 | i128 | u128 => match b {
            BigIntExportBehavior::String => STRING,
            BigIntExportBehavior::Number => NUMBER,
            BigIntExportBehavior::BigInt => BIGINT,
            BigIntExportBehavior::Fail => return Err(ExportError::BigIntForbidden(todo!())),
            BigIntExportBehavior::FailWithReason(reason) => return Err(ExportError::Other(todo!(), reason.to_string())),
        }
        PrimitiveType::bool => BOOLEAN,
        String | char => STRING,
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
        None => write!(s, "{NULL}"),
        // We panic because this is a bug in Specta.
        v => unreachable!("attempted to export unsupported LiteralType variant {v:?}"),
    }.expect("writing to a string is an infallible operation");
}

fn list_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, l: &List) -> Result<(), ExportError> {
    // We use `T[]` instead of `Array<T>` to avoid issues with circular references.

    let mut result = String::new();
    datatype(&mut result, ts, types, &l.ty())?;
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
            iter_with_sep(s, (0..len), |s, _| {
                s.push_str(&result);
                Ok(())
            }, ", ")?;
            s.push_str("]");

        },
        None => {
            s.push_str(&result);
            s.push_str("[]");
        }
    }

    Ok(())
}

fn map_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, m: &Map) -> Result<(), ExportError> {
    // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
    // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
    s.push_str("Partial<{ [key in ");
    datatype(s, ts, types, m.key_ty())?;
    s.push_str("]: ");
    datatype(s, ts, types, m.value_ty())?;
    s.push_str(" }>");
    Ok(())
}

fn enum_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, e: &EnumType) -> Result<(), ExportError> {
    if e.variants().is_empty() {
        s.push_str(NEVER);
        return Ok(());
    }

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

    let variants = e
        .variants()
        .iter()
        .filter(|(_, variant)| !variant.skip());

    // TODO: variants.dedup();
    iter_with_sep(s, variants, |s, (variant_name, variant)| {
        validate_key(variant_name)?;

        // TODO
        // variant.deprecated()
        // variant.docs()

        fields_dt(s, ts, types, variant.fields())
    }, " | ")?;
    Ok(())
}

fn fields_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, f: &Fields) -> Result<(), ExportError> {
    match f {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(f) => {
            let mut fields = f.fields().into_iter().filter(|f| f.ty().is_some());

            if fields.clone().count() == 1 {
                return field_dt(s, ts, types, &fields.next().expect("checked above"));
            }

            s.push_str("[");
            iter_with_sep(s, fields, |s, f| field_dt(s, ts, types, f), ", ")?;
            s.push_str("]");
        }
        Fields::Named(f) => {
            let fields = f.fields().into_iter().filter(|(_, f)| f.ty().is_some());
            if fields.clone().count() == 0 {
                s.push_str("Record<string, never>");
                return Ok(());
            }

            s.push_str("{ ");
            iter_with_sep(s, fields, |s, (key, f)| {
                validate_key(key)?;
                s.push_str(key);
                s.push_str(": ");
                field_dt(s, ts, types, f)
            }, ", ")?;
            s.push_str(" }");
        }
    }
    Ok(())
}

fn field_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, f: &Field) -> Result<(), ExportError> {
    let Some(ty) = f.ty() else {
        // These should be filtered out before getting here.
        return unreachable!();
    };

    // TODO
    // if f.inline() {
    //     todo!("inline field");
    // }

    // TODO
    // field.deprecated(),
    //     field.docs(),

    // // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    // let (key, ty) = match field.optional() {
    //     true => (format!("{field_name_safe}?").into(), ty),
    //     false => (field_name_safe, ty),
    // };

    datatype(s, ts, types, &ty)?;

    Ok(())
}

fn tuple_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, t: &TupleType) -> Result<(), ExportError> {
    match &t.elements()[..] {
        [] => s.push_str(NULL),
        elems => {
            s.push_str("[");
            iter_with_sep(s, elems, |s, dt| datatype(s, ts, types, &dt), ", ")?;
            s.push_str("]");
        }
    }
    Ok(())
}

fn reference_dt(s: &mut String, ts: &Typescript, types: &TypeCollection, r: &Reference) -> Result<(), ExportError> {
    let ndt = types.get(r.sid())
        // Should be impossible without a bug in Specta.
        .unwrap_or_else(|| panic!("Missing {:?} in `TypeCollection`", r.sid()));

    if r.inline() {
        todo!("inline reference!");
    }

    s.push_str(ndt.name());
    // TODO: We could possible break this out, the root `export` function also has to emit generics.
    match r.generics() {
        [] => {},
        generics => {
            s.push('<');
            iter_with_sep(s, generics, |s, dt| datatype(s, ts, types, &dt), ", ")?;
            s.push('>');
        }
    }

    Ok(())
}

fn validate_name(ident: &str) -> Result<(), ExportError> {
    // TODO: Use a perfect hash-map for faster lookups?
    if let Some(_) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        todo!();
    }

    if let Some(first_char) = ident.chars().next() {
        if !first_char.is_alphabetic() && first_char != '_' {
            todo!();
        }
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        todo!();
    }

    Ok(())
}

fn validate_key(ident: &str) -> Result<(), ExportError> {
    // let valid = field_name
    //     .chars()
    //     .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    //     && field_name
    //         .chars()
    //         .next()
    //         .map(|first| !first.is_numeric())
    //         .unwrap_or(true);

    // if force_string || !valid {
    //     format!(r#""{field_name}""#).into()
    // } else {
    //     field_name
    // }
    Ok(())
}

/// Iterate with separate and error handling
fn iter_with_sep<T>(s: &mut String, i: impl IntoIterator<Item = T>, mut item: impl FnMut(&mut String, T) -> Result<(), ExportError>, sep: &'static str) -> Result<(), ExportError> {
    for (i, e) in i.into_iter().enumerate() {
        if i != 0 {
            s.push_str(sep);
        }
        (item)(s, e)?;
    }
    Ok(())
}
