// Example demonstrating the skip_attr feature
// Run with: cargo run --example skip_attr_test --features derive

use specta::Type;

// Example 1: Skip doc attributes
#[derive(Type)]
#[specta(skip_attr = "doc")]
struct SkipDocExample {
    #[doc = "This doc comment will be skipped"]
    field1: String,
    field2: i32,
}

// Example 2: Skip multiple attribute types
#[derive(Type)]
#[specta(skip_attr = "doc")]
#[specta(skip_attr = "allow")]
struct SkipMultipleExample {
    #[doc = "Field documentation"]
    #[allow(dead_code)]
    field1: String,
}

// Example 3: Works with enums too
#[derive(Type)]
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

// Example 4: Specta and serde attributes still work
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

fn main() {
    println!("skip_attr example - all types compiled successfully!");

    // Create type definitions
    let mut types = specta::TypeCollection::default();

    println!("\n1. SkipDocExample:");
    let dt = SkipDocExample::definition(&mut types);
    println!("   Type definition created: {:?}", dt);

    println!("\n2. SkipMultipleExample:");
    let dt = SkipMultipleExample::definition(&mut types);
    println!("   Type definition created: {:?}", dt);

    println!("\n3. SkipEnumExample:");
    let dt = SkipEnumExample::definition(&mut types);
    println!("   Type definition created: {:?}", dt);

    println!("\n4. WorksWithOtherAttrs:");
    let dt = WorksWithOtherAttrs::definition(&mut types);
    println!("   Type definition created: {:?}", dt);

    println!("\n✓ All examples compiled and executed successfully!");
}
