use crate::{DataType, DefOpts, Type};

/// A type that is unconstructable but is typed as `any` in TypeScript.
///
/// This can be use like the following:
/// ```rust
/// use serde::Serialize;
/// use specta::{Type, Any};
///
/// #[derive(Serialize, Type)]
/// pub struct Demo {
///     #[specta(type = Any)]
///     pub field: String,
/// }
/// ```
pub enum Any {}

impl Type for Any {
    fn inline(_: DefOpts, _: &[DataType]) -> DataType {
        DataType::Any
    }
}
