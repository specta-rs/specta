use std::borrow::Cow;

use specta::datatype::Attributes;

use crate::{Error, parser::SerdeContainerAttrs};

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

impl EnumRepr {
    pub(crate) fn from_attrs(attrs: &Attributes) -> Result<Self, Error> {
        let Some(container_attrs) = SerdeContainerAttrs::from_attributes(attrs)? else {
            return Ok(Self::External);
        };

        if container_attrs.untagged {
            return Ok(Self::Untagged);
        }

        match (
            container_attrs.tag.as_deref(),
            container_attrs.content.as_deref(),
        ) {
            (Some(tag), Some(content)) => Ok(Self::Adjacent {
                tag: Cow::Owned(tag.to_string()),
                content: Cow::Owned(content.to_string()),
            }),
            (Some(tag), None) => Ok(Self::Internal {
                tag: Cow::Owned(tag.to_string()),
            }),
            (None, Some(_)) => Err(Error::invalid_enum_representation(
                "`content` is set without `tag`",
            )),
            (None, None) => Ok(Self::External),
        }
    }
}
