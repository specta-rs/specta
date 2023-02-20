/// Allows you to configure how Specta's Typescript exporter will deal with BigInt types (i64 u64 i128 u128).
#[derive(Default)]
pub enum BigIntExportBehavior {
    /// Export BigInt as a Typescript `string`
    /// WARNING: Specta takes no responsibility that the Rust number is encoded as a string.
    /// Make sure you instruct serde <https://github.com/serde-rs/json/issues/329#issuecomment-305608405> or your other serializer of this.
    String,
    /// Export BigInt as a Typescript `number`.
    /// WARNING: `JSON.parse` in JS will truncate your number resulting in data loss so ensure your deserializer supports bigint types.
    Number,
    /// Export BigInt as a Typescript `BigInt`.
    /// WARNING: Specta takes no responsibility that the Rust number is decoded into this type on the frontend.
    /// Ensure you deserializer is able to do this.
    BigInt,
    /// Abort the export with an error
    /// This is the default behavior because without integration from your serializer and deserializer we can't guarantee data loss won't occur.
    #[default]
    Fail,
    /// Same as `Self::Fail` but it allows a library to configure the message shown to the end user.
    #[doc(hidden)]
    FailWithReason(&'static str),
}

/// The signature for a function responsible for exporting Typescript comments.
pub type CommentFormatterFn = fn(&'static [&'static str]) -> String;

/// Export the Typescript comments as JS Doc comments. This means all JS Doc attributes will work.
pub fn js_doc(comments: &'static [&'static str]) -> String {
    if comments.is_empty() {
        return "".to_owned();
    }

    let mut result = "/**\n".to_owned();
    for comment in comments {
        result.push_str(&format!(" * {comment}\n"));
    }
    result.push_str(" */\n");
    result
}

// Assert that the function signature matches the expected type.
const _: CommentFormatterFn = js_doc;
