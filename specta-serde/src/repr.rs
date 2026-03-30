use std::borrow::Cow;

/// Serde representation of an enum.
/// Refer to the [Serde documentation](https://serde.rs/enum-representations.html) for more information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumRepr {
    /// Untagged enum representation.
    Untagged,
    /// Externally tagged enum representation.
    External,
    /// Internally tagged enum representation.
    Internal {
        /// Field name used as the tag discriminator.
        tag: Cow<'static, str>,
    },
    /// Adjacently tagged enum representation.
    Adjacent {
        /// Field name used as the tag discriminator.
        tag: Cow<'static, str>,
        /// Field name used to hold the variant content.
        content: Cow<'static, str>,
    },
}
