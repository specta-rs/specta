//! Generates the sources compiled by the `tests/kotlin-typecheck` harness.
//!
//! The harness compiles every configuration with `kotlinc -Werror` and round-trips the
//! `serde_json` samples written next to them. See `tests/kotlin-typecheck/README.md`.
#![allow(dead_code)]

use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};
use specta::{Type, Types};
use specta_kotlin::{Kotlin, Serialization};

/// The reporter's shapes from specta-rs/specta#630: adjacently tagged enums with unit and
/// struct variants whose wire names are renamed.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "context", content = "data", rename_all = "camelCase")]
enum ApiError {
    Auth(AuthApiError),
    RateLimited,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "reason", content = "params", rename_all = "camelCase")]
enum AuthApiError {
    CredentialsInvalid,
    SignInRemoved { cause: SignInRemovalCause },
}

/// Externally tagged unit variants, which Kotlinx encodes as a JSON string.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum SignInRemovalCause {
    Remote,
    AccountDeleted,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum RouteSessionType {
    Anonymous,
    SignupOnboarding,
}

/// Internally tagged, so the variant payload is flattened into the tag object.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InternallyTagged {
    Empty,
    Named { cause: SignInRemovalCause },
    NewType(InternalPayload),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct InternalPayload {
    value: i32,
}

/// Externally tagged with every payload shape Kotlinx supports.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum ExternalEvent {
    Started,
    Message(String),
    Progress { current: u32, total: u32 },
    Data(ExternalPayload),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct ExternalPayload {
    note: String,
}

/// A defaulted optional still has to be written as `null` by Serde, so Kotlinx must not
/// fall back to omitting it.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Settings {
    #[serde(default)]
    nickname: Option<String>,
    maybe: Option<String>,
}

/// Only plain declarations can express these shapes. They exist to keep the `Serialization::None`
/// output compiled against shapes that `Serialization::Kotlinx` rejects.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedValue {
    Text(String),
    Count(i32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum GenericEvent<T> {
    Started,
    Payload(T),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct TupleRecord(u8, String, bool);

fn kotlinx_types() -> Types {
    Types::default()
        .register::<ApiError>()
        .register::<AuthApiError>()
        .register::<SignInRemovalCause>()
        .register::<RouteSessionType>()
        .register::<InternallyTagged>()
        .register::<InternalPayload>()
        .register::<ExternalEvent>()
        .register::<ExternalPayload>()
        .register::<Settings>()
}

fn known<T: Serialize>(name: &str, value: T) -> Value {
    json!({
        "name": name,
        "unknown": false,
        "fallback": false,
        "value": serde_json::to_value(value).unwrap(),
    })
}

/// A payload a newer producer may send. `fallback` asserts the decoded value is the generated
/// `Unknown` variant; nested fallbacks only need the round-trip to succeed.
fn unknown(name: &str, fallback: bool, value: Value) -> Value {
    json!({
        "name": name,
        "unknown": true,
        "fallback": fallback,
        "value": value,
    })
}

fn samples() -> Vec<Value> {
    vec![
        known("ApiError", ApiError::Auth(AuthApiError::CredentialsInvalid)),
        known(
            "ApiError",
            ApiError::Auth(AuthApiError::SignInRemoved {
                cause: SignInRemovalCause::Remote,
            }),
        ),
        known("ApiError", ApiError::RateLimited),
        known(
            "AuthApiError",
            AuthApiError::SignInRemoved {
                cause: SignInRemovalCause::AccountDeleted,
            },
        ),
        known("SignInRemovalCause", SignInRemovalCause::Remote),
        known("SignInRemovalCause", SignInRemovalCause::AccountDeleted),
        known("RouteSessionType", RouteSessionType::Anonymous),
        known("RouteSessionType", RouteSessionType::SignupOnboarding),
        known("InternallyTagged", InternallyTagged::Empty),
        known(
            "InternallyTagged",
            InternallyTagged::Named {
                cause: SignInRemovalCause::AccountDeleted,
            },
        ),
        known(
            "InternallyTagged",
            InternallyTagged::NewType(InternalPayload { value: 7 }),
        ),
        known("ExternalEvent", ExternalEvent::Started),
        known("ExternalEvent", ExternalEvent::Message("hello".into())),
        known(
            "ExternalEvent",
            ExternalEvent::Progress {
                current: 1,
                total: 4,
            },
        ),
        known(
            "ExternalEvent",
            ExternalEvent::Data(ExternalPayload { note: "x".into() }),
        ),
        known(
            "Settings",
            Settings {
                nickname: None,
                maybe: None,
            },
        ),
        // A newer server added a variant carrying content.
        unknown(
            "ApiError",
            true,
            json!({"context": "maintenance", "data": {"retryAfterSeconds": 60}}),
        ),
        // A newer server added a unit variant.
        unknown("ApiError", true, json!({"context": "teapot"})),
        // A newer reason inside a known top-level variant.
        unknown(
            "ApiError",
            false,
            json!({"context": "auth", "data": {"reason": "accountSuspended", "params": {"cause": "remote"}}}),
        ),
        // A newer cause nested in a struct payload inside a known variant.
        unknown(
            "ApiError",
            false,
            json!({"context": "auth", "data": {"reason": "signInRemoved", "params": {"cause": "account_suspended"}}}),
        ),
        unknown(
            "AuthApiError",
            true,
            json!({"reason": "accountSuspended", "params": {"cause": "remote"}}),
        ),
        unknown("SignInRemovalCause", true, json!("account_suspended")),
        unknown("RouteSessionType", true, json!("futureMode")),
        unknown(
            "InternallyTagged",
            true,
            json!({"kind": "future", "payload": 1}),
        ),
        unknown("ExternalEvent", true, json!("exploded")),
        unknown("ExternalEvent", true, json!({"exploded": {"code": 9}})),
    ]
}

fn main() {
    let generated = Path::new(env!("CARGO_MANIFEST_DIR")).join("kotlin-typecheck/generated");
    std::fs::create_dir_all(&generated).unwrap();

    for (name, serialization, unknown_variants, plain_only) in [
        ("none", Serialization::None, false, false),
        ("none-unknown", Serialization::None, true, false),
        ("kotlinx", Serialization::Kotlinx, false, false),
        ("kotlinx-unknown", Serialization::Kotlinx, true, false),
        ("none-wide", Serialization::None, false, true),
    ] {
        let mut types = kotlinx_types();
        if plain_only {
            types = types
                .register::<UntaggedValue>()
                .register::<GenericEvent<String>>()
                .register::<TupleRecord>();
        }
        Kotlin::default()
            .package("dev.specta.kotlincheck")
            .serialization(serialization)
            .unknown_variants(unknown_variants)
            .export_to(
                generated.join(name).join("Bindings.kt"),
                &types,
                specta_serde::Format,
            )
            .unwrap();
    }

    let samples = serde_json::to_string_pretty(&samples()).unwrap();
    std::fs::write(generated.join("samples.json"), samples + "\n").unwrap();
}
