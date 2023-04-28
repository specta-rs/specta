use std::future::Future;

use crate::{DataType, DefOpts, ExportError, Type};

#[derive(Debug, Clone)]
pub enum SpectaFunctionResultVariant {
    Value(DataType),
    Result(DataType, DataType),
}

/// Implemented by types that can be returned from a function annotated with
/// [`specta`](crate::specta).
pub trait SpectaFunctionResult<TMarker> {
    /// Gets the type of the result as a [`DataType`].
    fn to_datatype(opts: DefOpts) -> Result<SpectaFunctionResultVariant, ExportError>;
}

#[doc(hidden)]
pub enum SpectaFunctionResultType {}
impl<T: Type> SpectaFunctionResult<SpectaFunctionResultType> for T {
    fn to_datatype(opts: DefOpts) -> Result<SpectaFunctionResultVariant, ExportError> {
        T::reference(opts, &[]).map(SpectaFunctionResultVariant::Value)
    }
}

#[doc(hidden)]
pub struct SpectaFunctionResultResult {}
impl<T: Type, E: Type> SpectaFunctionResult<SpectaFunctionResultResult> for Result<T, E> {
    fn to_datatype(opts: DefOpts) -> Result<SpectaFunctionResultVariant, ExportError> {
        Ok(SpectaFunctionResultVariant::Result(
            T::reference(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                &[],
            )?,
            E::reference(
                DefOpts {
                    parent_inline: opts.parent_inline,
                    type_map: opts.type_map,
                },
                &[],
            )?,
        ))
    }
}

#[doc(hidden)]
pub struct SpectaFunctionResultFuture<TMarker>(TMarker);
impl<TMarker, T: SpectaFunctionResult<TMarker>, TFut: Future<Output = T>>
    SpectaFunctionResult<SpectaFunctionResultFuture<TMarker>> for TFut
{
    fn to_datatype(opts: DefOpts) -> Result<SpectaFunctionResultVariant, ExportError> {
        T::to_datatype(opts)
    }
}
