use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Wire {
    value: i32,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(into = "Wire")]
struct IntoOnly {
    value: i32,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(from = "Wire", into = "Wire")]
struct Symmetric {
    value: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Parent {
    child: IntoOnly,
}

impl From<IntoOnly> for Wire {
    fn from(value: IntoOnly) -> Self {
        Self { value: value.value }
    }
}

impl From<Symmetric> for Wire {
    fn from(value: Symmetric) -> Self {
        Self { value: value.value }
    }
}

impl From<Wire> for Symmetric {
    fn from(value: Wire) -> Self {
        Self { value: value.value }
    }
}

fn type_names(types: &TypeCollection) -> Vec<String> {
    types
        .into_unsorted_iter()
        .map(|ndt| ndt.name().to_string())
        .collect()
}

#[test]
fn apply_rejects_asymmetric_container_conversion() {
    let err = specta_serde::apply(TypeCollection::default().register::<IntoOnly>())
        .expect_err("apply should reject asymmetric serde conversions");

    assert!(
        err.to_string()
            .contains("Incompatible container conversion"),
        "unexpected error: {err}"
    );
}

#[test]
fn apply_phases_splits_container_and_dependents_for_conversions() {
    let types = specta_serde::apply_phases(TypeCollection::default().register::<Parent>())
        .expect("apply_phases should support asymmetric serde conversions");
    let names = type_names(&types);

    assert!(names.iter().any(|name| name == "IntoOnly_Serialize"));
    assert!(names.iter().any(|name| name == "IntoOnly_Deserialize"));
    assert!(names.iter().any(|name| name == "Parent_Serialize"));
    assert!(names.iter().any(|name| name == "Parent_Deserialize"));
}

#[test]
fn apply_accepts_symmetric_container_conversion() {
    specta_serde::apply(TypeCollection::default().register::<Symmetric>())
        .expect("apply should accept symmetric serde conversions");
}
