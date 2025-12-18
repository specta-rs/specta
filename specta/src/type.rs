use crate::{TypeCollection, datatype::DataType};

mod impls;
mod macros;
// TODO: We don't care much about these cause they are gonna go so this will do for now.
#[cfg(feature = "derive")]
mod legacy_impls;

/// Provides runtime type information that can be fed into a language exporter to generate a type definition for another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
///
/// This should be only implemented by the [`Type`](derive@crate::Type) macro.
/// TODO: Discuss how to avoid custom implementations.
#[diagnostic::on_unimplemented(
    message = "the trait `specta::Type` is not implemented for `{Self}`",
    label = "`{Self}` must implement `Type`",
    note = "Depending on your use case, this can be fixed in multiple ways:
 - If your using an type defined in one of your own crates, ensure you have `#[derive(specta::Type)]` on it.
 - If your using a crate with official Specta support enable the feature flag on the 'specta' crate, refer to the documentation at https://docs.rs/specta/latest/specta/#feature-flags.
 - If your using an external crate without Specta support, you may need to wrap your type in a new-type wrapper, refer to the examples at https://docs.rs/specta/latest/specta/trait.Type.html
"
)]
pub trait Type {
    /// returns a [`DataType`](crate::datatype::DataType) that represents the type.
    /// This will also register this and any dependent types into the [`TypeCollection`].
    fn definition(types: &mut TypeCollection) -> DataType;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
