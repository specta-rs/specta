// Test the skip_attr functionality
// The skip_attr feature allows skipping attributes that may have non-standard syntax
// that Specta's attribute parser might not understand.
use specta::Type;

// Test basic functionality - skip_attr should prevent specta from trying to parse
// the specified attribute, allowing compilation to succeed
#[test]
fn skip_attr_basic() {
    #[derive(Type)]
    #[specta(skip_attr = "doc")] // Skip doc comments as a test
    struct TestStruct {
        #[doc = "This is a field"]
        field1: String,
        field2: i32,
    }

    // Verify the type definition is created successfully
    use specta::Type as _;
    let mut type_map = specta::TypeCollection::default();
    let _ = TestStruct::definition(&mut type_map);
}

// Test that normal serde and specta attributes still work when other attrs are skipped
#[test]
fn skip_attr_preserves_other_attrs() {
    #[derive(Type, serde::Serialize)]
    #[specta(skip_attr = "doc")]
    struct WithSerde {
        #[serde(rename = "newName")]
        #[doc = "Field documentation"]
        field: String,

        #[specta(optional)]
        optional_field: Option<i32>,
    }

    use specta::Type as _;
    let mut type_map = specta::TypeCollection::default();
    let _ = WithSerde::definition(&mut type_map);
}

// Test skip_attr on enums
#[test]
fn skip_attr_on_enum() {
    #[derive(Type)]
    #[specta(skip_attr = "doc")]
    enum TestEnum {
        #[doc = "Variant 1"]
        Variant1,

        #[doc = "Variant 2"]
        Variant2 {
            #[doc = "Field in variant"]
            field: String,
        },
    }

    use specta::Type as _;
    let mut type_map = specta::TypeCollection::default();
    let _ = TestEnum::definition(&mut type_map);
}

// Test multiple skip_attr declarations
#[test]
fn skip_multiple_attrs() {
    #[derive(Type)]
    #[specta(skip_attr = "doc")]
    #[specta(skip_attr = "allow")]
    struct MultiSkip {
        #[doc = "Field 1"]
        #[allow(dead_code)]
        field1: String,

        #[doc = "Field 2"]
        field2: i32,
    }

    use specta::Type as _;
    let mut type_map = specta::TypeCollection::default();
    let _ = MultiSkip::definition(&mut type_map);
}
