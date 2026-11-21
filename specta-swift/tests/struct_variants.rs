#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

/// Test enum with struct-like variants (named fields)
#[derive(Type)]
pub enum Event {
    /// Job started event with named fields
    JobStarted { job_id: String, job_type: String },
    /// Job completed event with named fields
    JobCompleted {
        job_id: String,
        result: String,
        duration: u64,
    },
    /// Simple unit variant
    JobCancelled,
    /// Tuple variant (unnamed fields)
    JobFailed(String, u32),
}

/// Test enum with mixed variant types
#[derive(Type)]
pub enum ApiResponse {
    /// Success with data
    Success { data: String, status: u16 },
    /// Error with details
    Error {
        message: String,
        code: u32,
        details: Option<String>,
    },
    /// Loading state
    Loading,
    /// Tuple variant
    Redirect(String),
}

#[test]
fn test_struct_variants_generation() {
    let types = Types::default()
        .register::<Event>()
        .register::<ApiResponse>();
    let swift = Swift::default();
    let result = swift.export(&types, specta_serde::Format).unwrap();

    println!("Generated Swift for struct variants:");
    println!("{}", result);

    // Event enum should have struct-like cases
    assert!(result.contains("enum Event"));
    assert!(result.contains("case jobStarted(EventJobStartedData)"));
    assert!(result.contains("case jobCompleted(EventJobCompletedData)"));
    assert!(result.contains("case jobCancelled"));
    assert!(result.contains("case jobFailed(String, UInt32)"));

    // Should generate structs for named field variants
    assert!(result.contains("struct EventJobStartedData: Codable"));
    assert!(result.contains("let jobId: String"));
    assert!(result.contains("let jobType: String"));

    assert!(result.contains("struct EventJobCompletedData: Codable"));
    assert!(result.contains("let jobId: String"));
    assert!(result.contains("let result: String"));
    assert!(result.contains("let duration: UInt64"));

    // ApiResponse enum should have struct-like cases
    assert!(result.contains("enum ApiResponse"));
    assert!(result.contains("case success(ApiResponseSuccessData)"));
    assert!(result.contains("case error(ApiResponseErrorData)"));
    assert!(result.contains("case loading"));
    assert!(result.contains("case redirect(String)"));

    // Should generate structs for ApiResponse variants
    assert!(result.contains("struct ApiResponseSuccessData: Codable"));
    assert!(result.contains("let data: String"));
    assert!(result.contains("let status: UInt16"));

    assert!(result.contains("struct ApiResponseErrorData: Codable"));
    assert!(result.contains("let message: String"));
    assert!(result.contains("let code: UInt32"));
    assert!(result.contains("let details: String?"));
}

// A `rename_all = "camelCase"` rule hands the exporter the wire name
// `notFound`. A wrapper struct is still a type name, so it has to keep the
// inner capital instead of lowercasing it into `NotfoundData`.
#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ErrorResponse {
    NotFound { message: String },
    BadRequest { code: u32 },
}

// The wrapper struct is also named from the wire name when the enum is
// externally tagged, whose case is decoded by the custom `Codable`.
#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalError {
    NotFound { message: String },
}

// An all-caps word from a SCREAMING rule is not an acronym, so it is cased
// like any other word rather than shouting in the type name.
#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScreamingError {
    NotFound { message: String },
}

#[test]
fn test_renamed_struct_variant_names() {
    let types = Types::default()
        .register::<ErrorResponse>()
        .register::<ExternalError>()
        .register::<ScreamingError>();
    let result = Swift::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    println!("Generated Swift for renamed struct variants:");
    println!("{}", result);

    assert!(result.contains("case notFound(ErrorResponseNotFoundData)"));
    assert!(result.contains("struct ErrorResponseNotFoundData: Codable"));
    assert!(result.contains("case badRequest(ErrorResponseBadRequestData)"));
    assert!(result.contains("struct ErrorResponseBadRequestData: Codable"));
    assert!(result.contains("case notFound(ExternalErrorNotFoundData)"));
    assert!(result.contains("struct ExternalErrorNotFoundData: Codable"));
    assert!(result.contains("case notFound(ScreamingErrorNotFoundData)"));
    assert!(result.contains("struct ScreamingErrorNotFoundData: Codable"));
}
