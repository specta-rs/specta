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
        NamedDataType, PrimitiveType, TupleType,
    },
    TypeCollection,
};

use crate::{reserved_names::*, BigIntExportBehavior, Error, Typescript};

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
    //     validate_name(dt.name(), &vec![])?;

    //     let generics = dt
    //         .inner
    //         .generics()
    //         .into_iter()
    //         .filter(|g| !g.is_empty())
    //         .map(|g| {
    //             iter::once("<")
    //                 .chain(intersperse(g.into_iter().map(|g| g.borrow()), ", "))
    //                 .chain(iter::once(">"))
    //         })
    //         .into_iter()
    //         .flatten();

    //     let s = iter::empty()
    //         .chain(["export type ", dt.name()])
    //         .chain(generics)
    //         .chain([" = "])
    //         .collect::<String>();

    //     // TODO: Collecting directly into `result` insetad of allocating `s`?
    //     let mut result = "".to_string(); // TODO:
    //                                      //     ts
    //                                      //     .comment_exporter
    //                                      //     .map(|v| {
    //                                      //         v(CommentFormatterArgs {
    //                                      //             docs: dt.docs(),
    //                                      //             deprecated: dt.deprecated(),
    //                                      //         })
    //                                      //     })
    //                                      //     .unwrap_or_default();
    //     result.push_str(&s);

    //     datatype(
    //         &mut result,
    //         ts,
    //         types,
    //         &dt.inner,
    //         vec![dt.name().clone()],
    //         State { flattening: false },
    //     )?;
    //     result.push_str(";");

    //     Ok(result)

    // TODO: This is the legacy way. Remove it once everything is ready to go.
    {
        // let mut types = TypeCollection::default();
        // T::definition(&mut types);
        // let ty = types.get(T::ID).unwrap();
        // let ty = specta::datatype::inline_and_flatten_ndt(ty.clone(), &types);

        // validate_dt(ty.ty(), &types)?;
        let result = crate::legacy::export_named_datatype(&Default::default(), &dt, &types);

        // if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        //     return Err(Error::DuplicateTypeNameLegacy(ty_name, l0, l1));
        // }

        result
    }
}

/// Generate an inlined Typescript string for a specific [`DataType`].
///
/// This methods leaves all the same things as the [`export`] method up to the user.
///
pub fn inline(ts: &Typescript, types: &TypeCollection, dt: &DataType) -> Result<String, Error> {
    // let mut s = String::new();
    // datatype(&mut s, ts, types, dt, vec![], State { flattening: false })?;
    // Ok(s)

    // TODO: This is the legacy way. Remove it once everything is ready to go.
    {
        let mut types = TypeCollection::default();

        // let ty = T::definition(&mut types);
        let ty = specta::datatype::inline(dt.clone(), &types);

        specta_serde::validate_dt(&ty, &types)?;
        let result = crate::legacy::datatype(
            &Default::default(),
            &specta::datatype::FunctionResultVariant::Value(ty.clone()),
            &types,
        );

        result
    }
}

// /// Generate an `export Type = ...` Typescript string for a specific [`DataType`].
// ///
// /// Similar to [`export`] but works on a [`FunctionResultVariant`].
// pub fn export_func(ts: &Typescript, types: &TypeCollection, dt: FunctionResultVariant) -> Result<String, ExportError> {
//     todo!();
// }

// TODO: Private
pub(crate) fn primitive_dt(
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

// TODO: Private
pub(crate) fn literal_dt(s: &mut String, l: &LiteralType) {
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
