//! [Rust](https://www.rust-lang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use std::path::Path;

use inflector::Inflector;
use specta::{
    datatype::{DataType, NamedDataType, Fields},
    Type, TypeCollection,
};

type Error = String; // TODO: Proper error type

#[derive(Debug, Default, Clone)]
pub struct Rust;

impl Rust {
    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        todo!();
    }

    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        todo!();
    }
}

// /// TODO
// pub fn export<T: Type>() -> Result<String, Error> {
//     datatype(&T::inline(
//         &mut TypeCollection::default(),
//         Generics::Definition,
//     ))
// }

// pub fn export_named_datatype(
//     // conf: &Typescript,
//     typ: &NamedDataType,
//     // types: &TypeCollection,
// ) -> Result<String, Error> {
//     Ok(format!(
//         "pub type {} = {}",
//         typ.name(),
//         datatype(&typ.inner, types)?
//     ))
// }

// pub fn datatype(t: &DataType, types: &TypeCollection) -> Result<String, Error> {
//     // TODO: This system does lossy type conversions. That is something I want to fix in the future but for now this works. Eg. `HashSet<T>` will be exported as `Vec<T>`
//     // TODO: Serde serialize + deserialize on types

//     Ok(match t {
//         DataType::Unknown => todo!(),
//         DataType::Any => "serde_json::Value".to_owned(),
//         DataType::Primitive(ty) => ty.to_rust_str().to_owned(),
//         DataType::Literal(_) => todo!(),
//         DataType::Nullable(t) => format!("Option<{}>", datatype(t, types)?),
//         DataType::Map(t) => format!(
//             "HashMap<{}, {}>",
//             datatype(&t.key_ty(), types)?,
//             datatype(&t.value_ty(), types)?
//         ),
//         DataType::List(t) => format!("Vec<{}>", datatype(t.ty(), types)?),
//         DataType::Tuple(tuple) => match &tuple.elements()[..] {
//             [] => "()".to_string(),
//             [ty] => datatype(ty, types)?,
//             tys => format!(
//                 "({})",
//                 tys.iter()
//                     .map(|v| datatype(v, types))
//                     .collect::<Result<Vec<_>, _>>()?
//                     .join(", ")
//             ),
//         },
//         DataType::Struct(s) => match &s.fields() {
//             Fields::Unit => {
//                 if s.generics().len() != 0 {
//                     return Err("generics can't be used on a unit struct".into());
//                 }

//                 format!("pub struct {};", s.name())
//             }
//             Fields::Named(fields) => {
//                 assert!(s.generics().len() == 0, "missing support for generics"); // TODO
//                 // assert!(s.tag().is_some(), "missing support for tagging"); // TODO

//                 // TODO: Error if any of the generics are not used or add `PhantomData` field?

//                 let mut s = format!("pub struct {} {{", s.name());

//                 for (k, field) in fields.fields() {
//                     // TODO: Documentation, deprecated, etc.

//                     let key = k.to_camel_case();
//                     if &*key != k {
//                     s.push_str(&format!("\n    #[serde(rename = \"{key}\")]"));
//                     }
//                     // TODO: Don't `unwrap` here
//                     s.push_str(&format!("\n    pub {}: {},", key.to_camel_case(), datatype(field.ty().unwrap(), types)?));
//                 }

//                 s.push_str("\n}");
//                 s
//             }
//             Fields::Unnamed(_) => todo!(),
//         },
//         DataType::Enum(e) => {
//             let mut s = format!("pub enum {} {{ \n", e.name());
//             for (name, variant) in e.variants().iter() {
//                 let variant = match variant.fields() {
//                     Fields::Unit => format!("\t{},", name.to_class_case()),
//                     Fields::Named(fields) => format!("\t{},", name.to_class_case()),
//                     Fields::Unnamed(fields) => format!("\t{},", name.to_class_case()),
//                 };
//                 s.push_str(&variant);
//             }
//             s.push_str("}");
//             s
//         }
//         DataType::Reference(reference) => {
//             let definition = types.get(reference.sid()).unwrap(); // TODO: Error handling

//             match &reference.generics()[..] {
//                 [] => definition.name().to_string(),
//                 generics => {
//                     let generics = generics
//                         .iter()
//                         .map(|(_, t)| datatype(t, types))
//                         .collect::<Result<Vec<_>, _>>()?
//                         .join(", ");

//                     format!("{}<{generics}>", definition.name())
//                 }
//             }
//         },
//         DataType::Generic(t) => t.to_string(),
//     })
// }
