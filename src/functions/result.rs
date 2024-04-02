mod private {
    use std::future::Future;

    use crate::{DataType, Type, TypeMap};

    /// Implemented by types that can be returned from a function annotated with
    /// [`specta`](crate::specta).
    pub trait FunctionResult<TMarker> {
        /// Gets the type of the result as a [`DataType`].
        fn to_datatype(type_map: &mut TypeMap) -> DataType;
    }

    pub enum FunctionResultMarker {}
    impl<T: Type> FunctionResult<FunctionResultMarker> for T {
        fn to_datatype(type_map: &mut TypeMap) -> DataType {
            T::reference(type_map, &[]).inner
        }
    }

    pub enum FunctionResultFutureMarker {}
    impl<F> FunctionResult<FunctionResultFutureMarker> for F
    where
        F: Future,
        F::Output: Type,
    {
        fn to_datatype(type_map: &mut TypeMap) -> DataType {
            F::Output::reference(type_map, &[]).inner
        }
    }
}

pub(crate) use private::FunctionResult;
