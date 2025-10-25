use specta::{Type, TypeCollection};
use specta_swift::Swift;

#[derive(Type)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Type)]
struct Admin {
    id: u32,
    name: String,
    permissions: Vec<String>,
    level: u8,
}

#[derive(Type)]
struct Guest {
    session_id: String,
    expires_at: String,
}

#[derive(Type)]
enum UserType {
    // Unit variant
    Anonymous,

    // Tuple variant
    User(String, u32),

    // Named fields variant (should become a struct-like case)
    Admin {
        id: u32,
        name: String,
        permissions: Vec<String>,
    },

    // Nested struct variant
    Registered(User),

    // Complex nested struct variant
    SuperAdmin(Admin),

    // Mixed variant with nested struct
    Guest {
        info: Guest,
        created_at: String,
    },
}

#[derive(Type)]
enum ApiResult<T, E> {
    Success {
        data: T,
        status: u16,
    },
    Error {
        error: E,
        code: u32,
        message: String,
    },
    Loading {
        progress: f32,
    },
}

#[derive(Type)]
enum ComplexUnion {
    // Simple unit
    None,

    // Tuple with multiple types
    Tuple(String, u32, bool),

    // Named fields
    NamedFields {
        id: u32,
        name: String,
        active: bool,
    },

    // Nested struct
    UserStruct(User),

    // Nested enum
    UserType(UserType),

    // Complex nested structure
    Complex {
        user: User,
        metadata: Vec<String>,
        settings: Option<Admin>,
    },
}

#[test]
fn test_enum_with_nested_structs() {
    let types = TypeCollection::default()
        .register::<User>()
        .register::<Admin>()
        .register::<Guest>()
        .register::<UserType>()
        .register::<ApiResult<String, String>>()
        .register::<ComplexUnion>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift code:\n{}", output);

    // Test enum generation
    assert!(output.contains("enum UserType"));
    assert!(output.contains("enum ApiResult"));
    assert!(output.contains("enum ComplexUnion"));

    // Test unit variants
    assert!(output.contains("case anonymous"));
    assert!(output.contains("case none"));

    // Test tuple variants
    assert!(output.contains("case user(String, UInt32)"));
    assert!(output.contains("case tuple(String, UInt32, Bool)"));

    // Test named field variants (should be struct-like)
    assert!(output.contains("case admin(UserTypeAdminData)"));
    assert!(output.contains("case guest(UserTypeGuestData)"));
    assert!(output.contains("case success(ApiResultSuccessData)"));
    assert!(output.contains("case error(ApiResultErrorData)"));
    assert!(output.contains("case loading(ApiResultLoadingData)"));
    assert!(output.contains("case namedFields(ComplexUnionNamedFieldsData)"));
    assert!(output.contains("case complex(ComplexUnionComplexData)"));

    // Test nested struct variants
    assert!(output.contains("case registered(User)"));
    assert!(output.contains("case superAdmin(Admin)"));
    assert!(output.contains("case userStruct(User)"));
    assert!(output.contains("case userType(UserType)"));

    // Test that nested structs are properly defined
    assert!(output.contains("struct User"));
    assert!(output.contains("struct Admin"));
    assert!(output.contains("struct Guest"));
}

#[test]
fn test_swift_union_syntax() {
    let types = TypeCollection::default().register::<UserType>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("UserType Swift code:\n{}", output);

    // Verify proper Swift enum syntax (no redundant Codable in declaration)
    assert!(output.contains("enum UserType {"));

    // Unit variant
    assert!(output.contains("case anonymous"));

    // Tuple variant
    assert!(output.contains("case user(String, UInt32)"));

    // Named fields should be struct-like
    assert!(output.contains("case admin(UserTypeAdminData)"));
    assert!(output.contains("case registered(User)"));
    assert!(output.contains("case superAdmin(Admin)"));
    assert!(output.contains("case guest(UserTypeGuestData)"));
}
