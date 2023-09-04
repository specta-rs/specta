#![allow(unused_variables, dead_code)]

mod advanced_types;
mod bigints;
mod datatype;
mod duplicate_ty_name;
mod export;
mod flatten_and_inline;
mod functions;
mod macro_decls;
mod map_keys;
mod rename;
mod reserved_keywords;
mod selection;
mod serde;
mod sid;
mod transparent;
pub mod ts;
mod ts_rs;
mod ty_override;

#[test]
fn test_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro/compile_error.rs");
}
