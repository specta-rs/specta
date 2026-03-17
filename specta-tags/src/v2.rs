use std::borrow::Cow;

use specta::{TypeCollection, datatype::DataType};

// TODO: Allow configuring custom named types via NDT name and module path using config params.
// TODO: Tagging-style system for `rspc` w/ runtime

// TODO: Core changes to `BigInt` handling with Typescript exporter???
// TODO: How to handle `UInt8Array`?
// TODO: Could we support `Custom` tags? If the runtime is fixed thats hard.

// TODO: Renaming `Tags` struct???
// TODO: Documentations -> Explain how input types *just work* (double check that though)

/// A tag is used to identify the transformation required for a given data type.
pub enum Tag {
    /// [BigInt](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt)
    BigInt,
    /// [Uint8Array](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array)
    Uint8Array,
    /// [Date](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date)
    Date,
    /// A custom tag.
    ///
    /// TODO: Document this
    Custom(Box<dyn Fn(&str) -> Cow<'static, str> + Send + Sync>),
}

/// TODO
#[derive(Clone, Debug)] // TODO: Maybe other stuff, `Default`???
pub struct Tags {
    tagged: Vec<()>,
}

impl Tags {
    /// TODO
    pub fn analyze(dt: DataType, types: &TypeCollection) -> Self {
        // Scan all `DataType` references, etc. and collect tags and their object location for `Self::map` to use.
        //
        // You should match on `NamedDataType`'s name and module path to determine known named types.

        todo!();
    }

    /// TODO
    ///
    /// This should produce something like
    pub fn map<'a>(&self, t: &'a str) -> Cow<'a, str> {
        // If `t` is a struct and has tags we wanna decompose it to something like:
        // `{ ...t, field_with_override: { nested_override: new Date(t.field_with_override.nested_override) } }`
        //
        // If a nested tag doesn't have tags we should just return it to avoid deconstructing it.
        //
        // Otherwise we need to traverse the structure to apply the tags inline.
        // This should just be plain JS code so should work in Typescript (via inference) and in JS.
        //
        // If it has no tags we can just return `t` as is.

        t.into()
    }
}
