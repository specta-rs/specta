use std::borrow::Cow;

use super::CommentFormatterFn;

// Assert that the function signature matches the expected type.
const _: CommentFormatterFn = js_doc;

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
