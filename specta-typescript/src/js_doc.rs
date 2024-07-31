use std::borrow::Borrow;

use specta::{
    datatype::{DeprecatedType, GenericType},
    TypeMap,
};
use typescript::CommentFormatterArgs;

use super::*;

pub fn typedef_named_datatype(cfg: &Typescript, typ: &NamedDataType, type_map: &TypeMap) -> Output {
    typedef_named_datatype_inner(
        &ExportContext {
            cfg,
            path: vec![],
            // TODO: Should JS doc support per field or variant comments???
            is_export: false,
        },
        typ,
        type_map,
    )
}

fn typedef_named_datatype_inner(
    ctx: &ExportContext,
    typ: &NamedDataType,
    type_map: &TypeMap,
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
        type_map,
        &mut inline_ts,
    )?;

    let mut builder = super::comments::js_doc_builder(CommentFormatterArgs { docs, deprecated });

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
