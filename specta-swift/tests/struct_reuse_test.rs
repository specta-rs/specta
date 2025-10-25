use specta::{Type, TypeCollection};
use specta_swift::Swift;

#[derive(Type)]
struct UserData {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Type)]
enum ApiResponse {
    Success(UserData),
    Error { message: String, code: u32 },
    Loading,
}

#[derive(Type)]
struct ApiRequest {
    user: UserData,
    action: String,
}

#[test]
fn test_struct_reuse_between_standalone_and_enum() {
    let types = TypeCollection::default()
        .register::<UserData>()
        .register::<ApiResponse>()
        .register::<ApiRequest>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift for struct reuse test:");
    println!("{}", output);

    // Check that UserData is defined as a standalone struct
    assert!(output.contains("public struct UserData: Codable"));
    assert!(output.contains("public let id: UInt32"));
    assert!(output.contains("public let name: String"));
    assert!(output.contains("public let email: String?"));

    // Check that ApiResponse uses UserData directly (not a generated struct)
    assert!(output.contains("case success(UserData)"));

    // Check that ApiRequest also uses UserData directly
    assert!(output.contains("public struct ApiRequest: Codable"));
    assert!(output.contains("public let user: UserData"));
    assert!(output.contains("public let action: String"));

    // Ensure we don't have duplicate UserData definitions
    let user_data_count = output.matches("public struct UserData: Codable").count();
    assert_eq!(user_data_count, 1, "UserData should be defined only once");

    // Ensure we don't have any generated structs like ApiResponseSuccessData
    assert!(
        !output.contains("ApiResponseSuccessData"),
        "Should not generate ApiResponseSuccessData when UserData is standalone"
    );
}

#[test]
fn test_struct_reuse_with_different_ordering() {
    // Test with different ordering - enum first, then standalone struct
    let types = TypeCollection::default()
        .register::<ApiResponse>()
        .register::<UserData>()
        .register::<ApiRequest>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift for struct reuse test (enum first):");
    println!("{}", output);

    // Check that UserData is still defined as a standalone struct
    assert!(output.contains("public struct UserData: Codable"));

    // Check that ApiResponse uses UserData directly
    assert!(output.contains("case success(UserData)"));

    // Ensure we don't have duplicate UserData definitions
    let user_data_count = output.matches("public struct UserData: Codable").count();
    assert_eq!(user_data_count, 1, "UserData should be defined only once");

    // Ensure we don't have any generated structs like ApiResponseSuccessData
    assert!(
        !output.contains("ApiResponseSuccessData"),
        "Should not generate ApiResponseSuccessData when UserData is standalone"
    );
}
