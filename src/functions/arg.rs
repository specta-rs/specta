use crate::{DataType, DefOpts, ExportError, Type};

/// is a trait which is implemented by all types which can be used as a command argument.
pub trait SpectaFunctionArg<TMarker> {
    /// convert argument of the Rust function into a DataType
    fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError>;
}

#[doc(hidden)]
pub enum SpectaFunctionArgDeserializeMarker {}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + Type> SpectaFunctionArg<SpectaFunctionArgDeserializeMarker>
    for T
{
    fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError> {
        T::reference(opts, &[]).map(Some)
    }
}
