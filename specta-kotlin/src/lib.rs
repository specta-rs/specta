//! TODO
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use specta::{DataType, Generics, PrimitiveType, Type, TypeMap};

/// TODO
pub fn export<T: Type>() -> Result<String, String> {
    datatype(&T::inline(&mut TypeMap::default(), Generics::Definition))
}

fn datatype(t: &DataType) -> Result<String, String> {
    Ok(match t {
        DataType::Primitive(p) => match p {
            PrimitiveType::String => "String",
            PrimitiveType::char => "Char",
            PrimitiveType::i8 => "Byte",
            PrimitiveType::i16 => "Short",
            PrimitiveType::isize | PrimitiveType::i32 => "Int",
            PrimitiveType::i64 => "Long",
            PrimitiveType::u8 => "UByte",
            PrimitiveType::u16 => "UShort",
            PrimitiveType::usize | PrimitiveType::u32 => "UInt",
            PrimitiveType::u64 => "ULong",
            PrimitiveType::bool => "Boolean",
            PrimitiveType::f32 => "Float",
            PrimitiveType::f64 => "Double",
            PrimitiveType::i128 | PrimitiveType::u128 => {
                return Err("Swift does not support 128 numbers!".to_owned())
            }
        }
        .to_string(),
        DataType::List(t) => format!("List<{}>", datatype(t.ty())?),
        DataType::Tuple(_) => return Err("Kotlin does not support tuple types".to_owned()),
        DataType::Map(t) => format!(
            "HashMap<{}, {}>",
            datatype(&t.key_ty())?,
            datatype(&t.value_ty())?
        ),
        DataType::Generic(t) => t.to_string(),
        DataType::Reference(reference) => {
            let name = reference.name();
            let generics = reference.generics();

            match &generics[..] {
                [] => name.to_string(),
                generics => {
                    let generics = generics
                        .iter()
                        .map(|(_, t)| datatype(t))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");

                    format!("{name}<{generics}>")
                }
            }
        }
        DataType::Nullable(t) => format!("{}?", datatype(&t)?),
        DataType::Struct(s) => {
            let name = s.name();
            let generics = s.generics();
            let fields = s.fields();
            let tag = s.tag();

            // let decl = match &fields[..] {
            //     [] => "class {name}".to_string(),
            //     fields => {
            //         let generics = (!generics.is_empty())
            //             .then(|| format!("<{}>", generics.join(", ")))
            //             .unwrap_or_default();

            //         let fields = fields
            //             .iter()
            //             .map(|f| {
            //                 let name = &f.name;
            //                 let typ = datatype(&f.ty)?;
            //                 let optional = matches!(f.ty, DataType::Nullable(_))
            //                     .then(|| "= null")
            //                     .unwrap_or_default();

            //                 Ok(format!("\tvar {name}: {typ}{optional}"))
            //             })
            //             .collect::<Result<Vec<_>, String>>()?
            //             .join(", ");

            //         let tag = tag
            //             .clone()
            //             .map(|t| format!("var {t}: String"))
            //             .unwrap_or_default();

            //         format!("data class {name}{generics} ({fields}{tag})")
            //     }
            // };
            // format!("@Serializable\n{decl}\n")
            todo!();
        }
        DataType::Literal(_) => return Err("Kotlin does not support literal types!".to_owned()),
        _ => todo!(),
    })
}
