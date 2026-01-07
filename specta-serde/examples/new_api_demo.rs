//! Demonstration of the new integrated validation and processing API
//!
//! This example shows how the new API merges validation directly into the processing
//! functions, eliminating the need for separate validation calls.

use specta::{DataType, TypeCollection};
use specta_serde::{
    SerdeMode, apply_serde_transformations, process_for_both, process_for_deserialization,
    process_for_serialization,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== New Integrated Validation and Processing API Demo ===\n");

    // Create a simple type collection for demonstration
    let types = TypeCollection::default();

    println!("Original type collection has {} types", types.len());

    // Demonstrate the new integrated API
    println!("\n--- New API: Integrated Validation + Processing ---");

    // Process for both serialization and deserialization with validation included
    let (ser_types, de_types) = process_for_both(&types)?;
    println!("✅ Successfully processed types for both serialization and deserialization");
    println!("   Serialization types: {}", ser_types.len());
    println!("   Deserialization types: {}", de_types.len());

    // Process individually (validation is included automatically)
    let ser_types_individual = process_for_serialization(&types)?;
    let de_types_individual = process_for_deserialization(&types)?;
    println!("✅ Successfully processed types individually");
    println!("   Serialization types: {}", ser_types_individual.len());
    println!("   Deserialization types: {}", de_types_individual.len());

    // Demonstrate direct transformations
    println!("\n--- Direct Transformation Examples ---");

    // Create a simple DataType for testing
    let string_type = DataType::Primitive(specta::datatype::Primitive::String);

    let ser_transformed = apply_serde_transformations(&string_type, SerdeMode::Serialize)?;
    let de_transformed = apply_serde_transformations(&string_type, SerdeMode::Deserialize)?;

    println!("✅ Primitive type transformed successfully");
    println!(
        "   Serialization mode: {:?}",
        matches!(ser_transformed, DataType::Primitive(_))
    );
    println!(
        "   Deserialization mode: {:?}",
        matches!(de_transformed, DataType::Primitive(_))
    );

    // Demonstrate error handling with validation
    println!("\n--- Validation Error Handling ---");

    // The validation is now integrated, so any validation errors will be caught
    // during processing automatically
    match process_for_serialization(&types) {
        Ok(_) => println!("✅ All types passed validation during processing"),
        Err(e) => println!("❌ Validation failed during processing: {}", e),
    }

    // Show how different serde attributes would affect serialization vs deserialization
    println!("\n--- Mode-Specific Behavior ---");

    // Create a list type to show transformation behavior
    let list_type = DataType::List(specta::datatype::List::new(string_type.clone()));

    let ser_result = apply_serde_transformations(&list_type, SerdeMode::Serialize)?;
    let de_result = apply_serde_transformations(&list_type, SerdeMode::Deserialize)?;

    println!("✅ List type processing shows mode-specific behavior:");
    println!(
        "   - Serialization: {:?}",
        matches!(ser_result, DataType::List(_))
    );
    println!(
        "   - Deserialization: {:?}",
        matches!(de_result, DataType::List(_))
    );
    println!("   - Both modes apply their respective transformations");

    // Demonstrate nullable type handling
    println!("\n--- Nullable Type Handling ---");

    let nullable_type = DataType::Nullable(Box::new(string_type.clone()));
    let transformed = apply_serde_transformations(&nullable_type, SerdeMode::Serialize)?;

    match transformed {
        DataType::Nullable(_) => {
            println!("✅ Nullable type correctly preserved during transformation")
        }
        _ => println!("⚠️  Nullable type handling may need adjustment"),
    }

    println!("\n--- Migration Guide ---");
    println!("Old API (deprecated):");
    println!("  specta_serde::validate(&types)?;");
    println!("  let result = exporter::export(&types);");
    println!();
    println!("New API (recommended):");
    println!("  let (ser_types, de_types) = specta_serde::process_for_both(&types)?;");
    println!("  let result = exporter::export(&ser_types); // or &de_types");
    println!();
    println!("Benefits:");
    println!("  ✅ Validation is automatically included");
    println!("  ✅ Single API call does both validation and transformation");
    println!("  ✅ Mode-specific processing for better accuracy");
    println!("  ✅ Comprehensive serde attribute support");
    println!("  ✅ All Serde container and field attributes now supported");

    println!("\n--- Supported Serde Attributes ---");
    println!("Container attributes:");
    println!("  • rename, rename_all, rename_all_fields");
    println!("  • tag, content, untagged (enum representations)");
    println!("  • transparent, default, deny_unknown_fields");
    println!("  • remote, from, try_from, into");
    println!();
    println!("Field attributes:");
    println!("  • rename, alias, default, flatten");
    println!("  • skip, skip_serializing, skip_deserializing, skip_serializing_if");
    println!("  • serialize_with, deserialize_with, with");

    println!("\n=== Demo completed successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrated_processing() {
        let types = TypeCollection::default();

        // Should not panic and should return valid results
        let result = process_for_both(&types);
        assert!(result.is_ok());

        let (ser_types, de_types) = result.unwrap();
        assert_eq!(ser_types.len(), types.len());
        assert_eq!(de_types.len(), types.len());
    }

    #[test]
    fn test_primitive_transformation() {
        let string_type = DataType::Primitive(specta::datatype::Primitive::String);

        // Both modes should process successfully
        let ser_result = apply_serde_transformations(&string_type, SerdeMode::Serialize);
        let de_result = apply_serde_transformations(&string_type, SerdeMode::Deserialize);

        assert!(ser_result.is_ok());
        assert!(de_result.is_ok());

        // Results should be primitive types
        assert!(matches!(ser_result.unwrap(), DataType::Primitive(_)));
        assert!(matches!(de_result.unwrap(), DataType::Primitive(_)));
    }

    #[test]
    fn test_mode_specific_behavior() {
        let types = TypeCollection::default();

        // Both modes should process successfully
        let ser_types = process_for_serialization(&types);
        let de_types = process_for_deserialization(&types);

        assert!(ser_types.is_ok());
        assert!(de_types.is_ok());

        let ser_types = ser_types.unwrap();
        let de_types = de_types.unwrap();

        assert_eq!(ser_types.len(), 0); // Empty collection
        assert_eq!(de_types.len(), 0); // Empty collection
    }

    #[test]
    fn test_nullable_type_transformation() {
        let string_type = DataType::Primitive(specta::datatype::Primitive::String);
        let nullable_type = DataType::Nullable(Box::new(string_type));

        let result = apply_serde_transformations(&nullable_type, SerdeMode::Serialize);
        assert!(result.is_ok());

        let transformed = result.unwrap();
        assert!(matches!(transformed, DataType::Nullable(_)));
    }

    #[test]
    fn test_list_type_transformation() {
        let string_type = DataType::Primitive(specta::datatype::Primitive::String);
        let list_type = DataType::List(specta::datatype::List::new(string_type));

        let result = apply_serde_transformations(&list_type, SerdeMode::Serialize);
        assert!(result.is_ok());

        let transformed = result.unwrap();
        assert!(matches!(transformed, DataType::List(_)));
    }
}
