use std::borrow::Cow;

use specta::{Types, datatype::DataType};

use crate::exporter::FormatError;

/// Map a full [`Types`] graph using unified serde handling.
pub fn map_types(types: &Types) -> Result<Cow<'_, Types>, FormatError> {
    Ok(Cow::Owned(specta_serde::apply(types.clone())?))
}

/// Map a full [`Types`] graph using split-phase serde handling.
pub fn map_phases_types(types: &Types) -> Result<Cow<'_, Types>, FormatError> {
    Ok(Cow::Owned(specta_serde::apply_phases(types.clone())?))
}

/// Map a single [`DataType`] using unified serde handling.
pub fn map_datatype<'a>(_: &'a Types, dt: &'a DataType) -> Result<Cow<'a, DataType>, FormatError> {
    Ok(Cow::Borrowed(dt))
}

/// Map a single [`DataType`] using split-phase serde handling.
pub fn map_phases_datatype<'a>(
    types: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, FormatError> {
    Ok(Cow::Owned(specta_serde::select_phase_datatype(
        dt,
        types,
        specta_serde::Phase::Serialize,
    )))
}

/// Formatter helpers for `specta-serde` in unified serde mode.
pub fn format() -> (
    impl for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError>,
    impl for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>,
) {
    (map_types, map_datatype)
}

/// Formatter helpers for `specta-serde` in split-phase serde mode.
///
/// The type graph is expanded to include both `*_Serialize` and `*_Deserialize`
/// named types. Inline primitive rendering selects the serialize-facing shape.
pub fn format_phases() -> (
    impl for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError>,
    impl for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>,
) {
    (map_phases_types, map_phases_datatype)
}
