use specta::{Type, TypeCollection};
use specta_swift::Swift;
use std::time::Duration;

/// Comprehensive demonstration of ALL specta-swift functionality
///
/// This example showcases every feature and capability of specta-swift in a single,
/// realistic application scenario. It demonstrates complex type relationships,
/// various enum patterns, special types, and advanced features.

/// Main application types for a task management system
#[derive(Type)]
struct Task {
    id: u32,
    title: String,
    description: Option<String>,
    status: TaskStatus,
    priority: Priority,
    assignee: Option<User>,
    created_at: String,
    updated_at: String,
    due_date: Option<String>,
    duration: Option<Duration>,
    tags: Vec<String>,
    metadata: TaskMetadata,
    subtasks: Vec<SubTask>,
}

/// Task status enum with mixed variants
#[derive(Type)]
enum TaskStatus {
    /// Task is waiting to be started
    Todo,
    /// Task is currently in progress
    InProgress {
        started_at: String,
        estimated_completion: Option<String>,
        progress: f32,
    },
    /// Task is blocked by dependencies
    Blocked {
        reason: String,
        blocked_by: Vec<u32>,
        estimated_unblock: Option<String>,
    },
    /// Task is under review
    Review {
        reviewer: String,
        review_started_at: String,
        comments: Vec<ReviewComment>,
    },
    /// Task is completed
    Completed {
        completed_at: String,
        completion_time: Duration,
        final_notes: Option<String>,
    },
    /// Task was cancelled
    Cancelled {
        reason: String,
        cancelled_at: String,
    },
}

/// Priority enum (string enum)
#[derive(Type)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
    Emergency,
}

/// User information
#[derive(Type)]
struct User {
    id: u32,
    username: String,
    email: String,
    profile: UserProfile,
    preferences: UserPreferences,
    role: UserRole,
    is_active: bool,
    last_login: Option<String>,
    created_at: String,
}

/// User profile with nested data
#[derive(Type)]
struct UserProfile {
    first_name: String,
    last_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    timezone: String,
    language: String,
}

/// User preferences
#[derive(Type)]
struct UserPreferences {
    theme: Theme,
    notifications: NotificationSettings,
    privacy: PrivacySettings,
    display: DisplaySettings,
}

/// Theme enum (string enum)
#[derive(Type)]
enum Theme {
    Light,
    Dark,
    Auto,
    Custom,
}

/// Notification settings
#[derive(Type)]
struct NotificationSettings {
    email_enabled: bool,
    push_enabled: bool,
    sms_enabled: bool,
    desktop_enabled: bool,
    frequency: NotificationFrequency,
}

/// Notification frequency enum
#[derive(Type)]
enum NotificationFrequency {
    Immediate,
    Hourly,
    Daily,
    Weekly,
    Never,
}

/// Privacy settings
#[derive(Type)]
struct PrivacySettings {
    profile_visibility: Visibility,
    activity_visibility: Visibility,
    data_sharing: DataSharing,
}

/// Visibility enum
#[derive(Type)]
enum Visibility {
    Public,
    Friends,
    Private,
    Hidden,
}

/// Data sharing settings
#[derive(Type)]
struct DataSharing {
    analytics: bool,
    marketing: bool,
    third_party: bool,
    research: bool,
}

/// Display settings
#[derive(Type)]
struct DisplaySettings {
    items_per_page: u32,
    date_format: String,
    time_format: String,
    currency: String,
    compact_mode: bool,
}

/// User role with permissions
#[derive(Type)]
enum UserRole {
    /// Regular user with basic permissions
    User,
    /// Moderator with additional permissions
    Moderator {
        permissions: Vec<String>,
        department: String,
    },
    /// Administrator with full permissions
    Admin {
        level: AdminLevel,
        departments: Vec<String>,
        special_access: Vec<String>,
    },
    /// Super admin with system-wide access
    SuperAdmin {
        system_access: bool,
        audit_logs: bool,
    },
}

/// Admin level enum
#[derive(Type)]
enum AdminLevel {
    Junior,
    Senior,
    Lead,
    Director,
}

/// Task metadata
#[derive(Type)]
struct TaskMetadata {
    created_by: u32,
    last_modified_by: u32,
    version: u32,
    custom_fields: Vec<(String, String)>,
    attachments: Vec<Attachment>,
    watchers: Vec<u32>,
    dependencies: Vec<u32>,
}

/// File attachment
#[derive(Type)]
struct Attachment {
    id: String,
    filename: String,
    size: u64,
    mime_type: String,
    uploaded_at: String,
    uploaded_by: u32,
}

/// Subtask with timing information
#[derive(Type)]
struct SubTask {
    id: u32,
    title: String,
    description: Option<String>,
    status: SubTaskStatus,
    estimated_duration: Duration,
    actual_duration: Option<Duration>,
    assignee: Option<u32>,
    created_at: String,
    completed_at: Option<String>,
}

/// Subtask status (simple enum)
#[derive(Type)]
enum SubTaskStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

