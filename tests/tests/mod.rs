//! We register a single entrypoint so all tests are compiled into a single binary.
#![allow(unused_parens, unused_variables, dead_code, unused_mut)]

use std::borrow::Cow;

use specta::{Format, Types, datatype::DataType};

macro_rules! register {
    ($types:expr, $dts:expr; $($ty:ty),* $(,)?) => {{
        $(
            {
                let ty = <$ty as specta::Type>::definition(&mut $types);
                $dts.push((stringify!($ty), ty));
            }
        )*
    }};
}

mod bound;
mod errors;
mod functions;
mod jsdoc;
mod layouts;
mod legacy_impls;
mod references;
mod serde_conversions;
mod serde_identifiers;
mod serde_other;
mod swift;
mod types;
mod typescript;
mod utils;
mod zod;

pub use types::{types, types_phased};
pub use utils::fs_to_string;

fn raw_map_types(types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
    Ok(Cow::Borrowed(types))
}

fn raw_map_datatype<'a>(
    _types: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(Cow::Borrowed(dt))
}

#[allow(non_upper_case_globals)]
pub const raw_format: Format = Format::new(raw_map_types, raw_map_datatype);

#[test]
fn compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro/compile_error.rs");
}
