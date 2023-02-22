use tauri::{AppHandle, Runtime, State, Window};

use crate::{functions::SpectaFunctionArg, DataType, DefOpts, ExportError};

#[doc(hidden)]
pub enum TauriMarker {}

impl<R: Runtime> SpectaFunctionArg<TauriMarker> for Window<R> {
    fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
        Ok(None)
    }
}

impl<'r, T: Send + Sync + 'static> SpectaFunctionArg<TauriMarker> for State<'r, T> {
    fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
        Ok(None)
    }
}

impl<R: Runtime> SpectaFunctionArg<TauriMarker> for AppHandle<R> {
    fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
        Ok(None)
    }
}
