// Example demonstrating the skip_attr feature
use serde::{Deserialize, Serialize};
use specta::Type;

// Example 1: Skip doc attributes
#[test]
fn example_skip_doc() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(skip_attr = "doc")]
    struct SkipDocExample {
        #[doc = "This doc comment will be skipped"]
        field1: String,
        field2: i32,
    }

    let mut types = specta::TypeCollection::default();
    let _dt = SkipDocExample::definition(&mut types);
}

// Example 2: Skip multiple attribute types
#[test]
fn example_skip_multiple() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(skip_attr = "doc")]
    #[specta(skip_attr = "allow")]
    struct SkipMultipleExample {
        #[doc = "Field documentation"]
        #[allow(dead_code)]
        field1: String,
    }

    let mut types = specta::TypeCollection::default();
    let _dt = SkipMultipleExample::definition(&mut types);
}

// Example 3: Works with enums too
#[test]
fn example_skip_on_enum() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(skip_attr = "doc")]
    enum SkipEnumExample {
        #[doc = "Variant A"]
        VariantA,

        #[doc = "Variant B"]
        VariantB {
            #[doc = "Field in variant"]
            data: String,
        },
    }

    let mut types = specta::TypeCollection::default();
    let _dt = SkipEnumExample::definition(&mut types);
}

// Example 4: Specta and serde attributes still work
#[test]
fn example_works_with_other_attrs() {
    #[derive(Type, serde::Serialize)]
    #[specta(skip_attr = "doc")]
    struct WorksWithOtherAttrs {
        #[serde(rename = "renamedField")]
        #[doc = "This field is renamed by serde"]
        field1: String,

        #[specta(optional)]
        #[doc = "This field is optional"]
        field2: Option<i32>,
    }

    let mut types = specta::TypeCollection::default();
    let _dt = WorksWithOtherAttrs::definition(&mut types);
}
