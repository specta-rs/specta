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
mod core_type_impls;
mod errors;
mod functions;
mod go;
mod java;
mod jsdoc;
mod jsonschema;
mod kotlin;
mod layouts;
mod legacy_impls;
mod macro_doc_attrs;
mod macro_structured_deprecated;
mod maybe_undefined;
mod openapi;
mod python;
mod references;
mod rust;
mod semantic;
mod serde_container_rename;
mod serde_conversions;
mod serde_default_phases;
mod serde_empty_payloads;
mod serde_enum_rewrite;
mod serde_flatten_option;
mod serde_identifiers;
mod serde_internal_tag_payloads;
mod serde_other;
mod serde_unified_asymmetry;
mod serde_unknown_attrs;
mod serde_untagged_unit;
mod serde_validate_coverage;
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
