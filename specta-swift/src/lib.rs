//! [Swift](https://www.swift.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use specta::{
    datatype::{DataType, PrimitiveType},
    Generics, Type, TypeCollection,
};

/// TODO
pub fn export<T: Type>() -> Result<String, String> {
    datatype(&T::inline(
        &mut TypeCollection::default(),
        Generics::Definition,
    ))
}

fn datatype(t: &DataType) -> Result<String, String> {
    Ok(match t {
        DataType::Primitive(p) => match p {
            PrimitiveType::String | PrimitiveType::char => "String",
            PrimitiveType::i8 => "Int8",
            PrimitiveType::u8 => "UInt8",
            PrimitiveType::i16 => "Int16",
            PrimitiveType::u16 => "UInt16",
            PrimitiveType::usize => "UInt",
            PrimitiveType::isize => "Int",
            PrimitiveType::i32 => "Int32",
            PrimitiveType::u32 => "UInt32",
            PrimitiveType::i64 => "Int64",
            PrimitiveType::u64 => "UInt64",
            PrimitiveType::bool => "Bool",
            PrimitiveType::f32 => "Float",
            PrimitiveType::f64 => "Double",
            PrimitiveType::i128 | PrimitiveType::u128 => {
                return Err("Swift does not support 128 numbers!".to_owned());
            }
        }
        .to_string(),
        DataType::Any => "Codable".to_string(),
        DataType::List(t) => format!("[{}]", datatype(&t.ty())?),
        DataType::Tuple(tuple) => match &tuple.elements()[..] {
            [] => "CodableVoid".to_string(),
            [ty] => datatype(ty)?,
            tys => format!(
                "({})",
                tys.iter()
                    .map(datatype)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ),
        },
        DataType::Map(t) => format!("[{}: {}]", datatype(&t.key_ty())?, datatype(&t.value_ty())?),
        DataType::Generic(t) => t.to_string(),
        DataType::Reference(reference) => match &reference.generics()[..] {
            [] => reference.name().to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|(_, t)| datatype(t))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                format!("{}<{generics}>", reference.name())
            }
        },
        DataType::Nullable(t) => format!("{}?", datatype(t)?),
        DataType::Struct(s) => {
            //         match &s.fields()[..] {
            //             [] => "CodableVoid".to_string(),
            //             fields => {
            //                 // TODO: Handle invalid field names
            //                 let generics = (!s.generics().is_empty())
            //                     .then(|| {
            //                         format!(
            //                             "<{}>",
            //                             s.generics()
            //                                 .iter()
            //                                 .map(|g| format!("{}: Codable", g))
            //                                 .collect::<Vec<_>>()
            //                                 .join(", ")
            //                         )
            //                     })
            //                     .unwrap_or_default();

            //                 let fields = fields
            //                     .iter()
            //                     .map(|f| {
            //                         let name = &f.name;
            //                         let typ = datatype(&f.ty)?;

            //                         Ok(format!("\tpublic let {name}: {typ}"))
            //                     })
            //                     .collect::<Result<Vec<_>, String>>()?
            //                     .join("\n");

            //                 let tag = s
            //                     .tag()
            //                     .clone()
            //                     .map(|t| format!("\t{t}: String"))
            //                     .unwrap_or_default();

            //                 r#"public struct {name}{generics}: Codable {{
            //     {tag}{fields}
            // }}"#
            //                 .to_string()
            //             }
            //         }

            todo!();
        }
        DataType::Literal(_) => return Err("Swift does not support literal types!".to_owned()),
        _ => todo!(),
    })
}
