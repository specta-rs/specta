mod private {
    use std::future::Future;

    use crate::{DataType, DefOpts, ExportError, Type};

    /// Implemented by types that can be returned from a function annotated with
    /// [`specta`](crate::specta).
    pub trait SpectaFunctionResult<TMarker> {
        /// Gets the type of the result as a [`DataType`].
        fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError>;
    }

    pub enum SpectaFunctionResultMarker {}
    impl<T: Type> SpectaFunctionResult<SpectaFunctionResultMarker> for T {
        fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError> {
            T::reference(opts, &[])
        }
    }

    pub enum SpectaFunctionResultFutureMarker {}
    impl<F> SpectaFunctionResult<SpectaFunctionResultFutureMarker> for F
    where
        F: Future,
        F::Output: Type,
    {
        fn to_datatype(opts: DefOpts) -> Result<DataType, ExportError> {
            F::Output::reference(opts, &[])
        }
    }
}

pub(crate) use private::SpectaFunctionResult;
