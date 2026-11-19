//! Elm identifier rules for field names.

use std::borrow::Cow;

use crate::{Error, elm::RESERVED_TYPE_NAMES};

/// Whether `name` is a valid Elm record field name.
fn is_elm_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !RESERVED_TYPE_NAMES.contains(&name)
}

/// Lower the first character of a Rust field name so it is a valid Elm field name.
pub(crate) fn sanitise_key(
    field_name: Cow<'static, str>,
    path: &str,
) -> Result<Cow<'static, str>, Error> {
    if is_elm_identifier(&field_name) {
        return Ok(field_name);
    }

    let mut chars = field_name.chars();
    if let Some(first) = chars.next()
        && first.is_ascii_uppercase()
    {
        let candidate = first.to_ascii_lowercase().to_string() + chars.as_str();
        if is_elm_identifier(&candidate) {
            return Ok(Cow::Owned(candidate));
        }
    }

    Err(Error::invalid_name(path, field_name))
}
