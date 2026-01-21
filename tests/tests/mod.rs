//! We register a single entrypoint so all tests are compiled into a single binary.
#![allow(unused_parens, unused_variables, dead_code, unused_mut)]

mod functions;
mod jsdoc;
mod references;
mod types;
mod typescript;
mod utils;

pub use types::types;
pub use utils::fs_to_string;

#[test]
fn compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro/compile_error.rs");
}
