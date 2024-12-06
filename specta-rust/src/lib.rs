//! [Rust](https://www.rust-lang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use specta::{
    datatype::{DataType, NamedDataType, StructFields},
    Generics, Type, TypeCollection,
};

/// TODO
pub fn export<T: Type>() -> Result<String, String> {
    datatype(&T::inline(
        &mut TypeCollection::default(),
        Generics::Definition,
    ))
}

pub fn export_named_datatype(
    // conf: &Typescript,
    typ: &NamedDataType,
    // type_map: &TypeCollection,
) -> Result<String, String> {
    Ok(format!(
        "pub type {} = {}",
        typ.name(),
        datatype(&typ.inner)?
    ))
}

pub fn datatype(t: &DataType) -> Result<String, String> {
    // TODO: This system does lossy type conversions. That is something I want to fix in the future but for now this works. Eg. `HashSet<T>` will be exported as `Vec<T>`
    // TODO: Serde serialize + deserialize on types

    Ok(match t {
        DataType::Unknown => todo!(),
        DataType::Any => "serde_json::Value".to_owned(),
        DataType::Primitive(ty) => ty.to_rust_str().to_owned(),
        DataType::Literal(_) => todo!(),
        DataType::Nullable(t) => format!("Option<{}>", datatype(t)?),
        DataType::Map(t) => format!(
            "HashMap<{}, {}>",
            datatype(&t.key_ty())?,
            datatype(&t.value_ty())?
        ),
        DataType::List(t) => format!("Vec<{}>", datatype(t.ty())?),
        DataType::Tuple(tuple) => match &tuple.elements()[..] {
            [] => "()".to_string(),
            [ty] => datatype(ty)?,
            tys => format!(
                "({})",
                tys.iter()
                    .map(|v| datatype(v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ),
        },
        DataType::Struct(s) => match &s.fields() {
            StructFields::Unit => "struct {name}".to_string(),
            StructFields::Named(_) => todo!(),
            StructFields::Unnamed(_) => todo!(),
            // fields => {
            //     let generics = (!s.generics().is_empty())
            //         .then(|| format!("<{}>", s.generics().join(", ")))
            //         .unwrap_or_default();

            //     let fields = fields
            //         .iter()
            //         .map(|f| {
            //             let name = &f.name;
            //             let typ = datatype(&f.ty)?;
            //             Ok(format!("\t{name}: {typ}"))
            //         })
            //         .collect::<Result<Vec<_>, String>>()?
            //         .join(", ");

            //     let tag = s
            //         .tag()
            //         .clone()
            //         .map(|t| format!("{t}: String"))
            //         .unwrap_or_default();

            //     format!("struct {}{generics} {{ {fields}{tag} }}\n", s.name())
            // }
        },
        DataType::Enum(_) => todo!(),
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
        DataType::Generic(t) => t.to_string(),
    })
}
