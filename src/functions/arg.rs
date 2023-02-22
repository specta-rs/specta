use crate::{DataType, DefOpts, ExportError, Type};

/// Implemented by types that can be used as an argument in a function annotated with
/// [`specta`](crate::specta).
pub trait SpectaFunctionArg<TMarker> {
    /// Gets the type of an argument as a [`DataType`].
    ///
    /// Some argument types should be ignored (eg Tauri command State),
    /// so the value is optional.
    fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError>;
}

#[doc(hidden)]
pub enum SpectaFunctionArgDeserializeMarker {}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + Type> SpectaFunctionArg<SpectaFunctionArgDeserializeMarker>
    for T
{
    fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError> {
        T::reference(opts, &[]).map(Some)
    }
}
