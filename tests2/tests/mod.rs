//! We register a single entrypoint so all tests are compiled into a single binary.
#![allow(unused_parens, dead_code)]

mod functions;
mod jsdoc;
mod types;
mod typescript;
mod utils;

pub use types::types;
pub use utils::fs_to_string;
