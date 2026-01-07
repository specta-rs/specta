/// Controls how JSON schemas are organized in output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Single file with all types in $defs section (default)
    /// All type definitions are placed in a single JSON file under the
    /// definitions/$defs key.
    SingleFile,

    /// Separate .json file per type, organized by module path
    /// Each type gets its own file like: `my_module/MyType.schema.json`
    Files,
}

impl Default for Layout {
    fn default() -> Self {
        Self::SingleFile
    }
}
