use crate::{Type, TypeCollection, datatype::DataType};

/// Implemented by types that can be used as an argument in a function annotated with
/// [`specta`](crate::specta).
pub trait FunctionArg {
    /// Gets the type of an argument as a [`DataType`].
    ///
    /// Some argument types should be ignored (eg. when doing dependency injection),
    /// so the value is optional.
    fn to_datatype(types: &mut TypeCollection) -> Option<DataType>;
}

impl<T: Type> FunctionArg for T {
    fn to_datatype(types: &mut TypeCollection) -> Option<DataType> {
        Some(T::definition(types))
    }
}
