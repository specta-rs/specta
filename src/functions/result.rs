use std::future::Future;

use crate::{DataType, DefOpts, ExportError, Type};

/// Implemented by types that can be returned from a function annotated with
/// [`specta`](crate::specta).
pub trait SpectaFunctionResult<TMarker> {
    /// Gets the type of the result as a [`DataType`].
    fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError>;
}

#[cfg(feature = "serde")]
#[doc(hidden)]
pub enum SpectaFunctionResultSerialize {}

#[cfg(feature = "serde")]
impl<T: serde::Serialize + Type> SpectaFunctionResult<SpectaFunctionResultSerialize> for T {
    fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError> {
        T::reference(opts, &[])
    }
}

#[doc(hidden)]
pub struct SpectaFunctionResultResult<TMarker>(TMarker);
impl<TMarker, T: SpectaFunctionResult<TMarker>, E>
    SpectaFunctionResult<SpectaFunctionResultResult<TMarker>> for Result<T, E>
{
    fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError> {
        T::to_datatype(opts)
    }
}

#[doc(hidden)]
pub struct SpectaFunctionResultFuture<TMarker>(TMarker);
impl<TMarker, T: SpectaFunctionResult<TMarker>, TFut: Future<Output = T>>
    SpectaFunctionResult<SpectaFunctionResultFuture<TMarker>> for TFut
{
    fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError> {
        T::to_datatype(opts)
    }
}
