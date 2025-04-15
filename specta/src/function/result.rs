use std::future::Future;

use crate::{datatype::FunctionReturnType, Type, TypeCollection};

/// Implemented by types that can be returned from a function annotated with
/// [`specta`](crate::specta).
pub trait FunctionResult<TMarker> {
    /// Gets the type of the result as a [`DataType`].
    fn to_datatype(types: &mut TypeCollection) -> FunctionReturnType;
}

#[doc(hidden)]
pub enum FunctionValueMarker {}
impl<T: Type> FunctionResult<FunctionValueMarker> for T {
    fn to_datatype(types: &mut TypeCollection) -> FunctionReturnType {
        FunctionReturnType::Value(T::definition(types))
    }
}

#[doc(hidden)]
pub enum FunctionResultMarker {}
impl<T: Type, E: Type> FunctionResult<FunctionResultMarker> for Result<T, E> {
    fn to_datatype(types: &mut TypeCollection) -> FunctionReturnType {
        FunctionReturnType::Result(T::definition(types), E::definition(types))
    }
}

#[doc(hidden)]
pub enum FunctionFutureMarker {}
impl<F> FunctionResult<FunctionFutureMarker> for F
where
    F: Future,
    F::Output: Type,
{
    fn to_datatype(types: &mut TypeCollection) -> FunctionReturnType {
        FunctionReturnType::Value(F::Output::definition(types))
    }
}

#[doc(hidden)]
pub enum FunctionResultFutureMarker {}
impl<F, T, E> FunctionResult<FunctionResultFutureMarker> for F
where
    F: Future<Output = Result<T, E>>,
    T: Type,
    E: Type,
{
    fn to_datatype(types: &mut TypeCollection) -> FunctionReturnType {
        FunctionReturnType::Result(T::definition(types), E::definition(types))
    }
}
