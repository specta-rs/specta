use specta::{Type, Types};

/// Comprehensive demonstration of all specta-rescript functionality.
///
/// Models a task management system to exercise every supported type
/// pattern in a realistic application scenario.

// ── Enums ──────────────────────────────────────────────────────────────────

/// Current status of a task.
#[derive(Type)]
enum TaskStatus {
    /// Waiting to be picked up.
    Todo,
    /// Being worked on.
    InProgress {
        /// ISO-8601 UTC timestamp when work started.
        started_at: String,
        /// Progress percentage (0–100).
        progress: f32,
    },
    /// Blocked on an external dependency.
    Blocked {
        reason: String,
        blocked_by: Vec<u32>,
    },
    /// Under review.
    Review {
        reviewer: String,
        review_started_at: String,
    },
    /// Successfully completed.
    Completed {
        completed_at: String,
        final_notes: Option<String>,
    },
    /// Cancelled before completion.
    Cancelled {
        reason: String,
        cancelled_at: String,
    },
}

/// Task importance level.
#[derive(Type)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// User role with permission levels.
#[derive(Type)]
enum UserRole {
    /// Standard user with no elevated access.
    User,
    /// Can moderate content in their department.
    Moderator {
        permissions: Vec<String>,
        department: String,
    },
    /// Full administrative access.
    Admin {
        level: AdminLevel,
        departments: Vec<String>,
    },
}

/// Seniority level for admin accounts.
#[derive(Type)]
enum AdminLevel {
    Junior,
    Senior,
    Lead,
}

/// UI theme preference.
#[derive(Type)]
enum Theme {
    Light,
    Dark,
    Auto,
}

/// How often notification digests are sent.
#[derive(Type)]
enum NotificationFrequency {
    Immediate,
    Hourly,
    Daily,
    Never,
}

/// Visibility scope for profile data.
#[derive(Type)]
enum Visibility {
    Public,
    FriendsOnly,
    Private,
}

/// Status of a background service.
#[derive(Type)]
enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Status of a subtask.
#[derive(Type)]
enum SubTaskStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

// ── Generic types ───────────────────────────────────────────────────────────

/// Generic API response with typed ok and error variants.
#[derive(Type)]
enum TaskResult<T, E> {
    Ok(T),
    Err(E),
}

/// Paginated list of items.
#[derive(Type)]
struct Page<T> {
    items: Vec<T>,
    total: u32,
    page: u32,
    page_size: u32,
    has_next: bool,
    has_prev: bool,
}

// ── Structs ─────────────────────────────────────────────────────────────────

/// Notification delivery preferences.
#[derive(Type)]
struct NotificationSettings {
    email_enabled: bool,
    push_enabled: bool,
    frequency: NotificationFrequency,
}

/// Privacy controls for a user's profile.
#[derive(Type)]
struct PrivacySettings {
    profile_visibility: Visibility,
    activity_visibility: Visibility,
    data_sharing: bool,
}

/// Display and locale preferences.
#[derive(Type)]
struct DisplaySettings {
    items_per_page: u32,
    date_format: String,
    compact_mode: bool,
}

/// Aggregated user preferences.
#[derive(Type)]
struct UserPreferences {
    theme: Theme,
    notifications: NotificationSettings,
    privacy: PrivacySettings,
    display: DisplaySettings,
}

/// Public profile for a user account.
#[derive(Type)]
struct UserProfile {
    first_name: String,
    last_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    timezone: String,
}

/// A user account.
#[derive(Type)]
struct User {
    id: u32,
    username: String,
    email: String,
    profile: UserProfile,
    preferences: UserPreferences,
    role: UserRole,
    is_active: bool,
    created_at: String,
    last_login: Option<String>,
}

/// An uploaded file attachment.
#[derive(Type)]
struct Attachment {
    id: String,
    filename: String,
    size: u64,
    mime_type: String,
    uploaded_at: String,
    uploaded_by: u32,
}

/// A code review comment on a task.
#[derive(Type)]
struct ReviewComment {
    id: u32,
    author: u32,
    content: String,
    created_at: String,
    is_resolved: bool,
    attachments: Vec<Attachment>,
}

/// A small unit of work within a parent task.
#[derive(Type)]
struct SubTask {
    id: u32,
    title: String,
    description: Option<String>,
    status: SubTaskStatus,
    assignee: Option<u32>,
    created_at: String,
    completed_at: Option<String>,
}

/// Metadata tracking creation and modification history.
#[derive(Type)]
struct TaskMetadata {
    created_by: u32,
    last_modified_by: u32,
    version: u32,
    attachments: Vec<Attachment>,
    watchers: Vec<u32>,
    dependencies: Vec<u32>,
}

/// A work item tracked through the task management system.
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
    tags: Vec<String>,
    metadata: TaskMetadata,
    subtasks: Vec<SubTask>,
    comments: Vec<ReviewComment>,
}

/// Pagination metadata included in list responses.
#[derive(Type)]
struct PaginationInfo {
    page: u32,
    per_page: u32,
    total_pages: u32,
    total_items: u64,
}

/// Performance metrics for a background service.
#[derive(Type)]
struct ServiceStatus {
    name: String,
    status: HealthStatus,
    last_check: String,
    error_count: u32,
}

/// Application-level health summary.
#[derive(Type)]
struct SystemHealth {
    status: HealthStatus,
    last_check: String,
    services: Vec<ServiceStatus>,
    cpu_usage: f64,
    memory_usage: f64,
}

pub fn types() -> Types {
    Types::default()
        // Enums
        .register::<TaskStatus>()
        .register::<Priority>()
        .register::<UserRole>()
        .register::<AdminLevel>()
        .register::<Theme>()
        .register::<NotificationFrequency>()
        .register::<Visibility>()
        .register::<HealthStatus>()
        .register::<SubTaskStatus>()
        // Generic types
        .register::<TaskResult<String, String>>()
        .register::<Page<Task>>()
        // Structs
        .register::<NotificationSettings>()
        .register::<PrivacySettings>()
        .register::<DisplaySettings>()
        .register::<UserPreferences>()
        .register::<UserProfile>()
        .register::<User>()
        .register::<Attachment>()
        .register::<ReviewComment>()
        .register::<SubTask>()
        .register::<TaskMetadata>()
        .register::<Task>()
        .register::<PaginationInfo>()
        .register::<ServiceStatus>()
        .register::<SystemHealth>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Comprehensive Demo - Full specta-rescript feature showcase");
    println!("{}", "=".repeat(60));

    let types = types();

    let output = ReScript::default().export(&types).unwrap();
    let lines = output.lines().count();
    let types_count = types.len();

    println!("Generated {lines} lines of ReScript across {types_count} types.\n");
    println!("First 40 lines:\n");
    for line in output.lines().take(40) {
        println!("{}", line);
    }
    println!("...\n[See ComprehensiveDemo.res for full output]");

    ReScript::default()
        .export_to(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/generated/ComprehensiveDemo.res"
            ),
            &types,
        )
        .unwrap();
    println!("\nExported to ComprehensiveDemo.res");

    println!("\nKey Features Demonstrated:");
    println!("• Unit enums -> polymorphic variants [ #A | #B ]");
    println!("• Data enums -> regular variants | A | B(t)");
    println!("• Named-field variants -> auxiliary record types");
    println!("• Generic types: result<'t, 'e>, page<'t>");
    println!("• Deeply nested struct references");
    println!("• option<t>, array<t>, dict<v> field types");
    println!("• Topological ordering (dependencies before dependents)");
}
