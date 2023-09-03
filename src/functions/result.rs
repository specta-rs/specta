mod private {
    use std::future::Future;

    use crate::{DataType, DefOpts, Type};

    /// Implemented by types that can be returned from a function annotated with
    /// [`specta`](crate::specta).
    pub trait SpectaFunctionResult<TMarker> {
        /// Gets the type of the result as a [`DataType`].
        fn to_datatype(opts: DefOpts) -> DataType;
    }

    pub enum SpectaFunctionResultMarker {}
    impl<T: Type> SpectaFunctionResult<SpectaFunctionResultMarker> for T {
        fn to_datatype(opts: DefOpts) -> DataType {
            T::reference(opts, &[]).inner
        }
    }

    pub enum SpectaFunctionResultFutureMarker {}
    impl<F> SpectaFunctionResult<SpectaFunctionResultFutureMarker> for F
    where
        F: Future,
        F::Output: Type,
    {
        fn to_datatype(opts: DefOpts) -> DataType {
            F::Output::reference(opts, &[]).inner
        }
    }
}

pub(crate) use private::SpectaFunctionResult;
