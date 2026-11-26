// `specta_serde::Format::map_type` relies on `map_types` having validated each
// declaration, but an inline body exists only at its use sites, so `map_type`
// must still validate it.

use serde::{Deserialize, Serialize};
use specta::{Format as _, Type, Types};

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, inline)]
struct FragmentInline {
    #[serde(flatten)]
    v: Vec<u8>,
}

#[test]
fn map_type_validates_inline_bodies_outside_the_graph() {
    let mut types = Types::default();
    let fragment = <FragmentInline as Type>::definition(&mut types);
    let mapped = specta_serde::Format
        .map_types(&types)
        .expect("the graph holds no body to reject");

    let err = specta_serde::Format
        .map_type(&mapped, &fragment)
        .expect_err("serde can't flatten a sequence")
        .to_string();
    assert!(err.contains("flatten"), "{err}");
}
