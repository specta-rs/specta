//! This module contains functions that are public for the sole reason of the macros.
//!
//! They will not be documented and may go through breaking changes without a major version bump!
//!
//! DO NOT USE THEM! You have been warned!

#[cfg(feature = "function")]
pub use paste::paste;

/// Functions used to construct `crate::datatype` types (they have private fields so can't be constructed directly).
/// We intentionally keep their fields private so we can modify them without a major version bump.
/// As this module is `#[doc(hidden)]` we allowed to make breaking changes within a minor version as it's only used by the macros.
pub mod construct {
    use std::borrow::Cow;

    use crate::datatype::*;

    pub const fn field(
        optional: bool,
        deprecated: Option<DeprecatedType>,
        docs: Cow<'static, str>,
        inline: bool,
        attributes: Vec<RuntimeAttribute>,
        ty: Option<DataType>,
    ) -> Field {
        Field {
            optional,
            deprecated,
            docs,
            inline,
            attributes,
            ty,
        }
    }

    pub const fn fields_unnamed(fields: Vec<Field>, attributes: Vec<RuntimeAttribute>) -> Fields {
        Fields::Unnamed(UnnamedFields { fields, attributes })
    }

    pub const fn fields_named(
        fields: Vec<(Cow<'static, str>, Field)>,
        attributes: Vec<RuntimeAttribute>,
    ) -> Fields {
        Fields::Named(NamedFields { fields, attributes })
    }
}

#[cfg(feature = "function")]
mod functions {
    use std::borrow::Cow;

    use crate::{TypeCollection, datatype::DeprecatedType, datatype::Function, function::SpectaFn};

    #[doc(hidden)]
    /// A helper for exporting a command to a [`CommandDataType`].
    /// You shouldn't use this directly and instead should use [`fn_datatype!`](crate::fn_datatype).
    pub fn get_fn_datatype<TMarker, T: SpectaFn<TMarker>>(
        _: T,
        asyncness: bool,
        name: Cow<'static, str>,
        types: &mut TypeCollection,
        fields: &[Cow<'static, str>],
        docs: Cow<'static, str>,
        deprecated: Option<DeprecatedType>,
        no_return_type: bool,
    ) -> Function {
        T::to_datatype(
            asyncness,
            name,
            types,
            fields,
            docs,
            deprecated,
            no_return_type,
        )
    }
}
#[cfg(feature = "function")]
pub use functions::*;
