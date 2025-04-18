//! [Rust](https://www.rust-lang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use std::{borrow::Cow, iter, path::Path};

use inflector::Inflector;
use specta::{
    datatype::{DataType, EnumRepr, Fields},
    TypeCollection,
};

static STANDARD_DERIVE: &str = "#[derive(Debug, Clone, Deserialize, Serialize)]";

type Error = String; // TODO: Proper error type

#[derive(Debug, Default, Clone)]
pub struct Rust {
    any_value: Option<Cow<'static, str>>,
    extras: String,
    custom_struct_attributes: Option<Cow<'static, str>>,
    custom_enum_attributes: Option<Cow<'static, str>>,
    custom_field_attributes: Option<Cow<'static, str>>,
}

impl Rust {
    pub fn append(mut self, value: &str) -> Self {
        self.extras.push_str(&value);
        self
    }

    // TODO: Can we just avoid this being a thing?
    pub fn with_any(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.any_value = Some(value.into());
        self
    }

    #[doc(hidden)]
    // TODO: Remove this in favor of a better system. Should the `DataType` structs hold the raw attribute so we can emit the same ones?
    pub fn custom_container_attributes(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        let value = value.into();
        self.custom_struct_attributes = Some(value.clone());
        self.custom_enum_attributes = Some(value);
        self
    }

    #[doc(hidden)]
    // TODO: Remove this in favor of a better system. Should the `DataType` structs hold the raw attribute so we can emit the same ones?
    pub fn custom_struct_attributes(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.custom_struct_attributes = Some(value.into());
        self
    }

    #[doc(hidden)]
    // TODO: Remove this in favor of a better system. Should the `DataType` structs hold the raw attribute so we can emit the same ones?
    pub fn custom_enum_attributes(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.custom_enum_attributes = Some(value.into());
        self
    }

    #[doc(hidden)]
    // TODO: Remove this in favor of a better system. Should the `DataType` structs hold the raw attribute so we can emit the same ones?
    pub fn custom_field_attributes(mut self, value: impl Into<Cow<'static, str>>) -> Self {
        self.custom_field_attributes = Some(value.into());
        self
    }

    // TODO: Configuring the derive attributes which are applied. // TODO: How are we gonna handle imports?

    // TODO: `&mut self` versions of these methods?

    pub fn export(&self, types: &TypeCollection) -> Result<String, Error> {
        // TODO
        // let mut out = self.header.to_string();
        // if !out.is_empty() {
        //     out.push('\n');
        // }
        // out += &self.framework_header;
        // out.push_str("\n\n");

        // if let Some((name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
        //     return Err(Error::DuplicateTypeName {
        //         types: (l0, l1),
        //         name
        //     });
        // }

        // specta_serde::validate(types)?;

        let mut s = format!("//! This file was generated by [Specta](https://github.com/specta-rs/specta)\nuse serde::{{Deserialize, Serialize}};\n\n");
        for ndt in types.into_unsorted_iter() {
            s.push_str(&datatype(self, ndt.ty(), types)?);
            s.push_str("\n\n");
        }
        s.push_str(&self.extras);
        Ok(s)
    }

    pub fn export_to(&self, path: impl AsRef<Path>, types: &TypeCollection) -> Result<(), Error> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap(); // TODO: Error handling
        }
        std::fs::write(
            &path,
            self.export(types)?, // TODO: .map(|s| format!("{}{s}", self.header))?,
        )
        .unwrap(); // TODO: Error handling
                   // TODO
                   // if let Some(formatter) = self.formatter {
                   //     formatter(path)?;
                   // }
        Ok(())
    }
}

