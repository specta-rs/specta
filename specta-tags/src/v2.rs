use std::borrow::Cow;

use specta::datatype::DataType;

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
    Custom(Cow<'static, str>),
}

/// TODO
#[derive(Clone, Debug)] // TODO: Maybe other stuff???
pub struct Tags {
    todo: Vec<()>,
}

impl Tags {
    /// TODO
    pub fn analyze(dt: DataType) -> Self {
        todo!();
    }

    /// TODO
    pub fn map(&self, t: &str) -> String {
        todo!();
    }
}

// // TODO: Features
// pub mod typescript {
//     use super::*;

//     // pub const RUNTIME: &str = ""; // TODO

//

//     // pub fn todo() {}
// }
