use crate::{Types, datatype::DataType};

mod impls;
mod legacy_impls;
mod macros;

/// Provides runtime type information for a Rust type.
///
/// Exporters call this trait to build a [`DataType`] graph and collect any
/// referenced named types into [`Types`]. Prefer deriving this trait with
/// [`#[derive(Type)]`](derive@crate::Type); hand-written implementations must
/// preserve the same invariants as the derive macro.
///
/// # Invariants
///
/// Implementations should register every named dependency they reference in the
/// provided [`Types`] collection. Generic placeholders should only be emitted
/// inside the canonical [`NamedDataType`](crate::datatype::NamedDataType)
/// definition for the declaring type, not as arbitrary top-level results.
///
#[diagnostic::on_unimplemented(
    message = "the trait `specta::Type` is not implemented for `{Self}`",
    label = "`{Self}` must implement `Type`",
    note = "Depending on your use case, this can be fixed in multiple ways:
 - If your using an type defined in one of your own crates, ensure you have `#[derive(specta::Type)]` on it.
 - If your using a crate with official Specta support, enable the matching feature flag on the `specta` crate.
 - If your using an external crate without Specta support, you may need to wrap your type in a new-type wrapper.
"
)]
pub trait Type {
    /// Returns a [`DataType`] that represents `Self`.
    ///
    /// This may mutate `types` by registering `Self` and any named datatypes
    /// needed by the returned definition.
    fn definition(types: &mut Types) -> DataType;
}
