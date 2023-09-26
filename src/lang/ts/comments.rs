use std::{borrow::Cow, iter};

use crate::DeprecatedType;

use super::{CommentFormatterArgs, CommentFormatterFn};

// Assert that the function signature matches the expected type.
const _: CommentFormatterFn = js_doc;

/// Converts Typescript comments into JSDoc comments.
pub fn js_doc(arg: CommentFormatterArgs) -> String {
    js_doc_internal(arg.docs, arg.deprecated, iter::empty())
}

pub(crate) fn js_doc_internal(
    docs: &Cow<'static, str>,
    deprecated: Option<&DeprecatedType>,
    extra_lines: impl Iterator<Item = String>,
) -> String {
    if docs.is_empty() && deprecated.is_none() {
        return "".into();
    }

    let mut comment = String::with_capacity(docs.len());
    comment.push_str("/**\n");
    if !docs.is_empty() {
        for line in docs.split('\n') {
            comment.push_str(" * ");
            comment.push_str(line.trim());
            comment.push('\n');
        }
    }

    if let Some(deprecated) = deprecated {
        comment.push_str(" * @deprecated");
        if let DeprecatedType::DeprecatedWithSince {
            since,
            note: message,
        } = deprecated
        {
            comment.push_str(" ");
            comment.push_str(message);
            if let Some(since) = since {
                comment.push_str(" since ");
                comment.push_str(since);
            }
        }
        comment.push('\n');
    }

    comment.extend(extra_lines);
    comment.push_str(" */\n");

    comment
}