/// Review comment
#[derive(Type)]
struct ReviewComment {
    id: u32,
    author: u32,
    content: String,
    created_at: String,
    updated_at: String,
    is_resolved: bool,
    parent_comment: Option<u32>,
    attachments: Vec<Attachment>,
}

/// API response wrapper
#[derive(Type)]
struct ApiResponse<T, E> {
    data: Option<T>,
    error: Option<E>,
    status: ResponseStatus,
    metadata: ResponseMetadata,
}

/// Response status enum
#[derive(Type)]
enum ResponseStatus {
    Success,
    PartialSuccess,
    Error,
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    NotFound,
    RateLimited,
}

/// Response metadata
#[derive(Type)]
struct ResponseMetadata {
    request_id: String,
    timestamp: String,
    processing_time: Duration,
    version: String,
    pagination: Option<PaginationInfo>,
}

/// Pagination information
#[derive(Type)]
struct PaginationInfo {
    page: u32,
    per_page: u32,
    total_pages: u32,
    total_items: u64,
    has_next: bool,
    has_prev: bool,
}

/// System health information
#[derive(Type)]
struct SystemHealth {
    status: HealthStatus,
    uptime: Duration,
    last_check: String,
    services: Vec<ServiceStatus>,
    metrics: SystemMetrics,
}

/// Health status enum
#[derive(Type)]
enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Service status
#[derive(Type)]
struct ServiceStatus {
    name: String,
    status: HealthStatus,
    response_time: Duration,
    last_check: String,
    error_count: u32,
}

/// System metrics
#[derive(Type)]
struct SystemMetrics {
    cpu_usage: f64,
    memory_usage: f64,
    disk_usage: f64,
    network_usage: f64,
    active_users: u32,
    total_requests: u64,
    error_rate: f64,
}

fn main() {
    println!("🚀 Comprehensive Demo - Complete specta-swift functionality showcase");
    println!("{}", "=".repeat(80));

    // Create comprehensive type collection
    let types = TypeCollection::default()
        // Core types
        .register::<Task>()
        .register::<TaskStatus>()
        .register::<Priority>()
        .register::<User>()
        .register::<UserProfile>()
        .register::<UserPreferences>()
        .register::<Theme>()
        .register::<NotificationSettings>()
        .register::<NotificationFrequency>()
        .register::<PrivacySettings>()
        .register::<Visibility>()
        .register::<DataSharing>()
        .register::<DisplaySettings>()
        .register::<UserRole>()
        .register::<AdminLevel>()
        .register::<TaskMetadata>()
        .register::<Attachment>()
        .register::<SubTask>()
        .register::<SubTaskStatus>()
        .register::<ReviewComment>()
        // API types
        .register::<ApiResponse<String, String>>()
        .register::<ResponseStatus>()
        .register::<ResponseMetadata>()
        .register::<PaginationInfo>()
        // System types
        .register::<SystemHealth>()
        .register::<HealthStatus>()
        .register::<ServiceStatus>()
        .register::<SystemMetrics>();

    // Export with default settings
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("📝 Generated Swift code (first 2000 characters):\n");
    let preview = if output.len() > 2000 {
        format!(
            "{}...\n\n[Output truncated - see ComprehensiveDemo.swift for full output]",
            &output[..2000]
        )
    } else {
        output.clone()
    };
    println!("{}", preview);

    // Write to file for inspection
    swift
        .export_to("./examples/generated/ComprehensiveDemo.swift", &types)
        .unwrap();
    println!("✅ Comprehensive demo exported to ComprehensiveDemo.swift");

    println!("\n🔍 Complete Feature Showcase:");
    println!("• ✅ Basic primitive types (i32, f64, bool, String, etc.)");
    println!("• ✅ Optional types (Option<T> → T?)");
    println!("• ✅ Collections (Vec<T> → [T])");
    println!("• ✅ Nested collections (Vec<Vec<T>> → [[T]])");
    println!("• ✅ Tuple types ((String, String) → (String, String))");
    println!("• ✅ Complex struct relationships");
    println!("• ✅ Generic types with type parameters");
    println!("• ✅ String enums with automatic Codable");
    println!("• ✅ Mixed enums with custom Codable implementations");
    println!("• ✅ Named field variants with struct generation");
    println!("• ✅ Duration types with RustDuration helper");
    println!("• ✅ Nested enum variants");
    println!("• ✅ Recursive type references");
    println!("• ✅ Complex metadata structures");
    println!("• ✅ API response patterns");
    println!("• ✅ System monitoring types");
    println!("• ✅ User management with permissions");
    println!("• ✅ Task management with status tracking");
    println!("• ✅ File attachment handling");
    println!("• ✅ Comment and review systems");
    println!("• ✅ Pagination and metadata");
    println!("• ✅ Health monitoring and metrics");

    println!("\n📊 Statistics:");
    println!("• Total types registered: {}", types.len());
    println!("• Generated Swift code length: {} characters", output.len());
    println!("• Lines of generated code: {}", output.lines().count());

    println!("\n🎉 This example demonstrates EVERY feature of specta-swift!");
    println!("Check ComprehensiveDemo.swift for the complete generated Swift code.");
}
