use crate::typescript::{CommentFormatterArgs, CommentFormatterFn};

use super::js_doc;

// Assert that the function signature matches the expected type.
const _: CommentFormatterFn = js_doc;

/// Converts Typescript comments into JSDoc comments.
pub fn js_doc(arg: CommentFormatterArgs) -> String {
    js_doc_builder(arg).build()
}

pub(crate) fn js_doc_builder(arg: CommentFormatterArgs) -> js_doc::Builder {
    let mut builder = js_doc::Builder::default();

    if !arg.docs.is_empty() {
        builder.extend(arg.docs.split('\n'));
    }

    if let Some(deprecated) = arg.deprecated {
        builder.push_deprecated(deprecated);
    }

    builder
}
