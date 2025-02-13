use std::borrow::Borrow;

use specta::{
    datatype::{DeprecatedType, FunctionResultVariant, GenericType, NamedDataType},
    TypeCollection,
};

use super::*;

// TODO: Merge this into main expoerter
pub(crate) fn js_doc_builder(docs: &str, deprecated: Option<&DeprecatedType>) -> Builder {
    let mut builder = js_doc::Builder::default();

    if !docs.is_empty() {
        builder.extend(docs.split('\n'));
    }

    if let Some(deprecated) = deprecated {
        builder.push_deprecated(deprecated);
    }

    builder
}

pub fn typedef_named_datatype(
    cfg: &Typescript,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    typedef_named_datatype_inner(
        &ExportContext {
            cfg,
            path: vec![],
            // TODO: Should JS doc support per field or variant comments???
            is_export: false,
        },
        typ,
        types,
    )
}

fn typedef_named_datatype_inner(
    ctx: &ExportContext,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    let name = typ.name();
    let docs = typ.docs();
    let deprecated = typ.deprecated();
    let item = &typ.inner;

    let ctx = ctx.with(PathItem::Type(name.clone()));

    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let mut inline_ts = String::new();
    datatype_inner(
        ctx.clone(),
        &FunctionResultVariant::Value(typ.inner.clone()),
        types,
        &mut inline_ts,
    )?;

    let mut builder = super::js_doc::js_doc_builder(docs, deprecated);

    item.generics()
        .into_iter()
        .flatten()
        .for_each(|generic| builder.push_generic(generic));

    builder.push_internal(["@typedef { ", &inline_ts, " } ", &name]);

    Ok(builder.build())
}

const START: &str = "/**\n";

pub struct Builder {
    value: String,
}

impl Builder {
    pub fn push(&mut self, line: &str) {
        self.push_internal([line.trim()]);
    }

    pub(crate) fn push_internal<'a>(&mut self, parts: impl IntoIterator<Item = &'a str>) {
        self.value.push_str(" * ");

        for part in parts.into_iter() {
            self.value.push_str(part);
        }

        self.value.push('\n');
    }

    pub fn push_deprecated(&mut self, typ: &DeprecatedType) {
        self.push_internal(
            ["@deprecated"].into_iter().chain(
                match typ {
                    DeprecatedType::DeprecatedWithSince {
                        note: message,
                        since,
                    } => Some((since.as_ref(), message)),
                    _ => None,
                }
                .map(|(since, message)| {
                    [" ", message.trim()].into_iter().chain(
                        since
                            .map(|since| [" since ", since.trim()])
                            .into_iter()
                            .flatten(),
                    )
                })
                .into_iter()
                .flatten(),
            ),
        );
    }

    pub fn push_generic(&mut self, generic: &GenericType) {
        self.push_internal(["@template ", generic.borrow()])
    }

    pub fn build(mut self) -> String {
        if self.value == START {
            return String::new();
        }

        self.value.push_str(" */\n");
        self.value
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            value: START.to_string(),
        }
    }
}

impl<T: AsRef<str>> Extend<T> for Builder {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item.as_ref());
        }
    }
}

// TODO: Bring all this back
// use std::borrow::Cow;

// use specta_typescript::BigIntExportBehavior;

// // TODO: Ensure this is up to our `Typescript` exporters standards.

// /// JSDoc language exporter.
// #[derive(Debug, Clone, Default)]
// pub struct JSDoc(pub specta_typescript::Typescript);

// impl From<specta_typescript::Typescript> for JSDoc {
//     fn from(ts: specta_typescript::Typescript) -> Self {
//         Self(ts)
//     }
// }

// impl JSDoc {
//     /// Construct a new JSDoc exporter with the default options configured.
//     pub fn new() -> Self {
//         Default::default()
//     }

//     /// Configure a header for the file.
//     ///
//     /// This is perfect for configuring lint ignore rules or other file-level comments.
//     pub fn header(mut self, header: impl Into<Cow<'static, str>>) -> Self {
//         self.0.header = header.into();
//         self
//     }

//     /// Configure the BigInt handling behaviour
//     pub fn bigint(mut self, bigint: BigIntExportBehavior) -> Self {
//         self.0.bigint = bigint;
//         self
//     }

//     // /// Configure a function which is responsible for styling the comments to be exported
//     // ///
//     // /// Implementations:
//     // ///  - [`js_doc`](specta_typescript::lang::ts::js_doc)
//     // ///
//     // /// Not calling this method will default to the [`js_doc`](specta_typescript::lang::ts::js_doc) exporter.
//     // /// `None` will disable comment exporting.
//     // /// `Some(exporter)` will enable comment exporting using the provided exporter.
//     // pub fn comment_style(mut self, exporter: CommentFormatterFn) -> Self {
//     //     self.0.comment_exporter = Some(exporter);
//     //     self
//     // }

//     // /// Configure a function which is responsible for formatting the result file or files
//     // ///
//     // ///
//     // /// Built-in implementations:
//     // ///  - [`prettier`](specta_typescript:formatter:::prettier)
//     // ///  - [`ESLint`](specta_typescript::formatter::eslint)
//     // ///  - [`Biome`](specta_typescript::formatter::biome)e
//     // pub fn formatter(mut self, formatter: FormatterFn) -> Self {
//     //     self.0.formatter = Some(formatter);
//     //     self
//     // }
// }

// // impl Language for JSDoc {
// //     type Error = specta_typescript::ExportError; // TODO: Custom error type

// //     // TODO: Make this properly export JSDoc
// //     fn export(&self, _types: &TypeCollection) -> Result<String, Self::Error> {
// //         todo!("Coming soon...");
// //         // let mut out = self.0.header.to_string();
// //         // if !self.0.remove_default_header {
// //         //     out += "// This file has been generated by Specta. DO NOT EDIT.\n\n";
// //         // }

// //         // if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&types).into_iter().next() {
// //         //     return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
// //         // }

// //         // for (_, ty) in types.iter() {
// //         //     is_valid_ty(&ty.inner, &types)?;

// //         //     out += &export_named_datatype(&self.0, ty, &types)?;
// //         //     out += "\n\n";
// //         // }

// //         // Ok(out)
// //     }

// //     fn format(&self, path: &Path) -> Result<(), Self::Error> {
// //         if let Some(formatter) = self.0.formatter {
// //             formatter(path)?;
// //         }
// //         Ok(())
// //     }
// // }
