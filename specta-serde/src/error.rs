use std::{borrow::Cow, error, fmt};

/// Result type for `specta-serde` operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for serde transformation and validation failures.
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    InvalidEnumRepresentation {
        reason: Cow<'static, str>,
    },
    InvalidExternalTaggedVariant {
        variant: String,
    },
    InvalidAdjacentTaggedVariant {
        variant: String,
    },
    InvalidInternallyTaggedVariant {
        variant: String,
        reason: Cow<'static, str>,
    },
    IncompatibleRename {
        context: Cow<'static, str>,
        name: String,
        serialize: Option<String>,
        deserialize: Option<String>,
    },
}

impl Error {
    pub(crate) fn invalid_enum_representation(reason: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: ErrorKind::InvalidEnumRepresentation {
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn invalid_external_tagged_variant(variant: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidExternalTaggedVariant {
                variant: variant.into(),
            },
        }
    }

    pub(crate) fn invalid_adjacent_tagged_variant(variant: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidAdjacentTaggedVariant {
                variant: variant.into(),
            },
        }
    }

    pub(crate) fn invalid_internally_tagged_variant(
        variant: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidInternallyTaggedVariant {
                variant: variant.into(),
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn incompatible_rename(
        context: impl Into<Cow<'static, str>>,
        name: impl Into<String>,
        serialize: Option<String>,
        deserialize: Option<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::IncompatibleRename {
                context: context.into(),
                name: name.into(),
                serialize,
                deserialize,
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidEnumRepresentation { reason } => {
                write!(f, "Invalid serde enum representation: {reason}")
            }
            ErrorKind::InvalidExternalTaggedVariant { variant } => write!(
                f,
                "Invalid externally tagged enum variant '{variant}': variant payload is fully skipped"
            ),
            ErrorKind::InvalidAdjacentTaggedVariant { variant } => write!(
                f,
                "Invalid adjacently tagged enum variant '{variant}': variant payload is fully skipped"
            ),
            ErrorKind::InvalidInternallyTaggedVariant { variant, reason } => write!(
                f,
                "Invalid internally tagged enum variant '{variant}': {reason}"
            ),
            ErrorKind::IncompatibleRename {
                context,
                name,
                serialize,
                deserialize,
            } => write!(
                f,
                "Incompatible {context} for '{name}' in unified mode: serialize={serialize:?}, deserialize={deserialize:?}"
            ),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl error::Error for Error {}
