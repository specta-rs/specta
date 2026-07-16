use serde::Serialize;
use specta::{Type, datatype::DataType};

#[derive(Debug, thiserror::Error, Serialize, Type)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase", untagged)]
enum InheritanceError {
    #[error("Circular dependency detected: {}", chain.join(" -> "))]
    CircularDependency {
        #[specta(type = Vec<String>)]
        chain: Vec<String>,
    },

    #[error(
        "Maximum inheritance depth exceeded ({max_depth}): {}",
        chain.join(" -> ")
    )]
    MaxDepthExceeded {
        #[specta(type = i32)]
        max_depth: i32,
        #[specta(type = Vec<String>)]
        chain: Vec<String>,
    },
}

#[derive(Debug, thiserror::Error, Type)]
#[specta(collect = false)]
enum OrdinaryError {
    #[error("ordinary error: {message}")]
    Message { message: String },
}

#[test]
fn thiserror_method_call_arguments_compile_and_serde_attributes_are_captured() {
    let mut types = specta::Types::default();
    InheritanceError::definition(&mut types);

    let data_type = types
        .into_unsorted_iter()
        .find(|data_type| data_type.name == "InheritanceError")
        .and_then(|data_type| data_type.ty.clone())
        .expect("InheritanceError should be registered");
    let untagged = match data_type {
        DataType::Enum(data_type) => data_type
            .attributes
            .get_named_as::<bool>("serde:container:untagged")
            .copied(),
        _ => None,
    };

    assert_eq!(untagged, Some(true));
}

#[test]
fn ordinary_thiserror_enum_still_compiles_with_specta() {
    let mut types = specta::Types::default();
    OrdinaryError::definition(&mut types);
}
