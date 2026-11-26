// `specta_serde::Format::map_type` on datatypes outside the registered graph.
//
// `map_types` validates every declaration and root of the graph, so `map_type`
// trusts closed references to it. An inline container's body only exists at
// its use sites, so a fragment that uses one is still checked there.

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
