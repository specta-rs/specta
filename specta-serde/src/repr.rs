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
    /// String enum representation for unit-only enums with serde rename_all
    #[allow(dead_code)]
    String {
        /// Optional rename strategy applied to variant names.
        rename_all: Option<Cow<'static, str>>,
    },
}

impl EnumRepr {
    /// Check if this is a string enum representation
    #[allow(dead_code)]
    pub fn is_string(&self) -> bool {
        matches!(self, EnumRepr::String { .. })
    }

    /// Get the rename_all inflection for string enums
    #[allow(dead_code)]
    pub fn rename_all(&self) -> Option<&str> {
        match self {
            EnumRepr::String { rename_all } => rename_all.as_deref(),
            _ => None,
        }
    }
}
