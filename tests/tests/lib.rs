#![allow(unused_variables, dead_code)]

mod advanced_types;
mod bigints;
mod comments;
mod const_types;
mod datatype;
mod deprecated;
mod duplicate_ty_name;
mod export;
mod flatten_and_inline;
mod functions;
mod macro_decls;
mod map_keys;
mod optional;
mod recursive;
mod remote_impls;
mod rename;
mod reserved_keywords;
mod selection;
mod serde;
mod sid;
mod static_types;
mod transparent;
pub mod ts;
mod ts_rs;
mod ty_override;
mod type_collection;
mod type_map;
mod typescript;
// TODO: Bring this back
// #[cfg(all(feature = "ulid", feature = "typescript"))]
// mod ulid;

#[test]
fn test_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macro/compile_error.rs");
}
