mod context;
mod datatype;
mod error;
mod export_config;
mod formatter;
mod reserved_terms;

pub use context::*;
pub(crate) use datatype::*;
pub use error::*;
pub use export_config::*;
pub use formatter::*;

use crate::*;

use std::borrow::Cow;

use self::reserved_terms::RESERVED_TYPE_NAMES;

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, ExportError>;

pub type Output = Result<String>;

/// Allows you to configure how Specta's Typescript exporter will deal with BigInt types ([i64], [i128] etc).
///
/// WARNING: None of these settings affect how your data is actually ser/deserialized.
/// It's up to you to adjust your ser/deserialize settings.
#[derive(Debug, Clone, Default)]
pub enum BigIntExportBehavior {
    /// Export BigInt as a Typescript `string`
    ///
    /// Doing this is serde is [pretty simple](https://github.com/serde-rs/json/issues/329#issuecomment-305608405).
    String,
    /// Export BigInt as a Typescript `number`.
    ///
    /// WARNING: `JSON.parse` in JS will truncate your number resulting in data loss so ensure your deserializer supports large numbers.
    Number,
    /// Export BigInt as a Typescript `BigInt`.
    BigInt,
    /// Abort the export with an error.
    ///
    /// This is the default behavior because without integration from your serializer and deserializer we can't guarantee data loss won't occur.
    #[default]
    Fail,
    /// Same as `Self::Fail` but it allows a library to configure the message shown to the end user.
    #[doc(hidden)]
    FailWithReason(&'static str),
}

// Assert that the function signature matches the expected type.
const _: CommentFormatterFn = js_doc;

pub(crate) fn sanitise_type_name(ctx: ExportContext, loc: NamedLocation, ident: &str) -> Output {
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(ExportError::ForbiddenName(loc, ctx.export_path(), name));
    }

    Ok(ident.to_string())
}

/// Converts Typescript comments into JSDoc comments.
pub fn js_doc(comments: &[Cow<'static, str>]) -> String {
    if comments.is_empty() {
        return "".to_owned();
    }

    let mut result = "/**\n".to_owned();
    for comment in comments {
        let comment = comment.trim_start();
        result.push_str(&format!(" * {comment}\n"));
    }
    result.push_str(" */\n");
    result
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfig, typ: &DataType, type_map: &TypeMap) -> Output {
    // TODO: Duplicate type name detection?

    datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
        },
        typ,
        type_map,
        "null",
    )
}
