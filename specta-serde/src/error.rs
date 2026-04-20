use std::{borrow::Cow, error, fmt};

/// Error type for serde transformation and validation failures.
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    InvalidUsageOfSkip {
        path: String,
        reason: Cow<'static, str>,
    },
    InvalidInternallyTaggedEnum {
        path: String,
        variant: String,
        reason: Cow<'static, str>,
    },
    UnresolvedGenericReference {
        path: String,
        generic: String,
    },
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
    IncompatibleConversion {
        context: Cow<'static, str>,
        name: String,
        serialize: Option<String>,
        deserialize: Option<String>,
    },
    InvalidConversionUsage {
        path: String,
        reason: Cow<'static, str>,
    },
    UnsupportedSerdeCustomCodec {
        path: String,
        attribute: Cow<'static, str>,
    },
    InvalidPhasedTypeUsage {
        path: String,
        reason: Cow<'static, str>,
    },
    InvalidRenameRule {
        attribute: Cow<'static, str>,
        value: String,
    },
}

impl Error {
    pub(crate) fn invalid_usage_of_skip(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidUsageOfSkip {
                path: path.into(),
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn invalid_internally_tagged_enum(
        path: impl Into<String>,
        variant: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidInternallyTaggedEnum {
                path: path.into(),
                variant: variant.into(),
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn unresolved_generic_reference(
        path: impl Into<String>,
        generic: impl Into<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::UnresolvedGenericReference {
                path: path.into(),
                generic: generic.into(),
            },
        }
    }

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

    pub(crate) fn incompatible_conversion(
        context: impl Into<Cow<'static, str>>,
        name: impl Into<String>,
        serialize: Option<String>,
        deserialize: Option<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::IncompatibleConversion {
                context: context.into(),
                name: name.into(),
                serialize,
                deserialize,
            },
        }
    }

    pub(crate) fn invalid_conversion_usage(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidConversionUsage {
                path: path.into(),
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn unsupported_serde_custom_codec(
        path: impl Into<String>,
        attribute: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::UnsupportedSerdeCustomCodec {
                path: path.into(),
                attribute: attribute.into(),
            },
        }
    }

    pub(crate) fn invalid_phased_type_usage(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidPhasedTypeUsage {
                path: path.into(),
                reason: reason.into(),
            },
        }
    }

    pub(crate) fn invalid_rename_rule(
        attribute: impl Into<Cow<'static, str>>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidRenameRule {
                attribute: attribute.into(),
                value: value.into(),
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidUsageOfSkip { path, reason } => {
                write!(f, "Invalid usage of #[serde(skip)] at '{path}': {reason}")
            }
            ErrorKind::InvalidInternallyTaggedEnum {
                path,
                variant,
                reason,
            } => write!(
                f,
                "Invalid internally tagged enum at '{path}', variant '{variant}': {reason}"
            ),
            ErrorKind::UnresolvedGenericReference { path, generic } => write!(
                f,
                "Unresolved generic reference '{generic}' while validating '{path}'"
            ),
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
            ErrorKind::IncompatibleConversion {
                context,
                name,
                serialize,
                deserialize,
            } => write!(
                f,
                "Incompatible {context} for '{name}' in unified mode: serialize={serialize:?}, deserialize={deserialize:?}. Use format_phases for asymmetric serde conversions"
            ),
            ErrorKind::InvalidConversionUsage { path, reason } => {
                write!(
                    f,
                    "Invalid usage of serde conversion attributes at '{path}': {reason}"
                )
            }
            ErrorKind::UnsupportedSerdeCustomCodec { path, attribute } => write!(
                f,
                "Unsupported serde attribute at '{path}': #[serde({attribute})] changes the wire type. Add #[specta(type = ...)] (or #[specta(type = specta_serde::Phased<Serialize, Deserialize>)])"
            ),
            ErrorKind::InvalidPhasedTypeUsage { path, reason } => {
                write!(f, "Invalid phased type usage at '{path}': {reason}")
            }
            ErrorKind::InvalidRenameRule { attribute, value } => {
                write!(f, "Invalid serde rename rule for '{attribute}': {value:?}")
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl error::Error for Error {}
