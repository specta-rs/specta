use std::future::Future;

use crate::{Type, Types, datatype::DataType};

/// Implemented by types that can be returned from a function annotated with
/// [`specta`](crate::specta).
pub trait FunctionResult<TMarker> {
    /// Gets the function return type as a [`DataType`].
    fn to_datatype(types: &mut Types) -> DataType;
}

#[doc(hidden)]
pub enum FunctionValueMarker {}
impl<T: Type> FunctionResult<FunctionValueMarker> for T {
    fn to_datatype(types: &mut Types) -> DataType {
        T::definition(types)
    }
}

#[doc(hidden)]
pub enum FunctionFutureMarker {}
impl<F> FunctionResult<FunctionFutureMarker> for F
where
    F: Future,
    F::Output: Type,
{
    fn to_datatype(types: &mut Types) -> DataType {
        F::Output::definition(types)
    }
}
