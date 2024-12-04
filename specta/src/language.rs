use std::path::Path;

use crate::TypeMap;

/// TODO
///
/// Warning: The structure of this trait is not final and may change in the future.
// TODO: Finish this
pub trait Language {
    /// TODO
    type Error: std::error::Error + From<std::io::Error>;

    /// TODO
    fn export(&self, type_map: &TypeMap) -> Result<String, Self::Error>;

    /// TODO
    // TODO: Not sure I love this here but it's for Tauri Specta.
    // TODO: Really a formatter can support multiple languages so it would be nice if we don't need `specta_typescript::eslint`, `specta_jsdoc::eslint`, etc.
    fn format(&self, path: &Path) -> Result<(), Self::Error>;
}

impl<T: Language> Language for &T {
    type Error = T::Error;

    fn export(&self, type_map: &TypeMap) -> Result<String, Self::Error> {
        (*self).export(type_map)
    }

    fn format(&self, path: &Path) -> Result<(), Self::Error> {
        (*self).format(path)
    }
}
