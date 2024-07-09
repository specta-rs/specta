use crate::TypeMap;

/// TODO
///
/// Warning: The structure of this trait is not final and may change in the future.
// TODO: Finish this
pub trait Language {
    /// TODO
    type Error: std::error::Error + From<std::io::Error>;

    /// TODO
    fn export(&self, type_map: TypeMap) -> Result<String, Self::Error>;
}
