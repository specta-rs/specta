//! We register a single entrypoint so all tests are compiled into a single binary.
#![allow(unused_parens, unused_variables, dead_code, unused_mut)]

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
mod jsonschema;
mod layouts;
mod legacy_impls;
mod maybe_undefined;
mod references;
mod semantic;
mod serde_conversions;
mod serde_enum_rewrite;
mod serde_flatten_option;
mod serde_identifiers;
mod serde_other;
mod serde_unknown_attrs;
mod serde_untagged_unit;
mod serde_validate_recursion;
mod swift;
mod types;
mod typescript;
mod utils;
mod zod;

pub use types::{types, types_phased};
pub use utils::fs_to_string;

#[test]
fn compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro/associated_items.rs");
    t.compile_fail("tests/macro/compile_error.rs");
}