fn datatype(r: &Rust, t: &DataType, types: &TypeCollection) -> Result<String, Error> {
    // TODO: This system does lossy type conversions. That is something I want to fix in the future but for now this works. Eg. `HashSet<T>` will be exported as `Vec<T>`
    // TODO: Serde serialize + deserialize on types

    Ok(match t {
        // DataType::Unknown => todo!(),
        // // TODO: This should definetly be configurable cause they might not be using JSON.
        // DataType::Any => r
        //     .any_value
        //     .as_ref()
        //     .map(|v| v.to_string())
        //     .unwrap_or_else(|| "serde_json::Value".to_string()),
        DataType::Primitive(ty) => ty.to_rust_str().to_owned(),
        DataType::Literal(_) => todo!(),
        DataType::Nullable(t) => format!("Option<{}>", datatype(r, t, types)?),
        DataType::Map(t) => format!(
            "HashMap<{}, {}>",
            datatype(r, &t.key_ty(), types)?,
            datatype(r, &t.value_ty(), types)?
        ),
        DataType::List(t) => {
            if let Some(len) = t.length() {
                format!(
                    "({},)",
                    iter::repeat(datatype(r, t.ty(), types)?)
                        .take(len)
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            } else {
                format!("Vec<{}>", datatype(r, t.ty(), types)?)
            }
        }
        DataType::Tuple(tuple) => match &tuple.elements()[..] {
            [] => "()".to_string(),
            [ty] => datatype(r, ty, types)?,
            tys => format!(
                "({})",
                tys.iter()
                    .map(|v| datatype(r, v, types))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ),
        },
        DataType::Struct(s) => {
            let mut docs = s
                .sid()
                .map(|sid| types.get(sid).unwrap().docs().clone())
                .unwrap_or_default()
                .replace("\n", "\n /// ");
            if !docs.is_empty() {
                docs = format!("/// {}\n", docs);
            }

            match &s.fields() {
                Fields::Unit => {
                    // if s.generics().len() != 0 {
                    //     return Err("generics can't be used on a unit struct".into());
                    // }

                    format!(
                        "{docs}{STANDARD_DERIVE}{}\npub struct {};",
                        r.custom_struct_attributes
                            .clone()
                            .map(|v| format!("\n{v}"))
                            .unwrap_or("".into()),
                        s.name()
                    )
                }
                Fields::Named(fields) => {
                    // assert!(s.generics().len() == 0, "missing support for generics"); // TODO
                    // assert!(s.tag().is_some(), "missing support for tagging"); // TODO

                    // TODO: Error if any of the generics are not used or add `PhantomData` field?

                    let mut s = format!(
                        "{docs}{STANDARD_DERIVE}{}\npub struct {} {{",
                        r.custom_struct_attributes
                            .clone()
                            .map(|v| format!("\n{v}"))
                            .unwrap_or("".into()),
                        s.name()
                    );

                    for (k, field) in fields.fields() {
                        // TODO: Documentation, deprecated, etc.

                        if !field.docs().is_empty() {
                            s.push_str("\n    /// ");
                            s.push_str(&field.docs().replace("\n", "\n    /// "));
                        }

                        let mut key = k.to_snake_case();
                        // TODO: Handle any reserved keywords
                        if key == "type" || key == "where" {
                            key = format!("r#{key}");
                        }
                        if &*key != k {
                            s.push_str(&format!("\n    #[serde(rename = \"{k}\")]"));
                        }

                        if field.optional() || matches!(field.ty(), Some(DataType::Nullable(_))) {
                            s.push_str(
                                "\n    #[serde(default, skip_serializing_if = \"Option::is_none\"",
                            );
                            if let Some(attr) = &r.custom_field_attributes {
                                s.push_str(&", ");
                                s.push_str(&attr);
                            }
                            s.push_str(")]");
                        }

                        // TODO: Don't `unwrap` here
                        s.push_str(&format!(
                            "\n    pub {}: {},",
                            key,
                            datatype(r, field.ty().unwrap(), types)?
                        ));
                    }

                    s.push_str("\n}");
                    s
                }
                Fields::Unnamed(_) => todo!(),
            }
        }
        DataType::Enum(e) => {
            let mut docs = e
                .sid()
                .map(|sid| types.get(sid).unwrap().docs().clone())
                .unwrap_or_default()
                .replace("\n", "\n/// ");
            if !docs.is_empty() {
                docs = format!("/// {}\n", docs);
            }

            let repr = match e.repr() {
                EnumRepr::Untagged => format!("#[serde(untagged)]\n"),
                EnumRepr::External => format!(""),
                EnumRepr::Internal { tag } => format!("#[serde(tag = \"{tag}\")]\n"),
                EnumRepr::Adjacent { tag, content } => {
                    format!("#[serde(tag = \"{tag}\", content = \"{content}\")]\n")
                }
            };

            // TODO
            // let serde_repr = e.variants().iter().all(|(_, v)| {
            //     if let Fields::Unnamed(fields) = v.fields() {
            //         use LiteralType::*;
            //         return fields.fields.len() == 1
            //             && matches!(
            //                 fields.fields[0].ty(),
            //                 Some(DataType::Literal(
            //                     i8(..)
            //                         | i16(..)
            //                         | i32(..)
            //                         | u8(..)
            //                         | u16(..)
            //                         | u32(..)
            //                         | f32(..)
            //                         | f64(..)
            //                 ))
            //             );
            //     };

            //     false
            // });

            // if serde_repr {
            //     todo!();
            // }

            let mut s = format!(
                "{docs}{STANDARD_DERIVE}{}\n{repr}pub enum {} {{ \n",
                r.custom_enum_attributes
                    .clone()
                    .map(|v| format!("\n{v}"))
                    .unwrap_or("".into()),
                e.name()
            );
            for (name, variant) in e.variants().iter() {
                let mut docs2 = variant.docs().replace("\n", "\n\t/// ");
                if !docs2.is_empty() {
                    docs2 = format!("\t/// {}\n", docs2);
                }

                let mut key = name.to_snake_case();
                // TODO: Handle any reserved keywords
                if key == "type" || key == "where" {
                    key = format!("r#{key}");
                }
                if &*key != name {
                    s.push_str(&format!("    #[serde(rename = \"{name}\")]\n"));
                }

                let variant = match variant.fields() {
                    Fields::Unit => format!("\t{},\n", name.to_class_case()),
                    Fields::Unnamed(fields) => {
                        let fields = fields
                            .fields()
                            .iter()
                            .filter_map(|field| {
                                let Some(ty) = field.ty() else {
                                    return None;
                                };

                                Some(datatype(r, ty, types).unwrap()) // TODO: Error handling
                            })
                            .collect::<Vec<String>>();

                        format!("\t{}({}),\n", name.to_class_case(), fields.join("\n"))
                    }
                    Fields::Named(fields) => {
                        let fields = fields
                            .fields()
                            .iter()
                            .filter_map(|(_, field)| {
                                let Some(ty) = field.ty() else {
                                    return None;
                                };

                                Some(datatype(r, ty, types).unwrap()) // TODO: Error handling
                            })
                            .collect::<Vec<String>>();

                        format!("\t{}({}),\n", name.to_class_case(), fields.join("\n"))
                    }
                };
                s.push_str(&docs2);
                s.push_str(&variant);
            }
            s.push_str("}");
            s
        }
        DataType::Reference(reference) => {
            let definition = types.get(reference.sid()).unwrap(); // TODO: Error handling

            if reference.inline() {
                todo!();
            }

            if reference.generics().len() == 0 {
                definition.name().to_string()
            } else {
                let generics = reference
                    .generics()
                    .iter()
                    .map(|(_, t)| datatype(r, t, types))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                format!("{}<{generics}>", definition.name())
            }
        }
        DataType::Generic(t) => t.to_string(),
    })
}
