use std::borrow::Cow;

use specta::{Types, datatype::DataType};

use crate::exporter::FormatError;

fn identity_datatype<'a>(_: &'a Types, dt: &'a DataType) -> Result<Cow<'a, DataType>, FormatError> {
    Ok(Cow::Borrowed(dt))
}

fn format_types(types: &Types) -> Result<Cow<'_, Types>, FormatError> {
    Ok(Cow::Owned(specta_serde::apply(types.clone())?))
}

fn format_phases_types(types: &Types) -> Result<Cow<'_, Types>, FormatError> {
    Ok(Cow::Owned(specta_serde::apply_phases(types.clone())?))
}

fn format_phases_datatype<'a>(
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
    (format_types, identity_datatype)
}

/// Formatter helpers for `specta-serde` in split-phase serde mode.
///
/// The type graph is expanded to include both `*_Serialize` and `*_Deserialize`
/// named types. Inline primitive rendering selects the serialize-facing shape.
pub fn format_phases() -> (
    impl for<'a> Fn(&'a Types) -> Result<Cow<'a, Types>, FormatError>,
    impl for<'a> Fn(&'a Types, &'a DataType) -> Result<Cow<'a, DataType>, FormatError>,
) {
    (format_phases_types, format_phases_datatype)
}
