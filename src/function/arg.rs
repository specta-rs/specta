use crate::{DataType, DefOpts, Type};

/// is a trait which is implemented by all types which can be used as a command argument.
pub trait SpectaFunctionArg<TMarker> {
    /// convert argument of the Rust function into a DataType
    fn to_datatype(opts: DefOpts) -> Option<DataType>;
}

#[doc(hidden)]
pub enum SpectaFunctionArgMarker {}

impl<'de, T: Type> SpectaFunctionArg<SpectaFunctionArgMarker> for T {
    fn to_datatype(opts: DefOpts) -> Option<DataType> {
        Some(T::reference(opts, &[]))
    }
}
