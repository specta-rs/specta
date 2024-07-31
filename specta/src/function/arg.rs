use crate::{datatype::DataType, Type, TypeMap};

/// Implemented by types that can be used as an argument in a function annotated with
/// [`specta`](crate::specta).
pub trait FunctionArg {
    /// Gets the type of an argument as a [`DataType`].
    ///
    /// Some argument types should be ignored (eg. when doing dependency injection),
    /// so the value is optional.
    fn to_datatype(type_map: &mut TypeMap) -> Option<DataType>;
}

impl<T: Type> FunctionArg for T {
    fn to_datatype(type_map: &mut TypeMap) -> Option<DataType> {
        Some(T::reference(type_map, &[]).inner)
    }
}
