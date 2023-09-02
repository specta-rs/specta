mod private {
    use crate::{DataType, DefOpts, ExportError, Type};

    /// Implemented by types that can be used as an argument in a function annotated with
    /// [`specta`](crate::specta).
    pub trait SpectaFunctionArg<TMarker> {
        /// Gets the type of an argument as a [`DataType`].
        ///
        /// Some argument types should be ignored (eg Tauri command State),
        /// so the value is optional.
        fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError>;
    }

    pub enum FunctionArgMarker {}

    impl<T: Type> SpectaFunctionArg<FunctionArgMarker> for T {
        fn to_datatype(opts: DefOpts) -> Result<Option<DataType>, ExportError> {
            T::reference(opts, &[]).map(|r| Some(r.inner))
        }
    }

    #[cfg(feature = "tauri")]
    const _: () = {
        pub enum FunctionArgTauriMarker {}

        impl<R: tauri::Runtime> SpectaFunctionArg<FunctionArgTauriMarker> for tauri::Window<R> {
            fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
                Ok(None)
            }
        }

        impl<'r, T: Send + Sync + 'static> SpectaFunctionArg<FunctionArgTauriMarker>
            for tauri::State<'r, T>
        {
            fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
                Ok(None)
            }
        }

        impl<R: tauri::Runtime> SpectaFunctionArg<FunctionArgTauriMarker> for tauri::AppHandle<R> {
            fn to_datatype(_: DefOpts) -> Result<Option<DataType>, ExportError> {
                Ok(None)
            }
        }
    };
}

pub(crate) use private::SpectaFunctionArg;
